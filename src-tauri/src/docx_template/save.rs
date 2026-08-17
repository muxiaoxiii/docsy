use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::docx_template::index::DocumentIndex;
use crate::docx_template::ooxml::{XmlNode, XmlTree};

use super::TemplateField;

/// Parse a coordinate-based mark_id: "{part}-p{paragraph}-r{run}"
fn parse_mark_coords(mark_id: &str) -> Option<(String, usize, usize)> {
    // Format: "part_name-p{paragraph_idx}-r{run_idx}"
    let (part, rest) = mark_id.rsplit_once("-p")?;
    let (p_str, r_str) = rest.split_once("-r")?;
    Some((part.to_string(), p_str.parse().ok()?, r_str.parse().ok()?))
}

/// Wrap marked runs in <w:sdt> content controls using coordinate-based matching
pub fn build_template_docx(
    package_xml: &[(String, Vec<u8>)],
    fields: &[TemplateField],
    index: &DocumentIndex,
) -> Result<Vec<(String, Vec<u8>)>> {
    // Do not alter authoring highlights until every selected target is proven
    // to exist in the source snapshot. A stale preview must fail safely rather
    // than produce a template containing unmarked sample text.
    validate_coordinate_targets(fields, index)?;
    let mut results = Vec::new();

    for (part_name, xml_bytes) in package_xml {
        if !super::is_word_xml_part(part_name) {
            results.push((part_name.clone(), xml_bytes.clone()));
            continue;
        }

        let mut tree =
            XmlTree::parse(xml_bytes).with_context(|| format!("解析 XML 失败: {part_name}"))?;

        // Check for existing content controls
        detect_existing_sdt(&tree.root, part_name)?;
        // Yellow is an authoring signal, never template content. Clear it from
        // every run, including marks the user chose to keep as ordinary text.
        strip_all_yellow_highlights(&mut tree.root);

        // Build a map from (part, paragraph_idx, run_idx) to field info.
        // Repair the truncated placeholder siblings produced by older ranged
        // re-save logic before walking coordinates, so saving an affected
        // library template permanently heals its package.
        let coord_map = build_coordinate_field_map(fields);
        super::render::remove_known_placeholder_fragments(
            &mut tree.root,
            coord_map
                .values()
                .flat_map(|entries| entries.iter().map(|entry| &entry.0)),
        );

        // Walk the tree and wrap runs at the specified coordinates
        wrap_runs_by_coordinates(&mut tree.root, part_name, &coord_map, &mut (0, 0))?;

        let out_xml = tree
            .to_xml()
            .with_context(|| format!("序列化 XML 失败: {part_name}"))?;
        results.push((part_name.clone(), out_xml.into_bytes()));
    }
    Ok(results)
}

fn validate_coordinate_targets(fields: &[TemplateField], index: &DocumentIndex) -> Result<()> {
    // Pre-index (part, paragraph, run) → node so per-mark lookup is O(1).
    let mut node_index: HashMap<(String, usize, usize), &super::index::TextNodeRef> =
        HashMap::new();
    for (part, part_index) in &index.parts {
        for node in &part_index.nodes {
            node_index.insert((part.clone(), node.paragraph_index, node.run_index), node);
        }
    }
    let mut occupied: HashMap<(String, usize, usize), Vec<(usize, usize)>> = HashMap::new();
    for field in fields {
        let targets: Vec<(&str, Option<usize>, Option<usize>)> =
            if is_marker_field(&field.field_type) {
                field
                    .options
                    .iter()
                    .map(|option| (option.marker_mark_id.as_str(), None, None))
                    .collect()
            } else if field.mark_refs.is_empty() {
                field
                    .marks
                    .iter()
                    .map(|mark| (mark.as_str(), None, None))
                    .collect()
            } else {
                field
                    .mark_refs
                    .iter()
                    .map(|mark| (mark.mark_id.as_str(), mark.start, mark.end))
                    .collect()
            };

        for (mark_id, start, end) in targets {
            let (part, paragraph_index, run_index) = parse_mark_coords(mark_id)
                .with_context(|| format!("字段“{}”包含无效标记坐标", field.label))?;
            let node = node_index
                .get(&(part.clone(), paragraph_index, run_index))
                .copied()
                .with_context(|| {
                    format!(
                        "字段“{}”的标记已不在源 Word 中。请重新读取 Word 后再保存模板",
                        field.label
                    )
                })?;
            let text_len = node.text.chars().count();
            let range = match (start, end) {
                (Some(start), Some(end)) if start < end && end <= text_len => (start, end),
                (None, None) => (0, text_len),
                _ => anyhow::bail!(
                    "字段“{}”的标记范围无效。请重新读取 Word 后再保存模板",
                    field.label
                ),
            };
            let key = (part, paragraph_index, run_index);
            let ranges = occupied.entry(key).or_default();
            if ranges
                .iter()
                .any(|(left, right)| range.0 < *right && *left < range.1)
            {
                anyhow::bail!("字段“{}”与其他字段使用了重叠的文本范围", field.label);
            }
            ranges.push(range);
        }
    }
    Ok(())
}

/// A field target within a run: tag, field_type, is_delete, (start, end) char range
type FieldTarget = (String, String, bool, Option<usize>, Option<usize>);

