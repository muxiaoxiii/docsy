//! Requires real qpdf: these tests fail explicitly if it is unavailable.
use super::*;
use crate::util::fs::{temp_named_path, TempDirGuard};
use lopdf::{dictionary, Document, Object, Stream};
use std::path::{Path, PathBuf};

fn fixture(pages: u32, bookmarks: bool) -> (TempDirGuard, PathBuf, PathBuf) {
    let dir = temp_named_path("docsy-freeze-test", "dir");
    std::fs::create_dir(&dir).unwrap();
    let guard = TempDirGuard::new(dir.clone()).unwrap();
    let path = dir.join("source.pdf");
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let cmap = doc.add_object(Stream::new(dictionary! {}, b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n1 beginbfchar\n<41> <0041>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n".to_vec()));
    let font = doc.add_object(dictionary! {"Type"=>"Font", "Subtype"=>"Type1", "BaseFont"=>"Helvetica", "ToUnicode"=>cmap});
    let resources = doc.add_object(dictionary! {"Font"=>dictionary! {"F1"=>font}});
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 20 20 Td (A) Tj ET".to_vec(),
    ));
    let mut ids = Vec::new();
    for _ in 0..pages {
        ids.push(doc.add_object(dictionary! {"Type"=>"Page", "Parent"=>pages_id, "MediaBox"=>vec![0.into(),0.into(),595.into(),842.into()], "Resources"=>resources, "Contents"=>content}));
    }
    doc.objects.insert(pages_id, Object::Dictionary(dictionary! {"Type"=>"Pages", "Count"=>pages, "Kids"=>ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>()}));
    let mut catalog = dictionary! {"Type"=>"Catalog", "Pages"=>pages_id};
    if bookmarks {
        let root = doc.new_object_id();
        let item = doc.add_object(dictionary! {"Title"=>Object::string_literal("Bookmark"), "Parent"=>root, "Dest"=>vec![Object::Reference(ids[0]), Object::Name(b"Fit".to_vec())]});
        doc.objects.insert(
            root,
            Object::Dictionary(
                dictionary! {"Type"=>"Outlines", "First"=>item,"Last"=>item,"Count"=>1},
            ),
        );
        catalog.set("Outlines", root);
    }
    let root = doc.add_object(catalog);
    let info =
        doc.add_object(dictionary! {"Title"=>Object::string_literal("Docsy freeze fixture")});
    doc.trailer.set("Root", root);
    doc.trailer.set("Info", info);
    // Unreachable bytes make successful optimization (changed=true) measurable.
    doc.add_object(Stream::new(dictionary! {}, vec![42; 20000]));
    doc.save(&path).unwrap();
    (guard, dir, path)
}

#[test]
fn optimize_preserves_bookmarks_info_and_real_anti_copy_recovery() {
    for method in [
        anti_ocr::AntiCopyMethod::CmapRemove,
        anti_ocr::AntiCopyMethod::CmapScramble,
    ] {
        let (_guard, dir, source) = fixture(2, true);
        let protected = dir.join("protected.pdf");
        assert_eq!(
            anti_ocr::apply_anti_copy(&source, &protected, method).unwrap(),
            1
        );
        let optimized = qpdf::optimize_lossless_to_dir(protected.to_str().unwrap(), None).unwrap();
        assert!(optimized.changed);
        let out = Document::load(&optimized.output_path).unwrap();
        assert!(out.catalog().unwrap().has(b"Outlines"));
        let info = out
            .get_dictionary(out.trailer.get(b"Info").unwrap().as_reference().unwrap())
            .unwrap();
        assert_eq!(
            info.get(b"Title").unwrap().as_str().unwrap(),
            b"Docsy freeze fixture"
        );
        assert!(
            anti_ocr::detect_anti_copy(Path::new(&optimized.output_path))
                .unwrap()
                .has_backup
        );
        let restored = dir.join("restored.pdf");
        assert_eq!(
            anti_ocr::remove_anti_copy(Path::new(&optimized.output_path), &restored).unwrap(),
            1
        );
        assert!(!anti_ocr::detect_anti_copy(&restored).unwrap().has_backup);
        assert!(Document::load(restored)
            .unwrap()
            .extract_text(&[1])
            .unwrap()
            .contains('A'));
    }
}

