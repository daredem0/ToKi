use glam::{IVec2, UVec2};
use toki_core::camera::{
    viewport_to_world, world_to_viewport, Camera, CameraController, CameraMode, RuntimeState,
};
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
        movement_accumulator: glam::Vec2::ZERO,
    }
}

#[test]
fn camera_new_has_correct_defaults() {
    let camera = Camera::new();
    assert_eq!(camera.position, IVec2::ZERO);
    assert_eq!(camera.viewport_size, UVec2::new(160, 144));
    assert_eq!(camera.zoom, 1.0);
}

#[test]
fn camera_default_matches_new() {
    let camera_new = Camera::new();
    let camera_default = Camera::default();

    assert_eq!(camera_new.position, camera_default.position);
    assert_eq!(camera_new.viewport_size, camera_default.viewport_size);
    assert_eq!(camera_new.zoom, camera_default.zoom);
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

    // With zoom = 1.0, visible area is viewport_size
    // Center on target: position = target - visible_size / 2
    let visible_size = camera.visible_world_size();
    let expected_pos = target - IVec2::new(visible_size.x as i32 / 2, visible_size.y as i32 / 2);
    assert_eq!(camera.position, expected_pos);
    assert_eq!(camera.position, IVec2::new(20, 28)); // 100 - 160/2, 100 - 144/2
}

#[test]
fn camera_center_on_with_different_viewport_sizes() {
    let mut camera = Camera {
        position: IVec2::ZERO,
        viewport_size: UVec2::new(320, 240),
        zoom: 1.0,
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
fn camera_calculate_projection_with_zoom() {
    let mut camera = Camera::new();
    camera.zoom = 2.0; // Zoom in 2x, showing half the world

    let projection = camera.calculate_projection();

    // Matrix should be valid
    assert!(projection.determinant() != 0.0);

    // With zoom 2, the viewport covers half the area (zoomed in)
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
    let visible_size = camera.visible_world_size();
    let expected_pos =
        IVec2::new(200, 150) - IVec2::new(visible_size.x as i32 / 2, visible_size.y as i32 / 2);
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
    let visible_size = camera.visible_world_size();
    let expected_pos =
        IVec2::new(300, 200) - IVec2::new(visible_size.x as i32 / 2, visible_size.y as i32 / 2);
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
    camera.zoom = 0.5; // Zoom out: showing 2x more world (like old scale = 2)

    // Calculate viewport bounds in world space
    let visible_size = camera.visible_world_size();
    let left = camera.position.x as f32;
    let top = camera.position.y as f32;
    let right = left + visible_size.x;
    let bottom = top + visible_size.y;

    assert_eq!(left, 100.0);
    assert_eq!(top, 50.0);
    assert_eq!(right, 420.0); // 100 + 160 / 0.5 = 100 + 320
    assert_eq!(bottom, 338.0); // 50 + 144 / 0.5 = 50 + 288
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
    // visible_size = viewport / zoom = 160 / 1.0 = 160
    // max_x = (500 - 160).max(0) = 340
    // max_y = (400 - 144).max(0) = 256
    assert_eq!(camera.position, IVec2::new(340, 256));
}

#[test]
fn camera_clamp_to_world_bounds_with_zoom_out() {
    let mut camera = Camera::new();
    camera.zoom = 0.5; // Zoom out: visible area is 2x larger (like old scale = 2)
    camera.position = IVec2::new(1000, 800); // Out of bounds

    let world_size = UVec2::new(500, 400);
    camera.clamp_to_world_bounds(world_size);

    // With zoom 0.5, visible area is 320x288
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

// ============================================================================
// Coordinate conversion tests
// ============================================================================

#[test]
fn viewport_to_world_at_origin_with_no_offset() {
    let camera = Camera::new(); // position (0,0), zoom 1.0
    let viewport_pos = glam::Vec2::new(80.0, 72.0); // center of 160x144 viewport

    let world_pos = camera.viewport_to_world(viewport_pos);

    assert_eq!(world_pos, glam::Vec2::new(80.0, 72.0));
}

#[test]
fn viewport_to_world_with_camera_offset() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(100, 50);

    let viewport_pos = glam::Vec2::new(0.0, 0.0);
    let world_pos = camera.viewport_to_world(viewport_pos);

    // Viewport origin maps to camera position
    assert_eq!(world_pos, glam::Vec2::new(100.0, 50.0));
}

#[test]
fn viewport_to_world_with_zoom_out() {
    let mut camera = Camera::new();
    camera.zoom = 0.5; // Zoom out: 1 viewport pixel = 2 world pixels
    camera.position = IVec2::new(0, 0);

    let viewport_pos = glam::Vec2::new(10.0, 5.0);
    let world_pos = camera.viewport_to_world(viewport_pos);

    // With zoom 0.5, effective scale is 1/0.5 = 2
    // One viewport pixel = 2 world pixels
    assert_eq!(world_pos, glam::Vec2::new(20.0, 10.0));
}

#[test]
fn viewport_to_world_with_offset_and_zoom_out() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(50, 30);
    camera.zoom = 0.5; // Zoom out: effective scale = 2

    let viewport_pos = glam::Vec2::new(10.0, 5.0);
    let world_pos = camera.viewport_to_world(viewport_pos);

    // world = camera_pos + viewport_pos * (1/zoom)
    // = (50, 30) + (10, 5) * 2 = (50 + 20, 30 + 10) = (70, 40)
    assert_eq!(world_pos, glam::Vec2::new(70.0, 40.0));
}

#[test]
fn world_to_viewport_at_origin() {
    let camera = Camera::new(); // position (0,0), zoom 1.0
    let world_pos = glam::Vec2::new(80.0, 72.0);

    let viewport_pos = camera.world_to_viewport(world_pos);

    assert_eq!(viewport_pos, glam::Vec2::new(80.0, 72.0));
}

#[test]
fn world_to_viewport_with_camera_offset() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(100, 50);

    let world_pos = glam::Vec2::new(100.0, 50.0);
    let viewport_pos = camera.world_to_viewport(world_pos);

    // World position at camera origin maps to viewport origin
    assert_eq!(viewport_pos, glam::Vec2::new(0.0, 0.0));
}

