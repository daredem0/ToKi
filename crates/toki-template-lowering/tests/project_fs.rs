use std::collections::{BTreeMap, HashMap};
use std::fs;

use tempfile::tempdir;
use toki_core::entity::{
    AiBehavior, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
    MovementProfile, MovementSoundTrigger, RenderingDef,
};
use toki_template_builtins::BuiltInTemplateRegistry;
use toki_template_lowering::lower_and_apply_plan_to_project;
use toki_templates::{TemplateProvider, TemplateValue};

fn sample_actor_definition() -> EntityDefinition {
    EntityDefinition {
        name: "player".to_string(),
        display_name: "Player".to_string(),
        description: "Template application target".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 1,
            visible: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: HashMap::new(),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: Some(toki_core::entity::PrimaryProjectileDef {
                sheet: "fauna".to_string(),
                object_name: "legacy_rock".to_string(),
                size: [16, 16],
                speed: 4,
                damage: 8,
                lifetime_ticks: 20,
                spawn_offset: [0, 0],
            }),
            primary_action: None,
            pickup: None,
            has_inventory: true,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "sfx_step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "players.json".to_string(),
            clips: vec![toki_core::entity::AnimationClipDef {
                state: "attack_right".to_string(),
                frame_tiles: vec!["player/attack_right_a".to_string()],
                frame_duration_ms: 180.0,
                loop_mode: "once".to_string(),
            }],
            default_state: "idle_down".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["player".to_string()],
    }
}

fn instantiate_melee_attack_plan() -> toki_templates::TemplateSemanticPlan {
    let registry = BuiltInTemplateRegistry::new();
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "actor_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("player".into()),
    );
    parameters.insert("attack_mode".into(), TemplateValue::Enum("melee".into()));
    parameters.insert("damage".into(), TemplateValue::Integer(9));
    parameters.insert("cooldown_ticks".into(), TemplateValue::Integer(18));
    parameters.insert(
        "animation_state".into(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::String("attack_right".into())))),
    );
    parameters.insert(
        "sound_id".into(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::AssetReference(
            "sfx_attack".into(),
        )))),
    );

    registry
        .instantiate_template("toki/player_attack", parameters)
        .expect("instantiation should succeed")
        .plan
}

#[test]
fn lower_and_apply_plan_to_project_persists_lowered_attack_behavior_to_entity_file() {
    let temp = tempdir().expect("temp dir should exist");
    let project_root = temp.path();
    fs::create_dir_all(project_root.join("entities")).expect("entities dir should exist");
    let actor_path = project_root.join("entities/player.json");
    fs::write(
        &actor_path,
        serde_json::to_string_pretty(&sample_actor_definition())
            .expect("actor definition should serialize"),
    )
    .expect("actor definition should write");

    lower_and_apply_plan_to_project(project_root, &instantiate_melee_attack_plan())
        .expect("lowering and apply should succeed");

    let reloaded: EntityDefinition = serde_json::from_str(
        &fs::read_to_string(&actor_path).expect("entity definition should read"),
    )
    .expect("entity definition should deserialize");
    let primary_action = reloaded
        .attributes
        .primary_action
        .expect("lowered primary action should persist");

    assert_eq!(primary_action.damage, 9);
    assert_eq!(primary_action.cooldown_ticks, 18);
    assert_eq!(
        primary_action.animation_state.as_deref(),
        Some("attack_right")
    );
    assert_eq!(primary_action.sound_id.as_deref(), Some("sfx_attack"));
    assert!(
        reloaded.attributes.primary_projectile.is_none(),
        "legacy projectile field should be cleared after lowering"
    );
}

#[test]
fn lower_and_apply_plan_to_project_surfaces_projectile_lowering_error_without_mutating_file() {
    let temp = tempdir().expect("temp dir should exist");
    let project_root = temp.path();
    fs::create_dir_all(project_root.join("entities")).expect("entities dir should exist");
    let actor_definition = sample_actor_definition();
    let actor_path = project_root.join("entities/player.json");
    fs::write(
        &actor_path,
        serde_json::to_string_pretty(&actor_definition).expect("actor definition should serialize"),
    )
    .expect("actor definition should write");

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
        "projectile_entity_definition_id".into(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::EntityDefinitionReference(
            "rock_projectile".into(),
        )))),
    );
    let plan = registry
        .instantiate_template("toki/player_attack", parameters)
        .expect("instantiation should succeed")
        .plan;

    let error = lower_and_apply_plan_to_project(project_root, &plan)
        .expect_err("projectile lowering should fail in this slice");

    assert!(error.message.contains("projectile attack lowering"));
    let reloaded: EntityDefinition = serde_json::from_str(
        &fs::read_to_string(&actor_path).expect("entity definition should read"),
    )
    .expect("entity definition should deserialize");
    assert_eq!(reloaded.attributes.primary_action, None);
    assert_eq!(
        reloaded.attributes.primary_projectile,
        actor_definition.attributes.primary_projectile
    );
}