/// Build a map from (part, paragraph_idx, run_idx) to field targets
fn build_coordinate_field_map(
    fields: &[TemplateField],
) -> HashMap<(String, usize, usize), Vec<FieldTarget>> {
    let mut map: HashMap<_, Vec<_>> = HashMap::new();
    for field in fields {
        if is_marker_field(&field.field_type) {
            for option in &field.options {
                add_target(
                    &mut map,
                    &option.marker_mark_id,
                    option_marker_tag(field, option),
                    &field.field_type,
                    false,
                    None,
                    None,
                );
            }
            continue;
        }

        if field.mark_refs.is_empty() {
            for mark_id in &field.marks {
                add_target(
                    &mut map,
                    mark_id,
                    if field.field_type == "delete_text" {
                        format!("__delete_{}", field.id)
                    } else {
                        field.id.clone()
                    },
                    &field.field_type,
                    field.field_type == "delete_text",
                    None,
                    None,
                );
            }
            continue;
        }

        for (index, mark_ref) in field.mark_refs.iter().enumerate() {
            let tag = if field.field_type == "delete_text" {
                format!("__delete_{}", field.id)
            } else if mark_ref.tag.is_empty() {
                fallback_ref_tag(field, index)
            } else {
                mark_ref.tag.clone()
            };
            add_target(
                &mut map,
                &mark_ref.mark_id,
                tag,
                &field.field_type,
                field.field_type == "delete_text",
                mark_ref.start,
                mark_ref.end,
            );
        }
    }
    map
}

fn add_target(
    map: &mut HashMap<(String, usize, usize), Vec<FieldTarget>>,
    mark_id: &str,
    tag: String,
    field_type: &str,
    is_delete: bool,
    start: Option<usize>,
    end: Option<usize>,
) {
    if mark_id.is_empty() {
        return;
    }
    if let Some((part, p_idx, r_idx)) = parse_mark_coords(mark_id) {
        map.entry((part, p_idx, r_idx)).or_default().push((
            tag,
            field_type.to_string(),
            is_delete,
            start,
            end,
        ));
    }
}

fn fallback_ref_tag(field: &TemplateField, index: usize) -> String {
    if field.mark_refs.len() > 1 {
        format!("{}.ref.{}", field.id, index + 1)
    } else {
        field.id.clone()
    }
}

fn option_marker_tag(field: &TemplateField, option: &super::TemplateOption) -> String {
    if option.marker_tag.is_empty() {
        format!("{}.option.{}", field.id, option.id)
    } else {
        option.marker_tag.clone()
    }
}

fn is_marker_field(field_type: &str) -> bool {
    matches!(field_type, "checkbox" | "radio_group" | "checkbox_group")
}

/// Detect content controls overlapping the yellow marks and reject them.
/// Ordinary Word content controls outside marked areas are left untouched.
fn detect_existing_sdt(root: &XmlNode, part_name: &str) -> Result<()> {
    let mut sdt_paths = Vec::new();
    find_sdt_elements(root, &mut sdt_paths, Vec::new());
    if !sdt_paths.is_empty() {
        let paths = sdt_paths
            .iter()
            .take(3)
            .map(|p| p.join(" > "))
            .collect::<Vec<_>>()
            .join("; ");
        anyhow::bail!(
            "模板的标黄区域中包含 Word 自带的内容控件（{} 处）: {}。请先在 Word 中移除这些内容控件后重新保存。\n文件: {}",
            sdt_paths.len(),
            paths,
            part_name
        );
    }
    Ok(())
}

fn find_sdt_elements(node: &XmlNode, paths: &mut Vec<Vec<String>>, current: Vec<String>) {
    if let XmlNode::Element { name, children, .. } = node {
        let mut path = current.clone();
        path.push(name.clone());
        if name == "w:sdt" && subtree_has_yellow(children) {
            paths.push(path.clone());
        }
        for child in children {
            find_sdt_elements(child, paths, path.clone());
        }
    }
}

fn subtree_has_yellow(nodes: &[XmlNode]) -> bool {
    for node in nodes {
        if let XmlNode::Element {
            name,
            attrs,
            children,
        } = node
        {
            if name == "w:highlight" {
                let is_yellow = attrs.iter().any(|(k, v)| {
                    k == "w:val" && (v == "yellow" || v == "'yellow'" || v == "\"yellow\"")
                });
                if is_yellow {
                    return true;
                }
            }
            if subtree_has_yellow(children) {
                return true;
            }
        }
    }
    false
}

/// Walk the tree and wrap runs at specified coordinates.
/// The traversal deliberately mirrors `scan_document_index`: every paragraph
/// is visited once in document order, including paragraphs nested in text
/// boxes and tables. Keeping one walker prevents saved coordinates drifting
/// from the coordinates shown during template inspection.
fn wrap_runs_by_coordinates(
    node: &mut XmlNode,
    part: &str,
    coord_map: &HashMap<(String, usize, usize), Vec<FieldTarget>>,
    cursor: &mut (usize, usize),
) -> Result<()> {
    if let XmlNode::Element { name, children, .. } = node {
        if name == "w:p" {
            cursor.1 = 0;
            wrap_paragraph_runs(children, part, cursor, coord_map)?;
            cursor.0 += 1;
        } else {
            for child in children.iter_mut() {
                wrap_runs_by_coordinates(child, part, coord_map, cursor)?;
            }
        }
    }
    Ok(())
}

