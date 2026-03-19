mod exit_confirmation_dialog;
mod pickup_collect;
mod player_attack;

use std::collections::BTreeMap;

use toki_templates::{
    TemplateDescriptor, TemplateProviderError, TemplateSemanticPlan, TemplateValue,
};

pub(crate) use exit_confirmation_dialog::ExitConfirmationDialogTemplate;
pub(crate) use pickup_collect::PickupCollectTemplate;
pub(crate) use player_attack::PlayerAttackTemplate;

pub(crate) trait BuiltInTemplate: Send + Sync {
    fn descriptor(&self) -> TemplateDescriptor;
    fn instantiate(
        &self,
        parameters: &BTreeMap<String, TemplateValue>,
    ) -> Result<TemplateSemanticPlan, TemplateProviderError>;
}
