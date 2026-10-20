#![allow(dead_code)]
use std::{path::{Path,PathBuf},sync::Arc};
use pdfcraft_cos::{Document,SaveOptions,write_full,Object};
use serde_json::json;
mod external {
 pub struct QpdfTool;
 pub trait ExternalTool {fn binary_path(&self)->anyhow::Result<std::path::PathBuf>;}
 impl ExternalTool for QpdfTool {fn binary_path(&self)->anyhow::Result<std::path::PathBuf>{Ok("/opt/homebrew/bin/qpdf".into())}}
 pub fn hidden_command(p:impl AsRef<std::ffi::OsStr>)->std::process::Command {std::process::Command::new(p)}
 pub fn command_failure_detail(o:&std::process::Output)->String {String::from_utf8_lossy(&o.stderr).into_owned()}
}
mod util { #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/util/fs.rs"] pub mod fs; }
pub struct ConversionState;impl ConversionState {pub fn new()->Self{Self}}
mod operations {pub fn check_current_cancelled()->anyhow::Result<()>{Ok(())}}
mod pdf {
 #[path="/var/folders/tz/q100sk452ndb8fk0wtqhc3mc0000gn/T/docsy-native-followup-review-20261019-jr4q2tg3/src/bookmarks_before.rs"]pub mod bookmarks_before;
 #[path="/var/folders/tz/q100sk452ndb8fk0wtqhc3mc0000gn/T/docsy-native-followup-review-20261019-jr4q2tg3/src/page_info_before.rs"]pub mod page_info_before;
 #[path="/var/folders/tz/q100sk452ndb8fk0wtqhc3mc0000gn/T/docsy-native-followup-review-20261019-jr4q2tg3/src/normalize_before.rs"]pub mod normalize_before;
 #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/blank.rs"]pub mod blank;
 pub mod word_convert {
  pub fn is_word_file(s:&str)->bool{[".doc",".docx",".docm"].iter().any(|e|s.ends_with(e))}
  pub fn convert_word_to_pdf_file(_:&str,_:&std::path::Path,_:&std::sync::Arc<crate::ConversionState>)->anyhow::Result<String>{anyhow::bail!("Word not used in synthetic PDF probe")}
 }

 pub use crate::util::fs::temp_named_path;
 pub mod qpdf {pub fn status_is_success(s:&std::process::ExitStatus)->bool{s.success()||s.code()==Some(3)}}
 pub mod header_footer {pub struct BookmarkConfig {pub enabled:bool,pub label:String,pub page_index:u32}}
 pub mod chunked {
  pub fn should_chunk(_:&std::path::Path,_:u32)->bool{false}
  pub fn process_pdf_chunked<F>(a:&std::path::Path,b:&std::path::Path,n:u32,mut f:F)->anyhow::Result<()>where F:FnMut(&std::path::Path,&std::path::Path,usize,u32,u32)->anyhow::Result<()>{f(a,b,0,1,n)}
 }
 #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/page_info.rs"] pub mod page_info;
 #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/normalize.rs"] pub mod normalize;
 #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/bookmarks.rs"] pub mod bookmarks;
 pub mod backend {
  use std::{path::PathBuf,sync::Arc};use serde::{Serialize,Deserialize};
  #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/policy.rs"]pub mod policy;
  pub use policy::{OutputPolicy,PreflightSummary,BackendWarning,FallbackReason};
/// 支持的 PDF 处理后端类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    /// 成熟稳定的 qpdf 生产默认后端。
    Qpdf,
    /// 纯 Rust pdfcraft 原生页面组织候选后端。
    PdfCraft,
}

impl Default for BackendType {
    fn default() -> Self {
        Self::PdfCraft
    }
}

/// 后端统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("操作已取消")]
    Cancelled,
    #[error("不支持的 PDF 特性: {0}")]
    Unsupported(String),
    #[error("PDF 解析失败: {0}")]
    ParseError(String),
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    #[error("后端执行错误 ({backend:?}): {message}")]
    ExecutionError {
        backend: BackendType,
        message: String,
    },
    #[error("{0}")]
    Other(String),
}

/// 合并请求结构体。
#[derive(Clone)]
pub struct MergeRequest {
    pub inputs: Vec<String>,
    pub output_path: PathBuf,
    pub policy: OutputPolicy,
    pub preferred_backend: Option<BackendType>,
    pub conversion_state: Option<Arc<crate::ConversionState>>,
}

/// 合并执行结果结构体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub output_path: PathBuf,
    pub backend: BackendType,
    pub total_pages: u32,
    pub warnings: Vec<BackendWarning>,
    pub renamed_fields: Vec<(String, String)>,
    pub fallback_reason: Option<FallbackReason>,
}

/// 提取请求结构体（页码对外统一为 1-based）。
#[derive(Clone)]
pub struct ExtractRequest {
    pub input_path: PathBuf,
    pub pages: Vec<u32>,
    pub output_dir: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub preferred_backend: Option<BackendType>,
}

/// 提取执行结果结构体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub output_path: PathBuf,
    pub backend: BackendType,
    pub total_pages: u32,
    pub input_size: u64,
    pub output_size: u64,
    pub warnings: Vec<BackendWarning>,
    pub fallback_reason: Option<FallbackReason>,
}

