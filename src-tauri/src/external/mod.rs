pub mod ffmpeg;
pub mod libreoffice;
pub mod qpdf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub install_hint: String,
}

pub trait ExternalTool: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self) -> ToolStatus;
    fn try_install(&self) -> anyhow::Result<String>;
    fn binary_path(&self) -> anyhow::Result<std::path::PathBuf>;
}

pub use ffmpeg::FfmpegTool;
pub use libreoffice::LibreOfficeTool;
pub use qpdf::QpdfTool;

pub fn check_by_name(name: &str) -> ToolStatus {
    match name {
        "qpdf" => QpdfTool.check(),
        "ffmpeg" => FfmpegTool.check(),
        "libreoffice" => LibreOfficeTool.check(),
        _ => ToolStatus {
            available: false,
            path: None,
            version: None,
            install_hint: "未知工具".into(),
        },
    }
}

pub fn install_by_name(name: &str) -> anyhow::Result<String> {
    match name {
        "qpdf" => QpdfTool.try_install(),
        "ffmpeg" => FfmpegTool.try_install(),
        _ => anyhow::bail!("不支持自动安装 {}", name),
    }
}
