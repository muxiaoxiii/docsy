use anyhow::{Context, Result};
use std::collections::HashMap;

use serde_json::Value;

use crate::docx_template::ooxml::{XmlNode, XmlTree};

use super::{OptionalFieldRule, StructureOverride, TemplateField, TemplateManifest};

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
) -> Result<Vec<(String, Vec<u8>)>> {
    let tag_map = build_tag_map(manifest);

    let mut results = Vec::new();
    for (part_name, xml_bytes) in package_xml {
        if !super::is_word_xml_part(part_name) {
            results.push((part_name.clone(), xml_bytes.clone()));
            continue;
        }

        let mut tree = XmlTree::parse(xml_bytes)?;
        render_tree(&mut tree.root, &tag_map, values, structure_overrides)?;

        let out_xml = tree.to_xml()?;
        results.push((part_name.clone(), out_xml.into_bytes()));
    }
    Ok(results)
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
) -> Result<()> {
    if let XmlNode::Element { children, .. } = node {
        // Process children right-to-left so splice doesn't invalidate indices
        let mut i = children.len();
        while i > 0 {
            i -= 1;

            // Table row replication: if child is w:tr with party_list sdt, expand it
            if let XmlNode::Element { name, .. } = &children[i] {
                if name == "w:tr" {
                    if let Some(new_rows) =
                        try_expand_table_row(children, i, tag_map, values, overrides)?
                    {
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
                            let value = value_for_field(values, field)
                                .cloned()
                                .unwrap_or(Value::Null);
                            let rendered = rendered_base_text(field, *slot, &value);

                            if rendered.is_empty() {
                                if field.field_type == "party_list" {
                                    // Only the list separator is removable here. A comma
                                    // separating legal roles remains unless it is part of
                                    // the explicit optional prefix below.
                                    strip_party_separator_before(children, i);
                                }
                                if let Some(rule) = optional_rule_for_slot(field, *slot) {
                                    if rule.enabled {
                                        strip_prefix_before(children, i, &rule.remove_empty_prefix);
                                        strip_suffix_after(children, i, &rule.remove_empty_suffix);
                                        if field.field_type == "party_list"
                                            && is_party_role_prefix(&rule.remove_empty_prefix)
                                        {
                                            strip_role_separator_before(children, i);
                                        }
                                    }
                                }
                            } else {
                                apply_party_item_suffix(children, i, field, *slot, &value);
                                let sdt_index =
                                    apply_structure_override(children, i, field, *slot, overrides);
                                replace_sdt_content(
                                    &mut children[sdt_index],
                                    field,
                                    *slot,
                                    &tag,
                                    &value,
                                )?;
                                let sdt = std::mem::replace(
                                    &mut children[sdt_index],
                                    XmlNode::Text(String::new()),
                                );
                                let content = unwrap_sdt_content(sdt);
                                children.splice(sdt_index..sdt_index + 1, content);
                                continue;
                            }

                            replace_sdt_content(&mut children[i], field, *slot, &tag, &value)?;
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
            render_tree(&mut children[i], tag_map, values, overrides)?;
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
) -> Result<Option<Vec<XmlNode>>> {
    let row_xml = {
        let row = &children[idx];
        if !matches!(row, XmlNode::Element { name, .. } if name == "w:tr") {
            return Ok(None);
        }
        XmlTree { root: row.clone() }
            .to_xml()
            .context("表格行: 序列化失败")?
    };

    // Find the first party_list field with >1 items
    let sdt_tags = collect_sdt_tags_in_tree_simple(&children[idx]);
    for tag in sdt_tags {
        let Some((field, _)) = tag_map.get(&tag) else {
            continue;
        };
        if field.field_type == "party_list" {
            let value = value_for_field(values, field)
                .cloned()
                .unwrap_or(Value::Null);
            let items = party_items(&value);
            if items.len() > 1 {
                let mut new_rows = Vec::new();
                for item in &items {
                    let mut item_values = values.clone();
                    item_values.insert(
                        field.id.clone(),
                        serde_json::json!({ "text": item.text, "suffix": item.suffix }),
                    );
                    let mut row_tree =
                        XmlTree::parse(row_xml.as_bytes()).context("表格行复制: 解析失败")?;
                    render_tree(&mut row_tree.root, tag_map, &item_values, overrides)?;
                    new_rows.push(row_tree.root);
                }
                return Ok(Some(new_rows));
            }
        }
    }
    Ok(None)
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
                let text = rendered_text_for_field(field, slot, tag, value);
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
) -> String {
    if matches!(
        field.field_type.as_str(),
        "checkbox" | "radio_group" | "checkbox_group"
    ) {
        marker_text_for_tag(field, tag, value)
    } else {
        rendered_base_text(field, slot, value)
    }
}

fn rendered_base_text(field: &TemplateField, slot: Option<usize>, value: &Value) -> String {
    if field.field_type == "party_list" {
        let items = party_items(value);
        return match (field.mark_refs.len(), slot) {
            (_, None) => items
                .iter()
                .map(PartyItem::rendered)
                .collect::<Vec<_>>()
                .join("、"),
            (0..=1, Some(index)) => items
                .iter()
                .map(|item| render_party_item(field, index, item, items.len() > 1))
                .collect::<Vec<_>>()
                .join("、"),
            (count, Some(index)) if index >= count => String::new(),
            (count, Some(index)) if index + 1 == count && items.len() > count => {
                // The last source slot carries the overflow. Its static suffix
                // can only appear once, so include each selected suffix inline.
                items[index..]
                    .iter()
                    .map(PartyItem::rendered)
                    .collect::<Vec<_>>()
                    .join("、")
            }
            (_, Some(index)) => items
                .get(index)
                .map(|item| render_party_item(field, index, item, false))
                .unwrap_or_default(),
        };
    }
    let text = scalar_value(value);
    // A field may have multiple document positions. A later occurrence must be
    // made an explicit reference rather than silently duplicating a value.
    if slot.unwrap_or(0) > 0 && field.mark_refs.len() > 1 {
        String::new()
    } else {
        text
    }
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
fn apply_structure_override(
    siblings: &mut Vec<XmlNode>,
    at: usize,
    field: &TemplateField,
    slot: Option<usize>,
    overrides: &HashMap<String, StructureOverride>,
) -> usize {
    let Some(override_) = overrides.get(&field.id) else {
        return at;
    };
    let ref_count = field.mark_refs.len();
    let index = slot.unwrap_or(0);
    let is_first = ref_count <= 1 || index == 0;
    let is_last = ref_count <= 1 || index + 1 >= ref_count;
    let rule = optional_rule_for_slot(field, slot);

    let mut sdt_index = at;
    if is_first {
        if let Some(prefix) = override_.prefix.as_deref() {
            let source = rule
                .map(|item| item.remove_empty_prefix.as_str())
                .unwrap_or("");
            if replace_prefix_before(siblings, at, source, prefix) {
                sdt_index += 1;
            }
        }
    }
    if is_last {
        if let Some(suffix) = override_.suffix.as_deref() {
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
    for index in (0..at).rev() {
        let text = collect_text_from_element(&siblings[index]);
        if text.is_empty() {
            continue;
        }
        if text.trim() == "、" {
            siblings[index] = XmlNode::Text(String::new());
        }
        return;
    }
}

fn is_party_role_prefix(prefix: &str) -> bool {
    matches!(
        prefix.trim(),
        "原告" | "被告" | "第三人" | "上诉人" | "被上诉人" | "申请人" | "被申请人"
    )
}

fn strip_role_separator_before(siblings: &mut [XmlNode], at: usize) {
    for index in (0..at).rev() {
        let text = collect_text_from_element(&siblings[index]);
        if text.is_empty() {
            continue;
        }
        if matches!(text.trim(), "，" | ",") {
            siblings[index] = XmlNode::Text(String::new());
        }
        return;
    }
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
                    new_children.push(XmlNode::Element {
                        name: "w:t".to_string(),
                        attrs: vec![("xml:space".to_string(), "preserve".to_string())],
                        children: vec![XmlNode::Text(text.to_string())],
                    });
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
) -> Option<&'a Value> {
    values.get(&field.id).or_else(|| values.get(&field.name))
}

#[derive(Debug, Clone)]
struct PartyItem {
    text: String,
    suffix: String,
}

impl PartyItem {
    fn rendered(&self) -> String {
        format!("{}{}", self.text, self.suffix)
    }
}

fn render_party_item(
    field: &TemplateField,
    slot: usize,
    item: &PartyItem,
    force_inline_suffix: bool,
) -> String {
    let has_static_suffix = optional_rule_for_slot(field, Some(slot))
        .is_some_and(|rule| !rule.remove_empty_suffix.is_empty());
    if has_static_suffix && !force_inline_suffix {
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
                text: text.to_string(),
                suffix: String::new(),
            })
            .collect(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(value) if !value.trim().is_empty() => Some(PartyItem {
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
                        text: text.to_string(),
                        suffix: values
                            .get("suffix")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    })
                }
                Value::Number(value) => Some(PartyItem {
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
        text: text.to_string(),
        suffix: values
            .get("suffix")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

fn apply_party_item_suffix(
    siblings: &mut Vec<XmlNode>,
    at: usize,
    field: &TemplateField,
    slot: Option<usize>,
    value: &Value,
) {
    if field.field_type != "party_list" {
        return;
    }
    let Some(rule) = optional_rule_for_slot(field, slot) else {
        return;
    };
    if rule.remove_empty_suffix.is_empty() {
        return;
    }

    let items = party_items(value);
    let index = slot.unwrap_or(0);
    let count = field.mark_refs.len();
    if index >= items.len() {
        return;
    }
    if index + 1 >= count && items.len() > count.max(1) {
        // Overflow suffixes are already embedded in rendered_base_text.
        replace_suffix_after(siblings, at, &rule.remove_empty_suffix, "");
        return;
    }
    replace_suffix_after(
        siblings,
        at,
        &rule.remove_empty_suffix,
        &items[index].suffix,
    );
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

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new()).unwrap();
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

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new()).unwrap();
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
        render_tree(&mut tree.root, &fm, &HashMap::new(), &HashMap::new()).unwrap();
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
        render_tree(&mut tree.root, &fm, &HashMap::new(), &HashMap::new()).unwrap();
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

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new()).unwrap();
        let out = tree.to_xml().unwrap();

        assert!(out.contains("张三"), "first item rendered");
        assert!(out.contains("李四"), "second item rendered");
        assert!(
            out.contains("<w:b"),
            "bold format preserved in replicated rows"
        );
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

        render_tree(&mut tree.root, &fm, &vals, &HashMap::new()).unwrap();
        let out = tree.to_xml().unwrap();
        assert!(out.contains("plain text"), "unrelated text preserved");
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

        render_tree(&mut tree.root, &field_map(&m), &values, &HashMap::new()).unwrap();
        let out = tree.to_xml().unwrap();
        assert!(out.contains("李琼律师"));
        assert!(out.contains("吕晗实习律师"));
        assert!(!out.contains("{{lawyers"));
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

        render_tree(&mut tree.root, &field_map(&m), &values, &HashMap::new()).unwrap();
        let out = tree.to_xml().unwrap();
        assert_eq!(out.matches("原告甲").count(), 1);
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
            },
        );

        render_tree(
            &mut tree.root,
            &field_map(&manifest(vec![field])),
            &values,
            &overrides,
        )
        .unwrap();
        let out = tree.to_xml().unwrap();
        assert!(out.contains(">申请人<"));
        assert!(out.contains(">甲公司<"));
        assert!(!out.contains(">原告<"));
        assert!(!out.contains("w:sdt"));
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
                }),
                ..Default::default()
            },
            super::super::TemplateMarkRef {
                tag: "lawyers.ref.2".to_string(),
                optional_rule: Some(OptionalFieldRule {
                    enabled: true,
                    remove_empty_prefix: String::new(),
                    remove_empty_suffix: "实习律师".to_string(),
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
        )
        .unwrap();
        assert_eq!(compact_text(&tree.root), "甲律师、乙实习律师");
    }
}
