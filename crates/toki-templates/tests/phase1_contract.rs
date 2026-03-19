use std::collections::BTreeMap;

use toki_templates::{
    AssetReferenceKind, AttackMode, TemplateDescriptor, TemplateEnumOption,
    TemplateInstantiateRequest, TemplateParameter, TemplateParameterKind, TemplateProviderRequest,
    TemplateProviderResponse, TemplateSemanticItem, TemplateSemanticPlan, TemplateTargetDomain,
    TemplateValue, TEMPLATE_PROTOCOL_VERSION, TEMPLATE_SEMANTIC_VERSION,
};

fn sample_descriptor() -> TemplateDescriptor {
    TemplateDescriptor {
        id: "toki/player_attack".into(),
        display_name: "Player Attack".into(),
        category: "combat".into(),
        description: "Creates a primary player attack behavior.".into(),
        parameters: vec![
            TemplateParameter {
                id: "damage".into(),
                label: "Damage".into(),
                description: Some("Base damage dealt by the attack.".into()),
                kind: TemplateParameterKind::Integer {
                    min: Some(1),
                    max: Some(99),
                    step: Some(1),
                },
                default: Some(TemplateValue::Integer(8)),
                required: true,
            },
            TemplateParameter {
                id: "projectile_asset".into(),
                label: "Projectile".into(),
                description: None,
                kind: TemplateParameterKind::Optional {
                    inner: Box::new(TemplateParameterKind::AssetReference {
                        asset_kind: AssetReferenceKind::ObjectSheet,
                    }),
                },
                default: Some(TemplateValue::Optional(None)),
                required: false,
            },
            TemplateParameter {
                id: "attack_mode".into(),
                label: "Attack Mode".into(),
                description: None,
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
        ],
    }
}

#[test]
fn descriptor_round_trips_and_validates() {
    let descriptor = sample_descriptor();
    descriptor.validate().expect("descriptor should validate");

    let json = serde_json::to_string_pretty(&descriptor).expect("serialize descriptor");
    let decoded: TemplateDescriptor = serde_json::from_str(&json).expect("deserialize descriptor");

    assert_eq!(decoded, descriptor);
}

#[test]
fn descriptor_rejects_default_value_type_mismatch() {
    let mut descriptor = sample_descriptor();
    descriptor.parameters[0].default = Some(TemplateValue::String("wrong".into()));

    let error = descriptor
        .validate()
        .expect_err("mismatched default value must fail");

    assert!(
        error.to_string().contains("damage"),
        "error should mention the offending parameter: {error}"
    );
}

#[test]
fn descriptor_rejects_duplicate_parameter_ids() {
    let mut descriptor = sample_descriptor();
    descriptor.parameters.push(descriptor.parameters[0].clone());

    let error = descriptor
        .validate()
        .expect_err("duplicate parameter ids must fail");

    assert!(error.to_string().contains("damage"));
}

#[test]
fn semantic_plan_round_trips_and_exposes_target_domains() {
    let plan = TemplateSemanticPlan {
        semantic_version: TEMPLATE_SEMANTIC_VERSION,
        items: vec![
            TemplateSemanticItem::CreateAttackBehavior {
                id: "player_primary_attack".into(),
                actor_entity_definition_id: Some("player".into()),
                trigger_input_action: "attack_primary".into(),
                mode: AttackMode::Projectile,
                cooldown_ticks: 20,
                damage: 8,
                animation_state: Some("attack".into()),
                projectile_entity_definition_id: Some("rock_projectile".into()),
                sound_id: Some("sfx_attack".into()),
            },
            TemplateSemanticItem::CreateConfirmationDialog {
                id: "exit_confirm".into(),
                title: "Exit Game?".into(),
                body: "Unsaved progress may be lost.".into(),
                confirm_label: "Exit".into(),
                cancel_label: "Cancel".into(),
                confirm_action: toki_templates::TemplateSurfaceAction::ExitRuntime,
                cancel_action: toki_templates::TemplateSurfaceAction::CloseSurface,
            },
        ],
    };

    let json = serde_json::to_string_pretty(&plan).expect("serialize plan");
    let decoded: TemplateSemanticPlan = serde_json::from_str(&json).expect("deserialize plan");

    assert_eq!(decoded, plan);
    assert_eq!(
        plan.items[0].target_domains(),
        vec![
            TemplateTargetDomain::Rules,
            TemplateTargetDomain::EntityDefinitions
        ]
    );
    assert_eq!(
        plan.items[1].target_domains(),
        vec![TemplateTargetDomain::MenusDialogs]
    );
}

#[test]
fn instantiate_request_defaults_to_current_protocol_version() {
    let mut parameters = BTreeMap::new();
    parameters.insert("damage".into(), TemplateValue::Integer(8));

    let request = TemplateInstantiateRequest::new("toki/player_attack", parameters.clone());

    assert_eq!(request.protocol_version, TEMPLATE_PROTOCOL_VERSION);
    assert_eq!(request.template_id, "toki/player_attack");
    assert_eq!(request.parameters, parameters);
}

#[test]
fn provider_request_and_response_round_trip() {
    let request = TemplateProviderRequest::Describe {
        protocol_version: TEMPLATE_PROTOCOL_VERSION,
        template_id: "toki/player_attack".into(),
    };
    let response = TemplateProviderResponse::Describe {
        protocol_version: TEMPLATE_PROTOCOL_VERSION,
        descriptor: sample_descriptor(),
    };

    let request_json = serde_json::to_string(&request).expect("serialize request");
    let response_json = serde_json::to_string(&response).expect("serialize response");

    let decoded_request: TemplateProviderRequest =
        serde_json::from_str(&request_json).expect("deserialize request");
    let decoded_response: TemplateProviderResponse =
        serde_json::from_str(&response_json).expect("deserialize response");

    assert_eq!(decoded_request, request);
    assert_eq!(decoded_response, response);
}
