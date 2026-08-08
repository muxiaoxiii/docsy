use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DocsyError {
    #[error("文件不存在: {path}")]
    FileNotFound { path: String },

    #[error("参数错误: {message}")]
    InvalidArgument { message: String },
    #[error("未知错误: {message}")]
    Unknown { message: String },
}

impl From<anyhow::Error> for DocsyError {
    fn from(err: anyhow::Error) -> Self {
        DocsyError::Unknown {
            message: err.to_string(),
        }
    }
}
