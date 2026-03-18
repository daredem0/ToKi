use super::CameraManager;
use toki_core::assets::tilemap::TileMap;
use toki_core::camera::{Camera, CameraController, CameraMode, RuntimeState};
use toki_core::entity::{Entity, EntityAttributes, EntityKind};

fn sample_camera_manager() -> CameraManager {
    let camera = Camera {
        position: glam::IVec2::new(0, 0),
        viewport_size: glam::UVec2::new(32, 32),
        zoom: 1.0,
    };
    let controller = CameraController {
        mode: CameraMode::FreeScroll,
    };
    CameraManager::new(camera, controller)
}

fn sample_tilemap() -> TileMap {
    TileMap {
        size: glam::UVec2::new(4, 4),
        tile_size: glam::UVec2::new(16, 16),
        atlas: std::path::PathBuf::from("atlas.json"),
        tiles: vec!["floor".to_string(); 16],
        objects: vec![],
    }
}

#[test]
fn update_chunk_cache_reports_changes_then_stabilizes() {
    let mut manager = sample_camera_manager();
    let tilemap = sample_tilemap();

    assert!(manager.update_chunk_cache(&tilemap));
    assert!(!manager.update_chunk_cache(&tilemap));
}

#[test]
fn update_with_follow_mode_moves_camera_and_view_matrix_matches() {
    let mut manager = sample_camera_manager();
    manager.controller_mut().mode = CameraMode::FollowEntity(1);

    let entity = Entity {
        id: 1,
        position: glam::IVec2::new(20, 30),
        size: glam::UVec2::new(16, 16),
        entity_kind: EntityKind::Player,
        category: "human".to_string(),
        control_role: toki_core::entity::ControlRole::PlayerCharacter,
        audio: toki_core::entity::EntityAudioSettings::default(),
        attributes: EntityAttributes::default(),
        collision_box: None,
        definition_name: None,
    };
    let runtime = RuntimeState {
        entities: &[entity],
    };

    let changed = manager.update(&runtime, glam::UVec2::new(256, 256));
    assert!(changed);
    let position = manager.position();
    let view = manager.view_matrix();
    // With zoom = 1.0, the view matrix should be scale(1) * translate(-pos)
    let translation =
        glam::Mat4::from_translation(glam::vec3(-(position.x as f32), -(position.y as f32), 0.0));
    let scale = glam::Mat4::from_scale(glam::vec3(1.0, 1.0, 1.0));
    let expected = scale * translation;
    assert_eq!(view, expected);
}

#[test]
fn view_matrix_includes_zoom_scale() {
    let camera = Camera {
        position: glam::IVec2::new(100, 50),
        viewport_size: glam::UVec2::new(160, 144),
        zoom: 2.0,
    };
    let controller = CameraController {
        mode: CameraMode::FreeScroll,
    };
    let manager = CameraManager::new(camera, controller);

    let view = manager.view_matrix();
    // View matrix should be: scale(zoom) * translate(-position)
    let translation = glam::Mat4::from_translation(glam::vec3(-100.0, -50.0, 0.0));
    let scale = glam::Mat4::from_scale(glam::vec3(2.0, 2.0, 1.0));
    let expected = scale * translation;
    assert_eq!(view, expected);

    // Verify that a world point is transformed correctly:
    // World point (100, 50) should map to origin (0, 0) after translation, then scaled
    let point = glam::vec4(100.0, 50.0, 0.0, 1.0);
    let transformed = view * point;
    assert_eq!(transformed, glam::vec4(0.0, 0.0, 0.0, 1.0));

    // World point (108, 50) should be 8 pixels right of camera, scaled by 2 = 16 screen pixels
    let point2 = glam::vec4(108.0, 50.0, 0.0, 1.0);
    let transformed2 = view * point2;
    assert_eq!(transformed2, glam::vec4(16.0, 0.0, 0.0, 1.0));
}
