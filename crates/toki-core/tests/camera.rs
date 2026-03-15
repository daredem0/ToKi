use glam::{IVec2, UVec2};
use toki_core::camera::{Camera, CameraController, CameraMode, RuntimeState};
use toki_core::entity::{Entity, EntityAttributes, EntityAudioSettings, EntityId, EntityKind};

fn create_test_entity(id: EntityId, position: IVec2) -> Entity {
    Entity {
        id,
        position,
        size: UVec2::new(16, 16), // Standard sprite size
        entity_kind: EntityKind::Player,
        category: "human".to_string(),
        definition_name: None,
        control_role: toki_core::entity::ControlRole::PlayerCharacter,
        audio: EntityAudioSettings::default(),
        attributes: EntityAttributes::default(),
        collision_box: None,
    }
}

#[test]
fn camera_new_has_correct_defaults() {
    let camera = Camera::new();
    assert_eq!(camera.position, IVec2::ZERO);
    assert_eq!(camera.viewport_size, UVec2::new(160, 144));
    assert_eq!(camera.scale, 1);
}

#[test]
fn camera_default_matches_new() {
    let camera_new = Camera::new();
    let camera_default = Camera::default();

    assert_eq!(camera_new.position, camera_default.position);
    assert_eq!(camera_new.viewport_size, camera_default.viewport_size);
    assert_eq!(camera_new.scale, camera_default.scale);
}

#[test]
fn camera_move_by_updates_position() {
    let mut camera = Camera::new();
    let delta = IVec2::new(10, -5);

    camera.move_by(delta);

    assert_eq!(camera.position, IVec2::new(10, -5));

    // Move again to test accumulation
    camera.move_by(IVec2::new(-3, 8));
    assert_eq!(camera.position, IVec2::new(7, 3));
}

#[test]
fn camera_center_on_positions_correctly() {
    let mut camera = Camera::new();
    let target = IVec2::new(100, 100);

    camera.center_on(target);

    // Target should be at viewport center
    let expected_pos = target - camera.viewport_size.as_ivec2() / 2;
    assert_eq!(camera.position, expected_pos);
    assert_eq!(camera.position, IVec2::new(20, 28)); // 100 - 160/2, 100 - 144/2
}

#[test]
fn camera_center_on_with_different_viewport_sizes() {
    let mut camera = Camera {
        position: IVec2::ZERO,
        viewport_size: UVec2::new(320, 240),
        scale: 1,
    };

    camera.center_on(IVec2::new(200, 150));

    assert_eq!(camera.position, IVec2::new(40, 30)); // 200 - 320/2, 150 - 240/2
}

#[test]
fn camera_calculate_projection_with_default_values() {
    let camera = Camera::new();
    let projection = camera.calculate_projection();

    // Basic sanity checks for orthographic projection
    assert!(projection.determinant() != 0.0); // Matrix is invertible

    // Check that it's an orthographic projection matrix structure
    // For orthographic: w component should be 1.0 for points
    let test_point = projection * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
    assert_eq!(test_point.w, 1.0);
}

#[test]
fn camera_calculate_projection_with_offset() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(50, 30);

    let projection = camera.calculate_projection();

    // Transform origin point should reflect camera offset
    let _origin = projection * glam::Vec4::new(50.0, 30.0, 0.0, 1.0);

    // In orthographic projection, the camera position should map to viewport origin
    // This is a basic sanity check - specific values depend on projection implementation
    assert!(projection.determinant() != 0.0);
}

#[test]
fn camera_calculate_projection_with_scale() {
    let mut camera = Camera::new();
    camera.scale = 2;

    let projection = camera.calculate_projection();

    // Matrix should be valid
    assert!(projection.determinant() != 0.0);

    // Scale affects the projection bounds
    // With scale 2, the viewport covers 2x the area
    let test_point = projection * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
    assert_eq!(test_point.w, 1.0);
}

#[test]
fn camera_controller_free_scroll_does_nothing() {
    let mut controller = CameraController {
        mode: CameraMode::FreeScroll,
    };
    let mut camera = Camera::new();
    let initial_position = camera.position;

    let entities = vec![];
    let runtime = RuntimeState {
        entities: &entities,
    };

    controller.update(&mut camera, &runtime);

    // Free scroll mode shouldn't change camera position automatically
    assert_eq!(camera.position, initial_position);
}

#[test]
fn camera_controller_follow_entity_updates_position() {
    let entity_id = 42;
    let mut controller = CameraController {
        mode: CameraMode::FollowEntity(entity_id),
    };
    let mut camera = Camera::new();

    let entities = vec![create_test_entity(entity_id, IVec2::new(200, 150))];
    let runtime = RuntimeState {
        entities: &entities,
    };

    controller.update(&mut camera, &runtime);

    // Camera should be centered on entity
    let expected_pos = IVec2::new(200, 150) - camera.viewport_size.as_ivec2() / 2;
    assert_eq!(camera.position, expected_pos);
}

