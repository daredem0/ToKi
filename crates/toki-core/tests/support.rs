#![allow(dead_code)]

use glam::{IVec2, UVec2};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use toki_core::assets::{
    atlas::{AtlasMeta, ColorMode, TileInfo, TileProperties},
    tilemap::TileMap,
};
use toki_core::collision::CollisionBox;
use toki_core::entity::{
    AiBehavior, AiComponent, AiConfig, AnimationClipDef, AnimationsDef, AudioDef, CollisionDef,
    CombatComponent, ComponentsDef, ControlRole, Entity, EntityAudioSettings, EntityDefinition,
    EntityKind, EntityRendering, EntityStats, Inventory, MovementComponent, MovementProfile,
    MovementSoundTrigger, RenderingDef,
};
use toki_core::game::SceneSystem;
use toki_core::{FlagValue, GameState, Scene};

pub fn test_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(10, 10),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec!["floor".to_string(); 100],
    }
}

pub fn test_atlas() -> AtlasMeta {
    let mut tiles = HashMap::new();
    tiles.insert(
        "floor".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );

    AtlasMeta {
        image: PathBuf::from("test_atlas.png"),
        tile_size: UVec2::new(16, 16),
        color_mode: ColorMode::TrueColor,
        palette: None,
        tiles,
    }
}

pub fn test_entity_definition(name: &str, category: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string().into(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        solid: true,
        active: true,
        components: ComponentsDef {
            movement: Some(MovementComponent {
                speed: 2.0,
                movement_profile: if category == "human" {
                    MovementProfile::PlayerWasd
                } else {
                    MovementProfile::None
                },
                can_move: true,
            }),
            ai: (category == "creature").then_some(AiComponent {
                ai_config: AiConfig::from_legacy_behavior(AiBehavior::Wander),
            }),
            combat: Some(CombatComponent {
                health: Some(100),
                stats: EntityStats::from_legacy_health(Some(100)),
            }),
            primary_projectile: None,
            pickup: None,
            inventory: None,
            ..Default::default()
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
            movement_sound: "sfx_step".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["slime/idle_0".to_string()],
                frame_positions: None,
                frame_duration_ms: 150.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: vec!["test".to_string()],
    }
}

pub fn test_entity() -> Entity {
    let mut controller = AnimationController::new();
    controller.add_clip(AnimationClip {
        state: AnimationState::Walk,
        atlas_name: "test_atlas".to_string(),
        frame_tile_names: vec!["frame1".to_string(), "frame2".to_string()],
        frame_positions: None,
        frame_duration_ms: 100.0,
        frame_durations_ms: None,
        loop_mode: LoopMode::Loop,
    });

    Entity {
        id: 42,
        position: IVec2::new(10, 20),
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Player,
        category: "human".to_string(),
        definition_name: Some("player".to_string().into()),
        persistent_across_saves: false,
        control_role: ControlRole::PlayerCharacter,
        audio: EntityAudioSettings {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: Some("sfx_step".to_string()),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        rendering: EntityRendering {
            visible: true,
            has_shadow: true,
            palette_override: None,
            animation_controller: Some(controller),
            render_layer: 2,
            static_object_render: None,
            grounding: Default::default(),
        },
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
        solid: true,
        active: true,
        movement_accumulator: glam::Vec2::ZERO,
        tags: Vec::new(),
    }
}

pub fn scene_with_test_player(name: &str, position: IVec2) -> Scene {
    let mut scene = Scene::new(name.to_string());
    let mut template_state = GameState::new_empty();
    let player_id = SceneSystem::spawn_player_at(&mut template_state, position);
    let player = template_state
        .world()
        .entity_manager()
        .get_entity(player_id)
        .expect("template player should exist")
        .clone();
    scene.add_entity(player);
    scene
}

pub fn save_test_state() -> GameState {
    let mut game_state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    let mut player = test_entity();
    player.id = 1;
    player.position = IVec2::new(24, 40);
    scene.camera_position = Some(IVec2::new(6, 8));
    scene.camera_scale = Some(3);
    scene.add_entity(player);
    SceneSystem::add_scene(&mut game_state, scene);
    SceneSystem::load(&mut game_state, "main").expect("main scene should load");
    game_state.set_flag("quest_complete", FlagValue::Bool(true));
    game_state.set_flag("coins", FlagValue::Int(7));
    game_state
}
