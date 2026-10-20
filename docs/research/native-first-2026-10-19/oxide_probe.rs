use pdf_oxide::PdfDocument;
fn main() {
    for path in std::env::args().skip(1) {
        match PdfDocument::open(&path).and_then(|d| Ok((d.get_page_media_box(0)?, d.extract_spans(0)?))) {
            Ok((page_box, spans)) => println!("{path}\tpage_box={page_box:?}\t{spans:?}"),
            Err(error) => println!("{path}\tERROR {error}"),
        }
    }
}
