use anyhow::Result;
use std::collections::HashMap;

use serde_json::Value;

use crate::docx_template::ooxml::{XmlNode, XmlTree};

use super::{
    field_is_multiple, OptionalFieldRule, StructureOverride, TemplateField, TemplateManifest,
};

type TagTarget<'a> = (&'a TemplateField, Option<usize>);
type TagMap<'a> = HashMap<String, TagTarget<'a>>;

/// Render a template docx by replacing <w:sdt> content controls with values.
/// Preserves original run properties (w:rPr) for all field types.
/// Supports table row replication for party_list fields inside w:tr.
pub fn render_docx(
    package_xml: &[(String, Vec<u8>)],
    manifest: &TemplateManifest,
    values: &HashMap<String, Value>,
    structure_overrides: &HashMap<String, StructureOverride>,
    item_separator: &str,
) -> Result<Vec<(String, Vec<u8>)>> {
    let tag_map = build_tag_map(manifest);

    let mut results = Vec::new();
    for (part_name, xml_bytes) in package_xml {
        if !super::is_word_xml_part(part_name) {
            results.push((part_name.clone(), xml_bytes.clone()));
            continue;
        }

        let mut tree = XmlTree::parse(xml_bytes)?;
        // Clean legacy fragments before rendering as well: an orphan located
        // between the SDT and its static suffix would otherwise prevent the
        // affix replacement logic from finding that suffix.
        remove_known_placeholder_fragments(&mut tree.root, tag_map.keys());
        render_tree(
            &mut tree.root,
            &tag_map,
            values,
            structure_overrides,
            item_separator,
        )?;

        let out_xml = tree.to_xml()?;
        results.push((part_name.clone(), out_xml.into_bytes()));
    }
    Ok(results)
}

pub(super) fn remove_known_placeholder_fragments<'a>(
    node: &mut XmlNode,
    tags: impl Iterator<Item = &'a String>,
) {
    let mut tags = tags.map(String::as_str).collect::<Vec<_>>();
    tags.sort_unstable_by_key(|tag| std::cmp::Reverse(tag.len()));
    remove_known_placeholder_fragments_node(node, &tags, false);
}

fn remove_known_placeholder_fragments_node(node: &mut XmlNode, tags: &[&str], inside_sdt: bool) {
    match node {
        XmlNode::Text(value) if !inside_sdt => {
            for tag in tags {
                for fragment in [
                    format!("{{{{{tag}}}}}"),
                    format!("{tag}}}}}"),
                    format!("{{{{{tag}"),
                ] {
                    if value.contains(&fragment) {
                        *value = value.replace(&fragment, "");
                    }
                }
            }
        }
        XmlNode::Text(_) => {}
        XmlNode::Element { name, children, .. } => {
            let inside_sdt = inside_sdt || name == "w:sdt";
            for child in children {
                remove_known_placeholder_fragments_node(child, tags, inside_sdt);
            }
        }
    }
}

fn build_tag_map(manifest: &TemplateManifest) -> TagMap<'_> {
    let mut tags = HashMap::new();
    for field in &manifest.fields {
        if matches!(
            field.field_type.as_str(),
            "checkbox" | "radio_group" | "checkbox_group"
        ) {
            if field.options.is_empty() {
                tags.insert(field.id.clone(), (field, None));
            }
            for option in &field.options {
                let tag = if option.marker_tag.is_empty() {
                    format!("{}.option.{}", field.id, option.id)
                } else {
                    option.marker_tag.clone()
                };
                tags.insert(tag, (field, None));
            }
        } else if field.mark_refs.is_empty() {
            tags.insert(field.id.clone(), (field, None));
        } else {
            for (index, mark_ref) in field.mark_refs.iter().enumerate() {
                let tag = if mark_ref.tag.is_empty() {
                    if field.mark_refs.len() > 1 {
                        format!("{}.ref.{}", field.id, index + 1)
                    } else {
                        field.id.clone()
                    }
                } else {
                    mark_ref.tag.clone()
                };
                tags.insert(tag, (field, Some(index)));
            }
        }
    }
    tags
}

fn render_tree(
    node: &mut XmlNode,
    tag_map: &TagMap<'_>,
    values: &HashMap<String, Value>,
    overrides: &HashMap<String, StructureOverride>,
    item_separator: &str,
) -> Result<()> {
    if let XmlNode::Element { children, .. } = node {
        // Process children right-to-left so splice doesn't invalidate indices
        let mut i = children.len();
        while i > 0 {
            i -= 1;

            // Table row replication: if child is w:tr with party_list sdt, expand it
            if let XmlNode::Element { name, .. } = &children[i] {
                if name == "w:tr" {
                    if let Some(new_rows) = try_expand_table_row(
                        children,
                        i,
                        tag_map,
                        values,
                        overrides,
                        item_separator,
                    )? {
                        children.splice(i..i + 1, new_rows);
                        continue; // new rows already rendered by try_expand_table_row
                    }
                }
            }

            if let XmlNode::Element {
                name,
                children: sdt_children,
                ..
            } = &children[i]
            {
                if name == "w:sdt" {
                    if let Some(tag) = find_sdt_tag(sdt_children) {
                        if tag.starts_with("__delete_") {
                            children[i] = XmlNode::Text(String::new());
                            continue;
                        }

                        if let Some((field, slot)) = tag_map.get(&tag) {
                            let value = value_for_field(values, field, *slot)
                                .cloned()
                                .unwrap_or(Value::Null);
                            let field_separator =
                                effective_item_separator(field, overrides, item_separator);
                            let rendered =
                                rendered_base_text(field, *slot, &value, &field_separator);

                            if rendered.is_empty() {
                                if field_is_multiple(field) {
                                    // Only the list separator is removable here. A comma
                                    // separating legal roles remains unless it is part of
                                    // the explicit optional prefix below.
                                    strip_party_separator_before(children, i);
                                }
                                if let Some(rule) = optional_rule_for_slot(field, *slot) {
                                    if rule.enabled {
                                        strip_prefix_before(children, i, &rule.remove_empty_prefix);
                                        strip_suffix_after(children, i, &rule.remove_empty_suffix);
                                        if field_is_multiple(field)
                                            && is_party_role_prefix(&rule.remove_empty_prefix)
                                        {
                                            strip_role_separator_before(children, i);
                                        }
                                    }
                                }
                            } else {
                                let item_sdt_index = apply_multiple_item_affixes(
                                    children, i, field, *slot, &value, overrides,
                                );
                                // Reference 字段：仅当用户显式设置了覆盖前缀/后缀时才应用，
                                // 否则保留模板原文（因为引用值来自其他字段）
                                let is_reference = field.reference.is_some();
                                let has_explicit_override = overrides.get(&field.id).is_some();
                                let sdt_index = if is_reference && !has_explicit_override {
                                    item_sdt_index
                                } else {
                                    apply_structure_override(
                                        children,
                                        item_sdt_index,
                                        field,
                                        *slot,
                                        overrides,
                                    )
                                };
                                replace_sdt_content(
                                    &mut children[sdt_index],
                                    field,
                                    *slot,
                                    &tag,
                                    &value,
                                    &field_separator,
                                )?;
                                let sdt = std::mem::replace(
                                    &mut children[sdt_index],
                                    XmlNode::Text(String::new()),
                                );
                                let content = unwrap_sdt_content(sdt);
                                children.splice(sdt_index..sdt_index + 1, content);
                                continue;
                            }

                            replace_sdt_content(
                                &mut children[i],
                                field,
                                *slot,
                                &tag,
                                &value,
                                item_separator,
                            )?;
                            let sdt =
                                std::mem::replace(&mut children[i], XmlNode::Text(String::new()));
                            let content = unwrap_sdt_content(sdt);
                            children.splice(i..i + 1, content);
                            continue;
                        }
                    }
                }
            }

            // Standard recursive walk for non-content-control nodes.
            render_tree(&mut children[i], tag_map, values, overrides, item_separator)?;
        }
    }
    Ok(())
}

