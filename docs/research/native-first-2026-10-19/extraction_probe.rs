// Minimal adapter for the unchanged Docsy extraction PoC; copied beside extract_poc.rs.
#[derive(Debug)] pub enum BackendType { PdfCraft }
#[derive(Debug)] pub enum BackendError {
    IoError(std::io::Error), ParseError(String), Other(String),
    ExecutionError { backend: BackendType, message: String },
}
mod extract_poc;
fn main() {
    for path in std::env::args().skip(1) {
        let result = extract_poc::extract_document_text(std::path::Path::new(&path));
        match result {
            Ok(r) => println!("{}", serde_json::json!({"path":path,"result":r})),
            Err(e) => println!("{}", serde_json::json!({"path":path,"error":format!("{e:?}")})),
        }
    }
}