fn wrap_paragraph_runs(
    children: &mut Vec<XmlNode>,
    part: &str,
    cursor: &mut (usize, usize),
    coord_map: &HashMap<(String, usize, usize), Vec<FieldTarget>>,
) -> Result<()> {
    let p_idx = cursor.0;
    let mut i = 0;
    // Word field depth tracking: begin increments, end decrements.
    // Runs inside the field code region (begin → separate) are skipped
    // because they are structural (fldChar, instrText), not user content.
    // See MDG-012 and ECMA-376 §17.16.
    let mut field_depth: u32 = 0;
    let mut in_field_code = false;

    while i < children.len() {
        let is_wr = matches!(&children[i], XmlNode::Element { name, .. } if name == "w:r");
        if !is_wr {
            // 库模板“编辑再保存”时，document.xml 里已有上次保存写入的 w:sdt。
            // 已保存模板的 run 内容已被匿名化为 {{field_id}}。不能再按原文
            // start/end 切割这段占位符，否则会把开头的 "{{" 包成新 sdt，并把
            // "field_id}}" 留在文档中。对单 run、单目标的已有 sdt 直接刷新标识。
            if matches!(&children[i], XmlNode::Element { name, .. } if name == "w:sdt")
                && sdt_hits_target(&children[i], part, p_idx, cursor.1, coord_map)
            {
                let run_count = text_run_count(&children[i]);
                let key = (part.to_string(), p_idx, cursor.1);
                if run_count == 1 {
                    if let Some(entries) = coord_map.get(&key) {
                        if entries.len() == 1 {
                            let (tag, _field_type, is_delete, _start, _end) = &entries[0];
                            if *is_delete {
                                children[i] = XmlNode::Text(String::new());
                            } else {
                                refresh_existing_sdt(&mut children[i], tag);
                                i += 1;
                            }
                            cursor.1 += 1;
                            continue;
                        }
                    }
                }

                // 复杂的旧 sdt 结构无法从匿名占位符反推原文范围，此时保留
                // 旧行为，让后续坐标验证或渲染检查给出明确错误。
                let sdt = std::mem::replace(&mut children[i], XmlNode::Text(String::new()));
                let content = sdt_content_children(sdt);
                children.splice(i..i + 1, content);
                continue; // 不推进 i：提升上来的节点按普通兄弟节点继续处理
            }
            // Nested containers whose runs participate in coordinates must be
            // traversed exactly like scan does (w:sdt, w:sdtContent, w:hyperlink).
            if matches!(&children[i], XmlNode::Element { name, .. } if name == "w:sdt" || name == "w:sdtContent" || name == "w:hyperlink")
            {
                if let XmlNode::Element { children: sub, .. } = &mut children[i] {
                    wrap_paragraph_runs(sub, part, cursor, coord_map)?;
                }
            }
            i += 1;
            continue;
        }

        // Detect Word field boundaries (MDG-012)
        if let Some(fld_type) = fldchar_type(&children[i]) {
            match fld_type {
                "begin" => {
                    field_depth += 1;
                    in_field_code = true;
                }
                "separate" => {
                    in_field_code = false;
                }
                "end" => {
                    field_depth = field_depth.saturating_sub(1);
                    if field_depth == 0 {
                        in_field_code = false;
                    }
                }
                _ => {}
            }
            // fldChar runs are structural — never wrap as SDT
            i += 1;
            continue;
        }

        // Skip runs inside field code region (instrText etc.)
        if field_depth > 0 && in_field_code {
            i += 1;
            continue;
        }

        let key = (part.to_string(), p_idx, cursor.1);
        // Pre-check: record whether this run has text BEFORE modification
        let had_text = run_has_text(&children[i]);
        let mut inserted = 0u32;

        if let Some(field_entries) = coord_map.get(&key) {
            if !field_entries.is_empty() {
                // Process all field entries for this run, sorted by start offset
                let mut entries: Vec<&FieldTarget> = field_entries.iter().collect();
                entries.sort_by_key(|e| e.3.unwrap_or(0));

                if entries.len() == 1 {
                    // Fast path: single entry
                    let (tag, _field_type, is_delete, start, end) = entries[0].clone();
                    if is_delete {
                        if let (Some(s), Some(e)) = (start, end) {
                            delete_text_range(&mut children[i], s, e)?;
                        } else if run_has_non_text_payload(&children[i]) {
                            // 含 w:drawing/w:pict 等非文本负载的 run 不能整体
                            // 删除——清掉 run 会把嵌入对象一起抹掉。与部分删除
                            // （delete_text_range）的口径一致：不删，保留原样。
                        } else {
                            children[i] = XmlNode::Text(String::new());
                        }
                    } else {
                        strip_yellow_highlight(&mut children[i]);
                        if let (Some(s), Some(e)) = (start, end) {
                            let (prefix_run, sdt_node, suffix_run) =
                                split_run_for_range(&mut children[i], &tag, s, e)?;
                            let mut replacements = Vec::new();
                            if let Some(p) = prefix_run {
                                replacements.push(p);
                            }
                            replacements.push(sdt_node);
                            if let Some(s) = suffix_run {
                                replacements.push(s);
                            }
                            let count = replacements.len();
                            children.splice(i..i + 1, replacements);
                            inserted = count as u32;
                        } else {
                            let original =
                                std::mem::replace(&mut children[i], XmlNode::Text(String::new()));
                            children[i] = wrap_as_sdt(original, &tag);
                        }
                    }
                } else {
                    // Multi-field: take out the run first to avoid borrow conflict
                    let mut run_node =
                        std::mem::replace(&mut children[i], XmlNode::Text(String::new()));
                    let replacements = process_multi_field_run(&mut run_node, &entries)?;
                    let count = replacements.len();
                    children.splice(i..i + 1, replacements);
                    // The replacements are derived from one source run. Skip all of
                    // them so only the next source run advances run_idx; otherwise a
                    // remaining prefix/suffix run is counted again and every later
                    // coordinate in the paragraph shifts left.
                    inserted = count as u32;
                }
            }
        }

        if inserted > 0 {
            i += inserted as usize;
        } else {
            i += 1;
        }
        // Use pre-modification check so deletion/wrapping doesn't shift coordinates
        if had_text {
            cursor.1 += 1;
        }
    }

    // Recurse into children to find nested w:p (text boxes via w:drawing > w:txbxContent)
    for child in children.iter_mut() {
        find_nested_paragraphs(child, part, cursor, coord_map)?;
    }

    Ok(())
}

