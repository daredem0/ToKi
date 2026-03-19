mod descriptor;
mod error;
mod parameter;
mod protocol;
mod provider;
mod semantic;

pub use descriptor::TemplateDescriptor;
pub use error::TemplateContractError;
pub use parameter::{
    AssetReferenceKind, TemplateEnumOption, TemplateParameter, TemplateParameterKind, TemplateValue,
};
pub use protocol::{
    TemplateInstantiateRequest, TemplateProviderRequest, TemplateProviderResponse,
    TEMPLATE_PROTOCOL_VERSION,
};
pub use provider::{
    TemplateInstantiation, TemplateProvider, TemplateProviderError, TemplateProviderErrorCode,
};
pub use semantic::{
    AttackMode, TemplateSemanticItem, TemplateSemanticPlan, TemplateSurfaceAction,
    TemplateTargetDomain, TEMPLATE_SEMANTIC_VERSION,
};
