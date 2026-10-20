use std::{fs,path::Path,sync::Arc};
use pdfcraft_cos::{Document, SaveOptions, write_full};
use pdfcraft_organize::{Combiner,extract_pages,page_count};
fn main() -> Result<(),Box<dyn std::error::Error>> {
 let dir=std::env::args().nth(1).expect("data directory");let dir=Path::new(&dir);
 let mut combiner=Combiner::new();
 for name in ["A","B"] {
  let doc=Document::open(Arc::new(fs::read(dir.join(format!("input-{name}.pdf")))?))?;
  println!("source {name} pages={}",page_count(&doc)?);
  combiner.add(name,&doc,None)?;
 }
 let outcome=combiner.finish_with_renames()?;
 println!("merged pages={} renamed_fields={:?}",page_count(&outcome.doc)?,outcome.renamed_fields);
 fs::write(dir.join("candidate-merged.pdf"),write_full(&outcome.doc,&SaveOptions::default())?)?;
 let source=Document::open(Arc::new(fs::read(dir.join("input-A.pdf"))?))?;
 let extracted=extract_pages(&source,&[1,0])?;
 fs::write(dir.join("candidate-extracted.pdf"),write_full(&extracted,&SaveOptions::default())?)?;
 Ok(())
}