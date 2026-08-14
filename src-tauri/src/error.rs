use serde::Serialize;

/// 应用级错误类型，序列化后传递给前端展示。
///
/// 前端根据 `kind` 字段区分错误类型并展示不同提示。
/// `#[non_exhaustive]` 保留后续新增变体的空间。
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[non_exhaustive]
pub enum DocsyError {
    #[error("文件不存在: {path}")]
    FileNotFound { path: String },

    #[error("参数错误: {message}")]
    InvalidArgument { message: String },

    #[error("I/O 错误: {message}")]
    Io { message: String },

    #[error("外部工具错误: {message}")]
    #[allow(dead_code)]
    ExternalTool { message: String },

    #[error("PDF 解析错误: {message}")]
    #[allow(dead_code)]
    PdfParse { message: String },

    #[error("操作已取消")]
    Cancelled,

    #[error("未知错误: {message}")]
    Unknown { message: String },
}

impl From<anyhow::Error> for DocsyError {
    fn from(err: anyhow::Error) -> Self {
        // 沿错误链查找已知错误类型
        let msg = err.to_string();

        if msg.contains("不存在") || msg.contains("not found") || msg.contains("No such file") {
            // 尝试提取路径
            let path = msg
                .split(&[':', '\n'][..])
                .next_back()
                .unwrap_or(&msg)
                .trim()
                .to_string();
            return DocsyError::FileNotFound { path };
        }
        if msg.contains("已取消") || msg.contains("cancelled") || msg.contains("canceled") {
            return DocsyError::Cancelled;
        }
        if msg.contains("参数") || msg.contains("缺少") || msg.contains("invalid") {
            return DocsyError::InvalidArgument { message: msg };
        }

        DocsyError::Unknown { message: msg }
    }
}

impl From<std::io::Error> for DocsyError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => DocsyError::FileNotFound {
                path: err.to_string(),
            },
            _ => DocsyError::Io {
                message: err.to_string(),
            },
        }
    }
}
