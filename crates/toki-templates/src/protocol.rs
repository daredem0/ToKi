use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{TemplateDescriptor, TemplateProviderError, TemplateSemanticPlan, TemplateValue};

pub const TEMPLATE_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateInstantiateRequest {
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u32,
    pub template_id: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, TemplateValue>,
}

impl TemplateInstantiateRequest {
    pub fn new(
        template_id: impl Into<String>,
        parameters: BTreeMap<String, TemplateValue>,
    ) -> Self {
        Self {
            protocol_version: TEMPLATE_PROTOCOL_VERSION,
            template_id: template_id.into(),
            parameters,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateProviderRequest {
    List {
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
    },
    Describe {
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
        template_id: String,
    },
    Instantiate {
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
        template_id: String,
        #[serde(default)]
        parameters: BTreeMap<String, TemplateValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateProviderResponse {
    List {
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
        templates: Vec<TemplateDescriptor>,
    },
    Describe {
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
        descriptor: TemplateDescriptor,
    },
    Instantiate {
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
        descriptor: TemplateDescriptor,
        plan: TemplateSemanticPlan,
    },
    Error {
        #[serde(default = "default_protocol_version")]
        protocol_version: u32,
        error: TemplateProviderError,
    },
}

const fn default_protocol_version() -> u32 {
    TEMPLATE_PROTOCOL_VERSION
}
