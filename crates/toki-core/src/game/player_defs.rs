use crate::entity::{
    AiBehavior, AiConfig, AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef,
    EntityDefinition, MovementProfile, MovementSoundTrigger, RenderingDef,
};
use crate::ids::EntityDefName;

pub(super) struct PlayerDefinitionConfig {
    pub(super) name: &'static str,
    pub(super) display_name: &'static str,
    pub(super) description: &'static str,
    pub(super) health: Option<u32>,
    pub(super) speed: f32,
    pub(super) can_move: bool,
    pub(super) has_inventory: bool,
    pub(super) ai_config: AiConfig,
    pub(super) movement_profile: MovementProfile,
    pub(super) tags: &'static [&'static str],
}

fn player_definition(config: PlayerDefinitionConfig) -> EntityDefinition {
    EntityDefinition {
        name: EntityDefName::from(config.name),
        display_name: config.display_name.to_string(),
        description: config.description.to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        attributes: AttributesDef {
            health: config.health,
            stats: std::collections::HashMap::new(),
            speed: config.speed,
            solid: true,
            active: true,
            can_move: config.can_move,
            interactable: false,
            interaction_reach: 0,
            ai_config: config.ai_config,
            movement_profile: config.movement_profile,
            primary_projectile: None,
            pickup: None,
            has_inventory: config.has_inventory,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "sfx_slime_bounce".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![
                AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["slime/idle_0".to_string(), "slime/idle_1".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "walk".to_string(),
                    frame_tiles: vec![
                        "slime/walk_0".to_string(),
                        "slime/walk_1".to_string(),
                        "slime/walk_2".to_string(),
                        "slime/walk_3".to_string(),
                    ],
                    frame_positions: None,
                    frame_duration_ms: 150.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
            ],
            default_state: "idle".to_string(),
        },
        category: "human".to_string(),
        tags: config.tags.iter().map(|tag| (*tag).to_string()).collect(),
    }
}

pub(super) fn default_player_definition() -> EntityDefinition {
    player_definition(PlayerDefinitionConfig {
        name: "player",
        display_name: "Player",
        description: "Default player entity",
        health: Some(100),
        speed: 2.0,
        can_move: true,
        has_inventory: true,
        ai_config: AiConfig::default(),
        movement_profile: MovementProfile::PlayerWasd,
        tags: &["player"],
    })
}

pub(super) fn player_like_npc_definition() -> EntityDefinition {
    player_definition(PlayerDefinitionConfig {
        name: "player_like_npc",
        display_name: "Player-like NPC",
        description: "NPC using the player visual style",
        health: Some(50),
        speed: 1.0,
        can_move: false,
        has_inventory: false,
        ai_config: AiConfig::from_legacy_behavior(AiBehavior::Wander),
        movement_profile: MovementProfile::None,
        tags: &["npc"],
    })
}