#[test]
fn actual_threshold_chunks_preserve_info_and_clean_success_failure_and_cancel() {
    let (_guard, dir, source) = fixture(301, false);
    assert!(!chunked::should_chunk(&source, 299));
    assert!(chunked::should_chunk(&source, 300));
    for count in [300, 301] {
        let output = dir.join(format!("success-{count}.pdf"));
        let mut work_dir = None;
        chunked::process_pdf_chunked(&source, &output, count, |input, out, _, _, _| {
            work_dir = input.parent().map(Path::to_path_buf);
            std::fs::copy(input, out)?;
            Ok(())
        })
        .unwrap();
        assert!(!work_dir.unwrap().exists());
        assert_eq!(qpdf::page_count(output.to_str().unwrap()).unwrap(), count);
        let doc = Document::load(&output).unwrap();
        assert!(doc.trailer.has(b"Info"));
    }
    for cancel in [false, true] {
        let token = tokio_util::sync::CancellationToken::new();
        let output = dir.join("unfinished.pdf");
        let mut work_dir = None;
        let result = crate::operations::with_current_cancel(token.clone(), || {
            chunked::process_pdf_chunked(&source, &output, 301, |input, out, _, _, _| {
                work_dir = input.parent().map(Path::to_path_buf);
                std::fs::copy(input, out)?;
                if cancel {
                    token.cancel();
                    Ok(())
                } else {
                    anyhow::bail!("injected failure")
                }
            })
        });
        assert!(result.is_err());
        assert!(!output.exists());
        assert!(!work_dir.unwrap().exists());
        assert!(crate::operations::check_current_cancelled().is_ok());
    }
    let big = dir.join("size-boundary.pdf");
    let file = std::fs::File::create(&big).unwrap();
    file.set_len(chunked::CHUNK_SIZE_THRESHOLD_BYTES).unwrap();
    assert!(chunked::should_chunk(&big, 1));
}

#[test]
fn unsupported_large_bookmarks_stop_before_processing() {
    let (_guard, dir, source) = fixture(300, true);
    let output = dir.join("blocked.pdf");
    let err = chunked::process_pdf_chunked(&source, &output, 300, |_, _, _, _, _| {
        panic!("must not process")
    })
    .unwrap_err();
    assert!(err.to_string().contains("文档结构"));
    assert!(!output.exists());
}

#[test]
fn new_anti_copy_backup_survives_chunk_font_renumbering() {
    let (_guard, dir, source) = fixture(301, false);
    let protected = dir.join("protected.pdf");
    anti_ocr::apply_anti_copy(&source, &protected, anti_ocr::AntiCopyMethod::CmapRemove).unwrap();
    let output = dir.join("chunked.pdf");
    chunked::process_pdf_chunked(&protected, &output, 301, |input, out, _, _, _| {
        std::fs::copy(input, out)?;
        Ok(())
    })
    .unwrap();
    let restored = dir.join("restored.pdf");
    assert!(anti_ocr::remove_anti_copy(&output, &restored).unwrap() > 0);
    assert!(
        Document::load(restored)
            .unwrap()
            .extract_text(&[1, 81, 161, 241, 301])
            .unwrap()
            .matches('A')
            .count()
            >= 5
    );
}

#[test]
fn legacy_recovery_backup_is_blocked_before_qpdf_can_renumber_fonts() {
    let (_guard, dir, source) = fixture(2, false);
    let mut doc = Document::load(&source).unwrap();
    let info = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    doc.get_object_mut(info)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set(
            "DocsyAntiCopyBackup",
            Object::string_literal(r#"{"cmaps":{"(4, 0)":"old-map"}}"#),
        );
    doc.save(&source).unwrap();
    let before = std::fs::read(&source).unwrap();
    let result = qpdf::optimize_lossless_to_dir(source.to_str().unwrap(), Some(&dir));
    assert!(result.err().unwrap().to_string().contains("旧版防复制"));
    assert_eq!(std::fs::read(source).unwrap(), before);
}
