mod descriptor;
mod error;
mod parameter;
mod protocol;
mod semantic;

pub use descriptor::TemplateDescriptor;
pub use error::TemplateContractError;
pub use parameter::{
    AssetReferenceKind, TemplateEnumOption, TemplateParameter, TemplateParameterKind, TemplateValue,
};
pub use protocol::{
    TemplateInstantiateRequest, TemplateProviderError, TemplateProviderRequest,
    TemplateProviderResponse, TEMPLATE_PROTOCOL_VERSION,
};
pub use semantic::{
    AttackMode, TemplateSemanticItem, TemplateSemanticPlan, TemplateSurfaceAction,
    TemplateTargetDomain, TEMPLATE_SEMANTIC_VERSION,
};
