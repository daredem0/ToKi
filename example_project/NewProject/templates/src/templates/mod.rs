use std::collections::BTreeMap;

use toki_templates::{
    TemplateDescriptor, TemplateInstantiation, TemplateProviderError, TemplateProviderErrorCode,
    TemplateValue,
};

// __TOKI_TEMPLATE_MODULES__
pub mod player_attack_clone;

pub fn list_templates() -> Vec<TemplateDescriptor> {
    vec![
        // __TOKI_TEMPLATE_LIST__
        player_attack_clone::descriptor(),
    ]
}

pub fn describe_template(template_id: &str) -> Result<TemplateDescriptor, TemplateProviderError> {
    match template_id {
        // __TOKI_TEMPLATE_DESCRIBE__
        "project/player_attack_clone" => Ok(player_attack_clone::descriptor()),
        _ => Err(TemplateProviderError::new(
            TemplateProviderErrorCode::TemplateNotFound,
            format!("unknown project template '{template_id}'"),
        )),
    }
}

pub fn instantiate_template(
    template_id: &str,
    parameters: BTreeMap<String, TemplateValue>,
) -> Result<TemplateInstantiation, TemplateProviderError> {
    match template_id {
        // __TOKI_TEMPLATE_INSTANTIATE__
        "project/player_attack_clone" => player_attack_clone::instantiate(parameters)
            .map(|plan| TemplateInstantiation {
                descriptor: player_attack_clone::descriptor(),
                plan,
            }),
        _ => Err(TemplateProviderError::new(
            TemplateProviderErrorCode::TemplateNotFound,
            format!("unknown project template '{template_id}'"),
        )),
    }
}
