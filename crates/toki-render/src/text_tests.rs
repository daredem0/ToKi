use super::{
    apply_anchor, estimate_text_size, make_buffer_key, measure_buffer_size, to_screen_position,
};
use glyphon::{Buffer, FontSystem, Metrics, Shaping};
use toki_core::text::{TextAnchor, TextItem, TextSpace, TextStyle};

#[test]
fn apply_anchor_center_offsets_by_half_size() {
    let anchored = apply_anchor(
        glam::Vec2::new(100.0, 100.0),
        glam::Vec2::new(40.0, 20.0),
        TextAnchor::Center,
    );
    assert_eq!(anchored, glam::Vec2::new(80.0, 90.0));
}

#[test]
fn estimate_text_size_uses_max_width_for_wrapping() {
    let item = TextItem::new_screen(
        "A very long line",
        glam::Vec2::ZERO,
        TextStyle {
            size_px: 20.0,
            ..TextStyle::default()
        },
    )
    .with_max_width(40.0);
    let size = estimate_text_size(&item);
    assert!(size.x <= 40.0);
    assert!(size.y > 25.0);
}

#[test]
fn to_screen_position_passes_screen_space_directly() {
    let item = TextItem::new_screen("HUD", glam::Vec2::new(8.0, 12.0), TextStyle::default());
    let screen = to_screen_position(&item, glam::Mat4::IDENTITY, 320.0, 180.0)
        .expect("screen-space text should map directly");
    assert_eq!(screen, glam::Vec2::new(8.0, 12.0));
}

#[test]
fn to_screen_position_projects_world_space_coordinates() {
    let mut item = TextItem::new_world("NPC", glam::Vec2::ZERO, TextStyle::default());
    item.space = TextSpace::World;
    let screen = to_screen_position(&item, glam::Mat4::IDENTITY, 200.0, 100.0)
        .expect("origin should project into viewport");
    assert_eq!(screen, glam::Vec2::new(100.0, 50.0));
}

#[test]
fn buffer_key_ignores_position_and_color_for_layout_reuse() {
    let item_a = TextItem::new_screen(
        "FPS: 60",
        glam::Vec2::new(8.0, 8.0),
        TextStyle {
            color: [1.0, 1.0, 1.0, 1.0],
            ..TextStyle::default()
        },
    );
    let item_b = TextItem::new_screen(
        "FPS: 60",
        glam::Vec2::new(200.0, 120.0),
        TextStyle {
            color: [0.2, 1.0, 0.2, 1.0],
            ..TextStyle::default()
        },
    );

    let key_a = make_buffer_key(&item_a, 180.0, 320.0);
    let key_b = make_buffer_key(&item_b, 180.0, 320.0);
    assert_eq!(key_a, key_b);
}

#[test]
fn measure_buffer_size_uses_actual_shaped_line_width() {
    let mut font_system = FontSystem::new();
    let style = TextStyle {
        size_px: 16.0,
        ..TextStyle::default()
    };
    let mut buffer = Buffer::new(
        &mut font_system,
        Metrics::new(style.size_px, style.size_px * 1.25),
    );
    buffer.set_size(&mut font_system, Some(200.0), Some(100.0));
    let attrs = super::attrs_for_style(&style);
    buffer.set_text(&mut font_system, "Powered by ToKi", &attrs, Shaping::Basic);
    buffer.shape_until_scroll(&mut font_system, false);

    let measured = measure_buffer_size(&buffer);
    assert!(measured.x > 1.0);
    assert!(measured.y >= style.size_px * 1.25);
}
