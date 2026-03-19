use std::collections::{BTreeMap, HashMap};

use toki_core::entity::{
    AiBehavior, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
    MovementProfile, MovementSoundTrigger, RenderingDef,
};
use toki_template_builtins::BuiltInTemplateRegistry;
use toki_template_lowering::{
    EntityDefinitionResolver, LoweredTemplateOperation, TemplateLowerer, TemplateLoweringError,
    TemplateLoweringErrorCode,
};
use toki_templates::{TemplateProvider, TemplateValue};

#[derive(Default)]
struct InMemoryEntityResolver {
    definitions: HashMap<String, EntityDefinition>,
}

impl InMemoryEntityResolver {
    fn with_definition(mut self, definition: EntityDefinition) -> Self {
        self.definitions.insert(definition.name.clone(), definition);
        self
    }
}

impl EntityDefinitionResolver for InMemoryEntityResolver {
    fn load_entity_definition(
        &self,
        entity_definition_id: &str,
    ) -> Result<Option<EntityDefinition>, TemplateLoweringError> {
        Ok(self.definitions.get(entity_definition_id).cloned())
    }
}

fn sample_actor_definition() -> EntityDefinition {
    EntityDefinition {
        name: "player".to_string(),
        display_name: "Player".to_string(),
        description: "Test player".to_string(),
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

fn instantiate_player_attack(
    parameters: BTreeMap<String, TemplateValue>,
) -> toki_templates::TemplateInstantiation {
    let registry = BuiltInTemplateRegistry::new();
    registry
        .instantiate_template("toki/player_attack", parameters)
        .expect("instantiation should succeed")
}

#[test]
fn lowering_melee_player_attack_writes_primary_action_to_actor_definition() {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "actor_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("player".into()),
    );
    parameters.insert("attack_mode".into(), TemplateValue::Enum("melee".into()));
    parameters.insert("damage".into(), TemplateValue::Integer(11));
    parameters.insert("cooldown_ticks".into(), TemplateValue::Integer(24));
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

    let instantiation = instantiate_player_attack(parameters);
    let resolver = InMemoryEntityResolver::default().with_definition(sample_actor_definition());
    let lowered = TemplateLowerer::new()
        .lower_plan(&instantiation.plan, &resolver)
        .expect("lowering should succeed");

    assert_eq!(lowered.operations.len(), 1);
    let LoweredTemplateOperation::UpsertEntityDefinition {
        entity_definition_id,
        definition,
    } = &lowered.operations[0];
    assert_eq!(entity_definition_id, "player");
    let primary_action = definition
        .attributes
        .primary_action
        .as_ref()
        .expect("lowered definition should have a primary action");
    assert_eq!(
        primary_action.mode,
        toki_core::entity::PrimaryActionMode::Melee
    );
    assert_eq!(primary_action.damage, 11);
    assert_eq!(primary_action.cooldown_ticks, 24);
    assert_eq!(
        primary_action.animation_state.as_deref(),
        Some("attack_right")
    );
    assert_eq!(primary_action.sound_id.as_deref(), Some("sfx_attack"));
    assert!(primary_action.projectile.is_none());
    assert!(
        definition.attributes.primary_projectile.is_none(),
        "lowered authored primary action should replace legacy primary_projectile state"
    );
}

#[test]
fn lowering_player_attack_is_deterministic_for_same_inputs() {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "actor_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("player".into()),
    );
    parameters.insert("attack_mode".into(), TemplateValue::Enum("melee".into()));
    parameters.insert("damage".into(), TemplateValue::Integer(8));
    parameters.insert("cooldown_ticks".into(), TemplateValue::Integer(20));

    let instantiation = instantiate_player_attack(parameters);
    let resolver = InMemoryEntityResolver::default().with_definition(sample_actor_definition());
    let lowerer = TemplateLowerer::new();

    let first = lowerer
        .lower_plan(&instantiation.plan, &resolver)
        .expect("first lowering should succeed");
    let second = lowerer
        .lower_plan(&instantiation.plan, &resolver)
        .expect("second lowering should succeed");

    assert_eq!(first.operations.len(), second.operations.len());
    let LoweredTemplateOperation::UpsertEntityDefinition {
        entity_definition_id: first_id,
        definition: first_definition,
    } = &first.operations[0];
    let LoweredTemplateOperation::UpsertEntityDefinition {
        entity_definition_id: second_id,
        definition: second_definition,
    } = &second.operations[0];
    assert_eq!(first_id, second_id);
    let first_json = serde_json::to_string_pretty(first_definition)
        .expect("first lowered definition should serialize");
    let second_json = serde_json::to_string_pretty(second_definition)
        .expect("second lowered definition should serialize");
    assert_eq!(first_json, second_json);
}

#[test]
fn lowering_player_attack_rejects_missing_actor_definition() {
    let mut parameters = BTreeMap::new();
    parameters.insert(
        "actor_entity_definition_id".into(),
        TemplateValue::EntityDefinitionReference("missing_player".into()),
    );
    parameters.insert("attack_mode".into(), TemplateValue::Enum("melee".into()));
    parameters.insert("damage".into(), TemplateValue::Integer(8));
    parameters.insert("cooldown_ticks".into(), TemplateValue::Integer(20));

    let instantiation = instantiate_player_attack(parameters);
    let resolver = InMemoryEntityResolver::default();
    let error = TemplateLowerer::new()
        .lower_plan(&instantiation.plan, &resolver)
        .expect_err("missing actor definition must fail");

    assert_eq!(
        error.code,
        TemplateLoweringErrorCode::MissingEntityDefinition
    );
    assert!(error.message.contains("missing_player"));
}

#[test]
fn lowering_player_attack_projectile_mode_fails_cleanly_until_projectile_lowering_exists() {
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

    let instantiation = instantiate_player_attack(parameters);
    let resolver = InMemoryEntityResolver::default().with_definition(sample_actor_definition());
    let error = TemplateLowerer::new()
        .lower_plan(&instantiation.plan, &resolver)
        .expect_err("projectile lowering should fail explicitly in this slice");

    assert_eq!(
        error.code,
        TemplateLoweringErrorCode::UnsupportedSemanticItem
    );
    assert!(error.message.contains("projectile attack lowering"));
}
