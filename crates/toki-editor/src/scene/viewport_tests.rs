use super::{
    next_zoom_in_scale, next_zoom_out_scale, point_in_entity_bounds, request_viewport_size_state,
    screen_to_world_from_camera, world_to_i32_floor, SceneViewport, ViewportSizingMode,
};
use crate::project::assets::ProjectAssets;
use std::collections::HashMap;

fn make_unique_temp_dir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("toki_editor_viewport_tests_{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

#[test]
fn screen_to_world_uses_camera_and_has_no_hardcoded_tile_offset() {
    let display = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(160.0, 144.0));
    let world = screen_to_world_from_camera(
        egui::Pos2::new(0.0, 0.0),
        display,
        (160, 144),
        glam::IVec2::new(10, 20),
        1.0,
    );
    assert_eq!(world, glam::Vec2::new(10.0, 20.0));
}

#[test]
fn screen_to_world_clamps_letterbox_sides_to_viewport_bounds() {
    let display = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(320.0, 144.0));

    // In this setup, logical viewport is centered with 80px left/right letterboxes.
    let left_letterbox = screen_to_world_from_camera(
        egui::Pos2::new(0.0, 72.0),
        display,
        (160, 144),
        glam::IVec2::ZERO,
        1.0,
    );
    assert_eq!(left_letterbox.x, 0.0);

    let right_letterbox = screen_to_world_from_camera(
        egui::Pos2::new(320.0, 72.0),
        display,
        (160, 144),
        glam::IVec2::ZERO,
        1.0,
    );
    assert_eq!(right_letterbox.x, 160.0);
}

#[test]
fn zoom_in_progresses_below_native_scale() {
    assert_eq!(next_zoom_in_scale(2.0), 1.5);
    assert_eq!(next_zoom_in_scale(1.5), 1.0);
    assert_eq!(next_zoom_in_scale(1.0), 0.8);
    assert_eq!(next_zoom_in_scale(0.8), 0.6);
    assert_eq!(next_zoom_in_scale(0.6), 0.4);
    assert_eq!(next_zoom_in_scale(0.2), 0.1);
    assert_eq!(next_zoom_in_scale(0.1), 0.1);
}

#[test]
fn zoom_out_returns_fractional_zoom_to_native_then_outward() {
    assert_eq!(next_zoom_out_scale(0.1), 0.2);
    assert_eq!(next_zoom_out_scale(0.2), 0.4);
    assert_eq!(next_zoom_out_scale(0.4), 0.6);
    assert_eq!(next_zoom_out_scale(0.6), 0.8);
    assert_eq!(next_zoom_out_scale(0.8), 1.0);
    assert_eq!(next_zoom_out_scale(1.0), 1.5);
    assert_eq!(next_zoom_out_scale(1.5), 2.0);
    assert_eq!(next_zoom_out_scale(8.0), 8.0);
}

#[test]
fn world_to_i32_floor_uses_floor_for_negative_values() {
    assert_eq!(
        world_to_i32_floor(glam::Vec2::new(-0.1, -15.1)),
        glam::IVec2::new(-1, -16)
    );
}

#[test]
fn point_in_entity_bounds_is_left_top_inclusive_and_right_bottom_exclusive() {
    let pos = glam::IVec2::new(10, 20);
    let size = glam::UVec2::new(16, 16);

    assert!(point_in_entity_bounds(glam::IVec2::new(10, 20), pos, size));
    assert!(point_in_entity_bounds(glam::IVec2::new(25, 35), pos, size));
    assert!(!point_in_entity_bounds(glam::IVec2::new(26, 35), pos, size));
    assert!(!point_in_entity_bounds(glam::IVec2::new(25, 36), pos, size));
}

#[test]
fn responsive_viewport_accepts_requested_size_before_initialization() {
    let (current_size, requested_size, changed) = request_viewport_size_state(
        ViewportSizingMode::Responsive,
        false,
        (160, 144),
        None,
        (640, 480),
    );

    assert!(changed);
    assert_eq!(current_size, (640, 480));
    assert_eq!(requested_size, None);
}

#[test]
fn fixed_viewport_ignores_requested_size_changes() {
    let (current_size, requested_size, changed) = request_viewport_size_state(
        ViewportSizingMode::Fixed,
        true,
        (160, 144),
        None,
        (640, 480),
    );

    assert!(!changed);
    assert_eq!(current_size, (160, 144));
    assert_eq!(requested_size, None);
}

#[test]
fn responsive_initialized_viewport_defers_resize_until_render_phase() {
    let (current_size, requested_size, changed) = request_viewport_size_state(
        ViewportSizingMode::Responsive,
        true,
        (160, 144),
        None,
        (640, 480),
    );

    assert!(changed);
    assert_eq!(current_size, (160, 144));
    assert_eq!(requested_size, Some((640, 480)));
}

