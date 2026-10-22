use super::policy::*;
use super::*;
use lopdf::{dictionary, Document, Object, Stream};
use std::path::{Path, PathBuf};

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

fn test_temp_dir(prefix: &str) -> (crate::util::fs::TempDirGuard, PathBuf) {
    let dir = crate::util::fs::temp_named_path(prefix, "");
    std::fs::create_dir_all(&dir).unwrap();
    let guard = crate::util::fs::TempDirGuard::new(dir.clone()).unwrap();
    (guard, dir)
}

#[test]
fn test_default_backend_selection() {
    assert_eq!(select_backend(None), BackendType::PdfCraft);
    assert_eq!(
        select_backend(Some(BackendType::Qpdf)),
        BackendType::Qpdf
    );
}

#[test]
fn test_pdfcraft_merge_basic() {
    let (_guard, dir) = test_temp_dir("docsy_test_basic");
    let pdf_a = dir.join("a.pdf");
    let pdf_b = dir.join("b.pdf");
    let out_pdf = dir.join("merged.pdf");

    create_test_pdf(&pdf_a, 1, 300.0, 400.0);
    create_test_pdf(&pdf_b, 2, 595.0, 842.0);

    let backend = PdfCraftBackend::new();
    let req = MergeRequest {
        inputs: vec![
            pdf_a.to_string_lossy().to_string(),
            pdf_b.to_string_lossy().to_string(),
        ],
        output_path: out_pdf.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Structural, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };

    let result = backend.merge(&req).expect("PdfCraft 合并应成功");
    assert_eq!(result.backend, BackendType::PdfCraft);
    assert_eq!(result.total_pages, 3);
    assert!(out_pdf.exists());

    // 独立重新读取验证输出结构
    let bytes = std::fs::read(&out_pdf).unwrap();
    let read_doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)).unwrap();
    let count = pdfcraft_organize::page_count(&read_doc).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_pdfcraft_merge_duplex_separate() {
    let (_guard, dir) = test_temp_dir("docsy_test_duplex");
    let pdf_odd = dir.join("odd.pdf"); // 1 页（奇数，需要补白）
    let pdf_even = dir.join("even.pdf"); // 2 页（偶数，不需要补白）
    let out_pdf = dir.join("duplex_merged.pdf");

    create_test_pdf(&pdf_odd, 1, 400.0, 600.0);
    create_test_pdf(&pdf_even, 2, 595.0, 842.0);

    let backend = PdfCraftBackend::new();
    let req = MergeRequest {
        inputs: vec![
            pdf_odd.to_string_lossy().to_string(),
            pdf_even.to_string_lossy().to_string(),
        ],
        output_path: out_pdf.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Structural, true),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };

    let result = backend.merge(&req).expect("双面结构合并应成功");
    // 1 页补白 1 页 = 2 页；加上 2 页 = 总共 4 页
    assert_eq!(result.total_pages, 4);

    let bytes = std::fs::read(&out_pdf).unwrap();
    let read_doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)).unwrap();
    let count = pdfcraft_organize::page_count(&read_doc).unwrap();
    assert_eq!(count, 4);
    let pages = pdfcraft_organize::walk(&read_doc).unwrap();
    let separator = read_doc.get(pages[1].0);
    assert!(crate::pdf::blank::is_docsy_separator(separator.as_dict().unwrap()), "merge must preserve Docsy separator marker");
}

#[test]
fn test_pdfcraft_extract_pages_1based_reverse() {
    let (_guard, dir) = test_temp_dir("docsy_test_extract");
    let input_pdf = dir.join("input.pdf");
    let out_pdf = dir.join("extracted.pdf");

    create_test_pdf(&input_pdf, 4, 595.0, 842.0);

    let backend = PdfCraftBackend::new();
    let req = ExtractRequest {
        input_path: input_pdf.clone(),
        pages: vec![3, 1], // 提取第 3 页和第 1 页
        output_dir: None,
        output_path: Some(out_pdf.clone()),
        preferred_backend: Some(BackendType::PdfCraft),
    };

    let result = backend.extract(&req).expect("提取页面应成功");
    assert_eq!(result.total_pages, 2);
    assert_eq!(result.backend, BackendType::PdfCraft);
    assert!(out_pdf.exists());

    let bytes = std::fs::read(&out_pdf).unwrap();
    let read_doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)).unwrap();
    assert_eq!(pdfcraft_organize::page_count(&read_doc).unwrap(), 2);
}

#[test]
fn test_pdfcraft_extract_invalid_pages() {
    let (_guard, dir) = test_temp_dir("docsy_test_invalid_extract");
    let input_pdf = dir.join("input.pdf");

    create_test_pdf(&input_pdf, 2, 595.0, 842.0);
    let backend = PdfCraftBackend::new();

    // 0 必须被拒绝
    let req_zero = ExtractRequest {
        input_path: input_pdf.clone(),
        pages: vec![0, 1],
        output_dir: None,
        output_path: None,
        preferred_backend: Some(BackendType::PdfCraft),
    };
    assert!(backend.extract(&req_zero).is_err());

    // 超出总页数必须被拒绝
    let req_overflow = ExtractRequest {
        input_path: input_pdf,
        pages: vec![1, 99],
        output_dir: None,
        output_path: None,
        preferred_backend: Some(BackendType::PdfCraft),
    };
    assert!(backend.extract(&req_overflow).is_err());
}

