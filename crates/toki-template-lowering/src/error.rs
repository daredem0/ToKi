use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateLoweringErrorCode {
    UnsupportedSemanticVersion,
    MissingEntityDefinition,
    UnsupportedSemanticItem,
    InvalidLoweringTarget,
    ApplyFailed,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{message}")]
pub struct TemplateLoweringError {
    pub code: TemplateLoweringErrorCode,
    pub message: String,
}

impl TemplateLoweringError {
    pub fn new(code: TemplateLoweringErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