#[test]
fn viewport_resolves_shared_sprite_render_requests_for_static_entities() {
    let tmp = make_unique_temp_dir();
    let assets_dir = tmp.join("assets");
    let sprites_dir = tmp.join("assets/sprites");
    let maps_dir = tmp.join("assets/maps");
    std::fs::create_dir_all(&assets_dir).expect("assets dir should exist");
    std::fs::create_dir_all(&sprites_dir).expect("sprites dir should exist");
    std::fs::create_dir_all(&maps_dir).expect("maps dir should exist");
    let terrain_json = serde_json::json!({
        "image": "terrain.png",
        "tile_size": [16, 16],
        "tiles": {
            "grass": {
                "position": [0, 0],
                "properties": { "solid": false, "trigger": false }
            }
        }
    })
    .to_string();
    let creatures_json = serde_json::json!({
        "image": "creatures.png",
        "tile_size": [16, 16],
        "tiles": {
            "player/walk_down_a": {
                "position": [0, 0],
                "properties": { "solid": false, "trigger": false }
            }
        }
    })
    .to_string();
    std::fs::write(assets_dir.join("terrain.json"), &terrain_json)
        .expect("terrain atlas should be written");
    std::fs::write(assets_dir.join("creatures.json"), &creatures_json)
        .expect("creatures atlas should be written");
    std::fs::write(sprites_dir.join("terrain.json"), terrain_json)
        .expect("editor terrain atlas should be written");
    std::fs::write(sprites_dir.join("creatures.json"), creatures_json)
        .expect("editor creatures atlas should be written");
    std::fs::write(
        sprites_dir.join("items.json"),
        serde_json::json!({
            "sheet_type": "objects",
            "image": "items.png",
            "tile_size": [16, 16],
            "objects": {
                "coin": {
                    "position": [0, 0],
                    "size_tiles": [1, 1]
                }
            }
        })
        .to_string(),
    )
    .expect("object sheet should be written");
    std::fs::write(sprites_dir.join("items.png"), b"x").expect("image should be written");
    std::fs::write(
        maps_dir.join("new_town_map_64x64_crossings.json"),
        serde_json::json!({
            "size": [1, 1],
            "tile_size": [16, 16],
            "atlas": "terrain.json",
            "tiles": ["grass"],
            "objects": []
        })
        .to_string(),
    )
    .expect("default map should be written");

    let mut project_assets = ProjectAssets::new(tmp.clone());
    project_assets
        .scan_assets()
        .expect("project assets should scan");

    let mut game_state = toki_core::GameState::new_empty();
    let pickup_definition = toki_core::entity::EntityDefinition {
        name: "coin_pickup_render".to_string(),
        display_name: "Coin Pickup Render".to_string(),
        description: "Static object-sheet-backed pickup".to_string(),
        rendering: toki_core::entity::RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            static_object: Some(toki_core::entity::StaticObjectRenderDef {
                sheet: "items".to_string(),
                object_name: "coin".to_string(),
            }),
        },
        attributes: toki_core::entity::AttributesDef {
            health: None,
            stats: HashMap::new(),
            speed: 0.0,
            solid: false,
            active: true,
            can_move: false,
            interactable: false,
            interaction_reach: 0,
            ai_config: toki_core::entity::AiConfig::default(),
            movement_profile: toki_core::entity::MovementProfile::None,
            primary_projectile: None,
            pickup: Some(toki_core::entity::PickupDef {
                item_id: "coin".to_string(),
                count: 1,
            }),
            has_inventory: false,
        },
        collision: toki_core::entity::CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: true,
        },
        audio: toki_core::entity::AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 64,
            movement_sound_trigger: toki_core::entity::MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: toki_core::entity::AnimationsDef {
            atlas_name: String::new(),
            clips: vec![],
            default_state: String::new(),
        },
        category: "item".to_string(),
        tags: vec!["pickup".to_string()],
    };
    game_state
        .entity_manager_mut()
        .spawn_from_definition(&pickup_definition, glam::IVec2::new(24, 12))
        .expect("pickup should spawn");

    let resources =
        toki_core::ResourceManager::load_from_project_dir(&tmp).expect("resources should load");
    let mut viewport =
        SceneViewport::with_game_state_and_resources_for_tests(game_state, resources)
            .expect("viewport should exist");
    let requests = viewport.game_state().get_sprite_render_requests();

    let (sprites, failures) =
        viewport.resolve_sprite_requests_into_instances(&project_assets, Some(&tmp), &requests);

    assert!(failures.is_empty(), "unexpected failures: {failures:?}");
    assert_eq!(sprites.len(), 1);
    assert_eq!(sprites[0].size, glam::UVec2::new(16, 16));
    assert_eq!(
        sprites[0].texture_path,
        Some(tmp.join("assets/sprites/items.png"))
    );

    std::fs::remove_dir_all(tmp).expect("temp dir cleanup should succeed");
}