fn try_expand_table_row(
    children: &[XmlNode],
    idx: usize,
    tag_map: &TagMap<'_>,
    values: &HashMap<String, Value>,
    overrides: &HashMap<String, StructureOverride>,
    item_separator: &str,
) -> Result<Option<Vec<XmlNode>>> {
    let row_node = &children[idx];
    if !matches!(row_node, XmlNode::Element { name, .. } if name == "w:tr") {
        return Ok(None);
    }

    // 行内所有条目数 > 1 的 party_list 都参与展开（笛卡尔积），
    // 不再只处理第一个而让其余退化为单行内联文本
    let expanded = expand_party_rows(row_node, tag_map, values, 0)?;
    if expanded.len() <= 1 {
        return Ok(None);
    }
    let mut new_rows = Vec::with_capacity(expanded.len());
    for (mut row_clone, item_values) in expanded {
        // Clone the row subtree directly instead of a serialize →
        // parse round-trip per item.
        render_tree(
            &mut row_clone,
            tag_map,
            &item_values,
            overrides,
            item_separator,
        )?;
        new_rows.push(row_clone);
    }
    Ok(Some(new_rows))
}

/// 递归展开行内 party_list：每次取第一个仍可展开的字段，按条目克隆行并
/// 替换该字段的值后递归，直至行内不再有条目数 > 1 的 party_list。
/// 同一行出现多个 party_list 时得到笛卡尔积组合。
fn expand_party_rows(
    row: &XmlNode,
    tag_map: &TagMap<'_>,
    values: &HashMap<String, Value>,
    depth: usize,
) -> Result<Vec<(XmlNode, HashMap<String, Value>)>> {
    // 防御性上限：字段数有限，正常不会触达；防止异常数据导致组合爆炸
    const MAX_EXPAND_DEPTH: usize = 16;
    const MAX_EXPANDED_ROWS: usize = 512;
    if depth > MAX_EXPAND_DEPTH {
        anyhow::bail!("表格行内 party_list 展开层级过多，已中止渲染");
    }
    for tag in collect_sdt_tags_in_tree_simple(row) {
        let Some((field, _)) = tag_map.get(&tag) else {
            continue;
        };
        if !field_is_multiple(field) {
            continue;
        }
        let value = value_for_field(values, field, None)
            .cloned()
            .unwrap_or(Value::Null);
        let items = party_items(&value);
        if items.len() > 1 {
            let mut out = Vec::new();
            for item in &items {
                let mut item_values = values.clone();
                item_values.insert(
                    field.id.clone(),
                    serde_json::json!({
                        "prefix": item.prefix,
                        "text": item.text,
                        "suffix": item.suffix
                    }),
                );
                out.extend(expand_party_rows(row, tag_map, &item_values, depth + 1)?);
                if out.len() > MAX_EXPANDED_ROWS {
                    anyhow::bail!("表格行 party_list 展开行数超过限制，已中止渲染");
                }
            }
            return Ok(out);
        }
    }
    Ok(vec![(row.clone(), values.clone())])
}

fn collect_sdt_tags_in_tree_simple(node: &XmlNode) -> Vec<String> {
    if let XmlNode::Element { name, children, .. } = node {
        if name == "w:sdt" {
            return find_sdt_tag(children).into_iter().collect();
        }
        let mut tags = Vec::new();
        for c in children {
            tags.extend(collect_sdt_tags_in_tree_simple(c));
        }
        tags
    } else {
        Vec::new()
    }
}

fn find_sdt_tag(children: &[XmlNode]) -> Option<String> {
    for child in children {
        if let XmlNode::Element {
            name, children: cc, ..
        } = child
        {
            if name == "w:sdtPr" {
                for c in cc {
                    if let XmlNode::Element { name, attrs, .. } = c {
                        if name == "w:tag" {
                            return attrs
                                .iter()
                                .find(|(k, _)| k == "w:val")
                                .map(|(_, v)| v.clone());
                        }
                    }
                }
            }
        }
    }
    None
}

