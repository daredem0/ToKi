use super::PlacementInteraction;
use crate::ui::EditorUI;
use glam::{IVec2, UVec2, Vec2};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use toki_core::assets::tilemap::TileMap;
use toki_core::entity::{
    AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
    RenderingDef,
};

fn sample_entity_definition(name: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string(),
        display_name: "Sample Entity".to_string(),
        description: "Entity used for placement tests".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
        },
        attributes: AttributesDef {
            health: Some(10),
            stats: std::collections::HashMap::new(),
            speed: 1,
            solid: true,
            active: true,
            can_move: false,
            ai_behavior: toki_core::entity::AiBehavior::Wander,
            movement_profile: toki_core::entity::MovementProfile::None,
            primary_projectile: None,
            has_inventory: false,
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
            movement_sound_trigger: toki_core::entity::MovementSoundTrigger::Distance,
            movement_sound: "sfx_step".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["slime/idle_0".to_string()],
                frame_duration_ms: 120.0,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: "creature".to_string(),
        tags: vec!["placement".to_string()],
    }
}

fn unique_temp_project_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "toki-placement-tests-{}-{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(dir.join("entities")).expect("failed to create temp entities directory");
    dir
}

fn write_entity_definition_file(project_dir: &Path, entity_def: &EntityDefinition) {
    let file_path = project_dir
        .join("entities")
        .join(format!("{}.json", entity_def.name));
    let json =
        serde_json::to_string_pretty(entity_def).expect("failed to serialize entity definition");
    fs::write(&file_path, json).expect("failed to write entity definition file");
}

fn placement_collision_assets() -> (TileMap, AtlasMeta) {
    let mut tiles = HashMap::new();
    tiles.insert(
        "solid".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );
    tiles.insert(
        "floor".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );

    let atlas = AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    };

    let tilemap = TileMap {
        size: UVec2::new(2, 2),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        // top-left is solid, others are floor
        tiles: vec![
            "solid".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
        ],
        objects: vec![],
    };

    (tilemap, atlas)
}

#[test]
fn placement_world_position_to_entity_position_uses_top_left_floored_coordinates() {
    let placed =
        PlacementInteraction::placement_world_position_to_entity_position(Vec2::new(64.9, 48.1));
    assert_eq!(placed, IVec2::new(64, 48));
}

#[test]
fn placement_world_position_to_entity_position_handles_negative_values_with_floor() {
    let placed =
        PlacementInteraction::placement_world_position_to_entity_position(Vec2::new(-0.1, -16.1));
    assert_eq!(placed, IVec2::new(-1, -17));
}

#[test]
fn next_entity_id_returns_one_for_empty_scene() {
    let next = PlacementInteraction::next_entity_id(&[]);
    assert_eq!(next, 1);
}

#[test]
fn next_entity_id_uses_max_id_plus_one() {
    let entity_def = sample_entity_definition("entity_a");
    let a = entity_def
        .create_entity(IVec2::new(0, 0), 7)
        .expect("failed to create entity a");
    let b = entity_def
        .create_entity(IVec2::new(0, 0), 42)
        .expect("failed to create entity b");
    let next = PlacementInteraction::next_entity_id(&[a, b]);
    assert_eq!(next, 43);
}

#[test]
fn load_entity_definition_succeeds_for_valid_file() {
    let project_dir = unique_temp_project_dir();
    let entity_def = sample_entity_definition("valid_entity");
    write_entity_definition_file(&project_dir, &entity_def);

    let loaded = PlacementInteraction::load_entity_definition(&project_dir, "valid_entity")
        .expect("expected valid entity definition to load");
    assert_eq!(loaded.name, "valid_entity");
    assert_eq!(loaded.category, "creature");
}

#[test]
fn load_entity_definition_fails_for_missing_file() {
    let project_dir = unique_temp_project_dir();
    let err = PlacementInteraction::load_entity_definition(&project_dir, "does_not_exist")
        .expect_err("expected missing definition to fail");
    assert!(err.contains("not found"));
}