fn text_run_count(node: &XmlNode) -> usize {
    match node {
        XmlNode::Element { name, children, .. } if name == "w:r" => usize::from(run_has_text(node)),
        XmlNode::Element { children, .. } => children.iter().map(text_run_count).sum(),
        XmlNode::Text(_) => 0,
    }
}

/// Keep an already-authored Docsy content control in place when a library
/// template is edited and saved again. Only its stable tag and anonymous text
/// are refreshed; the original run properties and surrounding suffix remain.
fn refresh_existing_sdt(sdt: &mut XmlNode, tag: &str) {
    let placeholder = format!("{{{{{tag}}}}}");
    let mut wrote_placeholder = false;
    refresh_existing_sdt_node(sdt, tag, &placeholder, &mut wrote_placeholder);
}

fn refresh_existing_sdt_node(
    node: &mut XmlNode,
    tag: &str,
    placeholder: &str,
    wrote_placeholder: &mut bool,
) {
    let XmlNode::Element {
        name,
        attrs,
        children,
    } = node
    else {
        return;
    };

    if name == "w:tag" {
        if let Some((_, value)) = attrs.iter_mut().find(|(key, _)| key == "w:val") {
            *value = tag.to_string();
        } else {
            attrs.push(("w:val".to_string(), tag.to_string()));
        }
    }

    if name == "w:rPr" {
        children.retain(|child| !is_yellow_highlight_elem(child));
    }

    if name == "w:t" {
        let value = if *wrote_placeholder {
            String::new()
        } else {
            *wrote_placeholder = true;
            placeholder.to_string()
        };
        children.clear();
        children.push(XmlNode::Text(value));
        return;
    }

    for child in children {
        refresh_existing_sdt_node(child, tag, placeholder, wrote_placeholder);
    }
}

fn find_nested_paragraphs(
    node: &mut XmlNode,
    part: &str,
    cursor: &mut (usize, usize),
    coord_map: &HashMap<(String, usize, usize), Vec<FieldTarget>>,
) -> Result<()> {
    if let XmlNode::Element { name, children, .. } = node {
        if name == "w:p" {
            cursor.1 = 0;
            wrap_paragraph_runs(children, part, cursor, coord_map)?;
            cursor.0 += 1;
        } else {
            for child in children.iter_mut() {
                find_nested_paragraphs(child, part, cursor, coord_map)?;
            }
        }
    }
    Ok(())
}

/// 取出 w:sdt 内 w:sdtContent 的子节点：只去外壳，内容 run 原样保留。
fn sdt_content_children(sdt: XmlNode) -> Vec<XmlNode> {
    if let XmlNode::Element { children, .. } = sdt {
        for child in children {
            if let XmlNode::Element { name, children, .. } = child {
                if name == "w:sdtContent" {
                    return children;
                }
            }
        }
    }
    Vec::new()
}

/// 判断 sdt 子树内是否有 run 命中目标坐标。run 计数方式与 scan/wrap 完全
/// 一致（w:sdt/w:sdtContent/w:hyperlink 透明；fldChar 与域代码区不计）。
fn sdt_hits_target(
    sdt: &XmlNode,
    part: &str,
    p_idx: usize,
    run_idx: usize,
    coord_map: &HashMap<(String, usize, usize), Vec<FieldTarget>>,
) -> bool {
    let mut idx = run_idx;
    let mut hit = false;
    if let XmlNode::Element { children, .. } = sdt {
        count_target_runs(children, part, p_idx, &mut idx, coord_map, &mut hit);
    }
    hit
}

