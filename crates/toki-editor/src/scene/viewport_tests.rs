use super::{
    next_zoom_in_scale, next_zoom_out_scale, point_in_entity_bounds, request_viewport_size_state,
    screen_to_world_from_camera, world_to_i32_floor, ViewportSizingMode,
};

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
