use std::collections::BTreeMap;

use toki_templates::{
    AssetReferenceKind, AttackMode, TemplateDescriptor, TemplateEnumOption, TemplateParameter,
    TemplateParameterKind, TemplateProviderError, TemplateProviderErrorCode, TemplateSemanticItem,
    TemplateSemanticPlan, TemplateValue, TEMPLATE_SEMANTIC_VERSION,
};

use crate::templates::BuiltInTemplate;
use crate::value_reader::ParameterReader;

pub(crate) struct PlayerAttackTemplate;

impl BuiltInTemplate for PlayerAttackTemplate {
    fn descriptor(&self) -> TemplateDescriptor {
        TemplateDescriptor {
            id: "toki/player_attack".into(),
            display_name: "Player Attack".into(),
            category: "combat".into(),
            description: "Creates a primary attack behavior for a player-like entity.".into(),
            parameters: vec![
                TemplateParameter {
                    id: "actor_entity_definition_id".into(),
                    label: "Actor".into(),
                    description: Some(
                        "Entity definition that should gain the attack behavior.".into(),
                    ),
                    kind: TemplateParameterKind::EntityDefinitionReference,
                    default: None,
                    required: true,
                },
                TemplateParameter {
                    id: "attack_mode".into(),
                    label: "Attack Mode".into(),
                    description: Some("Whether the attack behaves as melee or projectile.".into()),
                    kind: TemplateParameterKind::Enum {
                        options: vec![
                            TemplateEnumOption {
                                id: "melee".into(),
                                label: "Melee".into(),
                                description: None,
                            },
                            TemplateEnumOption {
                                id: "projectile".into(),
                                label: "Projectile".into(),
                                description: None,
                            },
                        ],
                    },
                    default: Some(TemplateValue::Enum("melee".into())),
                    required: true,
                },
                TemplateParameter {
                    id: "damage".into(),
                    label: "Damage".into(),
                    description: Some("Damage dealt by the attack.".into()),
                    kind: TemplateParameterKind::Integer {
                        min: Some(1),
                        max: Some(999),
                        step: Some(1),
                    },
                    default: Some(TemplateValue::Integer(8)),
                    required: true,
                },
                TemplateParameter {
                    id: "cooldown_ticks".into(),
                    label: "Cooldown".into(),
                    description: Some(
                        "Ticks required before the attack can be triggered again.".into(),
                    ),
                    kind: TemplateParameterKind::Integer {
                        min: Some(0),
                        max: Some(9999),
                        step: Some(1),
                    },
                    default: Some(TemplateValue::Integer(20)),
                    required: true,
                },
                TemplateParameter {
                    id: "animation_state".into(),
                    label: "Animation State".into(),
                    description: Some("Optional attack animation state.".into()),
                    kind: TemplateParameterKind::Optional {
                        inner: Box::new(TemplateParameterKind::String {
                            multiline: false,
                            min_length: None,
                            max_length: None,
                        }),
                    },
                    default: Some(TemplateValue::Optional(None)),
                    required: false,
                },
                TemplateParameter {
                    id: "projectile_entity_definition_id".into(),
                    label: "Projectile Entity".into(),
                    description: Some(
                        "Projectile entity definition used when attack mode is projectile.".into(),
                    ),
                    kind: TemplateParameterKind::Optional {
                        inner: Box::new(TemplateParameterKind::EntityDefinitionReference),
                    },
                    default: Some(TemplateValue::Optional(None)),
                    required: false,
                },
                TemplateParameter {
                    id: "sound_id".into(),
                    label: "Sound".into(),
                    description: Some("Optional audio asset id to play when attacking.".into()),
                    kind: TemplateParameterKind::Optional {
                        inner: Box::new(TemplateParameterKind::AssetReference {
                            asset_kind: AssetReferenceKind::Audio,
                        }),
                    },
                    default: Some(TemplateValue::Optional(None)),
                    required: false,
                },
            ],
        }
    }

    fn instantiate(
        &self,
        parameters: &BTreeMap<String, TemplateValue>,
    ) -> Result<TemplateSemanticPlan, TemplateProviderError> {
        let reader = ParameterReader::new(parameters);
        let actor_entity_definition_id =
            reader.required_entity_definition_reference("actor_entity_definition_id")?;
        let mode = match reader.required_enum("attack_mode")?.as_str() {
            "melee" => AttackMode::Melee,
            "projectile" => AttackMode::Projectile,
            other => {
                return Err(TemplateProviderError::new(
                    TemplateProviderErrorCode::SemanticValidation,
                    format!("unsupported attack_mode '{other}'"),
                ));
            }
        };
        let damage = u32::try_from(reader.required_integer("damage")?).map_err(|_| {
            TemplateProviderError::new(
                TemplateProviderErrorCode::SemanticValidation,
                "damage must be non-negative",
            )
        })?;
        let cooldown_ticks =
            u32::try_from(reader.required_integer("cooldown_ticks")?).map_err(|_| {
                TemplateProviderError::new(
                    TemplateProviderErrorCode::SemanticValidation,
                    "cooldown_ticks must be non-negative",
                )
            })?;
        let animation_state = reader.optional_string("animation_state")?;
        let projectile_entity_definition_id =
            reader.optional_entity_definition_reference("projectile_entity_definition_id")?;
        let sound_id = reader.optional_asset_reference("sound_id")?;

        if matches!(mode, AttackMode::Projectile) && projectile_entity_definition_id.is_none() {
            return Err(TemplateProviderError::new(
                TemplateProviderErrorCode::SemanticValidation,
                "projectile attack mode requires 'projectile_entity_definition_id'",
            ));
        }

        Ok(TemplateSemanticPlan {
            semantic_version: TEMPLATE_SEMANTIC_VERSION,
            items: vec![TemplateSemanticItem::CreateAttackBehavior {
                id: format!("{actor_entity_definition_id}_primary_attack"),
                actor_entity_definition_id: Some(actor_entity_definition_id),
                trigger_input_action: "attack_primary".into(),
                mode,
                cooldown_ticks,
                damage,
                animation_state,
                projectile_entity_definition_id,
                sound_id,
            }],
        })
    }
}
