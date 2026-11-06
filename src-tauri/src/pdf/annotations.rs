use anyhow::{Context, Result};
use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use super::{same_path, temp_named_path};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAnnotationsArgs {
    #[serde(alias = "input")]
    input_path: String,
    #[serde(alias = "output")]
    output_path: String,
    #[serde(default)]
    kinds: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAnnotationsResult {
    input_path: String,
    output_path: String,
    pub removed: usize,
    pub pages_touched: usize,
}

pub fn delete_annotations(args: &DeleteAnnotationsArgs) -> Result<DeleteAnnotationsResult> {
    delete_annotations_file(&args.input_path, &args.output_path, &args.kinds)
}

pub fn delete_annotations_file(
    input_path: &str,
    output_path: &str,
    kinds: &[String],
) -> Result<DeleteAnnotationsResult> {
    let input = Path::new(input_path);
    if !input.exists() {
        anyhow::bail!("PDF 不存在: {}", input.display());
    }
    let output = Path::new(output_path);
    if same_path(input, output) {
        anyhow::bail!("批注删除输出路径不能和原始 PDF 相同");
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).context("创建批注删除输出目录失败")?;
        }
    }

    let page_count = super::qpdf::page_count(input_path).unwrap_or(0);
    if page_count > 0 && super::chunked::should_chunk(input, page_count) {
        let mut removed = 0_usize;
        let mut pages_touched = 0_usize;
        super::chunked::process_pdf_chunked(
            input,
            output,
            page_count,
            |chunk_in, chunk_out, _, _, _| {
                let chunk_out_str = chunk_out.to_string_lossy().to_string();
                let result =
                    delete_annotations_whole(&chunk_in.to_string_lossy(), &chunk_out_str, kinds)?;
                removed += result.removed;
                pages_touched += result.pages_touched;
                Ok(())
            },
        )?;
        return Ok(DeleteAnnotationsResult {
            input_path: input_path.to_string(),
            output_path: output_path.to_string(),
            removed,
            pages_touched,
        });
    }

    delete_annotations_whole(input_path, output_path, kinds)
}