#[test]
fn test_pdfcraft_form_field_renamed_on_collision() {
    let (_guard, dir) = test_temp_dir("docsy_test_form_collision");
    let form_a = dir.join("form_a.pdf");
    let form_b = dir.join("form_b.pdf");
    let out_pdf = dir.join("merged_form.pdf");

    create_form_test_pdf(&form_a, "user_account", "Alice");
    create_form_test_pdf(&form_b, "user_account", "Bob");

    let backend = PdfCraftBackend::new();
    let req = MergeRequest {
        inputs: vec![
            form_a.to_string_lossy().to_string(),
            form_b.to_string_lossy().to_string(),
        ],
        output_path: out_pdf.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Structural, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };

    let result = backend.merge(&req).expect("表单合并应成功");
    // 两个同名表单合并，pdfcraft organize 必须重命名冲突字段
    assert!(!result.renamed_fields.is_empty());
    assert_eq!(result.renamed_fields[0].0, "user_account");
    assert!(result.renamed_fields[0].1 != "user_account");
}

#[test]
fn test_preflight_anticopy_backup_and_safe_fallback() {
    let (_guard, dir) = test_temp_dir("docsy_test_anticopy_fallback");
    let anticopy_pdf = dir.join("anticopy.pdf");
    let normal_pdf = dir.join("normal.pdf");
    let out_pdf = dir.join("out.pdf");

    create_anticopy_test_pdf(&anticopy_pdf, 2);
    create_test_pdf(&normal_pdf, 1, 595.0, 842.0);

    let candidate = PdfCraftBackend::new();
    let req = MergeRequest {
        inputs: vec![
            anticopy_pdf.to_string_lossy().to_string(),
            normal_pdf.to_string_lossy().to_string(),
        ],
        output_path: out_pdf.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Print, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };

    // 1. 验证候选后端直接预检识别出防复制备份
    let summary = candidate.preflight_merge(&req).unwrap();
    assert!(summary.any_anticopy_backup);
    assert_eq!(summary.documents[0].anticopy_version, Some(2));

    // 2. 验证受控分派器感知到防复制备份后，平滑回退至生产 Qpdf
    let result = dispatch_merge(&req).expect("通过回退应成功完成合并");
    assert_eq!(result.backend, BackendType::Qpdf);
    assert_eq!(
        result.fallback_reason,
        Some(FallbackReason::DocsyAntiCopyBackupDetected)
    );
    assert!(out_pdf.exists());
}

#[test]
fn test_preflight_signature_warning() {
    let (_guard, dir) = test_temp_dir("docsy_test_sig_warning");
    let sig_pdf = dir.join("sig.pdf");
    create_signature_test_pdf(&sig_pdf);

    let candidate = PdfCraftBackend::new();
    let req = MergeRequest {
        inputs: vec![sig_pdf.to_string_lossy().to_string()],
        output_path: dir.join("out.pdf"),
        policy: OutputPolicy::for_policy(MergePolicy::Print, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };

    let summary = candidate.preflight_merge(&req).unwrap();
    assert!(summary.any_signatures);
    assert!(summary
        .warnings
        .iter()
        .any(|w| matches!(w, BackendWarning::SignatureValidityInvalidated { .. })));
}

#[test]
fn test_view_matrix_four_rotations() {
    use super::marks::view_matrix_for;

    let crop = [50.0, 60.0, 550.0, 800.0];
    // 0 度：平移至原点 (x0, y0)
    let m0 = view_matrix_for(0, crop, 1.0);
    assert_eq!(m0, [1.0, 0.0, 0.0, 1.0, 50.0, 60.0]);

    // 90 度：顺时针 90 度放置，矩阵平移至 (x1, y0)
    let m90 = view_matrix_for(90, crop, 1.0);
    assert_eq!(m90, [0.0, 1.0, -1.0, 0.0, 550.0, 60.0]);

    // 180 度
    let m180 = view_matrix_for(180, crop, 1.0);
    assert_eq!(m180, [-1.0, 0.0, 0.0, -1.0, 550.0, 800.0]);

    // 270 度
    let m270 = view_matrix_for(270, crop, 1.0);
    assert_eq!(m270, [0.0, -1.0, 1.0, 0.0, 50.0, 800.0]);
}

#[test]
fn test_docsy_marks_lifecycle_isolated_and_lossless() {
    use super::marks::{apply_or_update_marks, inspect_marks, remove_marks, DocsyMarkFilter, DocsyMarkKind, DocsyMarkSpec};

    let (_guard, dir) = test_temp_dir("docsy_test_marks_lifecycle");
    let test_pdf = dir.join("test_pages.pdf");
    create_test_pdf(&test_pdf, 2, 595.0, 842.0);

    let bytes = std::fs::read(&test_pdf).unwrap();
    let mut doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)).unwrap();

    // 初始状态下无任何 Docsy 标记
    let initial_marks = inspect_marks(&doc).unwrap();
    assert!(initial_marks.is_empty());

    // 1. 为第 1 页添加 Header，为第 2 页添加 Footer
    let mark1 = DocsyMarkSpec {
        id: "header-mark-001".to_string(),
        kind: DocsyMarkKind::Header,
        batch_id: Some("batch-2026".to_string()),
        page_index: 0,
        content_ops: b"BT /F1 10 Tf 50 800 Td (Header Page 1) Tj ET".to_vec(),
    };
    let mark2 = DocsyMarkSpec {
        id: "footer-mark-002".to_string(),
        kind: DocsyMarkKind::Footer,
        batch_id: Some("batch-2026".to_string()),
        page_index: 1,
        content_ops: b"BT /F1 10 Tf 50 30 Td (Footer Page 2) Tj ET".to_vec(),
    };

    apply_or_update_marks(&mut doc, &[mark1, mark2]).expect("添加标记应成功");

    // 验证检出两个标记
    let marks_after_add = inspect_marks(&doc).unwrap();
    assert_eq!(marks_after_add.len(), 2);
    assert!(marks_after_add.iter().any(|m| m.id == "header-mark-001"));
    assert!(marks_after_add.iter().any(|m| m.id == "footer-mark-002"));

    // 2. 更新 Header（幂等更新，ID 相同）
    let mark1_updated = DocsyMarkSpec {
        id: "header-mark-001".to_string(),
        kind: DocsyMarkKind::Header,
        batch_id: Some("batch-2026".to_string()),
        page_index: 0,
        content_ops: b"BT /F1 12 Tf 60 800 Td (Updated Header Page 1) Tj ET".to_vec(),
    };
    apply_or_update_marks(&mut doc, &[mark1_updated]).expect("更新标记应成功");

    let marks_after_update = inspect_marks(&doc).unwrap();
    assert_eq!(marks_after_update.len(), 2);

    // 3. 仅移除 Header 类型的标记，Footer 应依然完好
    let filter_header = DocsyMarkFilter {
        kind: Some(DocsyMarkKind::Header),
        id: None,
        batch_id: None,
    };
    let removed = remove_marks(&mut doc, &filter_header).expect("移除 Header 应成功");
    assert_eq!(removed, 1);

    let marks_after_remove_header = inspect_marks(&doc).unwrap();
    assert_eq!(marks_after_remove_header.len(), 1);
    assert_eq!(marks_after_remove_header[0].id, "footer-mark-002");

    // 4. 按批次 ID 移除剩余所有标记
    let filter_batch = DocsyMarkFilter {
        kind: None,
        id: None,
        batch_id: Some("batch-2026".to_string()),
    };
    let removed_batch = remove_marks(&mut doc, &filter_batch).expect("按批次移除应成功");
    assert_eq!(removed_batch, 1);

    let marks_final = inspect_marks(&doc).unwrap();
    assert!(marks_final.is_empty());

    // 5. 验证原文档页数与基本结构完好无损
    let page_count = pdfcraft_organize::page_count(&doc).unwrap();
    assert_eq!(page_count, 2);
}

