use std::collections::BTreeMap;

use toki_templates::{
    AttackMode, TemplateDescriptor, TemplateEnumOption, TemplateParameter,
    TemplateParameterKind, TemplateProviderError, TemplateProviderErrorCode,
    TemplateSemanticItem, TemplateSemanticPlan, TemplateValue,
};

pub fn descriptor() -> TemplateDescriptor {
    TemplateDescriptor {
        id: "project/player_attack_clone".to_string(),
        display_name: "Project Attack".to_string(),
        category: "combat".to_string(),
        description: "A project-local melee attack template that mirrors the built-in attack flow. Use it to verify project template discovery, build, preview, apply, and removal.".to_string(),
        parameters: vec![
            TemplateParameter {
                id: "actor_entity_definition_id".to_string(),
                label: "Actor".to_string(),
                description: None,
                kind: TemplateParameterKind::EntityDefinitionReference,
                default: None,
                required: true,
            },
            TemplateParameter {
                id: "attack_mode".to_string(),
                label: "Attack Mode".to_string(),
                description: None,
                kind: TemplateParameterKind::Enum {
                    options: vec![TemplateEnumOption {
                        id: "melee".to_string(),
                        label: "Melee".to_string(),
                        description: None,
                    }],
                },
                default: Some(TemplateValue::Enum("melee".to_string())),
                required: true,
            },
            TemplateParameter {
                id: "damage".to_string(),
                label: "Damage".to_string(),
                description: None,
                kind: TemplateParameterKind::Integer {
                    min: Some(1),
                    max: Some(99),
                    step: Some(1),
                },
                default: Some(TemplateValue::Integer(8)),
                required: true,
            },
            TemplateParameter {
                id: "cooldown_ticks".to_string(),
                label: "Cooldown".to_string(),
                description: None,
                kind: TemplateParameterKind::Integer {
                    min: Some(1),
                    max: Some(999),
                    step: Some(1),
                },
                default: Some(TemplateValue::Integer(20)),
                required: true,
            },
        ],
    }
}

pub fn instantiate(
    parameters: BTreeMap<String, TemplateValue>,
) -> Result<TemplateSemanticPlan, TemplateProviderError> {
    let descriptor = descriptor();
    let resolved_parameters = descriptor.resolve_parameters(&parameters).map_err(|error| {
        TemplateProviderError::new(
            TemplateProviderErrorCode::InvalidParameters,
            error.to_string(),
        )
    })?;

    let actor_entity_definition_id = match resolved_parameters.get("actor_entity_definition_id") {
        Some(TemplateValue::EntityDefinitionReference(value)) => value.clone(),
        _ => {
            return Err(TemplateProviderError::new(
                TemplateProviderErrorCode::InvalidParameters,
                "actor_entity_definition_id must be an entity definition reference",
            ))
        }
    };
    let damage = match resolved_parameters.get("damage") {
        Some(TemplateValue::Integer(value)) => *value as u32,
        _ => 8,
    };
    let cooldown_ticks = match resolved_parameters.get("cooldown_ticks") {
        Some(TemplateValue::Integer(value)) => *value as u32,
        _ => 20,
    };

    Ok(TemplateSemanticPlan {
        semantic_version: 1,
        items: vec![TemplateSemanticItem::CreateAttackBehavior {
            id: "project_player_attack".to_string(),
            actor_entity_definition_id: Some(actor_entity_definition_id),
            trigger_input_action: "attack_primary".to_string(),
            mode: AttackMode::Melee,
            cooldown_ticks,
            damage,
            animation_state: None,
            projectile_entity_definition_id: None,
            sound_id: None,
        }],
    })
}