fn delete_annotations_whole(
    input_path: &str,
    output_path: &str,
    kinds: &[String],
) -> Result<DeleteAnnotationsResult> {
    let input = Path::new(input_path);
    let output = Path::new(output_path);
    let mut doc = Document::load(input).context("读取 PDF 失败")?;
    let targets = AnnotationKinds::from_user_values(kinds);
    let pages = doc
        .get_pages()
        .into_values()
        .map(|id| Ok((id, page_annotations(&doc, id)?)))
        .collect::<Result<Vec<_>>>()?;
    let mut removed_ids = BTreeSet::new();
    let mut dependents: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();

    // Include orphaned comments still reachable through Popup/IRT or structure
    // references. Page-array filtering alone leaves their text in the document.
    for (id, object) in &doc.objects {
        if targets.should_remove(&doc, object) {
            removed_ids.insert(*id);
        }
        let Some(dict) = annotation_dictionary(&doc, object) else {
            continue;
        };
        let kind = annotation_subtype(&doc, object).unwrap_or_default();
        if dict
            .get(b"Type")
            .ok()
            .and_then(|value| value.as_name().ok())
            == Some(b"OBJR")
        {
            if let Ok(target) = dict.get(b"Obj") {
                if let Some(parent_id) = reference_id(&doc, target) {
                    dependents.entry(parent_id).or_default().push(*id);
                }
            }
        }
        if !is_annotation_dictionary(&doc, dict) {
            continue;
        }
        if kind == "Popup" {
            if let Ok(parent) = dict.get(b"Parent") {
                if let Some(parent_id) = reference_id(&doc, parent) {
                    dependents.entry(parent_id).or_default().push(*id);
                }
            }
        }
        if is_review_kind(&normalize_kind(&kind)) {
            if let Ok(reply_to) = dict.get(b"IRT") {
                if let Some(parent_id) = reference_id(&doc, reply_to) {
                    dependents.entry(parent_id).or_default().push(*id);
                }
            }
        }
        if let Some(popup) = popup_id(&doc, dict) {
            dependents.entry(*id).or_default().push(popup);
        }
    }
    for (_, annots) in &pages {
        for annot in annots {
            if targets.should_remove(&doc, annot) {
                if let Some(dict) = annotation_dictionary(&doc, annot) {
                    if let Some(popup) = popup_id(&doc, dict) {
                        removed_ids.insert(popup);
                    }
                }
            }
        }
    }
    let mut pending: VecDeque<_> = removed_ids.iter().copied().collect();
    while let Some(id) = pending.pop_front() {
        for child in dependents.get(&id).into_iter().flatten() {
            if removed_ids.insert(*child) {
                pending.push_back(*child);
            }
        }
    }
    // Resolve reference aliases too, so no retained dictionary points at a
    // deleted comment through an intermediate reference object.
    loop {
        let aliases: Vec<_> = doc
            .objects
            .iter()
            .filter_map(|(id, object)| match object {
                Object::Reference(target) if removed_ids.contains(target) => Some(*id),
                _ => None,
            })
            .collect();
        let before = removed_ids.len();
        removed_ids.extend(aliases);
        if removed_ids.len() == before {
            break;
        }
    }
    let mut direct_removed = 0;
    let mut pages_touched = 0;
    for (page_id, annots) in pages {
        let original_len = annots.len();
        let kept: Vec<Object> = annots
            .into_iter()
            .filter(|annot| {
                let remove = targets.should_remove(&doc, annot)
                    || reference_id(&doc, annot).is_some_and(|id| removed_ids.contains(&id))
                    || annotation_dictionary(&doc, annot).is_some_and(|dict| {
                        if !is_annotation_dictionary(&doc, dict) {
                            return false;
                        }
                        let kind = annotation_subtype(&doc, annot).unwrap_or_default();
                        let relation = if kind == "Popup" {
                            b"Parent".as_slice()
                        } else if is_review_kind(&normalize_kind(&kind)) {
                            b"IRT".as_slice()
                        } else {
                            return false;
                        };
                        dict.get(relation)
                            .ok()
                            .and_then(|value| reference_id(&doc, value))
                            .is_some_and(|id| removed_ids.contains(&id))
                    });
                if remove && !matches!(annot, Object::Reference(_)) {
                    direct_removed += 1;
                }
                !remove
            })
            .collect();
        if kept.len() == original_len {
            continue;
        }
        pages_touched += 1;
        let page = doc.get_object_mut(page_id)?.as_dict_mut()?;
        if kept.is_empty() {
            page.remove(b"Annots");
        } else {
            page.set("Annots", Object::Array(kept));
        }
    }
    // Remove back-references from retained annotations and tagged-PDF metadata
    // before dropping the objects. Stream bytes (page text/images) are untouched.
    if !removed_ids.is_empty() {
        for (id, object) in &mut doc.objects {
            if !removed_ids.contains(id) {
                strip_removed_references(object, &removed_ids, 0)?;
            }
        }
        strip_dictionary_references(&mut doc.trailer, &removed_ids, 0)?;
        doc.objects.retain(|id, _| !removed_ids.contains(id));
    }
    let removed = removed_ids.len() + direct_removed;

    doc.prune_objects();
    doc.save(output).context("保存删除批注后的 PDF 失败")?;

    Ok(DeleteAnnotationsResult {
        input_path: input_path.to_string(),
        output_path: output_path.to_string(),
        removed,
        pages_touched,
    })
}

pub fn delete_annotations_to_temp(
    input_path: &str,
    kinds: &[String],
) -> Result<(PathBuf, DeleteAnnotationsResult)> {
    let output = temp_named_path("docsy_annotations_removed", "pdf");
    match delete_annotations_file(input_path, &output.to_string_lossy(), kinds) {
        Ok(result) => Ok((output, result)),
        Err(error) => {
            let _ = std::fs::remove_file(output);
            Err(error)
        }
    }
}

fn page_annotations(doc: &Document, page_id: ObjectId) -> Result<Vec<Object>> {
    let page = doc.get_object(page_id)?.as_dict()?;
    let Ok(annots) = page.get(b"Annots") else {
        return Ok(Vec::new());
    };
    Ok(doc
        .dereference(annots)
        .context("批注列表引用无效")?
        .1
        .as_array()
        .context("批注列表不是数组，已停止处理")?
        .clone())
}