fn count_target_runs(
    children: &[XmlNode],
    part: &str,
    p_idx: usize,
    idx: &mut usize,
    coord_map: &HashMap<(String, usize, usize), Vec<FieldTarget>>,
    hit: &mut bool,
) {
    let mut field_depth: u32 = 0;
    let mut in_field_code = false;
    for child in children {
        let XmlNode::Element {
            name,
            children: sub,
            ..
        } = child
        else {
            continue;
        };
        if name != "w:r" {
            if name == "w:sdt" || name == "w:sdtContent" || name == "w:hyperlink" {
                count_target_runs(sub, part, p_idx, idx, coord_map, hit);
            }
            continue;
        }
        if let Some(fld_type) = fldchar_type(child) {
            match fld_type {
                "begin" => {
                    field_depth += 1;
                    in_field_code = true;
                }
                "separate" => in_field_code = false,
                "end" => {
                    field_depth = field_depth.saturating_sub(1);
                    if field_depth == 0 {
                        in_field_code = false;
                    }
                }
                _ => {}
            }
            continue;
        }
        if field_depth > 0 && in_field_code {
            continue;
        }
        if run_has_text(child) {
            if coord_map.contains_key(&(part.to_string(), p_idx, *idx)) {
                *hit = true;
            }
            *idx += 1;
        }
    }
}

fn strip_yellow_highlight(node: &mut XmlNode) {
    if let XmlNode::Element { children, .. } = node {
        for child in children.iter_mut() {
            if let XmlNode::Element {
                name,
                children: rpr_children,
                ..
            } = child
            {
                if name == "w:rPr" {
                    rpr_children.retain(|c| !is_yellow_highlight_elem(c));
                    return;
                }
            }
        }
    }
}

fn strip_all_yellow_highlights(node: &mut XmlNode) {
    if let XmlNode::Element { children, .. } = node {
        for child in children {
            strip_yellow_highlight(child);
            strip_all_yellow_highlights(child);
        }
    }
}

fn is_yellow_highlight_elem(node: &XmlNode) -> bool {
    if let XmlNode::Element { name, attrs, .. } = node {
        name == "w:highlight"
            && attrs.iter().any(|(k, v)| {
                k == "w:val" && (v == "yellow" || v == "'yellow'" || v == "\"yellow\"")
            })
    } else {
        false
    }
}

/// Check if a w:r contains a w:fldChar and return its type ("begin"/"separate"/"end").
/// Returns None for runs without fldChar.
fn fldchar_type(node: &XmlNode) -> Option<&'static str> {
    if let XmlNode::Element { children, .. } = node {
        for child in children {
            if let XmlNode::Element { name, attrs, .. } = child {
                if name == "w:fldChar" {
                    for (k, v) in attrs {
                        if k == "w:fldCharType" {
                            return match v.as_str() {
                                "begin" => Some("begin"),
                                "separate" => Some("separate"),
                                "end" => Some("end"),
                                _ => None,
                            };
                        }
                    }
                }
            }
        }
    }
    None
}

fn wrap_as_sdt(run: XmlNode, tag: &str) -> XmlNode {
    let run = anonymize_run_text(run, &format!("{{{{{tag}}}}}"));
    XmlNode::Element {
        name: "w:sdt".to_string(),
        attrs: Vec::new(),
        children: vec![
            XmlNode::Element {
                name: "w:sdtPr".to_string(),
                attrs: Vec::new(),
                children: vec![XmlNode::Element {
                    name: "w:tag".to_string(),
                    attrs: vec![("w:val".to_string(), tag.to_string())],
                    children: Vec::new(),
                }],
            },
            XmlNode::Element {
                name: "w:sdtContent".to_string(),
                attrs: Vec::new(),
                children: vec![run],
            },
        ],
    }
}

fn anonymize_run_text(mut run: XmlNode, placeholder: &str) -> XmlNode {
    let mut wrote_placeholder = false;
    if let XmlNode::Element { children, .. } = &mut run {
        for child in children {
            if let XmlNode::Element {
                name,
                children: text_children,
                ..
            } = child
            {
                if name != "w:t" {
                    continue;
                }
                for text in text_children {
                    if let XmlNode::Text(value) = text {
                        if !wrote_placeholder {
                            *value = placeholder.to_string();
                            wrote_placeholder = true;
                        } else {
                            value.clear();
                        }
                    }
                }
            }
        }
    }
    run
}

/// Split a w:r element by character range.
/// Returns (prefix_run, sdt_wrapped_middle, suffix_run).
/// prefix_run: the modified run with prefix text, or None if no prefix
/// sdt_wrapped_middle: the middle portion wrapped in <w:sdt>
/// suffix_run: the modified run with suffix text, or None if no suffix
fn split_run_for_range(
    run: &mut XmlNode,
    tag: &str,
    start: usize,
    end: usize,
) -> Result<(Option<XmlNode>, XmlNode, Option<XmlNode>)> {
    let run_clone = run.clone();
    let chars: Vec<char> = collect_text_from_run(&run_clone).chars().collect();
    let start_idx = start.min(chars.len());
    let end_idx = end.min(chars.len()).max(start_idx);
    let prefix: String = chars[..start_idx].iter().collect();
    let middle: String = chars[start_idx..end_idx].iter().collect();
    let suffix_text: String = chars[end_idx..].iter().collect();

    if (!prefix.is_empty() || !suffix_text.is_empty()) && run_has_non_text_payload(&run_clone) {
        anyhow::bail!(
            "字段不能只覆盖同时包含换行、制表符或嵌入对象的部分文本；请在 Word 中把该字段拆成独立文本后重新标黄"
        );
    }

    let prefix_run = (!prefix.is_empty()).then(|| {
        let mut value = make_run_with_text(&run_clone, &prefix);
        strip_yellow_highlight(&mut value);
        value
    });
    let suffix_run = (!suffix_text.is_empty()).then(|| {
        let mut value = make_run_with_text(&run_clone, &suffix_text);
        strip_yellow_highlight(&mut value);
        value
    });
    let sdt = if middle.is_empty() {
        XmlNode::Text(String::new())
    } else {
        wrap_as_sdt(make_run_with_text(&run_clone, &middle), tag)
    };
    *run = XmlNode::Text(String::new());
    Ok((prefix_run, sdt, suffix_run))
}