/// 辅助函数：创建 2 页 PDF，第 1 页有文本，第 2 页空白（无文本流）。
fn create_test_pdf_with_blank_page(path: &Path) {
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

    // 第 1 页：包含文本
    let content1 = lopdf::content::Content {
        operations: vec![
            lopdf::content::Operation::new("BT", vec![]),
            lopdf::content::Operation::new(
                "Tf",
                vec![Object::Name(b"F1".to_vec()), 12.into()],
            ),
            lopdf::content::Operation::new("Td", vec![50.into(), 500.into()]),
            lopdf::content::Operation::new("Tj", vec![Object::string_literal("Hello Docsy PoC")]),
            lopdf::content::Operation::new("ET", vec![]),
        ],
    };
    let content1_id = doc.add_object(Stream::new(dictionary! {}, content1.encode().unwrap()));
    let page1_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content1_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });

    // 第 2 页：空白（空内容流）
    let content2 = lopdf::content::Content { operations: vec![] };
    let content2_id = doc.add_object(Stream::new(dictionary! {}, content2.encode().unwrap()));
    let page2_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content2_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page1_id), Object::Reference(page2_id)],
            "Count" => 2,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path).expect("保存含空白页测试 PDF 失败");
}

#[test]
fn test_extract_poc_structure_and_empty_page_notice() {
    use super::extract_poc::extract_document_text;

    let (_guard, dir) = test_temp_dir("docsy_test_extract_poc");
    let test_pdf = dir.join("text_and_blank.pdf");
    create_test_pdf_with_blank_page(&test_pdf);

    let result = extract_document_text(&test_pdf).expect("文本与 Markdown 提取 PoC 应成功");
    assert_eq!(result.total_pages, 2);
    assert_eq!(result.pages_with_text, 1);
    assert_eq!(result.empty_pages, vec![2]);
    assert_eq!(result.pages.len(), 2);

    // 验证第 1 页有文本
    assert!(!result.pages[0].is_empty);
    assert_eq!(result.pages[0].page_number, 1);
    assert!(result.pages[0].text.contains("Hello Docsy PoC"));
    assert!(!result.pages[0].spans.is_empty());

    // 验证第 2 页判定为空白页并带有明确提示
    assert!(result.pages[1].is_empty);
    assert_eq!(result.pages[1].page_number, 2);
    assert!(result.pages[1].text.contains("未检测到文本层"));

    // 验证生成的 Markdown 结构与扫描件提示
    assert!(result.markdown.contains("## 第 1 页"));
    assert!(result.markdown.contains("Hello Docsy PoC"));
    assert!(result.markdown.contains("## 第 2 页"));
    assert!(result.markdown.contains("[!NOTE]"));
    assert!(result.markdown.contains("未检测到可提取文本层"));
}