#[test]
fn camera_controller_follow_nonexistent_entity_does_nothing() {
    let entity_id = 42;
    let mut controller = CameraController {
        mode: CameraMode::FollowEntity(entity_id),
    };
    let mut camera = Camera::new();
    let initial_position = camera.position;

    // No entities with the target ID
    let entities = vec![create_test_entity(999, IVec2::new(200, 150))];
    let runtime = RuntimeState {
        entities: &entities,
    };

    controller.update(&mut camera, &runtime);

    // Camera position should be unchanged
    assert_eq!(camera.position, initial_position);
}

#[test]
fn camera_controller_follow_entity_with_multiple_entities() {
    let target_id = 1;
    let mut controller = CameraController {
        mode: CameraMode::FollowEntity(target_id),
    };
    let mut camera = Camera::new();

    let entities = vec![
        create_test_entity(0, IVec2::new(50, 60)),
        create_test_entity(1, IVec2::new(300, 200)),
        create_test_entity(2, IVec2::new(100, 120)),
    ];
    let runtime = RuntimeState {
        entities: &entities,
    };

    controller.update(&mut camera, &runtime);

    // Should follow entity with ID 1
    let expected_pos = IVec2::new(300, 200) - camera.viewport_size.as_ivec2() / 2;
    assert_eq!(camera.position, expected_pos);
}

#[test]
fn entity_position_conversion() {
    let entity = create_test_entity(1, IVec2::new(123, 456));

    // Position is already IVec2, no conversion needed
    assert_eq!(entity.position, IVec2::new(123, 456));
}

#[test]
fn camera_bounds_calculations() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(100, 50);
    camera.scale = 2;

    // Calculate viewport bounds in world space
    let left = camera.position.x as f32;
    let top = camera.position.y as f32;
    let right = left + (camera.viewport_size.x * camera.scale) as f32;
    let bottom = top + (camera.viewport_size.y * camera.scale) as f32;

    assert_eq!(left, 100.0);
    assert_eq!(top, 50.0);
    assert_eq!(right, 420.0); // 100 + 160 * 2
    assert_eq!(bottom, 338.0); // 50 + 144 * 2
}

#[test]
fn camera_clamp_to_world_bounds_basic() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(-10, -5); // Out of bounds

    let world_size = UVec2::new(1000, 800);
    camera.clamp_to_world_bounds(world_size);

    // Should be clamped to (0, 0)
    assert_eq!(camera.position, IVec2::new(0, 0));
}

#[test]
fn camera_clamp_to_world_bounds_max_boundary() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(1000, 800); // Way out of bounds

    let world_size = UVec2::new(500, 400);
    camera.clamp_to_world_bounds(world_size);

    // Should be clamped to max valid position
    // max_x = (500 - 160 * 1).max(0) = 340
    // max_y = (400 - 144 * 1).max(0) = 256
    assert_eq!(camera.position, IVec2::new(340, 256));
}

#[test]
fn camera_clamp_to_world_bounds_with_scale() {
    let mut camera = Camera::new();
    camera.scale = 2;
    camera.position = IVec2::new(1000, 800); // Out of bounds

    let world_size = UVec2::new(500, 400);
    camera.clamp_to_world_bounds(world_size);

    // With scale 2, viewport is 320x288
    // max_x = (500 - 320).max(0) = 180
    // max_y = (400 - 288).max(0) = 112
    assert_eq!(camera.position, IVec2::new(180, 112));
}

#[test]
fn camera_clamp_to_world_bounds_small_world() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(50, 50);

    // World smaller than viewport
    let world_size = UVec2::new(100, 80);
    camera.clamp_to_world_bounds(world_size);

    // Should be clamped to (0, 0) since world is smaller than viewport
    assert_eq!(camera.position, IVec2::new(0, 0));
}

#[test]
fn camera_clamp_to_world_bounds_exact_fit() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(50, 50);

    // World exactly matches viewport size
    let world_size = UVec2::new(160, 144);
    camera.clamp_to_world_bounds(world_size);

    // Should be clamped to (0, 0) since there's no room to move
    assert_eq!(camera.position, IVec2::new(0, 0));
}

#[test]
fn camera_clamp_to_world_bounds_valid_position_unchanged() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(100, 50);

    let world_size = UVec2::new(1000, 800);
    camera.clamp_to_world_bounds(world_size);

    // Position should remain unchanged as it's valid
    assert_eq!(camera.position, IVec2::new(100, 50));
}
