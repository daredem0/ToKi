mod templates;
mod value_reader;

use std::collections::BTreeMap;

use templates::{
    BuiltInTemplate, ExitConfirmationDialogTemplate, PickupCollectTemplate, PlayerAttackTemplate,
};
use toki_templates::{
    TemplateContractError, TemplateDescriptor, TemplateInstantiation, TemplateProvider,
    TemplateProviderError, TemplateProviderErrorCode, TemplateValue,
};

pub struct BuiltInTemplateRegistry {
    templates: BTreeMap<String, Box<dyn BuiltInTemplate>>,
}

impl BuiltInTemplateRegistry {
    pub fn new() -> Self {
        let mut templates: BTreeMap<String, Box<dyn BuiltInTemplate>> = BTreeMap::new();
        templates.insert("toki/player_attack".into(), Box::new(PlayerAttackTemplate));
        templates.insert(
            "toki/pickup_collect".into(),
            Box::new(PickupCollectTemplate),
        );
        templates.insert(
            "toki/exit_confirmation_dialog".into(),
            Box::new(ExitConfirmationDialogTemplate),
        );
        Self { templates }
    }

    fn template(&self, template_id: &str) -> Result<&dyn BuiltInTemplate, TemplateProviderError> {
        self.templates
            .get(template_id)
            .map(Box::as_ref)
            .ok_or_else(|| {
                TemplateProviderError::new(
                    TemplateProviderErrorCode::TemplateNotFound,
                    format!("unknown built-in template '{template_id}'"),
                )
            })
    }

    fn validated_descriptor(
        &self,
        template_id: &str,
    ) -> Result<TemplateDescriptor, TemplateProviderError> {
        let descriptor = self.template(template_id)?.descriptor();
        descriptor.validate().map_err(internal_contract_error)?;
        Ok(descriptor)
    }
}

impl Default for BuiltInTemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateProvider for BuiltInTemplateRegistry {
    fn list_templates(&self) -> Vec<TemplateDescriptor> {
        self.templates
            .keys()
            .filter_map(|template_id| self.validated_descriptor(template_id).ok())
            .collect()
    }

    fn describe_template(
        &self,
        template_id: &str,
    ) -> Result<TemplateDescriptor, TemplateProviderError> {
        self.validated_descriptor(template_id)
    }

    fn instantiate_template(
        &self,
        template_id: &str,
        parameters: BTreeMap<String, TemplateValue>,
    ) -> Result<TemplateInstantiation, TemplateProviderError> {
        let template = self.template(template_id)?;
        let descriptor = self.validated_descriptor(template_id)?;
        let resolved_parameters = descriptor
            .resolve_parameters(&parameters)
            .map_err(invalid_parameter_error)?;
        let plan = template.instantiate(&resolved_parameters)?;

        Ok(TemplateInstantiation { descriptor, plan })
    }
}

fn invalid_parameter_error(error: TemplateContractError) -> TemplateProviderError {
    TemplateProviderError::new(
        TemplateProviderErrorCode::InvalidParameters,
        error.to_string(),
    )
}

fn internal_contract_error(error: TemplateContractError) -> TemplateProviderError {
    TemplateProviderError::new(TemplateProviderErrorCode::Internal, error.to_string())
}