fn annotation_dictionary<'a>(doc: &'a Document, annot: &'a Object) -> Option<&'a Dictionary> {
    doc.dereference(annot).ok()?.1.as_dict().ok()
}

fn is_annotation_dictionary(doc: &Document, dict: &Dictionary) -> bool {
    match dict.get(b"Type") {
        Err(_) => true, // /Type is optional on an annotation dictionary.
        Ok(value) => doc
            .dereference(value)
            .ok()
            .and_then(|(_, value)| value.as_name().ok())
            .is_some_and(|kind| kind == b"Annot"),
    }
}

fn reference_id(doc: &Document, object: &Object) -> Option<ObjectId> {
    doc.dereference(object).ok()?.0
}

fn popup_id(doc: &Document, dict: &Dictionary) -> Option<ObjectId> {
    let popup = dict.get(b"Popup").ok()?;
    (annotation_subtype(doc, popup).as_deref() == Some("Popup"))
        .then(|| reference_id(doc, popup))
        .flatten()
}

fn strip_dictionary_references(
    dict: &mut Dictionary,
    removed: &BTreeSet<ObjectId>,
    depth: usize,
) -> Result<()> {
    let keys: Vec<_> = dict
        .iter()
        .filter_map(|(key, value)| {
            matches!(value, Object::Reference(id) if removed.contains(id)).then(|| key.clone())
        })
        .collect();
    for key in keys {
        dict.remove(&key);
    }
    for (_, value) in dict.iter_mut() {
        strip_removed_references(value, removed, depth + 1)?;
    }
    Ok(())
}

fn strip_removed_references(
    object: &mut Object,
    removed: &BTreeSet<ObjectId>,
    depth: usize,
) -> Result<()> {
    anyhow::ensure!(depth < 128, "PDF 对象嵌套过深，无法安全清理批注引用");
    match object {
        Object::Array(values) => {
            values.retain(|value| !matches!(value, Object::Reference(id) if removed.contains(id)));
            for value in values {
                strip_removed_references(value, removed, depth + 1)?;
            }
        }
        Object::Dictionary(dict) => strip_dictionary_references(dict, removed, depth)?,
        Object::Stream(stream) => strip_dictionary_references(&mut stream.dict, removed, depth)?,
        Object::Reference(id) if removed.contains(id) => *object = Object::Null,
        _ => {}
    }
    Ok(())
}

const REVIEW_KINDS: &[&str] = &[
    "text",
    "freetext",
    "highlight",
    "underline",
    "strikeout",
    "squiggly",
    "ink",
    "square",
    "circle",
    "line",
    "polygon",
    "polyline",
    "caret",
];

fn is_review_kind(kind: &str) -> bool {
    REVIEW_KINDS.contains(&kind)
}

#[derive(Debug, Clone)]
struct AnnotationKinds {
    values: BTreeSet<String>,
}

impl AnnotationKinds {
    fn from_user_values(values: &[String]) -> Self {
        let values: BTreeSet<_> = if values.is_empty() {
            REVIEW_KINDS.iter().map(|kind| kind.to_string()).collect()
        } else {
            // Legacy/custom filters must never turn this switch into stamp,
            // signature-widget or link deletion.
            values
                .iter()
                .map(normalize_kind)
                .filter(|kind| is_review_kind(kind))
                .collect()
        };
        Self { values }
    }

    fn should_remove(&self, doc: &Document, annot: &Object) -> bool {
        let Some(dict) = annotation_dictionary(doc, annot) else {
            return false;
        };
        if !is_annotation_dictionary(doc, dict) {
            return false;
        }
        let Some(kind) = annotation_subtype(doc, annot) else {
            return false;
        };
        self.values.contains(&normalize_kind(&kind))
    }
}

fn annotation_subtype(doc: &Document, annot: &Object) -> Option<String> {
    let dict = annotation_dictionary(doc, annot)?;
    let subtype = doc.dereference(dict.get(b"Subtype").ok()?).ok()?.1;
    subtype
        .as_name()
        .ok()
        .map(|name| String::from_utf8_lossy(name).to_string())
}