/// Delete a character range within a w:r element's text
fn delete_text_range(run: &mut XmlNode, start: usize, end: usize) -> Result<()> {
    let original = run.clone();
    let chars: Vec<char> = collect_text_from_run(&original).chars().collect();
    let start_idx = start.min(chars.len());
    let end_idx = end.min(chars.len()).max(start_idx);
    if (start_idx > 0 || end_idx < chars.len()) && run_has_non_text_payload(&original) {
        anyhow::bail!(
            "删除范围不能只覆盖同时包含换行、制表符或嵌入对象的部分文本；请在 Word 中先拆分该文本"
        );
    }
    let text = format!(
        "{}{}",
        chars[..start_idx].iter().collect::<String>(),
        chars[end_idx..].iter().collect::<String>(),
    );
    *run = make_run_with_text(&original, &text);
    Ok(())
}

/// A partial split may clone this run several times. Only plain text runs are
/// safe to clone: copying a tab, break, drawing or field node would duplicate
/// document content and change the layout.
fn run_has_non_text_payload(run: &XmlNode) -> bool {
    matches!(run, XmlNode::Element { children, .. } if children.iter().any(|child| {
        !matches!(child, XmlNode::Element { name, .. } if name == "w:rPr" || name == "w:t")
    }))
}

/// Clone a w:r element preserving its structure, replacing text with content
fn make_run_with_text(run: &XmlNode, text: &str) -> XmlNode {
    if let XmlNode::Element {
        name,
        attrs,
        children,
    } = run
    {
        let mut new_children = Vec::new();
        let mut wrote_text = false;
        for child in children {
            match child {
                XmlNode::Element {
                    name,
                    attrs,
                    children,
                } if name == "w:t" => {
                    let value = if wrote_text {
                        String::new()
                    } else {
                        text.to_string()
                    };
                    wrote_text = true;
                    new_children.push(XmlNode::Element {
                        name: name.clone(),
                        attrs: attrs.clone(),
                        children: vec![XmlNode::Text(value)],
                    });
                }
                XmlNode::Element {
                    name: cn,
                    attrs,
                    children,
                } if cn == "w:rPr" => {
                    new_children.push(XmlNode::Element {
                        name: cn.clone(),
                        attrs: attrs.clone(),
                        children: children.clone(),
                    });
                }
                other => {
                    new_children.push(other.clone());
                }
            }
        }
        XmlNode::Element {
            name: name.clone(),
            attrs: attrs.clone(),
            children: new_children,
        }
    } else {
        run.clone()
    }
}

