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

        // Build a map from (part, paragraph_idx, run_idx) to field info
        let coord_map = build_coordinate_field_map(fields);

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
    let mut occupied: HashMap<(String, usize, usize), Vec<(usize, usize)>> = HashMap::new();
    for field in fields {
        let targets: Vec<(&str, Option<usize>, Option<usize>)> = if is_marker_field(&field.field_type) {
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
            let node = index
                .parts
                .get(&part)
                .and_then(|part_index| {
                    part_index.nodes.iter().find(|node| {
                        node.paragraph_index == paragraph_index && node.run_index == run_index
                    })
                })
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

/// Detect existing content controls in the tree and reject if found inside marked areas
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
            "模板中存在 Word 自带的内容控件（{} 处）: {}。请先在 Word 中移除这些内容控件后重新保存。\n文件: {}",
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
        if name == "w:sdt" {
            paths.push(path);
            return; // don't descend into existing sdt to avoid double-counting
        }
        for child in children {
            find_sdt_elements(child, paths, path.clone());
        }
    }
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
    let mut run_idx = 0;
    let p_idx = cursor.0;
    let mut i = 0;

    while i < children.len() {
        let is_wr = matches!(&children[i], XmlNode::Element { name, .. } if name == "w:r");
        if !is_wr {
            i += 1;
            continue;
        }

        let key = (part.to_string(), p_idx, run_idx);
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
            run_idx += 1;
            cursor.1 = run_idx;
        }
    }

    // Recurse into children to find nested w:p (text boxes via w:drawing > w:txbxContent)
    for child in children.iter_mut() {
        find_nested_paragraphs(child, part, cursor, coord_map)?;
    }

    Ok(())
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
            if let XmlNode::Element { name, .. } = c {
                if name == "w:t" {
                    return true;
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
    fn detect_existing_sdt_rejects() {
        let xml = r#"<w:document><w:body><w:p>
            <w:sdt><w:sdtPr/><w:sdtContent><w:r><w:t>old</w:t></w:r></w:sdtContent></w:sdt>
        </w:p></w:body></w:document>"#;
        let tree = XmlTree::parse(xml.as_bytes()).unwrap();
        let err = detect_existing_sdt(&tree.root, "word/document.xml");
        assert!(err.is_err(), "should reject existing sdt: {:?}", err.err());
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
