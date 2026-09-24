//! Manual probe: compare office_oxide vs doc2x yellow-highlight preservation on a .doc fixture.
use std::path::PathBuf;

fn count_yellow(docx: &PathBuf) -> usize {
    let bytes = std::fs::read(docx).expect("read docx");
    // crude but effective: look for w:highlight with yellow in uncompressed members via zip crate if available,
    // else scan whole buffer for ascii markers (zip stores names but not always content).
    // Prefer zip crate already in dependency tree.
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("zip");
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().to_string();
        if !(name.ends_with(".xml") && (name.contains("word/") || name.contains("header") || name.contains("footer") || name.contains("document"))) {
            continue;
        }
        let mut buf = String::new();
        use std::io::Read;
        file.read_to_string(&mut buf).ok();
        // yellow highlight markers
        count += buf.matches("w:highlight").count();
        let _ = &buf;
    }
    count
}

fn convert_oxide(doc: &PathBuf, out: &PathBuf) {
    let d = office_oxide::Document::open(doc.display().to_string()).expect("oxide open");
    d.save_as(out.display().to_string()).expect("oxide save");
}

fn convert_doc2x(doc: &PathBuf, out: &PathBuf) {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/bin/doc2x");
    assert!(bin.is_file(), "doc2x missing at {}", bin.display());
    let status = std::process::Command::new(&bin)
        .arg(doc)
        .arg("-o")
        .arg(out)
        .arg("-v")
        .arg("error")
        .status()
        .expect("run doc2x");
    assert!(status.success(), "doc2x failed");
}

#[test]
fn compare_engines_on_user_fixture() {
    let doc = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test-pdf/无效授权委托书-模板.doc");
    if !doc.is_file() {
        eprintln!("skip: fixture missing at {}", doc.display());
        return;
    }
    let tmp = std::env::temp_dir();
    let oxide_out = tmp.join("docsy-probe-oxide.docx");
    let doc2x_out = tmp.join("docsy-probe-doc2x.docx");
    let _ = std::fs::remove_file(&oxide_out);
    let _ = std::fs::remove_file(&doc2x_out);

    convert_oxide(&doc, &oxide_out);
    convert_doc2x(&doc, &doc2x_out);

    let oxide_marks = count_yellow(&oxide_out);
    let doc2x_marks = count_yellow(&doc2x_out);
    println!("oxide highlights={oxide_marks} doc2x highlights={doc2x_marks}");
    // Also print yellow-specific via simple scan of document.xml
    for (label, path) in [("oxide", &oxide_out), ("doc2x", &doc2x_out)] {
        let bytes = std::fs::read(path).unwrap();
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut xml = String::new();
        if let Ok(mut f) = archive.by_name("word/document.xml") {
            use std::io::Read;
            f.read_to_string(&mut xml).ok();
        }
        let yellow = xml.matches("w:val=\"yellow\"").count() + xml.matches("w:val='yellow'").count();
        let any_hl = xml.matches("w:highlight").count();
        let shd = xml.matches("FFFF00").count() + xml.matches("ffff00").count();
        println!("{label}: document.xml yellow_val={yellow} highlight={any_hl} shd_FFFF00={shd}");
    }
}
