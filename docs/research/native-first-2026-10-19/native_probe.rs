// Isolated upstream probe; not compiled into Docsy.
use pdfcraft_cos::{Document, Object, SaveOptions, write_full};
use std::{fs, path::Path, sync::Arc};

fn counts(doc: &Document) -> (usize, usize, usize) {
    let mut annotations = 0;
    let mut widgets = 0;
    for page in pdfcraft_organize::pages(doc).unwrap_or_default() {
        let page_obj = doc.get(page.obj);
        let list = page_obj.as_dict().and_then(|d| d.get(b"Annots")).map(|o| doc.resolve(o));
        if let Some(list) = list.as_ref().and_then(|o| o.as_array()) {
            annotations += list.len();
            for item in list {
                if doc.resolve(item).as_dict().is_some_and(|d| d.name(b"Subtype") == Some(b"Widget")) {
                    widgets += 1;
                }
            }
        }
    }
    let fields = doc.root().and_then(|r| {
        let root = doc.get(r);
        let af = doc.resolve(root.as_dict()?.get(b"AcroForm")?);
        let fields = doc.resolve(af.as_dict()?.get(b"Fields")?);
        Some(fields.as_array()?.len())
    }).unwrap_or(0);
    (annotations, widgets, fields)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let root = Path::new(&args[1]);
    fs::create_dir_all(root.join("native"))?;
    let mut paths: Vec<_> = fs::read_dir(root.join("inputs"))?.filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        let name = path.file_stem().unwrap().to_string_lossy();
        let bytes = fs::read(&path)?;
        if name.starts_with("crypto_") {
            for (label, password) in [("none", None), ("wrong", Some("wrong")), ("user", Some("user")), ("owner", Some("owner"))] {
                match Document::open_with_password(Arc::new(bytes.clone()), password) {
                    Ok(mut doc) => {
                        let pages = pdfcraft_organize::page_count(&doc)?;
                        doc.remove_encryption();
                        let saved = write_full(&doc, &SaveOptions::default())?;
                        let reopened = Document::open(Arc::new(saved.clone()))?;
                        println!("crypto\t{name}\t{label}\tOK\tpages={pages}\tunencrypted={}", reopened.security().is_none());
                        fs::write(root.join("native").join(format!("{name}-{label}.pdf")), saved)?;
                    }
                    Err(e) => println!("crypto\t{name}\t{label}\tERR\t{e}"),
                }
            }
            continue;
        }
        let mut doc = match Document::open(Arc::new(bytes)) {
            Ok(d) => d,
            Err(e) => { println!("open\t{name}\tERR\t{e}"); continue; }
        };
        if name.starts_with("repair_") {
            println!("repair\t{name}\tOK\tpages={}\treconstructed={}\tlog={:?}", pdfcraft_organize::page_count(&doc)?, doc.reconstructed(), doc.repair_log());
        } else {
            let before = counts(&doc);
            match pdfcraft_edit::flatten(&mut doc, &[0], true, true) {
                Ok(n) => println!("flatten\t{name}\tOK\tdrawn={n}\tbefore={before:?}\tafter={:?}", counts(&doc)),
                Err(e) => println!("flatten\t{name}\tERR\t{e}"),
            }
        }
        fs::write(root.join("native").join(format!("{name}.pdf")), write_full(&doc, &SaveOptions::default())?)?;
    }
    // Full-save garbage collection: one unreachable object, one catalog-reachable payload.
    let mut doc = Document::open(Arc::new(fs::read(root.join("inputs/normal_widget.pdf"))?))?;
    doc.add(Object::String(pdfcraft_cos::PdfString::literal(b"UNREACHABLE_SENTINEL".to_vec())));
    let referenced = doc.add(Object::String(pdfcraft_cos::PdfString::literal(b"REACHABLE_SENTINEL".to_vec())));
    doc.update_dict(doc.root().unwrap(), |d| d.set(b"PrivateReachable".to_vec(), Object::Ref(referenced)))?;
    let saved = write_full(&doc, &SaveOptions::default())?;
    let reopened = Document::open(Arc::new(saved.clone()))?;
    let unreachable_found = reopened.scan_objects().any(|(_,o)| o.as_string().is_some_and(|s| s.bytes == b"UNREACHABLE_SENTINEL"));
    let catalog = reopened.get(reopened.root().unwrap());
    let kept = catalog.as_dict().and_then(|d| d.get(b"PrivateReachable")).map(|o| reopened.resolve(o)).is_some_and(|o| o.as_string().is_some_and(|s| s.bytes == b"REACHABLE_SENTINEL"));
    println!("gc\tunreachable_removed={}\treachable_kept={}", !unreachable_found, kept);
    fs::write(root.join("native/gc.pdf"), saved)?;
    // An unused XObject remains reachable until resource pruning removes its dictionary entry.
    let mut doc = Document::open(Arc::new(fs::read(root.join("inputs/normal_widget.pdf"))?))?;
    let mut form = pdfcraft_cos::Dict::new();
    form.set(b"Type".to_vec(), Object::name("XObject"));
    form.set(b"Subtype".to_vec(), Object::name("Form"));
    form.set(b"BBox".to_vec(), Object::Array(vec![0,0,10,10].into_iter().map(Object::Int).collect()));
    form.set(b"Resources".to_vec(), Object::Dict(pdfcraft_cos::Dict::new()));
    let unused = doc.add(Object::Stream(pdfcraft_cos::Stream::from_raw(form.clone(), b"%UNUSED_RESOURCE\n".to_vec())));
    let used = doc.add(Object::Stream(pdfcraft_cos::Stream::from_raw(form, b"0 1 0 rg 0 0 10 10 re f\n".to_vec())));
    let page_ref = pdfcraft_organize::pages(&doc)?[0].obj;
    doc.update_dict(page_ref, |d| {
        let mut xo = pdfcraft_cos::Dict::new();
        xo.set(b"Unused".to_vec(), Object::Ref(unused));
        xo.set(b"Used".to_vec(), Object::Ref(used));
        let mut resources = pdfcraft_cos::Dict::new();
        resources.set(b"XObject".to_vec(), Object::Dict(xo));
        d.set(b"Resources".to_vec(), Object::Dict(resources));
    })?;
    let has_unused = |d: &Document| d.scan_objects().any(|(_, o)| match &*o {
        Object::Stream(s) => s.decoded().is_ok_and(|b| b.windows(15).any(|w| w == b"UNUSED_RESOURCE")),
        _ => false,
    });
    let full = Document::open(Arc::new(write_full(&doc, &SaveOptions::default())?))?;
    let extracted = pdfcraft_organize::extract_pages(&doc, &[0])?;
    let extracted = Document::open(Arc::new(write_full(&extracted, &SaveOptions::default())?))?;
    let content = doc.add(Object::Stream(pdfcraft_cos::Stream::from_raw(pdfcraft_cos::Dict::new(), b"q /Used Do Q\n".to_vec())));
    doc.update_dict(page_ref, |d| d.set(b"Contents".to_vec(), Object::Ref(content)))?;
    let with_do = pdfcraft_organize::extract_pages(&doc, &[0])?;
    let with_do = Document::open(Arc::new(write_full(&with_do, &SaveOptions::default())?))?;
    println!("resources\tfull_save_keeps_unused={}\textract_without_do_keeps_unused={}\textract_with_do_prunes_unused={}", has_unused(&full), has_unused(&extracted), !has_unused(&with_do));
    pdfcraft_organize::add_bookmark(&mut doc, &[], 0, "证据一", 0)?;
    let reopened = Document::open(Arc::new(write_full(&doc, &SaveOptions::default())?))?;
    let bookmarks = pdfcraft_organize::bookmarks(&reopened);
    println!("bookmarks\tcount={}\tchinese_title={}\tpage={:?}", bookmarks.len(), bookmarks[0].title == "证据一", bookmarks[0].page);
    Ok(())
}