#[test]
fn test_prune_unused_xobject_when_page_has_no_do_operator() {
    let (_guard, dir) = test_temp_dir("docsy_test_prune_zero_do");
    let input_pdf = dir.join("zero_do.pdf");
    let out_pdf = dir.join("extracted.pdf");

    // 构造 2 页 PDF，其中第 1 页只有文本算子（无 Do 算子），但 Resources/XObject 中挂有 UnusedForm
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let unused_form_id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 10.into(), 10.into()],
        },
        b"0 1 0 rg 0 0 10 10 re f".to_vec(),
    ));
    let resources_id = doc.add_object(dictionary! {
        "XObject" => dictionary! {
            "UnusedForm" => Object::Reference(unused_form_id),
        },
    });
    let content_id = doc.add_object(Stream::new(dictionary! {}, b"q Q".to_vec()));
    let page1_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => Object::Reference(content_id),
        "Resources" => Object::Reference(resources_id),
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
    });
    let page2_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => Object::Reference(content_id),
        "Resources" => Object::Reference(resources_id),
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page1_id), Object::Reference(page2_id)],
            "Count" => 2,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(&input_pdf).unwrap();

    let backend = PdfCraftBackend::new();
    let res = backend
        .extract(&ExtractRequest {
            input_path: input_pdf,
            pages: vec![1],
            output_dir: None,
            output_path: Some(out_pdf.clone()),
            preferred_backend: Some(BackendType::PdfCraft),
        })
        .unwrap();

    let out_bytes = std::fs::read(&res.output_path).unwrap();
    let out_doc = pdfcraft_cos::Document::open(std::sync::Arc::new(out_bytes)).unwrap();
    let walked = pdfcraft_organize::walk(&out_doc).unwrap();
    let (_, page_attrs) = &walked[0];
    let has_unused_form = page_attrs
        .get(b"Resources")
        .and_then(|r| out_doc.dict(r))
        .and_then(|r| r.get(b"XObject").cloned())
        .and_then(|x| out_doc.dict(&x))
        .is_some_and(|x| x.contains(b"UnusedForm"));
    assert!(
        !has_unused_form,
        "页面无 Do 算子时，未使用的 UnusedForm 必须在提取后被清理"
    );
}

#[test]
fn test_user_unit_page_info_consistency() {
    let (_guard, dir) = test_temp_dir("docsy_test_user_unit");
    let pdf_path = dir.join("user_unit.pdf");

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 400.into(), 400.into()],
        "CropBox" => vec![20.into(), 30.into(), 380.into(), 370.into()],
        "Rotate" => 90,
        "UserUnit" => 2,
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
        "Pages" => Object::Reference(pages_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(&pdf_path).unwrap();

    let infos = crate::pdf::page_info::get_page_infos(&pdf_path.to_string_lossy()).unwrap();
    assert_eq!(infos.len(), 1);
    // CropBox 360x340, Rotate 90 -> 340x360, UserUnit 2 -> 680x720
    assert!((infos[0].width_pt - 680.0).abs() < 1e-3);
    assert!((infos[0].height_pt - 720.0).abs() < 1e-3);
}

#[test]
fn test_body_stream_with_docsy_wrapper_key_is_preserved() {
    use super::marks::{
        apply_or_update_marks, remove_marks, DocsyMarkFilter, DocsyMarkKind, DocsyMarkSpec,
    };

    let mut doc = pdfcraft_cos::Document::new_empty();
    let pages_ref = doc.add(pdfcraft_cos::Dict::new());
    let mut body_dict = pdfcraft_cos::Dict::new();
    // 伪造带 /DocsyWrapper false 的普通正文流
    body_dict.set(b"DocsyWrapper".to_vec(), pdfcraft_cos::Object::Bool(false));
    let body_ref = doc.add(pdfcraft_cos::Object::Stream(pdfcraft_cos::Stream::from_raw(
        body_dict,
        b"BT /F1 12 Tf 40 40 Td (Body Kept) Tj ET".to_vec(),
    )));

    let mut page = pdfcraft_cos::Dict::new();
    page.set(b"Type".to_vec(), pdfcraft_cos::Object::name("Page"));
    page.set(b"Parent".to_vec(), pdfcraft_cos::Object::Ref(pages_ref));
    page.set(
        b"MediaBox".to_vec(),
        pdfcraft_cos::Object::Array(vec![0.into(), 0.into(), 200.into(), 200.into()]),
    );
    page.set(b"Contents".to_vec(), pdfcraft_cos::Object::Ref(body_ref));
    let page_ref = doc.add(page);

    doc.update_dict(pages_ref, |d| {
        d.set(b"Type".to_vec(), pdfcraft_cos::Object::name("Pages"));
        d.set(
            b"Kids".to_vec(),
            pdfcraft_cos::Object::Array(vec![pdfcraft_cos::Object::Ref(page_ref)]),
        );
        d.set(b"Count".to_vec(), pdfcraft_cos::Object::Int(1));
    })
    .unwrap();

    let mut catalog = pdfcraft_cos::Dict::new();
    catalog.set(b"Type".to_vec(), pdfcraft_cos::Object::name("Catalog"));
    catalog.set(b"Pages".to_vec(), pdfcraft_cos::Object::Ref(pages_ref));
    let catalog_ref = doc.add(catalog);
    doc.trailer_mut()
        .set(b"Root".to_vec(), pdfcraft_cos::Object::Ref(catalog_ref));

    apply_or_update_marks(
        &mut doc,
        &[DocsyMarkSpec {
            id: "m1".to_string(),
            kind: DocsyMarkKind::Header,
            batch_id: None,
            page_index: 0,
            content_ops: b"1 0 0 rg 10 10 20 20 re f".to_vec(),
        }],
    )
    .unwrap();

    remove_marks(
        &mut doc,
        &DocsyMarkFilter {
            kind: None,
            id: None,
            batch_id: None,
        },
    )
    .unwrap();

    let contents = doc
        .get(page_ref)
        .as_dict()
        .and_then(|d| d.get(b"Contents").cloned())
        .unwrap();
    let kept_refs: Vec<pdfcraft_cos::ObjRef> = match contents {
        pdfcraft_cos::Object::Ref(r) => vec![r],
        pdfcraft_cos::Object::Array(arr) => arr.iter().filter_map(|o| o.as_ref()).collect(),
        _ => vec![],
    };
    assert!(
        kept_refs.contains(&body_ref),
        "带 /DocsyWrapper false 的普通正文流绝不能被当成包装流误删"
    );
}