fn normalize_kind(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::Dictionary;

    #[test]
    fn normalizes_annotation_kinds() {
        assert_eq!(normalize_kind("/Highlight"), "highlight");
        assert_eq!(normalize_kind("FreeText"), "freetext");
    }

    #[test]
    fn default_kinds_remove_review_annotations_not_links() {
        let doc = Document::with_version("1.7");
        let targets = AnnotationKinds::from_user_values(&[]);
        let mut highlight = Dictionary::new();
        highlight.set("Subtype", Object::Name(b"Highlight".to_vec()));
        let mut link = Dictionary::new();
        link.set("Subtype", Object::Name(b"Link".to_vec()));

        assert!(targets.should_remove(&doc, &Object::Dictionary(highlight)));
        assert!(!targets.should_remove(&doc, &Object::Dictionary(link)));
    }

    fn fixture(path: &Path, indirect: bool) -> (ObjectId, ObjectId, ObjectId, Vec<u8>) {
        use lopdf::{dictionary, Stream};
        let mut doc = Document::with_version("1.7");
        let pages = doc.new_object_id();
        let page = doc.new_object_id();
        let body = b"0 0 0 rg 20 20 40 40 re f\n% FLATTENED_BODY_MARK\n".to_vec();
        let contents = doc.add_object(Stream::new(dictionary! {}, body.clone()));
        let mut appearance = |rgb: &str| {
            doc.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form",
                    "BBox" => vec![0.into(), 0.into(), 40.into(), 40.into()],
                },
                format!("{rgb} rg 0 0 40 40 re f").into_bytes(),
            ))
        };
        let yellow = appearance("1 1 0");
        let red = appearance("1 0 0");
        let blue = appearance("0 0 1");
        let highlight = doc.new_object_id();
        let popup = doc.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Popup",
        "Parent" => highlight, "P" => page, "Open" => true,
        "Rect" => vec![10.into(), 200.into(), 120.into(), 250.into()]});
        doc.objects.insert(highlight, Object::Dictionary(dictionary! {
            "Type" => "Annot", "Subtype" => "Highlight", "P" => page, "F" => 4,
            "Contents" => Object::string_literal("PRIVATE_COMMENT"), "Popup" => popup,
            "Rect" => vec![20.into(), 160.into(), 60.into(), 200.into()],
            "QuadPoints" => vec![20.into(),200.into(),60.into(),200.into(),20.into(),160.into(),60.into(),160.into()],
            "AP" => dictionary! {"N" => yellow}}));
        let reply = doc.add_object(
            dictionary! {"Type" => "Annot", "Subtype" => "Text", "IRT" => highlight,
            "P" => page, "Contents" => Object::string_literal("PRIVATE_REPLY"),
            "Rect" => vec![70.into(), 160.into(), 90.into(), 180.into()]},
        );
        let stamp = doc.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Stamp", "P" => page,
            "F" => 4, "IRT" => highlight,
            "Rect" => vec![160.into(), 160.into(), 200.into(), 200.into()], "AP" => dictionary! {"N" => red}});
        let widget = doc.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Widget", "P" => page,
            "FT" => "Sig", "T" => Object::string_literal("Signature"), "F" => 4,
            "Rect" => vec![225.into(), 160.into(), 265.into(), 200.into()], "AP" => dictionary! {"N" => blue}});
        let link = doc.add_object(dictionary! {"Type" => "Annot", "Subtype" => "Link", "P" => page,
            "Rect" => vec![20.into(), 240.into(), 60.into(), 260.into()],
            "A" => dictionary! {"S" => "URI", "URI" => Object::string_literal("https://example.com")}});
        let objr = doc.add_object(dictionary! {"Type" => "OBJR", "Obj" => highlight, "Pg" => page});
        let structure =
            doc.add_object(dictionary! {"Type" => "StructTreeRoot", "K" => vec![objr.into()]});
        let mut annots = vec![
            highlight.into(),
            popup.into(),
            reply.into(),
            stamp.into(),
            widget.into(),
            link.into(),
        ];
        for kind in [
            "FreeText",
            "Underline",
            "StrikeOut",
            "Squiggly",
            "Ink",
            "Square",
            "Circle",
            "Line",
            "Polygon",
            "PolyLine",
            "Caret",
        ] {
            let subtype = doc.add_object(Object::Name(kind.as_bytes().to_vec()));
            let dict = dictionary! {"Type" => "Annot", "Subtype" => subtype, "P" => page,
            "Rect" => vec![100.into(), 160.into(), 140.into(), 200.into()], "F" => 4, "AP" => dictionary! {"N" => yellow}};
            if kind == "Underline" {
                annots.push(Object::Dictionary(dict));
            } else {
                annots.push(doc.add_object(dict).into());
            }
        }
        let annots = if indirect {
            doc.add_object(Object::Array(annots)).into()
        } else {
            Object::Array(annots)
        };
        doc.objects.insert(
            page,
            Object::Dictionary(dictionary! {"Type" => "Page", "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), 300.into(), 300.into()], "Contents" => contents,
            "Resources" => dictionary! {}, "Annots" => annots}),
        );
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! {"Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1},
            ),
        );
        let root = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages,
            "AcroForm" => dictionary! {"Fields" => vec![widget.into()]}, "StructTreeRoot" => structure});
        doc.trailer.set("Root", root);
        doc.save(path).unwrap();
        (highlight, popup, stamp, body)
    }

    fn test_folder() -> crate::util::fs::TempDirGuard {
        let dir = temp_named_path("docsy_annotation_regression", "dir");
        std::fs::create_dir_all(&dir).unwrap();
        crate::util::fs::TempDirGuard::new(dir).unwrap()
    }

    #[test]
    fn file_cleanup_removes_marks_comments_and_references_but_keeps_stamps_and_body() {
        for indirect in [false, true] {
            let dir = test_folder();
            let input = dir.path().join("source.pdf");
            let output = dir.path().join("clean.pdf");
            let (comment, popup, stamp, _) = fixture(&input, indirect);
            let source = Document::load(&input).unwrap();
            let body = source.get_page_content(source.get_pages()[&1]);
            let original = std::fs::read(&input).unwrap();
            let result =
                delete_annotations_file(input.to_str().unwrap(), output.to_str().unwrap(), &[])
                    .unwrap();
            assert_eq!(result.removed, 15); // 13 marks, popup and tagged-PDF OBJR
            assert_eq!(result.pages_touched, 1);
            let doc = Document::load(&output).unwrap();
            let annots = page_annotations(&doc, doc.get_pages()[&1]).unwrap();
            let kinds: Vec<_> = annots
                .iter()
                .filter_map(|a| annotation_subtype(&doc, a))
                .collect();
            assert_eq!(kinds, vec!["Stamp", "Widget", "Link"]);
            assert!(!doc.objects.contains_key(&comment));
            assert!(!doc.objects.contains_key(&popup));
            assert!(doc.get_dictionary(stamp).unwrap().get(b"IRT").is_err());
            assert_eq!(doc.get_page_content(doc.get_pages()[&1]), body);
            assert_eq!(std::fs::read(input).unwrap(), original);
            for object in doc.objects.values() {
                if let Some(dict) = annotation_dictionary(&doc, object) {
                    assert!(dict
                        .get(b"Contents")
                        .ok()
                        .and_then(|v| v.as_str().ok())
                        .is_none());
                }
            }
            let again = dir.path().join("again.pdf");
            let result =
                delete_annotations_file(output.to_str().unwrap(), again.to_str().unwrap(), &[])
                    .unwrap();
            assert_eq!(result.removed, 0);
            assert_eq!(result.pages_touched, 0);
        }
    }

    #[test]
    fn legacy_filters_cannot_remove_stamps_widgets_or_links_and_replies_follow_their_parent() {
        let dir = test_folder();
        let input = dir.path().join("source.pdf");
        let output = dir.path().join("clean.pdf");
        fixture(&input, true);
        let kinds = ["Stamp", "Widget", "Link", "/Highlight"].map(String::from);
        let result =
            delete_annotations_file(input.to_str().unwrap(), output.to_str().unwrap(), &kinds)
                .unwrap();
        assert_eq!(result.removed, 4); // highlight, popup, reply, OBJR
        let doc = Document::load(output).unwrap();
        let annots = page_annotations(&doc, doc.get_pages()[&1]).unwrap();
        let kinds: Vec<_> = annots
            .iter()
            .filter_map(|a| annotation_subtype(&doc, a))
            .collect();
        assert!(kinds.contains(&"Stamp".to_string()));
        assert!(kinds.contains(&"Widget".to_string()));
        assert!(kinds.contains(&"Link".to_string()));
        assert!(kinds.contains(&"Square".to_string()));
        assert!(!kinds.contains(&"Popup".to_string()));
        assert!(!kinds.contains(&"Text".to_string()));
    }

    #[test]
    fn orphan_popup_cannot_keep_a_previously_deleted_comment_alive() {
        let dir = test_folder();
        let input = dir.path().join("source.pdf");
        let output = dir.path().join("clean.pdf");
        let (comment, popup, _, _) = fixture(&input, false);
        let mut doc = Document::load(&input).unwrap();
        let page = doc.get_pages()[&1];
        let mut annots = page_annotations(&doc, page).unwrap();
        annots.retain(|a| reference_id(&doc, a) != Some(comment));
        doc.get_dictionary_mut(page).unwrap().set("Annots", annots);
        doc.save(&input).unwrap();
        delete_annotations_file(input.to_str().unwrap(), output.to_str().unwrap(), &[]).unwrap();
        let doc = Document::load(output).unwrap();
        assert!(!doc.objects.contains_key(&comment));
        assert!(!doc.objects.contains_key(&popup));
    }

    #[test]
    fn invalid_annotation_lists_fail_instead_of_reporting_a_no_op_success() {
        let dir = test_folder();
        let input = dir.path().join("source.pdf");
        let output = dir.path().join("clean.pdf");
        fixture(&input, false);
        let mut doc = Document::load(&input).unwrap();
        let page = doc.get_pages()[&1];
        doc.get_dictionary_mut(page).unwrap().set("Annots", 42);
        doc.save(&input).unwrap();
        std::fs::write(&output, b"existing-output").unwrap();
        assert!(
            delete_annotations_file(input.to_str().unwrap(), output.to_str().unwrap(), &[])
                .is_err()
        );
        assert_eq!(std::fs::read(output).unwrap(), b"existing-output");
    }

    #[test]
    fn evidence_merge_deletes_marks_before_flattening_stamp_and_signature_appearances() {
        let dir = test_folder();
        let input = dir.path().join("source.pdf");
        let output = dir.path().join("clean.pdf");
        let merged = dir.path().join("merged.pdf");
        fixture(&input, true);
        let original = std::fs::read(&input).unwrap();
        let args = serde_json::from_value(serde_json::json!({
            "items": [{"inputPath": input, "outputPath": output}],
            "session": {"annotationRule": {"removeAnnotations": true}},
            "merge": {"enabled": true, "outputMode": "files_and_merge", "outputPath": merged},
            "optimizeOutput": true,
        }))
        .unwrap();
        let result = super::super::evidence_session::apply_rules_cancellable(
            &args,
            &tokio_util::sync::CancellationToken::new(),
            &|_| {},
        )
        .unwrap();
        assert!(result.failed.is_empty(), "{:?}", result.failed);
        assert_eq!(result.results[0].annotations_removed, Some(15));
        assert_eq!(result.results[0].annotation_pages_touched, Some(1));
        assert_eq!(result.results[0].input_path, input.to_string_lossy());
        assert_eq!(result.merge.status.as_deref(), Some("done"));
        assert_eq!(std::fs::read(input).unwrap(), original);
        let doc = Document::load(&merged).unwrap();
        for (_, object) in &doc.objects {
            if let Some(kind) = annotation_subtype(&doc, object) {
                assert!(!is_review_kind(&normalize_kind(kind)));
            }
        }
        for page in doc.get_pages().into_values() {
            for annot in page_annotations(&doc, page).unwrap() {
                assert!(!matches!(
                    annotation_subtype(&doc, &annot).as_deref(),
                    Some("Stamp" | "Widget" | "Popup")
                ));
            }
        }
        // Verify the printed pixels, not merely the absence of annotation objects.
        let renderer = crate::external::PopplerTool::binary_path_for("pdftoppm").unwrap();
        let prefix = dir.path().join("render");
        let render = crate::external::hidden_command(renderer)
            .args(["-png", "-singlefile", "-r", "72"])
            .arg(&merged)
            .arg(&prefix)
            .output()
            .unwrap();
        assert!(
            render.status.success(),
            "{}",
            String::from_utf8_lossy(&render.stderr)
        );
        let image = image::open(prefix.with_extension("png")).unwrap().to_rgb8();
        assert_eq!(image.dimensions(), (300, 300));
        assert_eq!(
            image.get_pixel(180, 120).0,
            [255, 0, 0],
            "stamp must remain red"
        );
        assert_eq!(
            image.get_pixel(245, 120).0,
            [0, 0, 255],
            "signature appearance must remain blue"
        );
        assert_eq!(
            image.get_pixel(40, 120).0,
            [255, 255, 255],
            "highlight must disappear"
        );
        assert_eq!(
            image.get_pixel(35, 265).0,
            [0, 0, 0],
            "original page content must remain"
        );
    }

    #[test]
    fn chunked_cleanup_preserves_stamps_on_first_and_last_pages() {
        use lopdf::{dictionary, Stream};
        let dir = test_folder();
        let input = dir.path().join("large.pdf");
        let output = dir.path().join("clean.pdf");
        let mut doc = Document::with_version("1.7");
        let pages = doc.new_object_id();
        let appearance = doc.add_object(Stream::new(dictionary! {
            "Type" => "XObject", "Subtype" => "Form", "BBox" => vec![0.into(),0.into(),20.into(),20.into()],
        }, b"1 0 0 rg 0 0 20 20 re f".to_vec()));
        let body = doc.add_object(Stream::new(
            dictionary! {},
            b"0 0 0 rg 20 20 40 40 re f".to_vec(),
        ));
        let mut kids = Vec::new();
        for number in 1..=301 {
            let page = doc.new_object_id();
            let mut annots = Vec::<Object>::new();
            if number == 1 || number == 301 {
                for kind in ["Highlight", "Stamp"] {
                    annots.push(
                        doc.add_object(dictionary! {"Type" => "Annot", "Subtype" => kind,
                        "P" => page, "Rect" => vec![100.into(),100.into(),120.into(),120.into()],
                        "AP" => dictionary! {"N" => appearance}, "F" => 4})
                            .into(),
                    );
                }
            }
            doc.objects.insert(
                page,
                Object::Dictionary(dictionary! {"Type" => "Page", "Parent" => pages,
                "MediaBox" => vec![0.into(),0.into(),300.into(),300.into()], "Contents" => body,
                "Resources" => dictionary! {}, "Annots" => annots}),
            );
            kids.push(page.into());
        }
        doc.objects.insert(
            pages,
            Object::Dictionary(dictionary! {"Type" => "Pages", "Kids" => kids, "Count" => 301}),
        );
        let catalog = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
        doc.trailer.set("Root", catalog);
        doc.save(&input).unwrap();
        let original = std::fs::read(&input).unwrap();
        let result =
            delete_annotations_file(input.to_str().unwrap(), output.to_str().unwrap(), &[])
                .unwrap();
        assert_eq!(result.removed, 2);
        assert_eq!(result.pages_touched, 2);
        let doc = Document::load(output).unwrap();
        let pages = doc.get_pages();
        assert_eq!(pages.len(), 301);
        for number in [1, 301] {
            let annots = page_annotations(&doc, pages[&number]).unwrap();
            assert_eq!(annots.len(), 1);
            assert_eq!(
                annotation_subtype(&doc, &annots[0]).as_deref(),
                Some("Stamp")
            );
            assert!(
                String::from_utf8_lossy(&doc.get_page_content(pages[&number]))
                    .contains("40 40 re f")
            );
        }
        assert_eq!(std::fs::read(input).unwrap(), original);
    }
}
