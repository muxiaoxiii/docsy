use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[non_exhaustive] // 保留后续新增错误变体的空间，外部匹配须带 `_` 分支
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