#[test]
fn test_native_print_flatten_cases_and_merge() {
    use super::flatten::flatten_document;

    let (_guard, dir) = test_temp_dir("docsy_test_flatten");

    // 1. 测试 normal_widget + no_view + watermark + hidden + invisible + zero_bbox
    let pdf_path = dir.join("widgets.pdf");
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let ap_stream_id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 100.into(), 50.into()],
        },
        b"1 0 0 rg 0 0 100 50 re f".to_vec(),
    ));
    let zero_ap_id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 0.into(), 50.into()],
        },
        b"1 0 0 rg 0 0 100 50 re f".to_vec(),
    ));

    let w_normal = doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Widget",
        "FT" => "Tx",
        "T" => Object::string_literal("f_normal"),
        "Rect" => vec![20.into(), 20.into(), 120.into(), 70.into()],
        "F" => 4,
        "AP" => dictionary! { "N" => Object::Reference(ap_stream_id) },
    });
    let w_noview = doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Widget",
        "FT" => "Tx",
        "T" => Object::string_literal("f_noview"),
        "Rect" => vec![20.into(), 80.into(), 120.into(), 130.into()],
        "F" => 36, // NoView (32) + Print (4)
        "AP" => dictionary! { "N" => Object::Reference(ap_stream_id) },
    });
    let w_hidden = doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Widget",
        "FT" => "Tx",
        "T" => Object::string_literal("f_hidden"),
        "Rect" => vec![20.into(), 140.into(), 120.into(), 190.into()],
        "F" => 6, // Hidden (2) + Print (4)
        "AP" => dictionary! { "N" => Object::Reference(ap_stream_id) },
    });
    let w_zero = doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Widget",
        "FT" => "Tx",
        "T" => Object::string_literal("f_zero"),
        "Rect" => vec![20.into(), 200.into(), 120.into(), 250.into()],
        "F" => 4,
        "AP" => dictionary! { "N" => Object::Reference(zero_ap_id) },
    });
    let annot_watermark = doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Watermark",
        "Rect" => vec![40.into(), 40.into(), 140.into(), 90.into()],
        "F" => 4,
        "AP" => dictionary! { "N" => Object::Reference(ap_stream_id) },
    });

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 300.into(), 300.into()],
        "Annots" => vec![
            Object::Reference(w_normal),
            Object::Reference(w_noview),
            Object::Reference(w_hidden),
            Object::Reference(w_zero),
            Object::Reference(annot_watermark),
        ],
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
        "Fields" => vec![
            Object::Reference(w_normal),
            Object::Reference(w_noview),
            Object::Reference(w_hidden),
            Object::Reference(w_zero),
        ],
    });
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
        "AcroForm" => Object::Reference(acro_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(&pdf_path).unwrap();

    // 验证 PdfCraftBackend 在 Print 模式下直接执行原生压平
    let out_pdf = dir.join("flattened_out.pdf");
    let backend = PdfCraftBackend::new();
    let res = backend
        .merge(&MergeRequest {
            inputs: vec![pdf_path.to_string_lossy().to_string()],
            output_path: out_pdf.clone(),
            policy: OutputPolicy::for_policy(MergePolicy::Print, false),
            preferred_backend: Some(BackendType::PdfCraft),
            conversion_state: None,
        })
        .expect("Print 模式原生压平合并应成功");
    assert_eq!(res.backend, BackendType::PdfCraft);

    let out_bytes = std::fs::read(&res.output_path).unwrap();
    let out_doc = pdfcraft_cos::Document::open(std::sync::Arc::new(out_bytes)).unwrap();
    let walked = pdfcraft_organize::walk(&out_doc).unwrap();
    let (out_page_ref, _) = walked[0];
    let out_page = out_doc.get(out_page_ref).as_dict().cloned().unwrap();
    assert!(
        !out_page.contains(b"Annots"),
        "压平后页面不应残留任何已压平或隐藏的注释/Widget"
    );

    // 2. 验证 NeedAppearances=true 的过期表单触发 Unsupported
    let stale_pdf = dir.join("stale.pdf");
    create_form_test_pdf(&stale_pdf, "stale_field", "new_val");
    let stale_bytes = std::fs::read(&stale_pdf).unwrap();
    let mut stale_doc = pdfcraft_cos::Document::open(std::sync::Arc::new(stale_bytes)).unwrap();
    // 即使没有 NeedAppearances，缺少 /AP /N 的可见 Widget 也必须返回 Unsupported
    assert!(flatten_document(&mut stale_doc).is_err());
}

