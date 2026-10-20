use crate::pdf::backend::*;
use lopdf::{dictionary, Document, Object, Stream};
use std::path::{Path,PathBuf};
/// 辅助函数：使用 lopdf 创建指定页数和尺寸的基础测试 PDF。
fn create_test_pdf(path: &Path, page_count: usize, width: f64, height: f64) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });

    let mut page_ids = Vec::new();
    for i in 1..=page_count {
        let text = format!("Page {i}");
        let content = lopdf::content::Content {
            operations: vec![
                lopdf::content::Operation::new("BT", vec![]),
                lopdf::content::Operation::new(
                    "Tf",
                    vec![Object::Name(b"F1".to_vec()), 12.into()],
                ),
                lopdf::content::Operation::new("Td", vec![50.into(), 500.into()]),
                lopdf::content::Operation::new("Tj", vec![Object::string_literal(text)]),
                lopdf::content::Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), height.into()],
        });
        page_ids.push(Object::Reference(page_id));
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids,
            "Count" => page_count as i64,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("保存测试 PDF 失败");
}

/// 辅助函数：创建带同名表单字段的测试 PDF。
fn create_form_test_pdf(path: &Path, field_name: &str, field_val: &str) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let field_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        "Annots" => vec![Object::Reference(field_id)],
    });

    doc.objects.insert(
        field_id,
        Object::Dictionary(dictionary! {
            "Type" => "Annot",
            "Subtype" => "Widget",
            "FT" => "Tx",
            "T" => Object::string_literal(field_name),
            "V" => Object::string_literal(field_val),
            "Rect" => vec![50.into(), 700.into(), 200.into(), 730.into()],
            "P" => Object::Reference(page_id),
        }),
    );

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        }),
    );

    let acro_id = doc.add_object(dictionary! {
        "Fields" => vec![Object::Reference(field_id)],
    });

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
        "AcroForm" => Object::Reference(acro_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("保存带表单的测试 PDF 失败");
}

/// 辅助函数：创建带防复制恢复备份的测试 PDF。
fn create_anticopy_test_pdf(path: &Path, version: u64) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let info_id = doc.add_object(dictionary! {
        "DocsyAntiCopyBackup" => Object::string_literal(format!("u:{{\"version\":{version}}}"))
    });
    doc.trailer.set("Info", Object::Reference(info_id));

    doc.save(path).expect("保存防复制测试 PDF 失败");
}

/// 辅助函数：创建声明数字签名的测试 PDF。
#[allow(dead_code)]
fn create_signature_test_pdf(path: &Path) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        }),
    );

    let acro_id = doc.add_object(dictionary! {
        "SigFlags" => 3,
        "Fields" => vec![],
    });

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
        "AcroForm" => Object::Reference(acro_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("保存签名测试 PDF 失败");
}


