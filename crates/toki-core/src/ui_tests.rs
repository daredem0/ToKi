use super::{UiBlock, UiComposition, UiRect, UiTextBlock};
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