#[test]
fn test_combiner_preserves_need_appearances_and_merge_refuses_stale_widget() {
    use super::flatten::flatten_document;
    use pdfcraft_cos::{Dict, Document, Object, PdfString, SaveOptions, Stream, write_full};

    let mut doc = Document::new_empty();
    let root = doc.root().unwrap();
    let pages_ref = doc
        .get(root)
        .as_dict()
        .unwrap()
        .reference(b"Pages")
        .unwrap();

    let mut ap_dict = Dict::new();
    ap_dict.set(
        b"BBox".to_vec(),
        Object::Array(vec![
            Object::Int(0),
            Object::Int(0),
            Object::Int(100),
            Object::Int(40),
        ]),
    );
    let ap_ref = doc.add(Object::Stream(Stream::flate(
        ap_dict,
        b"1 0 0 rg 0 0 100 40 re f",
    )));

    let mut ap_map = Dict::new();
    ap_map.set(b"N".to_vec(), Object::Ref(ap_ref));

    let mut widget = Dict::new();
    widget.set(b"Type".to_vec(), Object::name("Annot"));
    widget.set(b"Subtype".to_vec(), Object::name("Widget"));
    widget.set(b"FT".to_vec(), Object::name("Tx"));
    widget.set(b"F".to_vec(), Object::Int(4));
    widget.set(b"T".to_vec(), Object::String(PdfString::text("field")));
    widget.set(b"V".to_vec(), Object::String(PdfString::text("NEW")));
    widget.set(
        b"Rect".to_vec(),
        Object::Array(vec![
            Object::Int(50),
            Object::Int(60),
            Object::Int(150),
            Object::Int(100),
        ]),
    );
    widget.set(b"AP".to_vec(), Object::Dict(ap_map));
    let widget_ref = doc.add(Object::Dict(widget));

    let mut page_dict = Dict::new();
    page_dict.set(b"Type".to_vec(), Object::name("Page"));
    page_dict.set(b"Parent".to_vec(), Object::Ref(pages_ref));
    page_dict.set(
        b"MediaBox".to_vec(),
        Object::Array(vec![
            Object::Int(0),
            Object::Int(0),
            Object::Int(200),
            Object::Int(220),
        ]),
    );
    page_dict.set(b"Annots".to_vec(), Object::Array(vec![Object::Ref(widget_ref)]));
    let page_ref = doc.add(Object::Dict(page_dict));

    doc.update_dict(pages_ref, |d| {
        d.set(b"Kids".to_vec(), Object::Array(vec![Object::Ref(page_ref)]));
        d.set(b"Count".to_vec(), Object::Int(1));
    })
    .unwrap();

    let mut acro = Dict::new();
    acro.set(b"Fields".to_vec(), Object::Array(vec![Object::Ref(widget_ref)]));
    acro.set(b"NeedAppearances".to_vec(), Object::Bool(true));
    doc.update_dict(root, |c| {
        c.set(b"AcroForm".to_vec(), Object::Dict(acro));
    })
    .unwrap();

    // 1. Combiner 合并后仍须保留 /NeedAppearances true，不能绕过过期表单检查
    let mut combiner = pdfcraft_organize::Combiner::new();
    combiner.add("stale", &doc, None).unwrap();
    let mut combined = combiner.finish_with_renames().unwrap().doc;
    assert!(matches!(
        flatten_document(&mut combined),
        Err(BackendError::Unsupported(_))
    ));

    // 2. PdfCraftBackend::merge 在 Print 模式下也必须返回 Unsupported
    let temp_in = crate::util::fs::temp_named_path("docsy_test_stale_widget_in", "pdf");
    let temp_out = crate::util::fs::temp_named_path("docsy_test_stale_widget_out", "pdf");
    std::fs::write(&temp_in, write_full(&doc, &SaveOptions::default()).unwrap()).unwrap();

    let backend = PdfCraftBackend::new();
    let req = MergeRequest {
        inputs: vec![temp_in.to_string_lossy().to_string()],
        output_path: temp_out.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Print, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };
    assert!(matches!(
        backend.merge(&req),
        Err(BackendError::Unsupported(_))
    ));

    let _ = std::fs::remove_file(&temp_in);
    let _ = std::fs::remove_file(&temp_out);
}

#[test]
fn test_flatten_synthesizes_highlight_without_ap_and_rejects_unknown_missing_ap() {
    use super::flatten::flatten_document;
    use pdfcraft_cos::{Dict, Document, Object};

    // 1. 缺少 /AP 的 /Highlight 注释应自动合成外观并压平（drawn == 1）
    let mut doc = Document::new_empty();
    let root = doc.root().unwrap();
    let pages_ref = doc
        .get(root)
        .as_dict()
        .unwrap()
        .reference(b"Pages")
        .unwrap();

    let mut hl = Dict::new();
    hl.set(b"Type".to_vec(), Object::name("Annot"));
    hl.set(b"Subtype".to_vec(), Object::name("Highlight"));
    hl.set(b"F".to_vec(), Object::Int(4));
    hl.set(
        b"Rect".to_vec(),
        Object::Array(vec![
            Object::Int(50),
            Object::Int(60),
            Object::Int(150),
            Object::Int(90),
        ]),
    );
    hl.set(
        b"QuadPoints".to_vec(),
        Object::Array(vec![
            Object::Int(50),
            Object::Int(90),
            Object::Int(150),
            Object::Int(90),
            Object::Int(50),
            Object::Int(60),
            Object::Int(150),
            Object::Int(60),
        ]),
    );
    hl.set(
        b"C".to_vec(),
        Object::Array(vec![Object::Int(1), Object::Int(1), Object::Int(0)]),
    );
    let hl_ref = doc.add(Object::Dict(hl));

    let mut page_dict = Dict::new();
    page_dict.set(b"Type".to_vec(), Object::name("Page"));
    page_dict.set(b"Parent".to_vec(), Object::Ref(pages_ref));
    page_dict.set(
        b"MediaBox".to_vec(),
        Object::Array(vec![
            Object::Int(0),
            Object::Int(0),
            Object::Int(200),
            Object::Int(220),
        ]),
    );
    page_dict.set(b"Annots".to_vec(), Object::Array(vec![Object::Ref(hl_ref)]));
    let page_ref = doc.add(Object::Dict(page_dict));

    doc.update_dict(pages_ref, |d| {
        d.set(b"Kids".to_vec(), Object::Array(vec![Object::Ref(page_ref)]));
        d.set(b"Count".to_vec(), Object::Int(1));
    })
    .unwrap();

    let drawn = flatten_document(&mut doc).unwrap();
    assert_eq!(drawn, 1);
    assert!(doc.get(page_ref).as_dict().unwrap().get(b"Annots").is_none());

    // 2. 未实现自动合成的可见注释（如无 /AP 的 /FreeText）必须返回 Unsupported，绝不静默删除
    let mut doc2 = Document::new_empty();
    let root2 = doc2.root().unwrap();
    let pages_ref2 = doc2
        .get(root2)
        .as_dict()
        .unwrap()
        .reference(b"Pages")
        .unwrap();
    let mut ft = Dict::new();
    ft.set(b"Type".to_vec(), Object::name("Annot"));
    ft.set(b"Subtype".to_vec(), Object::name("FreeText"));
    ft.set(
        b"Rect".to_vec(),
        Object::Array(vec![
            Object::Int(50),
            Object::Int(60),
            Object::Int(150),
            Object::Int(90),
        ]),
    );
    let ft_ref = doc2.add(Object::Dict(ft));
    let mut page_dict2 = Dict::new();
    page_dict2.set(b"Type".to_vec(), Object::name("Page"));
    page_dict2.set(b"Parent".to_vec(), Object::Ref(pages_ref2));
    page_dict2.set(
        b"MediaBox".to_vec(),
        Object::Array(vec![
            Object::Int(0),
            Object::Int(0),
            Object::Int(200),
            Object::Int(220),
        ]),
    );
    page_dict2.set(b"Annots".to_vec(), Object::Array(vec![Object::Ref(ft_ref)]));
    let page_ref2 = doc2.add(Object::Dict(page_dict2));
    doc2.update_dict(pages_ref2, |d| {
        d.set(b"Kids".to_vec(), Object::Array(vec![Object::Ref(page_ref2)]));
        d.set(b"Count".to_vec(), Object::Int(1));
    })
    .unwrap();

    assert!(matches!(
        flatten_document(&mut doc2),
        Err(BackendError::Unsupported(_))
    ));
}