/// PDF 后端统一接口 trait。
pub trait PdfBackend: Send + Sync {
    #[allow(dead_code)]
    fn backend_type(&self) -> BackendType;
    fn preflight_merge(&self, req: &MergeRequest) -> Result<PreflightSummary, BackendError>;
    fn preflight_extract(&self, req: &ExtractRequest) -> Result<PreflightSummary, BackendError>;
    fn merge(&self, req: &MergeRequest) -> Result<MergeResult, BackendError>;
    fn extract(&self, req: &ExtractRequest) -> Result<ExtractResult, BackendError>;
}

  #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/pdfcraft_backend.rs"]pub mod pdfcraft_backend;

  #[path="/Users/only/Documents/PythonProgram/Docsy/src-tauri/src/pdf/backend/flatten.rs"]pub mod flatten;
 }
}
fn open(p:&Path)->Document {Document::open(Arc::new(std::fs::read(p).unwrap())).unwrap()}
fn main()->anyhow::Result<()> {
 let root=PathBuf::from(std::env::args().nth(1).unwrap());let inputs=root.join("inputs");let out=root.join("outputs");let mut rows=vec![];
 for ent in std::fs::read_dir(&inputs)? {
  let p=ent?.path();let name=p.file_stem().unwrap().to_string_lossy();if p.extension().and_then(|s|s.to_str())!=Some("pdf"){continue;}
  let infos=pdf::page_info::get_page_infos(&p.to_string_lossy());
  rows.push(json!({"case":name,"page_info":infos.as_ref().ok().map(|is|is.iter().map(|i|json!({"width":i.width_pt,"height":i.height_pt,"raw_width":i.raw_width_pt,"x0":i.box_x0})).collect::<Vec<_>>())}));
  if name.starts_with("unit_") {
   let normalized_before=pdf::normalize_before::normalize_pdf_to_a4(&p,150,"portrait","none",0.0)?;std::fs::copy(normalized_before,out.join(format!("{name}-a4-before.pdf")))?;
   let normalized=pdf::normalize::normalize_pdf_to_a4(&p,150,"portrait","none",0.0)?;std::fs::copy(normalized,out.join(format!("{name}-a4.pdf")))?;
   let doc=open(&p);let mut combiner=pdfcraft_organize::Combiner::new();combiner.add("test",&doc,None)?;let combined=combiner.finish_with_renames()?.doc;std::fs::write(out.join(format!("{name}-combined.pdf")),write_full(&combined,&SaveOptions::default())?)?;
  }else if name.starts_with("bookmark_") {
   let target=out.join(format!("{name}-bookmarks.pdf"));std::fs::copy(&p,&target)?;
   pdf::bookmarks::apply_bookmarks(&target,&[pdf::header_footer::BookmarkConfig{enabled:true,label:"Probe".into(),page_index:0}],false)?;
   let doc=open(&target);rows.push(json!({"case":name,"encrypted_after_bookmark":doc.security().is_some()}));
   let before_target=out.join(format!("{name}-bookmarks-before.pdf"));std::fs::copy(&p,&before_target)?;pdf::bookmarks_before::apply_bookmarks(&before_target,&[pdf::header_footer::BookmarkConfig{enabled:true,label:"Probe".into(),page_index:0}],false)?;let before_doc=open(&before_target);rows.push(json!({"case":name,"encrypted_before_patch_bookmark":before_doc.security().is_some()}));

  } else {
   use pdf::backend::PdfBackend;
   let req=pdf::backend::MergeRequest{inputs:vec![p.to_string_lossy().to_string()],output_path:out.join(format!("{name}-backend.pdf")),policy:pdf::backend::OutputPolicy::for_policy(pdf::backend::policy::MergePolicy::Print,false),preferred_backend:Some(pdf::backend::BackendType::PdfCraft),conversion_state:None};
   let backend=pdf::backend::pdfcraft_backend::PdfCraftBackend::new();let result=backend.merge(&req);
   rows.push(json!({"case":name,"production_backend_success":result.is_ok(),"production_backend_error":result.err().map(|e|e.to_string())}));
   let mut original=open(&p);let direct=pdf::backend::flatten::flatten_document(&mut original);rows.push(json!({"case":name,"direct_flatten_success":direct.is_ok(),"direct_error":direct.err().map(|e|e.to_string())}));
   let doc=open(&p);let mut c=pdfcraft_organize::Combiner::new();c.add("test",&doc,None)?;let mut d=c.finish_with_renames()?.doc;
   let flat=pdf::backend::flatten::flatten_document(&mut d);let apn=flat.as_ref().ok().copied();let err=flat.err().map(|e|e.to_string());
   rows.push(json!({"case":name,"drawn":apn,"error":err}));
   std::fs::write(out.join(format!("{name}-native.pdf")),write_full(&d,&SaveOptions::default())?)?;
  }
 }
 std::fs::write(root.join("results.json"),serde_json::to_vec_pretty(&rows)?)?;println!("{}",serde_json::to_string_pretty(&rows)?);Ok(())
}
