use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{TemplateDescriptor, TemplateSemanticPlan, TemplateValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateProviderErrorCode {
    TemplateNotFound,
    InvalidParameters,
    SemanticValidation,
    UnsupportedProtocolVersion,
    UnsupportedSemanticVersion,
    BuildFailed,
    InvocationFailed,
    TimedOut,
    ProtocolViolation,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateProviderError {
    pub code: TemplateProviderErrorCode,
    pub message: String,
}

impl TemplateProviderError {
    pub fn new(code: TemplateProviderErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateInstantiation {
    pub descriptor: TemplateDescriptor,
    pub plan: TemplateSemanticPlan,
}

pub trait TemplateProvider {
    fn list_templates(&self) -> Result<Vec<TemplateDescriptor>, TemplateProviderError>;

    fn describe_template(
        &self,
        template_id: &str,
    ) -> Result<TemplateDescriptor, TemplateProviderError>;

    fn instantiate_template(
        &self,
        template_id: &str,
        parameters: BTreeMap<String, TemplateValue>,
    ) -> Result<TemplateInstantiation, TemplateProviderError>;
}