#[test]
fn test_flatten_respects_norotate_flag_on_rotated_page() {
    use super::flatten::flatten_document;
    use pdfcraft_cos::{Dict, Document, Object, Stream};

    let mut doc = Document::new_empty();
    let root = doc.root().unwrap();
    let pages_ref = doc
        .get(root)
        .as_dict()
        .unwrap()
        .reference(b"Pages")
        .unwrap();

    let mut ap_dict = Dict::new();
    ap_dict.set(
        b"BBox".to_vec(),
        Object::Array(vec![
            Object::Int(0),
            Object::Int(0),
            Object::Int(100),
            Object::Int(40),
        ]),
    );
    let ap_ref = doc.add(Object::Stream(Stream::flate(
        ap_dict,
        b"1 0 0 rg 0 0 100 40 re f",
    )));

    let mut ap_map = Dict::new();
    ap_map.set(b"N".to_vec(), Object::Ref(ap_ref));

    // /F 20 = Print (4) + NoRotate (16)
    let mut stamp = Dict::new();
    stamp.set(b"Type".to_vec(), Object::name("Annot"));
    stamp.set(b"Subtype".to_vec(), Object::name("Stamp"));
    stamp.set(b"F".to_vec(), Object::Int(20));
    stamp.set(
        b"Rect".to_vec(),
        Object::Array(vec![
            Object::Int(50),
            Object::Int(60),
            Object::Int(150),
            Object::Int(100),
        ]),
    );
    stamp.set(b"AP".to_vec(), Object::Dict(ap_map));
    let stamp_ref = doc.add(Object::Dict(stamp));

    let mut page_dict = Dict::new();
    page_dict.set(b"Type".to_vec(), Object::name("Page"));
    page_dict.set(b"Parent".to_vec(), Object::Ref(pages_ref));
    page_dict.set(b"Rotate".to_vec(), Object::Int(90));
    page_dict.set(
        b"MediaBox".to_vec(),
        Object::Array(vec![
            Object::Int(0),
            Object::Int(0),
            Object::Int(200),
            Object::Int(220),
        ]),
    );
    page_dict.set(
        b"Annots".to_vec(),
        Object::Array(vec![Object::Ref(stamp_ref)]),
    );
    let page_ref = doc.add(Object::Dict(page_dict));

    doc.update_dict(pages_ref, |d| {
        d.set(b"Kids".to_vec(), Object::Array(vec![Object::Ref(page_ref)]));
        d.set(b"Count".to_vec(), Object::Int(1));
    })
    .unwrap();

    let drawn = flatten_document(&mut doc).unwrap();
    assert_eq!(drawn, 1);

    let contents = doc
        .get(page_ref)
        .as_dict()
        .unwrap()
        .get(b"Contents")
        .unwrap()
        .clone();
    let overlay_stream_ref = match contents {
        Object::Ref(r) => r,
        Object::Array(arr) => arr.last().unwrap().as_ref().unwrap(),
        _ => panic!("unexpected /Contents type"),
    };
    let obj = doc.get(overlay_stream_ref);
    let Object::Stream(s) = &*obj else {
        panic!("expected overlay stream");
    };
    let text = String::from_utf8_lossy(&s.decoded().unwrap()).to_string();
    assert!(
        text.contains("0.000000 1.000000 -1.000000 0.000000 90.000000 100.000000 cm"),
        "unexpected NoRotate cm in overlay stream: {text}"
    );
}

