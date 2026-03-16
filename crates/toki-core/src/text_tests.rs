use super::{TextAnchor, TextItem, TextSlant, TextSpace, TextStyle, TextWeight};

#[test]
fn text_style_defaults_are_runtime_friendly() {
    let style = TextStyle::default();
    assert_eq!(style.font_family, "Sans");
    assert_eq!(style.size_px, 16.0);
    assert_eq!(style.weight, TextWeight::Normal);
    assert_eq!(style.slant, TextSlant::Normal);
    assert_eq!(style.color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn screen_text_builder_sets_expected_defaults() {
    let text = TextItem::new_screen("FPS", glam::Vec2::new(8.0, 8.0), TextStyle::default());
    assert_eq!(text.space, TextSpace::Screen);
    assert_eq!(text.anchor, TextAnchor::TopLeft);
    assert!(text.box_style.is_none());
}

#[test]
fn world_text_builder_uses_center_anchor() {
    let text = TextItem::new_world("NPC", glam::Vec2::new(16.0, 32.0), TextStyle::default());
    assert_eq!(text.space, TextSpace::World);
    assert_eq!(text.anchor, TextAnchor::Center);
}

#[test]
fn max_width_builder_clamps_to_positive_values() {
    let text =
        TextItem::new_screen("Hello", glam::Vec2::ZERO, TextStyle::default()).with_max_width(0.0);
    assert_eq!(text.max_width, Some(1.0));
}
