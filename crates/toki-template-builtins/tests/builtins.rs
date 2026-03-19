use std::collections::BTreeMap;

use toki_template_builtins::BuiltInTemplateRegistry;
use toki_templates::{
    AttackMode, TemplateProvider, TemplateProviderErrorCode, TemplateSemanticItem, TemplateValue,
};

#[test]
fn built_in_registry_lists_expected_templates() {
    let registry = BuiltInTemplateRegistry::new();
    let mut templates = registry.list_templates();
    templates.sort_by(|a, b| a.id.cmp(&b.id));

    let ids: Vec<_> = templates.into_iter().map(|d| d.id).collect();
    assert_eq!(
        ids,
        vec![
            "toki/exit_confirmation_dialog",
            "toki/pickup_collect",
            "toki/player_attack",
        ]
    );
}

#[test]
fn player_attack_descriptor_exposes_expected_parameter_schema() {
    let registry = BuiltInTemplateRegistry::new();
    let descriptor = registry
        .describe_template("toki/player_attack")
        .expect("player attack descriptor should exist");

    descriptor.validate().expect("descriptor should validate");
    assert!(descriptor.parameters.iter().any(|p| p.id == "attack_mode"));
    assert!(descriptor.parameters.iter().any(|p| p.id == "damage"));
    assert!(descriptor
        .parameters
        .iter()
        .any(|p| p.id == "cooldown_ticks"));
}

#[test]
fn player_attack_instantiation_returns_semantic_attack_behavior() {
    let registry = BuiltInTemplateRegistry::new();
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "actor_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("player".into()),
    );
    parameters.insert(
        "attack_mode".into(),
        TemplateValue::Enum("projectile".into()),
    );
    parameters.insert("damage".into(), TemplateValue::Integer(8));
    parameters.insert("cooldown_ticks".into(), TemplateValue::Integer(20));
    parameters.insert(
        "animation_state".into(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::AnimationStateReference(
            "attack".into(),
        )))),
    );
    parameters.insert(
        "projectile_entity_definition_id".into(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::EntityDefinitionReference(
            "rock_projectile".into(),
        )))),
    );

    let instantiation = registry
        .instantiate_template("toki/player_attack", parameters)
        .expect("player attack instantiation should succeed");

    assert_eq!(instantiation.plan.items.len(), 1);
    assert!(matches!(
        &instantiation.plan.items[0],
        TemplateSemanticItem::CreateAttackBehavior {
            mode: AttackMode::Projectile,
            damage: 8,
            cooldown_ticks: 20,
            ..
        }
    ));
}

#[test]
fn projectile_attack_requires_projectile_definition() {
    let registry = BuiltInTemplateRegistry::new();
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "actor_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("player".into()),
    );
    parameters.insert(
        "attack_mode".into(),
        TemplateValue::Enum("projectile".into()),
    );
    parameters.insert("damage".into(), TemplateValue::Integer(8));
    parameters.insert("cooldown_ticks".into(), TemplateValue::Integer(20));

    let error = registry
        .instantiate_template("toki/player_attack", parameters)
        .expect_err("projectile attack without projectile id must fail");

    assert_eq!(error.code, TemplateProviderErrorCode::SemanticValidation);
    assert!(error.message.contains("projectile_entity_definition_id"));
}

#[test]
fn pickup_collect_instantiation_returns_semantic_pickup_behavior() {
    let registry = BuiltInTemplateRegistry::new();
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "pickup_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("coin_pickup".into()),
    );
    parameters.insert(
        "collector_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("player".into()),
    );
    parameters.insert("item_id".into(), TemplateValue::String("coin".into()));
    parameters.insert("count".into(), TemplateValue::Integer(1));

    let instantiation = registry
        .instantiate_template("toki/pickup_collect", parameters)
        .expect("pickup collect instantiation should succeed");

    assert!(matches!(
        &instantiation.plan.items[0],
        TemplateSemanticItem::CreatePickupBehavior {
            pickup_entity_definition_id,
            collector_entity_definition_id,
            item_id,
            count,
            ..
        } if pickup_entity_definition_id == "coin_pickup"
            && collector_entity_definition_id == "player"
            && item_id == "coin"
            && *count == 1
    ));
}

#[test]
fn exit_confirmation_dialog_instantiation_returns_dialog_semantics() {
    let registry = BuiltInTemplateRegistry::new();
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "dialog_id".into(),
        TemplateValue::String("exit_confirm".into()),
    );
    parameters.insert("title".into(), TemplateValue::String("Exit Game?".into()));
    parameters.insert(
        "body".into(),
        TemplateValue::String("Unsaved progress may be lost.".into()),
    );
    parameters.insert("confirm_label".into(), TemplateValue::String("Exit".into()));
    parameters.insert(
        "cancel_label".into(),
        TemplateValue::String("Cancel".into()),
    );

    let instantiation = registry
        .instantiate_template("toki/exit_confirmation_dialog", parameters)
        .expect("exit confirmation dialog should succeed");

    assert!(matches!(
        &instantiation.plan.items[0],
        TemplateSemanticItem::CreateConfirmationDialog { title, confirm_label, .. }
            if title == "Exit Game?" && confirm_label == "Exit"
    ));
}

#[test]
fn missing_template_returns_not_found_error() {
    let registry = BuiltInTemplateRegistry::new();
    let error = registry
        .describe_template("toki/does_not_exist")
        .expect_err("missing template must fail");

    assert_eq!(error.code, TemplateProviderErrorCode::TemplateNotFound);
}
