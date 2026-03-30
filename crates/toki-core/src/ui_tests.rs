use super::{
    runtime_ui_text_scale, transform_logical_ui_composition, transform_logical_ui_rect, UiBlock,
    UiComposition, UiRect, UiTextBlock,
};
use crate::text::{TextAnchor, TextStyle, TextWeight};

#[test]
fn ui_text_block_converts_to_screen_text_item() {
    let block = UiTextBlock {
        content: "Paused".to_string(),
        position: glam::Vec2::new(100.0, 48.0),
        anchor: TextAnchor::TopCenter,
        style: TextStyle {
            font_family: "Sans".to_string(),
            size_px: 18.0,
            weight: TextWeight::Bold,
            ..TextStyle::default()
        },
        layer: 10,
    };

    let item = block.to_text_item();

    assert_eq!(item.content, "Paused");
    assert_eq!(item.position, glam::Vec2::new(100.0, 48.0));
    assert_eq!(item.anchor, TextAnchor::TopCenter);
    assert_eq!(item.layer, 10);
    assert_eq!(item.style.font_family, "Sans");
    assert_eq!(item.style.size_px, 18.0);
    assert_eq!(item.style.weight, TextWeight::Bold);
}

#[test]
fn ui_composition_preserves_block_order() {
    let mut composition = UiComposition::default();
    composition.push(UiBlock {
        rect: UiRect {
            x: 8.0,
            y: 16.0,
            width: 120.0,
            height: 40.0,
        },
        fill_color: Some([0.1, 0.2, 0.3, 1.0]),
        border_color: None,
        border_thickness: 0.0,
        text: None,
    });
    composition.push(UiBlock {
        rect: UiRect {
            x: 8.0,
            y: 60.0,
            width: 120.0,
            height: 24.0,
        },
        fill_color: None,
        border_color: Some([0.9, 0.9, 0.9, 1.0]),
        border_thickness: 2.0,
        text: Some(UiTextBlock {
            content: "Resume".to_string(),
            position: glam::Vec2::new(68.0, 66.0),
            anchor: TextAnchor::TopCenter,
            style: TextStyle::default(),
            layer: 10,
        }),
    });

    assert_eq!(composition.blocks.len(), 2);
    assert_eq!(composition.blocks[0].rect.height, 40.0);
    assert_eq!(
        composition.blocks[1]
            .text
            .as_ref()
            .expect("text block")
            .content,
        "Resume"
    );
}

#[test]
fn logical_ui_transform_scales_rects_borders_and_text() {
    let rect = transform_logical_ui_rect(
        UiRect {
            x: 10.0,
            y: 12.0,
            width: 20.0,
            height: 8.0,
        },
        glam::vec2(80.0, 16.0),
        4.0,
    );
    assert_eq!(rect.x, 120.0);
    assert_eq!(rect.y, 64.0);
    assert_eq!(rect.width, 80.0);
    assert_eq!(rect.height, 32.0);

    let mut composition = UiComposition::default();
    composition.push(UiBlock {
        rect: UiRect {
            x: 10.0,
            y: 12.0,
            width: 20.0,
            height: 8.0,
        },
        fill_color: Some([0.0, 0.0, 0.0, 1.0]),
        border_color: Some([1.0, 1.0, 1.0, 1.0]),
        border_thickness: 1.0,
        text: Some(UiTextBlock {
            content: "Hello".to_string(),
            position: glam::vec2(20.0, 20.0),
            anchor: TextAnchor::TopCenter,
            style: TextStyle {
                size_px: 8.0,
                ..TextStyle::default()
            },
            layer: 1,
        }),
    });

    let transformed = transform_logical_ui_composition(&composition, glam::vec2(80.0, 16.0), 4.0);
    let block = &transformed.blocks[0];
    assert_eq!(block.rect.x, 120.0);
    assert_eq!(block.rect.y, 64.0);
    assert_eq!(block.rect.width, 80.0);
    assert_eq!(block.rect.height, 32.0);
    assert!((block.border_thickness - 4.0).abs() < 0.01);
    let text = block.text.as_ref().expect("text should exist");
    assert_eq!(text.position, glam::vec2(160.0, 96.0));
    assert!((text.style.size_px - 32.0).abs() < 0.01);
}

#[test]
fn runtime_ui_text_scale_stays_consistent_for_common_hud_viewports() {
    let gb_like = runtime_ui_text_scale(glam::vec2(160.0, 144.0), glam::vec2(480.0, 432.0));
    let widescreen = runtime_ui_text_scale(glam::vec2(320.0, 180.0), glam::vec2(1280.0, 720.0));

    assert!(gb_like > 0.3 && gb_like <= 1.0);
    assert!(widescreen > 0.3 && widescreen <= 1.0);
    assert!((gb_like - widescreen).abs() < 0.3);
}