fn replace_sdt_content(
    sdt: &mut XmlNode,
    field: &TemplateField,
    slot: Option<usize>,
    tag: &str,
    value: &Value,
    item_separator: &str,
) -> Result<()> {
    if let XmlNode::Element { children, .. } = sdt {
        for child in children.iter_mut() {
            if let XmlNode::Element {
                name,
                children: content_children,
                ..
            } = child
            {
                if name != "w:sdtContent" {
                    continue;
                }

                // Preserve ALL run formatting: write rendered text into first w:r, clear rest
                let text = rendered_text_for_field(field, slot, tag, value, item_separator);
                let rendered = render_into_existing_runs(content_children, &text);
                *child = rendered;
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Get the rendered text for a field (without any XML wrapper)
fn rendered_text_for_field(
    field: &TemplateField,
    slot: Option<usize>,
    tag: &str,
    value: &Value,
    item_separator: &str,
) -> String {
    if matches!(
        field.field_type.as_str(),
        "checkbox" | "radio_group" | "checkbox_group"
    ) {
        marker_text_for_tag(field, tag, value)
    } else {
        rendered_base_text(field, slot, value, item_separator)
    }
}

fn rendered_base_text(
    field: &TemplateField,
    slot: Option<usize>,
    value: &Value,
    item_separator: &str,
) -> String {
    if field_is_multiple(field) {
        let items = party_items(value);
        // Table-row replication injects a single {text, suffix} object (not an
        // array); it must fill every slot of the replicated row, otherwise
        // cells beyond the first stay empty.
        if matches!(value, Value::Object(_)) && items.len() == 1 {
            // 与段落路径同一语义：有静态后缀时这里只给正文，条目后缀交给
            // apply_party_item_suffix 去替换静态后缀；否则内联条目后缀。
            return render_party_item(field, slot.unwrap_or(0), &items[0], false);
        }
        return match (field.mark_refs.len(), slot) {
            (_, None) => items
                .iter()
                .map(PartyItem::rendered)
                .collect::<Vec<_>>()
                .join(item_separator),
            (0..=1, Some(index)) => items
                .iter()
                .map(|item| render_party_item(field, index, item, items.len() > 1))
                .collect::<Vec<_>>()
                .join(item_separator),
            (count, Some(index)) if index >= count => String::new(),
            (count, Some(index)) if index + 1 == count && items.len() > count => {
                // The last source slot carries the overflow. Its static suffix
                // can only appear once, so include each selected suffix inline.
                items[index..]
                    .iter()
                    .map(PartyItem::rendered)
                    .collect::<Vec<_>>()
                    .join(item_separator)
            }
            (_, Some(index)) => items
                .get(index)
                .map(|item| render_party_item(field, index, item, false))
                .unwrap_or_default(),
        };
    }
    let text = scalar_value(value);
    // A field may have multiple document positions. When fill_all_positions is
    // set (same text marked in several separate places), every position gets the
    // value. Otherwise a later position is only filled when it is an explicit
    // reference — never silently duplicating (a single position split across
    // multiple runs with different formatting must stay single-valued).
    if slot.unwrap_or(0) > 0 && field.mark_refs.len() > 1 && !field.fill_all_positions {
        String::new()
    } else {
        text
    }
}

fn effective_item_separator(
    field: &TemplateField,
    overrides: &HashMap<String, StructureOverride>,
    fallback: &str,
) -> String {
    overrides
        .get(&field.id)
        .and_then(|item| item.item_separator.as_deref())
        .or_else(|| (!field.item_separator.is_empty()).then_some(field.item_separator.as_str()))
        .unwrap_or(fallback)
        .to_string()
}

fn default_repeat_suffix(field: &TemplateField) -> bool {
    field.repeat_suffix
        || (field_is_multiple(field)
            && field.mark_refs.iter().any(|mark_ref| {
                mark_ref
                    .optional_rule
                    .as_ref()
                    .is_some_and(|rule| !rule.effective_suffix().is_empty())
            }))
}

fn scalar_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => String::new(),
    }
}

fn optional_rule_for_slot(
    field: &TemplateField,
    slot: Option<usize>,
) -> Option<&OptionalFieldRule> {
    slot.and_then(|index| {
        field
            .mark_refs
            .get(index)
            .and_then(|reference| reference.optional_rule.as_ref())
    })
    .or(field.optional_rule.as_ref())
}

/// Apply a user-edited affix to its original document position. Affixes are
/// ordinary template text, not part of the field value; appending them inside
/// the SDT duplicates the source text and breaks empty-field cleanup.
///
/// When no explicit structure override exists, falls back to the optional rule's
/// prefix/suffix so that fields like 案号 automatically get their wrapper text
/// (e.g. "（案号：" / "）") applied even if the user never opened the structure editor.
fn apply_structure_override(
    siblings: &mut Vec<XmlNode>,
    at: usize,
    field: &TemplateField,
    slot: Option<usize>,
    overrides: &HashMap<String, StructureOverride>,
) -> usize {
    let override_ = overrides.get(&field.id);
    let rule = optional_rule_for_slot(field, slot);
    let repeat_prefix = field_is_multiple(field)
        && override_
            .and_then(|item| item.repeat_prefix)
            .unwrap_or(field.repeat_prefix);
    let repeat_suffix = field_is_multiple(field)
        && override_
            .and_then(|item| item.repeat_suffix)
            .unwrap_or_else(|| default_repeat_suffix(field));

    // User override takes priority; fall back to optionalRule prefix/suffix
    let effective_prefix = if repeat_prefix {
        None
    } else {
        override_.and_then(|o| o.prefix.as_deref()).or_else(|| {
            rule.and_then(|item| {
                (item.default_prefix.is_some() || !item.remove_empty_prefix.is_empty())
                    .then(|| item.effective_prefix())
            })
        })
    };
    let effective_suffix = if repeat_suffix {
        None
    } else {
        override_.and_then(|o| o.suffix.as_deref()).or_else(|| {
            rule.and_then(|item| {
                (item.default_suffix.is_some() || !item.remove_empty_suffix.is_empty())
                    .then(|| item.effective_suffix())
            })
        })
    };

    let ref_count = field.mark_refs.len();
    let index = slot.unwrap_or(0);
    let is_first = ref_count <= 1 || index == 0;
    let is_last = ref_count <= 1 || index + 1 >= ref_count;

    let mut sdt_index = at;
    if is_first {
        if let Some(prefix) = effective_prefix {
            let source = rule
                .map(|item| item.remove_empty_prefix.as_str())
                .unwrap_or("");
            if replace_prefix_before(siblings, at, source, prefix) {
                sdt_index += 1;
            }
        }
    }
    if is_last {
        if let Some(suffix) = effective_suffix {
            let source = rule
                .map(|item| item.remove_empty_suffix.as_str())
                .unwrap_or("");
            replace_suffix_after(siblings, sdt_index, source, suffix);
        }
    }
    sdt_index
}

fn replace_prefix_before(
    siblings: &mut Vec<XmlNode>,
    at: usize,
    source: &str,
    replacement: &str,
) -> bool {
    if source == replacement {
        return false;
    }
    let replaced = !source.is_empty() && text_before_ends_with(siblings, at, source);
    if replaced {
        strip_prefix_before(siblings, at, source);
    }
    if !replacement.is_empty() {
        let run = text_run_like_sdt(&siblings[at], replacement);
        siblings.insert(at, run);
        return true;
    }
    false
}

fn replace_suffix_after(siblings: &mut Vec<XmlNode>, at: usize, source: &str, replacement: &str) {
    if source == replacement {
        return;
    }
    let replaced = !source.is_empty() && text_after_starts_with(siblings, at, source);
    if replaced {
        strip_suffix_after(siblings, at, source);
    }
    if !replacement.is_empty() {
        let run = text_run_like_sdt(&siblings[at], replacement);
        siblings.insert(at + 1, run);
    }
}

fn text_before_ends_with(siblings: &[XmlNode], at: usize, expected: &str) -> bool {
    let mut text = String::new();
    for index in (0..at).rev() {
        let part = collect_text_from_element(&siblings[index]);
        if !part.is_empty() {
            text.insert_str(0, &part);
            if text.chars().count() >= expected.chars().count() {
                break;
            }
        }
    }
    text.ends_with(expected)
}

fn text_after_starts_with(siblings: &[XmlNode], at: usize, expected: &str) -> bool {
    let mut text = String::new();
    for sibling in siblings.iter().skip(at + 1) {
        let part = collect_text_from_element(sibling);
        if !part.is_empty() {
            text.push_str(&part);
            if text.chars().count() >= expected.chars().count() {
                break;
            }
        }
    }
    text.starts_with(expected)
}

fn text_run_like_sdt(sdt: &XmlNode, text: &str) -> XmlNode {
    if let Some(mut run) = first_run_in_node(sdt) {
        replace_text_in_run(&mut run, text);
        return run;
    }
    XmlNode::Element {
        name: "w:r".to_string(),
        attrs: Vec::new(),
        children: vec![XmlNode::Element {
            name: "w:t".to_string(),
            attrs: vec![("xml:space".to_string(), "preserve".to_string())],
            children: vec![XmlNode::Text(text.to_string())],
        }],
    }
}

fn first_run_in_node(node: &XmlNode) -> Option<XmlNode> {
    if let XmlNode::Element { name, children, .. } = node {
        if name == "w:r" {
            return Some(node.clone());
        }
        for child in children {
            if let Some(run) = first_run_in_node(child) {
                return Some(run);
            }
        }
    }
    None
}

fn replace_text_in_run(run: &mut XmlNode, text: &str) {
    let XmlNode::Element { children, .. } = run else {
        return;
    };
    let mut wrote = false;
    for child in children.iter_mut() {
        if let XmlNode::Element { name, children, .. } = child {
            if name == "w:t" {
                for node in children.iter_mut() {
                    if let XmlNode::Text(value) = node {
                        if wrote {
                            value.clear();
                        } else {
                            *value = text.to_string();
                            wrote = true;
                        }
                    }
                }
            }
        }
    }
    if !wrote {
        children.push(XmlNode::Element {
            name: "w:t".to_string(),
            attrs: vec![("xml:space".to_string(), "preserve".to_string())],
            children: vec![XmlNode::Text(text.to_string())],
        });
    }
}

fn unwrap_sdt_content(sdt: XmlNode) -> Vec<XmlNode> {
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

fn strip_party_separator_before(siblings: &mut [XmlNode], at: usize) {
    strip_separator_before(siblings, at, &['、']);
}

/// 剥离 at 之前最近一个非空兄弟节点尾部的分隔符。
/// 分隔符不一定独占一个节点：与其它文字合并进同一 run（跨 run）时，
/// 只剥掉末尾的分隔符字符，保留前面的文字；剥空后整节点移除。
fn strip_separator_before(siblings: &mut [XmlNode], at: usize, separators: &[char]) {
    for index in (0..at).rev() {
        if collect_text_from_element(&siblings[index]).is_empty() {
            continue;
        }
        strip_trailing_separator(&mut siblings[index], separators);
        if collect_text_from_element(&siblings[index]).is_empty() {
            siblings[index] = XmlNode::Text(String::new());
        }
        return;
    }
}

/// 递归找到节点内最后一个文本节点，剥掉其尾部（忽略尾随空白后）的分隔符字符。
fn strip_trailing_separator(node: &mut XmlNode, separators: &[char]) -> bool {
    match node {
        XmlNode::Text(text) => {
            let trimmed = text.trim_end();
            let Some(last) = trimmed.chars().last() else {
                return false;
            };
            if separators.contains(&last) {
                *text = trimmed[..trimmed.len() - last.len_utf8()].to_string();
                true
            } else {
                false
            }
        }
        XmlNode::Element { children, .. } => children
            .iter_mut()
            .rev()
            .any(|child| strip_trailing_separator(child, separators)),
    }
}

fn is_party_role_prefix(prefix: &str) -> bool {
    matches!(
        prefix.trim(),
        "原告" | "被告" | "第三人" | "上诉人" | "被上诉人" | "申请人" | "被申请人"
    )
}

fn strip_role_separator_before(siblings: &mut [XmlNode], at: usize) {
    strip_separator_before(siblings, at, &['，', ',']);
}

fn marker_text_for_tag(field: &TemplateField, tag: &str, value: &Value) -> String {
    let option = field.options.iter().find(|option| {
        let option_tag = if option.marker_tag.is_empty() {
            format!("{}.option.{}", field.id, option.id)
        } else {
            option.marker_tag.clone()
        };
        option_tag == tag
    });
    let Some(option) = option else {
        return scalar_value(value);
    };
    let checked = match field.field_type.as_str() {
        "checkbox" => value.as_bool().unwrap_or(false),
        "radio_group" => value
            .as_str()
            .map(|selected| selected == option.id || selected == option.label)
            .unwrap_or(false),
        "checkbox_group" => value
            .as_array()
            .map(|items| {
                items.iter().any(|item| {
                    item.as_str()
                        .map(|selected| selected == option.id || selected == option.label)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false),
        _ => false,
    };
    if checked {
        option.checked_text.clone()
    } else {
        option.unchecked_text.clone()
    }
}

/// Render text into existing w:r elements, preserving each run's w:rPr
fn render_into_existing_runs(children: &[XmlNode], text: &str) -> XmlNode {
    let mut new_runs: Vec<XmlNode> = Vec::new();
    let mut first = true;

    for child in children {
        if let XmlNode::Element {
            name, children: cc, ..
        } = child
        {
            if name == "w:r" {
                let mut new_children = cc.clone();
                for child in &mut new_children {
                    if let XmlNode::Element {
                        name: cn,
                        children: text_children,
                        ..
                    } = child
                    {
                        if cn == "w:t" {
                            for node in text_children {
                                if let XmlNode::Text(value) = node {
                                    if first {
                                        *value = text.to_string();
                                        first = false;
                                    } else {
                                        value.clear();
                                    }
                                }
                            }
                        }
                    }
                }
                if first {
                    // MDG-012: Don't inject w:t into runs that don't have one
                    // (e.g. fldChar, instrText). Only replace existing w:t content.
                    let has_wt = new_children
                        .iter()
                        .any(|c| matches!(c, XmlNode::Element { name, .. } if name == "w:t"));
                    if has_wt {
                        // w:t already exists but was empty — inject text
                        new_children.push(XmlNode::Element {
                            name: "w:t".to_string(),
                            attrs: vec![("xml:space".to_string(), "preserve".to_string())],
                            children: vec![XmlNode::Text(text.to_string())],
                        });
                    }
                    first = false;
                }
                new_runs.push(XmlNode::Element {
                    name: "w:r".to_string(),
                    attrs: match child {
                        XmlNode::Element { attrs, .. } => attrs.clone(),
                        _ => Vec::new(),
                    },
                    children: new_children,
                });
            }
        }
    }

    XmlNode::Element {
        name: "w:sdtContent".to_string(),
        attrs: Vec::new(),
        children: new_runs,
    }
}

/// Strip a prefix text from w:r/w:t nodes before index `at` in children
fn strip_prefix_before(children: &mut [XmlNode], at: usize, prefix: &str) {
    if prefix.is_empty() || at == 0 {
        return;
    }
    let prefix_len = prefix.chars().count();

    // Collect text from previous siblings, working backwards
    let mut texts: Vec<(usize, usize)> = Vec::new(); // (child_index, char_start_offset)
    let mut total = 0;
    for j in (0..at).rev() {
        let text = collect_text_from_element(&children[j]);
        let char_count = text.chars().count();
        if char_count == 0 {
            continue;
        }
        // We collect chars from the END of this node
        let need = prefix_len.saturating_sub(total);
        let take = need.min(char_count);
        // Record offset: how many chars from the END to potentially remove
        texts.push((j, take));
        total += take;
        if total >= prefix_len {
            break;
        }
    }

    // Verify the collected text ends with the prefix
    let mut collected = String::new();
    for &(j, _) in texts.iter().rev() {
        collected.push_str(&collect_text_from_element(&children[j]));
    }
    if collected.chars().count() < prefix_len || !collected.ends_with(prefix) {
        return;
    }

    // Actually strip: remove chars from the END of each affected node
    for (j, take) in &texts {
        trim_chars_from_end(&mut children[*j], *take);
    }
}

/// Strip a suffix text from w:r/w:t nodes after index `at` in children
fn strip_suffix_after(children: &mut [XmlNode], at: usize, suffix: &str) {
    if suffix.is_empty() || at >= children.len().saturating_sub(1) {
        return;
    }
    let suffix_len = suffix.chars().count();

    let mut texts: Vec<(usize, usize)> = Vec::new();
    let mut total = 0;
    for (j, node) in children.iter().enumerate().skip(at + 1) {
        let text = collect_text_from_element(node);
        let char_count = text.chars().count();
        if char_count == 0 {
            continue;
        }
        let need = suffix_len.saturating_sub(total);
        let take = need.min(char_count);
        texts.push((j, take));
        total += take;
        if total >= suffix_len {
            break;
        }
    }

    let mut collected = String::new();
    for &(j, _) in &texts {
        collected.push_str(&collect_text_from_element(&children[j]));
    }
    if collected.chars().count() < suffix_len || !collected.starts_with(suffix) {
        return;
    }

    for (j, take) in &texts {
        trim_chars_from_start(&mut children[*j], *take);
    }
}

fn collect_text_from_element(node: &XmlNode) -> String {
    if let XmlNode::Element { name, children, .. } = node {
        if name == "w:r" {
            for c in children {
                if let XmlNode::Element {
                    name: cn,
                    children: cc,
                    ..
                } = c
                {
                    if cn == "w:t" {
                        for t in cc {
                            if let XmlNode::Text(s) = t {
                                return s.clone();
                            }
                        }
                    }
                }
            }
        }
    }
    String::new()
}

fn trim_chars_from_end(node: &mut XmlNode, count: usize) {
    if count == 0 {
        return;
    }
    if let XmlNode::Element { name, children, .. } = node {
        if name == "w:r" {
            for c in children.iter_mut() {
                if let XmlNode::Element {
                    name: cn,
                    children: cc,
                    ..
                } = c
                {
                    if cn == "w:t" {
                        for t in cc.iter_mut() {
                            if let XmlNode::Text(s) = t {
                                let chars: Vec<char> = s.chars().collect();
                                let new_len = chars.len().saturating_sub(count);
                                *s = chars[..new_len].iter().collect();
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn trim_chars_from_start(node: &mut XmlNode, count: usize) {
    if count == 0 {
        return;
    }
    if let XmlNode::Element { name, children, .. } = node {
        if name == "w:r" {
            for c in children.iter_mut() {
                if let XmlNode::Element {
                    name: cn,
                    children: cc,
                    ..
                } = c
                {
                    if cn == "w:t" {
                        for t in cc.iter_mut() {
                            if let XmlNode::Text(s) = t {
                                let chars: Vec<char> = s.chars().collect();
                                let new_start = count.min(chars.len());
                                *s = chars[new_start..].iter().collect();
                                return;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn value_for_field<'a>(
    values: &'a HashMap<String, Value>,
    field: &TemplateField,
    slot: Option<usize>,
) -> Option<&'a Value> {
    // A per-position value (fill page "follower made independent by type
    // change") takes precedence; otherwise fall back to the shared field key.
    slot.and_then(|s| values.get(&format!("{}#{}", field.id, s)))
        .or_else(|| values.get(&field.id))
        .or_else(|| values.get(&field.name))
}

#[derive(Debug, Clone)]
struct PartyItem {
    prefix: String,
    text: String,
    suffix: String,
}

impl PartyItem {
    fn rendered(&self) -> String {
        format!("{}{}{}", self.prefix, self.text, self.suffix)
    }
}

fn render_party_item(
    field: &TemplateField,
    slot: usize,
    item: &PartyItem,
    force_inline_suffix: bool,
) -> String {
    let has_static_affix = optional_rule_for_slot(field, Some(slot)).is_some_and(|rule| {
        !rule.remove_empty_prefix.is_empty() || !rule.remove_empty_suffix.is_empty()
    });
    if has_static_affix && !force_inline_suffix {
        item.text.clone()
    } else {
        item.rendered()
    }
}

fn party_items(value: &Value) -> Vec<PartyItem> {
    match value {
        Value::String(s) if !s.is_empty() => s
            .split('、')
            .map(|text| PartyItem {
                prefix: String::new(),
                text: text.to_string(),
                suffix: String::new(),
            })
            .collect(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(value) if !value.trim().is_empty() => Some(PartyItem {
                    prefix: String::new(),
                    text: value.trim().to_string(),
                    suffix: String::new(),
                }),
                Value::Object(values) => {
                    let text = values
                        .get("name")
                        .or_else(|| values.get("text"))?
                        .as_str()?
                        .trim();
                    if text.is_empty() {
                        return None;
                    }
                    Some(PartyItem {
                        prefix: values
                            .get("prefix")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                        text: text.to_string(),
                        suffix: values
                            .get("suffix")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    })
                }
                Value::Number(value) => Some(PartyItem {
                    prefix: String::new(),
                    text: value.to_string(),
                    suffix: String::new(),
                }),
                _ => None,
            })
            .collect(),
        Value::Object(values) => party_item_from_object(values).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn party_item_from_object(values: &serde_json::Map<String, Value>) -> Option<PartyItem> {
    let text = values
        .get("name")
        .or_else(|| values.get("text"))?
        .as_str()?
        .trim();
    if text.is_empty() {
        return None;
    }
    Some(PartyItem {
        prefix: values
            .get("prefix")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        text: text.to_string(),
        suffix: values
            .get("suffix")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

fn apply_multiple_item_affixes(
    siblings: &mut Vec<XmlNode>,
    at: usize,
    field: &TemplateField,
    slot: Option<usize>,
    value: &Value,
    overrides: &HashMap<String, StructureOverride>,
) -> usize {
    if !field_is_multiple(field) {
        return at;
    }
    let Some(rule) = optional_rule_for_slot(field, slot) else {
        return at;
    };
    let items = party_items(value);
    let index = slot.unwrap_or(0);
    let count = field.mark_refs.len();
    if index >= items.len() {
        return at;
    }
    let override_ = overrides.get(&field.id);
    let repeat_prefix = override_
        .and_then(|item| item.repeat_prefix)
        .unwrap_or(field.repeat_prefix);
    let repeat_suffix = override_
        .and_then(|item| item.repeat_suffix)
        .unwrap_or_else(|| default_repeat_suffix(field));
    if count <= 1 && items.len() > 1 {
        if repeat_prefix && !rule.remove_empty_prefix.is_empty() {
            strip_prefix_before(siblings, at, &rule.remove_empty_prefix);
        }
        if repeat_suffix && !rule.remove_empty_suffix.is_empty() {
            strip_suffix_after(siblings, at, &rule.remove_empty_suffix);
        }
        return at;
    }
    let mut sdt_index = at;
    if repeat_prefix
        && !rule.remove_empty_prefix.is_empty()
        && replace_prefix_before(
            siblings,
            sdt_index,
            &rule.remove_empty_prefix,
            &items[index].prefix,
        )
    {
        sdt_index += 1;
    }
    if !repeat_suffix || rule.remove_empty_suffix.is_empty() {
        return sdt_index;
    }
    if index + 1 >= count && items.len() > count.max(1) {
        // Overflow suffixes are already embedded in rendered_base_text.
        replace_suffix_after(siblings, sdt_index, &rule.remove_empty_suffix, "");
        return sdt_index;
    }
    replace_suffix_after(
        siblings,
        sdt_index,
        &rule.remove_empty_suffix,
        &items[index].suffix,
    );
    sdt_index
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_xml(xml: &str) -> XmlTree {
        XmlTree::parse(xml.as_bytes()).unwrap()
    }

    fn field(id: &str, ftype: &str) -> TemplateField {
        TemplateField {
            id: id.to_string(),
            name: id.to_string(),
            label: id.to_string(),
            field_type: ftype.to_string(),
            ..Default::default()
        }
    }

    fn manifest(fields: Vec<TemplateField>) -> TemplateManifest {
        TemplateManifest {
            format_version: 2,
            template: super::super::TemplateMeta {
                id: "t0".to_string(),
                name: "test".to_string(),
                created: String::new(),
                updated: String::new(),
            },
            fields,
            filename_template: None,
        }
    }

    fn node_text(node: &XmlNode) -> String {
        match node {
            XmlNode::Text(text) => text.clone(),
            XmlNode::Element { children, .. } => children.iter().map(node_text).collect(),
        }
    }

    fn compact_text(node: &XmlNode) -> String {
        node_text(node)
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect()
    }

    fn field_map(m: &TemplateManifest) -> TagMap<'_> {
        build_tag_map(m)
    }

    #[test]
    fn render_text_field_preserves_rpr() {
        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:sdt>
                <w:sdtPr><w:tag w:val="f1"/></w:sdtPr>
                <w:sdtContent>
                    <w:r><w:rPr><w:b/><w:sz w:val="28"/></w:rPr><w:t>旧值</w:t></w:r>
                </w:sdtContent>
            </w:sdt>
        </w:body></w:document>"#,
        );

        let m = manifest(vec![field("f1", "text")]);
        let fm = field_map(&m);
        let mut vals = HashMap::new();
        vals.insert("f1".to_string(), Value::String("新值".to_string()));

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new(), "、").unwrap();
        let out = tree.to_xml().unwrap();

        assert!(out.contains("新值"), "rendered text missing");
        assert!(out.contains("<w:b"), "bold format preserved");
        assert!(out.contains("w:val=\"28\""), "font size preserved");
    }

    #[test]
    fn render_marker_preserves_font() {
        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:sdt>
                <w:sdtPr><w:tag w:val="c1"/></w:sdtPr>
                <w:sdtContent>
                    <w:r>
                        <w:rPr><w:rFonts w:ascii="Wingdings 2"/></w:rPr>
                        <w:t>☐</w:t>
                    </w:r>
                </w:sdtContent>
            </w:sdt>
        </w:body></w:document>"#,
        );

        let m = manifest(vec![field("c1", "checkbox")]);
        let fm = field_map(&m);
        let mut vals = HashMap::new();
        vals.insert("c1".to_string(), Value::String("☑".to_string()));

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new(), "、").unwrap();
        let out = tree.to_xml().unwrap();

        assert!(out.contains("☑"), "checkbox marker updated");
        assert!(out.contains("Wingdings"), "symbol font preserved");
    }

    #[test]
    fn render_empty_optional_rule_removes_content() {
        let mut field = field("f1", "text");
        field.optional_rule = Some(OptionalFieldRule {
            enabled: true,
            ..Default::default()
        });

        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:sdt>
                <w:sdtPr><w:tag w:val="f1"/></w:sdtPr>
                <w:sdtContent><w:r><w:t>旧值</w:t></w:r></w:sdtContent>
            </w:sdt>
        </w:body></w:document>"#,
        );

        let m = manifest(vec![field]);
        let fm = field_map(&m);
        render_tree(&mut tree.root, &fm, &HashMap::new(), &HashMap::new(), "、").unwrap();
        let out = tree.to_xml().unwrap();
        assert!(
            !out.contains("旧值"),
            "empty field should clear content when optional_rule enabled"
        );
    }

    #[test]
    fn render_delete_text_removes_sdt() {
        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:sdt>
                <w:sdtPr><w:tag w:val="__delete_d1"/></w:sdtPr>
                <w:sdtContent><w:r><w:t>del</w:t></w:r></w:sdtContent>
            </w:sdt>
        </w:body></w:document>"#,
        );

        let m = manifest(vec![field("d1", "delete_text")]);
        let fm = field_map(&m);
        render_tree(&mut tree.root, &fm, &HashMap::new(), &HashMap::new(), "、").unwrap();
        assert!(
            !tree.to_xml().unwrap().contains("del"),
            "delete_text sdt removed"
        );
    }

    #[test]
    fn render_table_row_replication() {
        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:tbl>
                <w:tr>
                    <w:tc><w:p>
                        <w:sdt>
                            <w:sdtPr><w:tag w:val="pl"/></w:sdtPr>
                            <w:sdtContent><w:r><w:rPr><w:b/></w:rPr><w:t>name</w:t></w:r></w:sdtContent>
                        </w:sdt>
                    </w:p></w:tc>
                </w:tr>
            </w:tbl>
        </w:body></w:document>"#,
        );

        let m = manifest(vec![field("pl", "party_list")]);
        let fm = field_map(&m);
        let mut vals = HashMap::new();
        vals.insert("pl".to_string(), Value::String("张三、李四".to_string()));

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new(), "、").unwrap();
        let out = tree.to_xml().unwrap();

        assert!(out.contains("张三"), "first item rendered");
        assert!(out.contains("李四"), "second item rendered");
        assert!(
            out.contains("<w:b"),
            "bold format preserved in replicated rows"
        );
    }

    #[test]
    fn generic_multiple_table_rows_keep_each_item_prefix() {
        let mut field = field("parties", "text");
        field.multiple = true;
        field.repeat_prefix = true;
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "parties".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_prefix: "申请人".to_string(),
                remove_empty_suffix: String::new(),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let m = manifest(vec![field]);
        let mut tree = parse_xml(
            r#"<w:document><w:body><w:tbl><w:tr><w:tc><w:p>
                <w:r><w:t>申请人</w:t></w:r>
                <w:sdt><w:sdtPr><w:tag w:val="parties"/></w:sdtPr><w:sdtContent><w:r><w:t>旧值</w:t></w:r></w:sdtContent></w:sdt>
            </w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#,
        );
        let mut values = HashMap::new();
        values.insert(
            "parties".to_string(),
            serde_json::json!([
                { "prefix": "原告", "name": "甲公司" },
                { "prefix": "被告", "name": "乙公司" }
            ]),
        );

        render_tree(
            &mut tree.root,
            &field_map(&m),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();

        assert_eq!(compact_text(&tree.root), "原告甲公司被告乙公司");
    }

    #[test]
    fn render_preserves_unrelated_sdt() {
        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:sdt><w:sdtPr><w:tag w:val="f1"/></w:sdtPr><w:sdtContent><w:r><w:t>a</w:t></w:r></w:sdtContent></w:sdt>
            <w:p><w:r><w:t>plain text</w:t></w:r></w:p>
        </w:body></w:document>"#,
        );

        let m = manifest(vec![field("f1", "text")]);
        let fm = field_map(&m);
        let mut vals = HashMap::new();
        vals.insert("f1".to_string(), Value::String("x".to_string()));

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new(), "、").unwrap();
        let out = tree.to_xml().unwrap();
        assert!(out.contains("plain text"), "unrelated text preserved");
    }

    #[test]
    fn render_removes_known_truncated_placeholder_from_legacy_template() {
        let mut field = field("lawyer", "text");
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "lawyer.ref.1".to_string(),
            ..Default::default()
        }];
        let package = vec![(
            "word/document.xml".to_string(),
            r#"<w:document><w:body><w:p>
                <w:sdt><w:sdtPr><w:tag w:val="lawyer.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>{{lawyer.ref.1}}</w:t></w:r></w:sdtContent></w:sdt>
                <w:r><w:t>lawyer.ref.1}}</w:t></w:r><w:r><w:t>律师</w:t></w:r>
            </w:p></w:body></w:document>"#
                .as_bytes()
                .to_vec(),
        )];
        let m = manifest(vec![field]);
        let mut values = HashMap::new();
        values.insert("lawyer".to_string(), Value::String("吕晗".to_string()));

        let rendered = render_docx(&package, &m, &values, &HashMap::new(), "、").unwrap();
        let out = String::from_utf8(rendered[0].1.clone()).unwrap();

        assert!(out.contains("吕晗"));
        assert!(out.contains("律师"));
        assert!(
            !out.contains("lawyer.ref.1}}"),
            "孤立占位符残片应清除: {out}"
        );
    }

    #[test]
    fn legacy_fragment_does_not_block_repeated_suffix_replacement() {
        let mut field = field("lawyer", "text");
        field.multiple = true;
        field.repeat_suffix = true;
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "lawyer.ref.1".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_suffix: "律师".to_string(),
                default_suffix: Some("律师".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let package = vec![(
            "word/document.xml".to_string(),
            r#"<w:document><w:body><w:p>
                <w:sdt><w:sdtPr><w:tag w:val="lawyer.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>{{lawyer.ref.1}}</w:t></w:r></w:sdtContent></w:sdt>
                <w:r><w:t>lawyer.ref.1}}</w:t></w:r><w:r><w:t>律师</w:t></w:r>
            </w:p></w:body></w:document>"#
                .as_bytes()
                .to_vec(),
        )];
        let m = manifest(vec![field]);
        let mut values = HashMap::new();
        values.insert(
            "lawyer".to_string(),
            serde_json::json!([
                { "name": "吕晗", "suffix": "律师" },
                { "name": "高海钧", "suffix": "律师" }
            ]),
        );

        let rendered = render_docx(&package, &m, &values, &HashMap::new(), "、").unwrap();
        let tree = XmlTree::parse(&rendered[0].1).unwrap();
        let text = compact_text(&tree.root);

        assert_eq!(text, "吕晗律师、高海钧律师");
        assert!(!text.contains("lawyer.ref.1}}"));
    }

    #[test]
    fn party_list_uses_frontend_array_payload_and_per_item_suffixes() {
        let mut field = field("lawyers", "party_list");
        field.mark_refs = vec![
            super::super::TemplateMarkRef {
                tag: "lawyers.ref.1".to_string(),
                ..Default::default()
            },
            super::super::TemplateMarkRef {
                tag: "lawyers.ref.2".to_string(),
                ..Default::default()
            },
        ];
        let m = manifest(vec![field]);
        let mut tree = parse_xml(
            r#"<w:p>
          <w:sdt><w:sdtPr><w:tag w:val="lawyers.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>{{lawyers.ref.1}}</w:t></w:r></w:sdtContent></w:sdt>
          <w:sdt><w:sdtPr><w:tag w:val="lawyers.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>{{lawyers.ref.2}}</w:t></w:r></w:sdtContent></w:sdt>
        </w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert(
            "lawyers".to_string(),
            serde_json::json!([
                { "name": "李琼", "suffix": "律师" },
                { "name": "吕晗", "suffix": "实习律师" }
            ]),
        );

        render_tree(
            &mut tree.root,
            &field_map(&m),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();
        let out = tree.to_xml().unwrap();
        assert!(out.contains("李琼律师"));
        assert!(out.contains("吕晗实习律师"));
        assert!(!out.contains("{{lawyers"));
    }

    #[test]
    fn generic_multiple_text_field_joins_items_and_repeats_suffix_once() {
        let mut field = field("lawyers", "text");
        field.multiple = true;
        field.item_separator = "、".to_string();
        field.repeat_suffix = true;
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "lawyers".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_prefix: String::new(),
                remove_empty_suffix: "律师".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let mut tree = parse_xml(
            r#"<w:p><w:sdt><w:sdtPr><w:tag w:val="lawyers"/></w:sdtPr><w:sdtContent><w:r><w:t>旧姓名</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>律师</w:t></w:r></w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert(
            "lawyers".to_string(),
            serde_json::json!([
                { "name": "李月春", "suffix": "律师" },
                { "name": "吕晗", "suffix": "律师" }
            ]),
        );

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();

        assert_eq!(compact_text(&tree.root), "李月春律师、吕晗律师");
    }

    #[test]
    fn generic_multiple_text_field_uses_custom_separator() {
        let mut field = field("parties", "text");
        field.multiple = true;
        field.item_separator = "；".to_string();
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "parties".to_string(),
            ..Default::default()
        }];
        let mut tree = parse_xml(
            r#"<w:p><w:sdt><w:sdtPr><w:tag w:val="parties"/></w:sdtPr><w:sdtContent><w:r><w:t>旧值</w:t></w:r></w:sdtContent></w:sdt></w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert(
            "parties".to_string(),
            serde_json::json!(["甲公司", "乙公司"]),
        );

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();

        assert_eq!(compact_text(&tree.root), "甲公司；乙公司");
    }

    #[test]
    fn repeated_plain_field_fills_only_the_primary_slot() {
        let mut field = field("party", "text");
        field.mark_refs = vec![
            super::super::TemplateMarkRef {
                tag: "party.ref.1".to_string(),
                ..Default::default()
            },
            super::super::TemplateMarkRef {
                tag: "party.ref.2".to_string(),
                ..Default::default()
            },
        ];
        let m = manifest(vec![field]);
        let mut tree = parse_xml(
            r#"<w:p>
          <w:sdt><w:sdtPr><w:tag w:val="party.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>one</w:t></w:r></w:sdtContent></w:sdt>
          <w:sdt><w:sdtPr><w:tag w:val="party.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>two</w:t></w:r></w:sdtContent></w:sdt>
        </w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert("party".to_string(), Value::String("原告甲".to_string()));

        render_tree(
            &mut tree.root,
            &field_map(&m),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();
        let out = tree.to_xml().unwrap();
        assert_eq!(out.matches("原告甲").count(), 1);
    }

    #[test]
    fn fill_all_positions_duplicates_value_to_every_slot() {
        let mut field = field("party", "text");
        field.fill_all_positions = true;
        field.mark_refs = vec![
            super::super::TemplateMarkRef {
                tag: "party.ref.1".to_string(),
                ..Default::default()
            },
            super::super::TemplateMarkRef {
                tag: "party.ref.2".to_string(),
                ..Default::default()
            },
        ];
        let m = manifest(vec![field]);
        let mut tree = parse_xml(
            r#"<w:p>
          <w:sdt><w:sdtPr><w:tag w:val="party.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>one</w:t></w:r></w:sdtContent></w:sdt>
          <w:sdt><w:sdtPr><w:tag w:val="party.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>two</w:t></w:r></w:sdtContent></w:sdt>
        </w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert("party".to_string(), Value::String("原告甲".to_string()));

        render_tree(
            &mut tree.root,
            &field_map(&m),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();
        let out = tree.to_xml().unwrap();
        assert_eq!(out.matches("原告甲").count(), 2);
    }

    #[test]
    fn party_list_removes_unused_separator_and_unwraps_sdt() {
        let mut field = field("pl", "party_list");
        field.mark_refs = vec![
            super::super::TemplateMarkRef {
                tag: "pl.ref.1".to_string(),
                optional_rule: Some(OptionalFieldRule {
                    enabled: true,
                    remove_empty_prefix: "原告".to_string(),
                    remove_empty_suffix: String::new(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            super::super::TemplateMarkRef {
                tag: "pl.ref.2".to_string(),
                ..Default::default()
            },
            super::super::TemplateMarkRef {
                tag: "pl.ref.3".to_string(),
                ..Default::default()
            },
        ];
        let mut tree = parse_xml(
            r#"<w:p><w:r><w:t>原告</w:t></w:r>
              <w:sdt><w:sdtPr><w:tag w:val="pl.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>甲</w:t></w:r></w:sdtContent></w:sdt>
              <w:r><w:t>、</w:t></w:r>
              <w:sdt><w:sdtPr><w:tag w:val="pl.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>乙</w:t></w:r></w:sdtContent></w:sdt>
              <w:sdt><w:sdtPr><w:tag w:val="pl.ref.3"/></w:sdtPr><w:sdtContent><w:r><w:t>丙</w:t></w:r></w:sdtContent></w:sdt>
              <w:r><w:t>与</w:t></w:r></w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert(
            "pl".to_string(),
            Value::Array(vec![Value::String("甲公司".to_string())]),
        );

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();
        let out = tree.to_xml().unwrap();
        assert!(out.contains(">原告<"));
        assert!(out.contains(">甲公司<"));
        assert!(out.contains(">与<"));
        assert!(!out.contains(">、<"));
        assert!(!out.contains("w:sdt"));
    }

    #[test]
    fn structure_override_replaces_source_affix_instead_of_duplicating_it() {
        let mut field = field("party", "text");
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "party".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_prefix: "原告".to_string(),
                remove_empty_suffix: String::new(),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let mut tree = parse_xml(
            r#"<w:p><w:r><w:t>原告</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="party"/></w:sdtPr><w:sdtContent><w:r><w:t>旧值</w:t></w:r></w:sdtContent></w:sdt></w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert("party".to_string(), Value::String("甲公司".to_string()));
        let mut overrides = HashMap::new();
        overrides.insert(
            "party".to_string(),
            StructureOverride {
                prefix: Some("申请人".to_string()),
                suffix: None,
                ..Default::default()
            },
        );

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &overrides,
            "、",
        )
        .unwrap();
        let out = tree.to_xml().unwrap();
        assert!(out.contains(">申请人<"));
        assert!(out.contains(">甲公司<"));
        assert!(!out.contains(">原告<"));
        assert!(!out.contains("w:sdt"));
    }

    #[test]
    fn edited_template_default_affix_replaces_embedded_source_text() {
        let mut field = field("principal", "text");
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "principal".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_prefix: "第三人".to_string(),
                default_prefix: Some("被上诉人".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let mut tree = parse_xml(
            r#"<w:p><w:r><w:t>第三人</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="principal"/></w:sdtPr><w:sdtContent><w:r><w:t>旧值</w:t></w:r></w:sdtContent></w:sdt></w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert("principal".to_string(), Value::String("甲公司".to_string()));

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();

        assert_eq!(compact_text(&tree.root), "被上诉人甲公司");
    }

    #[test]
    fn explicitly_empty_default_affix_removes_embedded_source_text() {
        let mut field = field("principal", "text");
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "principal".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_prefix: "第三人".to_string(),
                default_prefix: Some(String::new()),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let mut tree = parse_xml(
            r#"<w:p><w:r><w:t>第三人</w:t></w:r><w:sdt><w:sdtPr><w:tag w:val="principal"/></w:sdtPr><w:sdtContent><w:r><w:t>旧值</w:t></w:r></w:sdtContent></w:sdt></w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert("principal".to_string(), Value::String("甲公司".to_string()));

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();

        assert_eq!(compact_text(&tree.root), "甲公司");
    }

    #[test]
    fn table_row_replication_keeps_static_suffix_untouched_when_item_suffix_matches() {
        // 表格行复制路径：条目后缀与静态后缀相同（source == replacement）时，
        // 静态后缀保留一次、不重复，不出现“张三律师律师”。
        let mut field = field("pl", "party_list");
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "pl".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_prefix: String::new(),
                remove_empty_suffix: "律师".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let m = manifest(vec![field]);
        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:tbl>
                <w:tr>
                    <w:tc><w:p>
                        <w:sdt><w:sdtPr><w:tag w:val="pl"/></w:sdtPr><w:sdtContent><w:r><w:t>原告方</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>律师</w:t></w:r>
                    </w:p></w:tc>
                </w:tr>
            </w:tbl>
        </w:body></w:document>"#,
        );
        let mut vals = HashMap::new();
        vals.insert(
            "pl".to_string(),
            serde_json::json!([
                { "name": "张三", "suffix": "律师" },
                { "name": "李四", "suffix": "律师" }
            ]),
        );

        render_tree(&mut tree.root, &field_map(&m), &vals, &HashMap::new(), "、").unwrap();
        let text = compact_text(&tree.root);
        assert_eq!(text, "张三律师李四律师", "静态后缀不重复不残留: {text}");
    }

    #[test]
    fn table_row_replication_replaces_static_suffix_with_differing_item_suffix() {
        // 条目后缀与静态后缀不同（source != replacement）时，
        // 静态后缀被替换为条目后缀，不残留。
        let mut field = field("pl", "party_list");
        field.mark_refs = vec![super::super::TemplateMarkRef {
            tag: "pl".to_string(),
            optional_rule: Some(OptionalFieldRule {
                enabled: true,
                remove_empty_prefix: String::new(),
                remove_empty_suffix: "律师".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }];
        let m = manifest(vec![field]);
        let mut tree = parse_xml(
            r#"<w:document><w:body>
            <w:tbl>
                <w:tr>
                    <w:tc><w:p>
                        <w:sdt><w:sdtPr><w:tag w:val="pl"/></w:sdtPr><w:sdtContent><w:r><w:t>原告方</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>律师</w:t></w:r>
                    </w:p></w:tc>
                </w:tr>
            </w:tbl>
        </w:body></w:document>"#,
        );
        let mut vals = HashMap::new();
        vals.insert(
            "pl".to_string(),
            serde_json::json!([
                { "name": "张三", "suffix": "实习律师" },
                { "name": "李四", "suffix": "" }
            ]),
        );

        render_tree(&mut tree.root, &field_map(&m), &vals, &HashMap::new(), "、").unwrap();
        let text = compact_text(&tree.root);
        assert_eq!(text, "张三实习律师李四", "静态后缀被替换且不残留: {text}");
    }

    #[test]
    fn repeatable_party_suffix_replaces_its_template_text_once() {
        let mut field = field("lawyers", "party_list");
        field.mark_refs = vec![
            super::super::TemplateMarkRef {
                tag: "lawyers.ref.1".to_string(),
                optional_rule: Some(OptionalFieldRule {
                    enabled: true,
                    remove_empty_prefix: String::new(),
                    remove_empty_suffix: "律师".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
            super::super::TemplateMarkRef {
                tag: "lawyers.ref.2".to_string(),
                optional_rule: Some(OptionalFieldRule {
                    enabled: true,
                    remove_empty_prefix: String::new(),
                    remove_empty_suffix: "实习律师".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];
        let mut tree = parse_xml(
            r#"<w:p>
              <w:sdt><w:sdtPr><w:tag w:val="lawyers.ref.1"/></w:sdtPr><w:sdtContent><w:r><w:t>旧一</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>律师</w:t></w:r><w:r><w:t>、</w:t></w:r>
              <w:sdt><w:sdtPr><w:tag w:val="lawyers.ref.2"/></w:sdtPr><w:sdtContent><w:r><w:t>旧二</w:t></w:r></w:sdtContent></w:sdt><w:r><w:t>实习律师</w:t></w:r>
            </w:p>"#,
        );
        let mut values = HashMap::new();
        values.insert(
            "lawyers".to_string(),
            serde_json::json!([
                { "name": "甲", "suffix": "律师" },
                { "name": "乙", "suffix": "实习律师" }
            ]),
        );

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &HashMap::new(),
            "、",
        )
        .unwrap();
        assert_eq!(compact_text(&tree.root), "甲律师、乙实习律师");
    }
}