#[test]
fn world_to_viewport_with_zoom_out() {
    let mut camera = Camera::new();
    camera.zoom = 0.5; // Zoom out: 2 world pixels = 1 viewport pixel
    camera.position = IVec2::new(0, 0);

    let world_pos = glam::Vec2::new(20.0, 10.0);
    let viewport_pos = camera.world_to_viewport(world_pos);

    // With zoom 0.5, 20 world pixels = 10 viewport pixels
    assert_eq!(viewport_pos, glam::Vec2::new(10.0, 5.0));
}

#[test]
fn world_to_viewport_with_offset_and_zoom_out() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(50, 30);
    camera.zoom = 0.5; // Zoom out: effective scale = 2

    let world_pos = glam::Vec2::new(70.0, 40.0);
    let viewport_pos = camera.world_to_viewport(world_pos);

    // viewport = (world - camera_pos) * zoom
    // = ((70, 40) - (50, 30)) * 0.5 = (20, 10) * 0.5 = (10, 5)
    assert_eq!(viewport_pos, glam::Vec2::new(10.0, 5.0));
}

#[test]
fn viewport_to_world_and_back_roundtrips() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(123, 456);
    camera.zoom = 3.0; // Zoom in 3x

    let original = glam::Vec2::new(45.5, 67.25);
    let world_pos = camera.viewport_to_world(original);
    let back = camera.world_to_viewport(world_pos);

    assert!((back.x - original.x).abs() < 0.001);
    assert!((back.y - original.y).abs() < 0.001);
}

#[test]
fn world_to_viewport_and_back_roundtrips() {
    let mut camera = Camera::new();
    camera.position = IVec2::new(200, 100);
    camera.zoom = 0.5; // Zoom out

    let original = glam::Vec2::new(300.0, 200.0);
    let viewport_pos = camera.world_to_viewport(original);
    let back = camera.viewport_to_world(viewport_pos);

    assert!((back.x - original.x).abs() < 0.001);
    assert!((back.y - original.y).abs() < 0.001);
}

// ============================================================================
// Standalone utility function tests (for float scale support)
// ============================================================================

#[test]
fn standalone_viewport_to_world_with_float_scale() {
    let camera_position = IVec2::new(100, 50);
    let viewport_pos = glam::Vec2::new(10.0, 20.0);
    let scale = 1.5_f32;

    let world_pos = viewport_to_world(viewport_pos, camera_position, scale);

    // world = camera_pos + viewport * scale
    // = (100, 50) + (10, 20) * 1.5 = (100 + 15, 50 + 30) = (115, 80)
    assert!((world_pos.x - 115.0).abs() < 0.001);
    assert!((world_pos.y - 80.0).abs() < 0.001);
}

#[test]
fn standalone_world_to_viewport_with_float_scale() {
    let camera_position = IVec2::new(100, 50);
    let world_pos = glam::Vec2::new(115.0, 80.0);
    let scale = 1.5_f32;

    let viewport_pos = world_to_viewport(world_pos, camera_position, scale);

    // viewport = (world - camera_pos) / scale
    // = ((115, 80) - (100, 50)) / 1.5 = (15, 30) / 1.5 = (10, 20)
    assert!((viewport_pos.x - 10.0).abs() < 0.001);
    assert!((viewport_pos.y - 20.0).abs() < 0.001);
}

#[test]
fn standalone_functions_roundtrip_with_fractional_scale() {
    let camera_position = IVec2::new(50, 75);
    let scale = 0.8_f32;

    let original = glam::Vec2::new(45.5, 67.25);
    let world_pos = viewport_to_world(original, camera_position, scale);
    let back = world_to_viewport(world_pos, camera_position, scale);

    assert!((back.x - original.x).abs() < 0.001);
    assert!((back.y - original.y).abs() < 0.001);
}

// ============================================================================
// Zoom-specific tests
// ============================================================================

#[test]
fn camera_zoom_in_shows_less_world() {
    let mut camera = Camera::new(); // 160x144 viewport
    camera.zoom = 2.0; // Zoom in 2x

    let visible = camera.visible_world_size();

    // With zoom 2, we see half the world
    assert_eq!(visible.x, 80.0); // 160 / 2
    assert_eq!(visible.y, 72.0); // 144 / 2
}

#[test]
fn camera_zoom_out_shows_more_world() {
    let mut camera = Camera::new(); // 160x144 viewport
    camera.zoom = 0.5; // Zoom out

    let visible = camera.visible_world_size();

    // With zoom 0.5, we see twice the world
    assert_eq!(visible.x, 320.0); // 160 / 0.5
    assert_eq!(visible.y, 288.0); // 144 / 0.5
}

#[test]
fn camera_center_on_with_zoom_in() {
    let mut camera = Camera::new();
    camera.zoom = 2.0; // Zoom in 2x, visible area is 80x72

    camera.center_on(IVec2::new(100, 100));

    // With zoom 2, visible size is 80x72
    // position = target - visible_size / 2 = (100, 100) - (40, 36) = (60, 64)
    assert_eq!(camera.position, IVec2::new(60, 64));
}