#[test]
fn test_print_merge_strips_embedded_files_while_structural_preserves() {
    let (_guard, dir) = test_temp_dir("docsy_test_embedded_files");
    let input_pdf = dir.join("with_attachments.pdf");
    let out_print = dir.join("out_print.pdf");
    let out_struct = dir.join("out_struct.pdf");

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        }),
    );
    let files_tree_id = doc.add_object(dictionary! {
        "Names" => vec![Object::string_literal("test.txt"), Object::Reference(page_id)],
    });
    let names_id = doc.add_object(dictionary! {
        "EmbeddedFiles" => Object::Reference(files_tree_id),
    });
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
        "Names" => Object::Reference(names_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(&input_pdf).unwrap();

    let backend = PdfCraftBackend::new();

    // 1. Structural merge: attachments must be preserved
    let req_struct = MergeRequest {
        inputs: vec![input_pdf.to_string_lossy().to_string()],
        output_path: out_struct.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Structural, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };
    let res_struct = backend.merge(&req_struct).unwrap();
    let bytes_struct = std::fs::read(&res_struct.output_path).unwrap();
    let doc_struct = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes_struct)).unwrap();
    let cat_struct = doc_struct.get(doc_struct.root().unwrap()).as_dict().unwrap().clone();
    let names_struct = cat_struct.get(b"Names").and_then(|n| doc_struct.dict(n));
    assert!(names_struct.is_some() && names_struct.unwrap().contains(b"EmbeddedFiles"), "Structural 模式应保留 EmbeddedFiles");

    // 2. Print merge: attachments must be stripped
    let req_print = MergeRequest {
        inputs: vec![input_pdf.to_string_lossy().to_string()],
        output_path: out_print.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Print, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };
    let res_print = backend.merge(&req_print).unwrap();
    let bytes_print = std::fs::read(&res_print.output_path).unwrap();
    let doc_print = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes_print)).unwrap();
    let cat_print = doc_print.get(doc_print.root().unwrap()).as_dict().unwrap().clone();
    let names_print = cat_print.get(b"Names").and_then(|n| doc_print.dict(n));
    assert!(
        names_print.is_none() || !names_print.unwrap().contains(b"EmbeddedFiles"),
        "Print 模式必须移除 EmbeddedFiles 附件"
    );
}

#[test]
fn test_print_merge_fallback_keeps_anticopy_backup() {
    let (_guard, dir) = test_temp_dir("docsy_test_anticopy_fallback");
    let input_pdf = dir.join("with_backup.pdf");
    let out_pdf = dir.join("out_merged.pdf");

    create_test_pdf(&input_pdf, 1, 200.0, 200.0);

    // 注入 DocsyAntiCopyBackup 到 /Info
    let backup_data = serde_json::json!({
        "version": 2,
        "fonts": [{
            "font_obj": "12 0",
            "base_font": "/DocsyProtected",
            "mapping": {"A": "B"}
        }]
    }).to_string();

    let bytes = std::fs::read(&input_pdf).unwrap();
    let mut doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)).unwrap();
    let mut info_dict = pdfcraft_cos::Dict::new();
    info_dict.set(
        b"DocsyAntiCopyBackup".to_vec(),
        pdfcraft_cos::Object::String(pdfcraft_cos::PdfString::literal(
            format!("u:{backup_data}").into_bytes(),
        )),
    );
    let info_ref = doc.add(pdfcraft_cos::Object::Dict(info_dict));
    doc.trailer_mut().set(b"Info".to_vec(), pdfcraft_cos::Object::Ref(info_ref));
    let out_bytes = pdfcraft_cos::write_full(&doc, &pdfcraft_cos::SaveOptions::default()).unwrap();
    std::fs::write(&input_pdf, out_bytes).unwrap();

    let req = MergeRequest {
        inputs: vec![input_pdf.to_string_lossy().to_string()],
        output_path: out_pdf.clone(),
        policy: OutputPolicy::for_policy(MergePolicy::Print, false),
        preferred_backend: Some(BackendType::PdfCraft),
        conversion_state: None,
    };

    let res = dispatch_merge(&req).expect("Print 合并回退应成功");
    assert_eq!(res.backend, BackendType::Qpdf);
    assert!(res.fallback_reason.is_some());

    // 验证 qpdf 产物保留了 trailer /Info 中的 DocsyAntiCopyBackup
    let out_merged_bytes = std::fs::read(&res.output_path).unwrap();
    let out_merged_doc = pdfcraft_cos::Document::open(std::sync::Arc::new(out_merged_bytes)).unwrap();
    let info_text = pdfcraft_organize::info(&out_merged_doc, "DocsyAntiCopyBackup");
    assert!(info_text.is_some(), "qpdf 回退输出必须保留 DocsyAntiCopyBackup");
    assert!(info_text.unwrap().contains("\"version\":2"));
}

#[test]
fn test_bookmarks_native_rejects_v1_anticopy_backup() {
    let (_guard, dir) = test_temp_dir("docsy_test_bookmarks_v1");
    let input_pdf = dir.join("v1_backup.pdf");
    create_test_pdf(&input_pdf, 1, 200.0, 200.0);

    let backup_data = serde_json::json!({
        "version": 1,
        "fonts": []
    }).to_string();

    let bytes = std::fs::read(&input_pdf).unwrap();
    let mut doc = pdfcraft_cos::Document::open(std::sync::Arc::new(bytes)).unwrap();
    let mut info_dict = pdfcraft_cos::Dict::new();
    info_dict.set(
        b"DocsyAntiCopyBackup".to_vec(),
        pdfcraft_cos::Object::String(pdfcraft_cos::PdfString::literal(
            format!("u:{backup_data}").into_bytes(),
        )),
    );
    let info_ref = doc.add(pdfcraft_cos::Object::Dict(info_dict));
    doc.trailer_mut().set(b"Info".to_vec(), pdfcraft_cos::Object::Ref(info_ref));
    let out_bytes = pdfcraft_cos::write_full(&doc, &pdfcraft_cos::SaveOptions::default()).unwrap();
    std::fs::write(&input_pdf, &out_bytes).unwrap();

    let before = std::fs::read(&input_pdf).unwrap();
    let result = crate::pdf::bookmarks::apply_bookmarks(
        &input_pdf,
        &[crate::pdf::header_footer::BookmarkConfig {
            enabled: true,
            label: "测试书签".to_string(),
            page_index: 0,
        }],
        true,
    );
    assert!(result.is_err(), "v1 恢复备份必须被书签操作拦截拒绝");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("旧版防复制恢复备份"));
    assert_eq!(std::fs::read(&input_pdf).unwrap(), before, "原文件不得被修改");
}