fn review_dir(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("review-output").join(name);
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn request(inputs: &[&Path], out: &Path, policy: MergePolicy) -> MergeRequest {
    MergeRequest { inputs: inputs.iter().map(|p|p.to_string_lossy().into_owned()).collect(), output_path: out.into(), policy: OutputPolicy::for_policy(policy,false), preferred_backend: Some(BackendType::PdfCraft), conversion_state: None }
}
fn extract_request(input:&Path, out:&Path) -> ExtractRequest {
    ExtractRequest {input_path:input.into(), pages:vec![1], output_dir:None, output_path:Some(out.into()), preferred_backend:Some(BackendType::PdfCraft)}
}
fn has_info(doc:&Document, key:&[u8])->bool {
    doc.trailer.get(b"Info").ok().and_then(|o|doc.dereference(o).ok()).and_then(|(_,o)|o.as_dict().ok()).is_some_and(|d|d.has(key))
}
fn widget_count(doc:&Document)->usize {
    doc.objects.values().filter_map(|o|o.as_dict().ok()).filter(|d|d.get(b"Subtype").ok().and_then(|o|o.as_name().ok())==Some(b"Widget".as_slice())).count()
}
#[test]
fn review_merge_existing_target_is_preserved() {
    let dir=review_dir("existing-target"); let input=dir.join("input.pdf"); let out=dir.join("already-exists.pdf");
    create_test_pdf(&input,1,595.,842.); std::fs::write(&out,b"KEEP ORIGINAL FILE").unwrap();
    let before=std::fs::read(&out).unwrap();
    let mut req=request(&[&input],&out,MergePolicy::Print);
    req.preferred_backend=Some(BackendType::Qpdf);
    let control=dispatch_merge(&req).unwrap();
    assert_ne!(control.output_path,out); assert_eq!(std::fs::read(&out).unwrap(),before);
    req.preferred_backend=Some(BackendType::PdfCraft);
    let result=dispatch_merge(&req).unwrap();
    println!("EXISTING_TARGET backend={:?}, path={}",result.backend,result.output_path.display());
    assert_eq!(std::fs::read(&out).unwrap(),before,"candidate overwrote an existing user file");
}
#[test]
fn review_merge_does_not_overwrite_source() {
    let dir=review_dir("source-target"); let a=dir.join("a.pdf"); let b=dir.join("b.pdf");
    create_test_pdf(&a,1,595.,842.); create_test_pdf(&b,2,595.,842.);
    let before=std::fs::read(&a).unwrap();
    let result=dispatch_merge(&request(&[&a,&b],&a,MergePolicy::Structural)).unwrap();
    println!("SOURCE_TARGET output={} pages={}",result.output_path.display(),result.total_pages);
    assert_eq!(std::fs::read(&a).unwrap(),before,"candidate overwrote its source PDF");
}
#[test]
fn review_late_parse_failure_does_not_delete_existing_target() {
    let dir=review_dir("fallback-delete"); let input=dir.join("bad-tree.pdf"); let out=dir.join("user-file.pdf");
    let mut bad=Document::with_version("1.7");
    let catalog=bad.add_object(dictionary!{"Type"=>"Catalog"}); bad.trailer.set("Root",catalog); bad.save(&input).unwrap();
    std::fs::write(&out,b"KEEP ORIGINAL FILE").unwrap();
    let req=request(&[&input],&out,MergePolicy::Print);
    let preflight=PdfCraftBackend::new().preflight_merge(&req);
    assert!(preflight.is_ok(),"must reach candidate execution, got {preflight:?}");
    let result=dispatch_merge(&req);
    println!("LATE_ERROR result={result:?}, user_file_exists={}",out.exists());
    assert_eq!(std::fs::read(&out).ok(),Some(b"KEEP ORIGINAL FILE".to_vec()),"candidate fallback deleted an unrelated existing output");
}
#[test]
fn review_print_policy_flattens_form_appearance() {
    let dir=review_dir("print-policy"); let input=dir.join("form.pdf");
    create_form_test_pdf(&input,"account","VISIBLE VALUE");
    let mut doc=Document::load(&input).unwrap();
    let ap=doc.add_object(Stream::new(dictionary!{"Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),150.into(),30.into()]},b"1 0 0 rg 0 0 150 30 re f".to_vec()));
    for obj in doc.objects.values_mut() {
        if let Ok(d)=obj.as_dict_mut() {
            if d.get(b"Subtype").ok().and_then(|v|v.as_name().ok())==Some(b"Widget") {
                d.set("AP",dictionary!{"N"=>ap}); d.set("F",4);
            }
        }
    }
    doc.save(&input).unwrap();
    let mut control_req=request(&[&input],&dir.join("qpdf-print.pdf"),MergePolicy::Print);
    control_req.preferred_backend=Some(BackendType::Qpdf);
    let control=dispatch_merge(&control_req).unwrap();
    let control_doc=Document::load(control.output_path).unwrap();
    let result=dispatch_merge(&request(&[&input],&dir.join("candidate-print.pdf"),MergePolicy::Print)).unwrap();
    let output=Document::load(&result.output_path).unwrap();
    println!("PRINT widgets: qpdf={}, candidate={}, warnings={:?}",widget_count(&control_doc),widget_count(&output),result.warnings);
    assert_eq!(widget_count(&control_doc),0);
    assert_eq!(widget_count(&output),0,"Print policy left an interactive widget instead of flattening AP into page contents");
}
#[test]
fn review_structural_merge_honors_metadata_policy() {
    let dir=review_dir("metadata-policy"); let input=dir.join("source.pdf");
    create_test_pdf(&input,1,595.,842.);
    let mut doc=Document::load(&input).unwrap();
    let info=doc.add_object(dictionary!{"Title"=>Object::string_literal("Case title"),"CaseRef"=>Object::string_literal("CASE-123")});
    doc.trailer.set("Info",info);
    let metadata=doc.add_object(Stream::new(dictionary!{"Type"=>"Metadata","Subtype"=>"XML"},b"<?xpacket begin=''?><x:xmpmeta xmlns:x='adobe:ns:meta/'><case>CASE-123</case></x:xmpmeta><?xpacket end='w'?>".to_vec()));
    doc.catalog_mut().unwrap().set("Metadata",metadata); doc.save(&input).unwrap();
    let req=request(&[&input],&dir.join("merged.pdf"),MergePolicy::Structural);
    assert!(req.policy.preserve_info && req.policy.preserve_xmp);
    let result=dispatch_merge(&req).unwrap(); let output=Document::load(&result.output_path).unwrap();
    println!("METADATA title={}, custom={}, xmp={}, warnings={:?}",has_info(&output,b"Title"),has_info(&output,b"CaseRef"),output.catalog().unwrap().has(b"Metadata"),result.warnings);
    assert!(has_info(&output,b"Title") && has_info(&output,b"CaseRef") && output.catalog().unwrap().has(b"Metadata"),"Structural policy silently dropped promised Info/XMP");
}
#[test]
fn review_extract_respects_anti_copy_preflight() {
    let dir=review_dir("anti-copy-extract"); let input=dir.join("old-backup.pdf"); let out=dir.join("extracted.pdf");
    create_anticopy_test_pdf(&input,1);
    let req=extract_request(&input,&out);
    let preflight=PdfCraftBackend::new().preflight_extract(&req).unwrap();
    assert!(preflight.any_anticopy_backup);
    let result=dispatch_extract(&req).unwrap(); let output=Document::load(&result.output_path).unwrap();
    println!("ANTI_COPY_EXTRACT backend={:?}, fallback={:?}, backup_retained={}, warnings={:?}",result.backend,result.fallback_reason,has_info(&output,b"DocsyAntiCopyBackup"),result.warnings);
    assert!(result.backend!=BackendType::PdfCraft || has_info(&output,b"DocsyAntiCopyBackup"),"legacy recovery backup detected then ignored and dropped");
}
#[test]
fn review_structural_fallback_does_not_use_print_semantics() {
    let dir=review_dir("structural-fallback"); let input=dir.join("form-with-backup.pdf"); let out=dir.join("output.pdf");
    create_form_test_pdf(&input,"account","Alice");
    let mut doc=Document::load(&input).unwrap(); let info=doc.add_object(dictionary!{"DocsyAntiCopyBackup"=>Object::string_literal("{\"version\":2,\"cmaps\":{}}")}); doc.trailer.set("Info",info); doc.save(&input).unwrap();
    let req=request(&[&input],&out,MergePolicy::Structural);
    let result=dispatch_merge(&req).unwrap(); let output=Document::load(&result.output_path).unwrap();
    println!("STRUCTURAL_FALLBACK backend={:?}, fields={}, warnings={:?}",result.backend,widget_count(&output),result.warnings);
    assert!(output.catalog().unwrap().has(b"AcroForm"),"fallback silently lost Structural contract AcroForm");
}

#[test]
fn review_real_legacy_backup_roundtrip_is_not_broken_by_extract() {
    let dir=review_dir("real-legacy"); let input=dir.join("plain.pdf"); let protected=dir.join("protected.pdf"); let extracted=dir.join("extracted.pdf");
    create_test_pdf(&input,1,595.,842.);
    let mut doc=Document::load(&input).unwrap();
    let original=b"begincmap\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n1 beginbfchar\n<41> <0041>\nendbfchar\nendcmap\n";
    let cmap=doc.add_object(Stream::new(dictionary!{},original.to_vec()));
    for o in doc.objects.values_mut() { if let Ok(d)=o.as_dict_mut() {if d.get(b"Type").ok().and_then(|o|o.as_name().ok())==Some(b"Font") {d.set("ToUnicode",cmap);}}}
    doc.save(&input).unwrap();
    assert_eq!(crate::pdf::anti_ocr::apply_anti_copy(&input,&protected,crate::pdf::anti_ocr::AntiCopyMethod::CmapScramble).unwrap(),1);
    let v2out=dir.join("v2-extracted.pdf");
    let v2result=dispatch_extract(&extract_request(&protected,&v2out)).unwrap();
    let v2restore=crate::pdf::anti_ocr::process_copy(&v2out,&dir.join("v2-restored.pdf"),None);
    println!("REAL_V2 backend={:?}, fallback={:?}, restore_after={v2restore:?}",v2result.backend,v2result.fallback_reason);
    let mut old=Document::load(&protected).unwrap();
    let info=old.trailer.get(b"Info").unwrap().as_reference().unwrap();
    let value=old.get_dictionary(info).unwrap().get(b"DocsyAntiCopyBackup").unwrap().as_str().unwrap();
    let mut backup:serde_json::Value=serde_json::from_slice(value).unwrap(); backup["version"]=1.into();
    old.get_object_mut(info).unwrap().as_dict_mut().unwrap().set("DocsyAntiCopyBackup",Object::string_literal(serde_json::to_vec(&backup).unwrap()));
    for o in old.objects.values_mut() {if let Ok(d)=o.as_dict_mut() {d.remove(b"DocsyOriginalCMap");}}
    old.save(&protected).unwrap();
    let before=crate::pdf::anti_ocr::process_copy(&protected,&dir.join("restored-before.pdf"),None);
    assert!(before.is_ok(),"fixture must be restorable before extraction");
    assert_eq!(before.unwrap().modified_fonts,1);
    assert!(crate::pdf::qpdf::validate_recovery_backup(&protected).is_err(),"existing legacy rewrite gate should refuse");
    let result=dispatch_extract(&extract_request(&protected,&extracted)).unwrap();
    let after=crate::pdf::anti_ocr::process_copy(&extracted,&dir.join("restored-after.pdf"),None);
    println!("REAL_LEGACY backend={:?}, fallback={:?}, restore_after={after:?}",result.backend,result.fallback_reason);
    assert!(after.is_ok(),"candidate extraction destroyed a verified-restorable legacy backup");
}
#[test]
fn review_reverse_order_is_read_back_by_lopdf() {
    let dir=review_dir("reverse-independent"); let input=dir.join("input.pdf"); let out=dir.join("selected.pdf");
    create_test_pdf(&input,4,595.,842.); let mut req=extract_request(&input,&out); req.pages=vec![3,1];
    dispatch_extract(&req).unwrap(); let result=Document::load(&out).unwrap();
    assert!(result.extract_text(&[1]).unwrap().contains("Page 3"));
    assert!(result.extract_text(&[2]).unwrap().contains("Page 1"));
}
#[test]
fn review_geometry_and_duplex_is_read_back_by_lopdf() {
    let dir=review_dir("geometry-independent"); let a=dir.join("a.pdf"); let b=dir.join("b.pdf"); let c=dir.join("c.pdf");
    for (i,path) in [&a,&b,&c].iter().enumerate() {
        create_test_pdf(path,1,400.+i as f64*100.,600.);
        let mut d=Document::load(path).unwrap(); let page=*d.get_pages().values().next().unwrap();
        d.get_object_mut(page).unwrap().as_dict_mut().unwrap().set("Rotate",(i as i64)*90);
        d.get_object_mut(page).unwrap().as_dict_mut().unwrap().set("CropBox",vec![10.into(),20.into(),300.into(),500.into()]); d.save(path).unwrap();
    }
    let out=dir.join("merged.pdf"); let mut req=request(&[&a,&b,&c],&out,MergePolicy::Print); req.policy.duplex_separate=true;
    dispatch_merge(&req).unwrap(); let d=Document::load(&out).unwrap(); assert_eq!(d.get_pages().len(),6);
    for (i,id) in d.get_pages().values().enumerate() {
        let page=d.get_dictionary(*id).unwrap(); let rotation=page.get(b"Rotate").and_then(Object::as_i64).unwrap_or(0);
        assert_eq!(rotation,(i/2) as i64*90);
        let key=if i%2==0 {b"CropBox".as_slice()} else {b"MediaBox".as_slice()};
        let bbox=page.get(key).unwrap().as_array().unwrap();
        let values:Vec<f32>=bbox.iter().map(|o|o.as_float().unwrap()).collect(); assert_eq!(values,vec![10.,20.,300.,500.]);
    }
}
