use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidYaml,
    InvalidStructure,
    InvalidInterface,
    SourceMismatch,
    AegProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerError {
    pub code: ErrorCode,
    pub path: Option<String>,
    pub message: String,
}

impl CompilerError {
    pub(crate) fn new(
        code: ErrorCode,
        path: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for CompilerError {}