#[test]
fn load_entity_definition_fails_for_invalid_json() {
    let project_dir = unique_temp_project_dir();
    let file_path = project_dir.join("entities").join("broken.json");
    fs::write(&file_path, "{ this is not valid json").expect("failed to write broken json");

    let err = PlacementInteraction::load_entity_definition(&project_dir, "broken")
        .expect_err("expected invalid json to fail");
    assert!(err.contains("Failed to parse entity definition"));
}

#[test]
fn create_entity_in_scene_adds_entity_and_marks_scene_changed() {
    let mut ui_state = EditorUI::new();
    ui_state.enter_placement_mode("sample".to_string());
    let entity_def = sample_entity_definition("sample");

    let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
        &mut ui_state,
        entity_def,
        "sample",
        IVec2::new(32, 48),
        None,
        None,
    );
    assert!(placed);

    let scene = ui_state
        .scenes
        .iter()
        .find(|s| s.name == "Main Scene")
        .expect("missing default scene");
    assert_eq!(scene.entities.len(), 1);
    assert_eq!(scene.entities[0].position, IVec2::new(32, 48));
    assert_eq!(scene.entities[0].category, "creature");
    assert_eq!(scene.entities[0].definition_name.as_deref(), Some("sample"));
    assert!(ui_state.scene_content_changed);
    assert!(ui_state.can_undo());
    // Placement mode exits at a higher level after successful click.
    assert!(ui_state.is_in_placement_mode());

    assert!(ui_state.undo());
    let scene = ui_state
        .scenes
        .iter()
        .find(|s| s.name == "Main Scene")
        .expect("missing default scene");
    assert!(scene.entities.is_empty());

    assert!(ui_state.redo());
    let scene = ui_state
        .scenes
        .iter()
        .find(|s| s.name == "Main Scene")
        .expect("missing default scene");
    assert_eq!(scene.entities.len(), 1);
}

#[test]
fn create_entity_in_scene_exits_placement_mode_when_no_active_scene() {
    let mut ui_state = EditorUI::new();
    ui_state.active_scene = None;
    ui_state.enter_placement_mode("sample".to_string());

    let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
        &mut ui_state,
        sample_entity_definition("sample"),
        "sample",
        IVec2::new(0, 0),
        None,
        None,
    );
    assert!(!placed);
    assert!(!ui_state.is_in_placement_mode());
}

#[test]
fn create_entity_in_scene_exits_placement_mode_when_active_scene_missing() {
    let mut ui_state = EditorUI::new();
    ui_state.active_scene = Some("Missing Scene".to_string());
    ui_state.enter_placement_mode("sample".to_string());

    let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
        &mut ui_state,
        sample_entity_definition("sample"),
        "sample",
        IVec2::new(0, 0),
        None,
        None,
    );
    assert!(!placed);
    assert!(!ui_state.is_in_placement_mode());
}

#[test]
fn create_entity_in_scene_blocks_on_solid_terrain_and_keeps_placement_mode() {
    let mut ui_state = EditorUI::new();
    ui_state.enter_placement_mode("sample".to_string());
    let (tilemap, atlas) = placement_collision_assets();

    let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
        &mut ui_state,
        sample_entity_definition("sample"),
        "sample",
        IVec2::new(0, 0), // top-left tile is solid in test map
        Some(&tilemap),
        Some(&atlas),
    );

    assert!(!placed);
    let scene = ui_state
        .scenes
        .iter()
        .find(|s| s.name == "Main Scene")
        .expect("missing default scene");
    assert_eq!(scene.entities.len(), 0);
    assert!(!ui_state.scene_content_changed);
    assert!(ui_state.is_in_placement_mode());
}

#[test]
fn can_place_entity_returns_true_without_collision_context() {
    let entity = sample_entity_definition("sample")
        .create_entity(IVec2::new(0, 0), 1)
        .expect("failed to create entity");
    assert!(PlacementInteraction::can_place_entity(
        &entity,
        IVec2::new(0, 0),
        None,
        None
    ));
}
