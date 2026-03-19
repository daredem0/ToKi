use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TemplateContractError {
    #[error("template id must not be empty")]
    EmptyTemplateId,
    #[error("template display name must not be empty")]
    EmptyTemplateDisplayName,
    #[error("parameter id must not be empty")]
    EmptyParameterId,
    #[error("parameter '{parameter_id}' label must not be empty")]
    EmptyParameterLabel { parameter_id: String },
    #[error("duplicate parameter id '{id}'")]
    DuplicateParameterId { id: String },
    #[error("enum parameter '{parameter_id}' must define at least one option")]
    EmptyEnumOptions { parameter_id: String },
    #[error("enum parameter '{parameter_id}' has duplicate option id '{option_id}'")]
    DuplicateEnumOptionId {
        parameter_id: String,
        option_id: String,
    },
    #[error("parameter '{parameter_id}' has a default value of incompatible type: expected {expected}, got {actual}")]
    DefaultValueTypeMismatch {
        parameter_id: String,
        expected: String,
        actual: String,
    },
    #[error("missing required parameter '{parameter_id}'")]
    MissingRequiredParameter { parameter_id: String },
    #[error("unexpected parameter '{parameter_id}'")]
    UnexpectedParameter { parameter_id: String },
    #[error("parameter '{parameter_id}' has an incompatible value type: expected {expected}, got {actual}")]
    ParameterValueTypeMismatch {
        parameter_id: String,
        expected: String,
        actual: String,
    },
}