fn run_has_text(node: &XmlNode) -> bool {
    if let XmlNode::Element { children, .. } = node {
        for c in children {
            if let XmlNode::Element {
                name,
                children: text_children,
                ..
            } = c
            {
                if name == "w:t" {
                    // Match scan.rs behavior: only count runs with non-empty text content.
                    // Empty <w:t></w:t> elements (generated by WPS/部分工具) must not
                    // advance the run index, otherwise scan and save coordinates diverge.
                    let has_content = text_children
                        .iter()
                        .any(|tc| matches!(tc, XmlNode::Text(s) if !s.is_empty()));
                    if has_content {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn process_multi_field_run(run: &mut XmlNode, entries: &[&FieldTarget]) -> Result<Vec<XmlNode>> {
    if entries.is_empty() {
        return Ok(vec![XmlNode::Text(String::new())]);
    }

    let original_text = collect_text_from_run(run);
    let chars: Vec<char> = original_text.chars().collect();
    let run_clone = run.clone();
    if run_has_non_text_payload(&run_clone) {
        anyhow::bail!(
            "同一文本 run 内不能同时保存多个字段，因为该 run 含有非文本内容；请在 Word 中拆分文本后重新标黄"
        );
    }

    // Build parts: for each entry, extract [start..end] and keep remaining text around it
    let mut result: Vec<XmlNode> = Vec::new();
    let mut cursor = 0usize;

    for entry in entries {
        let (tag, _ft, is_delete, start, end) = (*entry).clone();
        let s = start.unwrap_or(0).min(chars.len());
        let e = end.unwrap_or(chars.len()).min(chars.len()).max(s);

        // Text before this entry's range
        if cursor < s {
            let prefix_text: String = chars[cursor..s].iter().collect();
            if !prefix_text.is_empty() {
                let mut pr = make_run_with_text(&run_clone, &prefix_text);
                strip_yellow_highlight(&mut pr);
                result.push(pr);
            }
        }

        // The entry's range text
        let field_text: String = chars[s..e].iter().collect();
        if !field_text.is_empty() {
            if is_delete {
                // Skip — deleted text produces no output
            } else {
                result.push(wrap_as_sdt(
                    make_run_with_text(&run_clone, &field_text),
                    &tag,
                ));
            }
        }

        cursor = e;
    }

    // Remaining text after last entry
    if cursor < chars.len() {
        let remaining: String = chars[cursor..].iter().collect();
        if !remaining.is_empty() {
            let mut sr = make_run_with_text(&run_clone, &remaining);
            strip_yellow_highlight(&mut sr);
            result.push(sr);
        }
    }

    if result.is_empty() {
        result.push(XmlNode::Text(String::new()));
    }
    Ok(result)
}

fn collect_text_from_run(run: &XmlNode) -> String {
    if let XmlNode::Element { children, .. } = run {
        return children
            .iter()
            .filter_map(|child| match child {
                XmlNode::Element { name, children, .. } if name == "w:t" => Some(
                    children
                        .iter()
                        .filter_map(|text| match text {
                            XmlNode::Text(value) => Some(value.as_str()),
                            _ => None,
                        })
                        .collect::<String>(),
                ),
                _ => None,
            })
            .collect();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mark_coords_valid() {
        let result = parse_mark_coords("word/document.xml-p0-r3").unwrap();
        assert_eq!(result.0, "word/document.xml");
        assert_eq!(result.1, 0);
        assert_eq!(result.2, 3);
    }

    #[test]
    fn parse_mark_coords_invalid() {
        assert!(parse_mark_coords("plain-string").is_none());
        assert!(parse_mark_coords("word/document.xml-pX-r3").is_none());
    }

    #[test]
    fn detect_existing_sdt_rejects_only_yellow_marked() {
        // sdt without yellow marks is an ordinary content control — allowed
        let plain = r#"<w:document><w:body><w:p>
            <w:sdt><w:sdtPr/><w:sdtContent><w:r><w:t>old</w:t></w:r></w:sdtContent></w:sdt>
        </w:p></w:body></w:document>"#;
        let tree = XmlTree::parse(plain.as_bytes()).unwrap();
        assert!(detect_existing_sdt(&tree.root, "word/document.xml").is_ok());

        // sdt inside a yellow-marked area must be rejected
        let marked = r#"<w:document><w:body><w:p>
            <w:sdt><w:sdtPr/><w:sdtContent><w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>old</w:t></w:r></w:sdtContent></w:sdt>
        </w:p></w:body></w:document>"#;
        let tree = XmlTree::parse(marked.as_bytes()).unwrap();
        assert!(detect_existing_sdt(&tree.root, "word/document.xml").is_err());
    }

    #[test]
    fn detect_no_sdt_passes() {
        let xml = r#"<w:document><w:body><w:p>
            <w:r><w:t>plain</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let tree = XmlTree::parse(xml.as_bytes()).unwrap();
        assert!(detect_existing_sdt(&tree.root, "word/document.xml").is_ok());
    }

    #[test]
    fn whole_run_delete_keeps_runs_with_non_text_payload() {
        // 整 run 删除遇到含 w:drawing 的 run 时保留原样，不抹掉嵌入对象
        let mut tree = XmlTree::parse(
            r#"<w:p><w:r><w:t>删除我</w:t><w:drawing/></w:r><w:r><w:t>纯文本</w:t></w:r></w:p>"#
                .as_bytes(),
        )
        .unwrap();
        let mut targets = HashMap::new();
        targets.insert(
            ("word/document.xml".to_string(), 0, 0),
            vec![(
                "__delete_d1".to_string(),
                "delete_text".to_string(),
                true,
                None,
                None,
            )],
        );
        targets.insert(
            ("word/document.xml".to_string(), 0, 1),
            vec![(
                "__delete_d2".to_string(),
                "delete_text".to_string(),
                true,
                None,
                None,
            )],
        );

        wrap_runs_by_coordinates(&mut tree.root, "word/document.xml", &targets, &mut (0, 0))
            .unwrap();
        let xml = tree.to_xml().unwrap();
        assert!(
            xml.contains("<w:drawing") && xml.contains("删除我"),
            "含非文本负载的 run 不删: {xml}"
        );
        assert!(!xml.contains("纯文本"), "纯文本 run 正常删除: {xml}");
    }

    #[test]
    fn resave_unwraps_previous_sdt_instead_of_nesting() {
        // 模拟“编辑库模板再保存”：第一次保存产物的 document.xml 已含 w:sdt，
        // 第二次保存必须先去掉旧壳再包新壳，不能嵌套。
        let xml = r#"<w:document><w:body><w:p>
            <w:r><w:rPr><w:highlight w:val="yellow"/></w:rPr><w:t>张三</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let fields = vec![TemplateField {
            id: "name".to_string(),
            name: "姓名".to_string(),
            label: "姓名".to_string(),
            field_type: "text".to_string(),
            marks: vec!["word/document.xml-p0-r0".to_string()],
            ..Default::default()
        }];
        let parts = vec![("word/document.xml".to_string(), xml.as_bytes().to_vec())];

        let scan_parts: Vec<(&str, &[u8])> = parts
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let index =
            crate::docx_template::scan::scan_package_index_to_document_index(&scan_parts).unwrap();
        let first = build_template_docx(&parts, &fields, &index).unwrap();
        let first_xml = String::from_utf8(first[0].1.clone()).unwrap();
        assert!(first_xml.contains("<w:sdt>"), "first save wraps sdt");

        // 第二次保存：源已是含 sdt 的上次产物（库模板编辑路径）
        let scan_parts2: Vec<(&str, &[u8])> = first
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let index2 =
            crate::docx_template::scan::scan_package_index_to_document_index(&scan_parts2).unwrap();
        let second = build_template_docx(&first, &fields, &index2).unwrap();
        let second_xml = String::from_utf8(second[0].1.clone()).unwrap();
        assert_eq!(
            second_xml.matches("<w:sdt>").count(),
            1,
            "再保存不能产生嵌套 sdt: {second_xml}"
        );

        // 渲染后字段必须有值（嵌套 sdt 时渲染端只认 w:r，字段会渲染为空）
        let manifest = crate::docx_template::TemplateManifest {
            format_version: 2,
            template: crate::docx_template::TemplateMeta {
                id: "t0".to_string(),
                name: "test".to_string(),
                created: String::new(),
                updated: String::new(),
            },
            fields,
            filename_template: None,
        };
        let mut values = HashMap::new();
        values.insert(
            "name".to_string(),
            serde_json::Value::String("李四".to_string()),
        );
        let rendered = crate::docx_template::render::render_docx(
            &second,
            &manifest,
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();
        let rendered_xml = String::from_utf8(rendered[0].1.clone()).unwrap();
        assert!(
            rendered_xml.contains("李四"),
            "再保存产物渲染后字段为空: {rendered_xml}"
        );
    }

    #[test]
    fn resave_preserves_ranged_sdt_without_splitting_anonymous_placeholder() {
        let xml = r#"<w:document><w:body><w:p>
            <w:r><w:rPr><w:highlight w:val="yellow"/><w:u w:val="single"/></w:rPr><w:t>吕晗律师</w:t></w:r>
        </w:p></w:body></w:document>"#;
        let fields = vec![TemplateField {
            id: "lawyer".to_string(),
            name: "受托人".to_string(),
            label: "受托人".to_string(),
            field_type: "text".to_string(),
            mark_refs: vec![crate::docx_template::TemplateMarkRef {
                mark_id: "word/document.xml-p0-r0".to_string(),
                start: Some(0),
                end: Some(2),
                tag: "lawyer.ref.1".to_string(),
                optional_rule: None,
            }],
            ..Default::default()
        }];
        let parts = vec![("word/document.xml".to_string(), xml.as_bytes().to_vec())];

        let first_index = crate::docx_template::scan::scan_package_index_to_document_index(&[(
            parts[0].0.as_str(),
            parts[0].1.as_slice(),
        )])
        .unwrap();
        let first = build_template_docx(&parts, &fields, &first_index).unwrap();
        let first_xml = String::from_utf8(first[0].1.clone()).unwrap();
        assert!(first_xml.contains("{{lawyer.ref.1}}"));
        assert!(first_xml.contains("律师"));

        let second_index = crate::docx_template::scan::scan_package_index_to_document_index(&[(
            first[0].0.as_str(),
            first[0].1.as_slice(),
        )])
        .unwrap();
        let second = build_template_docx(&first, &fields, &second_index).unwrap();
        let second_xml = String::from_utf8(second[0].1.clone()).unwrap();

        assert_eq!(second_xml.matches("{{lawyer.ref.1}}").count(), 1);
        assert!(!second_xml.contains(">lawyer.ref.1}}</w:t>"));
        assert!(
            second_xml.contains("律师"),
            "静态后缀必须保留: {second_xml}"
        );

        let damaged_xml = first_xml.replacen(
            "</w:sdt>",
            "</w:sdt><w:r><w:t>lawyer.ref.1}}</w:t></w:r>",
            1,
        );
        let damaged = vec![("word/document.xml".to_string(), damaged_xml.into_bytes())];
        let damaged_index = crate::docx_template::scan::scan_package_index_to_document_index(&[(
            damaged[0].0.as_str(),
            damaged[0].1.as_slice(),
        )])
        .unwrap();
        let repaired = build_template_docx(&damaged, &fields, &damaged_index).unwrap();
        let repaired_xml = String::from_utf8(repaired[0].1.clone()).unwrap();
        assert_eq!(repaired_xml.matches("{{lawyer.ref.1}}").count(), 1);
        assert!(!repaired_xml.contains(">lawyer.ref.1}}</w:t>"));
    }

    #[test]
    fn multi_range_run_does_not_shift_later_source_run_coordinates() {
        let mut tree = XmlTree::parse(
            r#"<w:p><w:r><w:t>甲乙丙丁</w:t></w:r><w:r><w:t>后续字段</w:t></w:r></w:p>"#.as_bytes(),
        )
        .unwrap();
        let mut targets = HashMap::new();
        targets.insert(
            ("word/document.xml".to_string(), 0, 0),
            vec![
                (
                    "first".to_string(),
                    "text".to_string(),
                    false,
                    Some(0),
                    Some(1),
                ),
                (
                    "second".to_string(),
                    "text".to_string(),
                    false,
                    Some(2),
                    Some(3),
                ),
            ],
        );
        targets.insert(
            ("word/document.xml".to_string(), 0, 1),
            vec![("later".to_string(), "text".to_string(), false, None, None)],
        );

        wrap_runs_by_coordinates(&mut tree.root, "word/document.xml", &targets, &mut (0, 0))
            .unwrap();

        let xml = tree.to_xml().unwrap();
        assert!(xml.contains("w:val=\"first\""));
        assert!(xml.contains("w:val=\"second\""));
        assert!(xml.contains("w:val=\"later\""));
        assert!(xml.contains("{{later}}"));
        assert!(!xml.contains(">后续字段<"));
    }
}
