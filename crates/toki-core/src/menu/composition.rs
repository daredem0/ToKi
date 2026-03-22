//! Menu UI composition.

use crate::text::{TextAnchor, TextStyle, TextWeight};
use crate::ui::{UiBlock, UiComposition, UiTextBlock};

use super::types::{MenuAppearance, MenuDialogLayout, MenuLayout};
use super::utilities::{apply_menu_opacity, menu_border_color, menu_fill_color_rgba, menu_hex_color_rgba};

/// Composes a menu UI from layout data.
pub fn compose_menu_ui(layout: &MenuLayout, appearance: &MenuAppearance) -> UiComposition {
    let border_color =
        menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]);
    let opacity_alpha = (appearance.opacity_percent.clamp(0, 100) as f32) / 100.0;
    let text_color = apply_menu_opacity(
        menu_hex_color_rgba(&appearance.text_color_hex).unwrap_or([1.0, 1.0, 1.0, 1.0]),
        appearance.opacity_percent,
    );
    let title_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32 + 4.0,
        weight: TextWeight::Bold,
        color: text_color,
        ..TextStyle::default()
    };
    let entry_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32,
        weight: TextWeight::Normal,
        color: text_color,
        ..TextStyle::default()
    };
    let selected_style = TextStyle {
        color: text_color,
        weight: TextWeight::Bold,
        ..entry_style.clone()
    };
    let hint_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: (appearance.font_size_px as f32 - 2.0).max(10.0),
        color: text_color,
        ..TextStyle::default()
    };

    let mut composition = UiComposition::default();

    // Panel background
    composition.push(UiBlock {
        rect: layout.panel,
        fill_color: menu_fill_color_rgba(
            &appearance.menu_background_color_hex,
            appearance.menu_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: menu_border_color(appearance.border_style, border_color, opacity_alpha),
        text: None,
    });

    // Title
    composition.push(UiBlock {
        rect: layout.title.rect,
        fill_color: menu_fill_color_rgba(
            &appearance.title_background_color_hex,
            appearance.title_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: menu_border_color(layout.title.border_style, border_color, opacity_alpha),
        text: Some(UiTextBlock {
            content: layout.title.text.clone(),
            position: glam::Vec2::new(layout.title.rect.center_x(), layout.title.rect.y + 10.0),
            anchor: TextAnchor::TopCenter,
            style: title_style,
            layer: 10,
        }),
    });

    // Entries
    for entry in &layout.entries {
        let style = if entry.selected {
            selected_style.clone()
        } else {
            entry_style.clone()
        };
        composition.push(UiBlock {
            rect: entry.rect,
            fill_color: menu_fill_color_rgba(
                &appearance.entry_background_color_hex,
                appearance.entry_background_transparent,
                appearance.opacity_percent,
            ),
            border_color: menu_border_color(entry.border_style, border_color, opacity_alpha),
            text: Some(UiTextBlock {
                content: if entry.selected {
                    format!("> {}", entry.text)
                } else {
                    format!("  {}", entry.text)
                },
                position: glam::Vec2::new(entry.rect.center_x(), entry.rect.y + 6.0),
                anchor: TextAnchor::TopCenter,
                style,
                layer: 10,
            }),
        });
    }

    // Hint footer
    composition.push(UiBlock {
        rect: layout.hint.rect,
        fill_color: None,
        border_color: None,
        text: Some(UiTextBlock {
            content: layout.hint.text.clone(),
            position: glam::Vec2::new(layout.hint.rect.center_x(), layout.hint.rect.y + 4.0),
            anchor: TextAnchor::BottomCenter,
            style: hint_style,
            layer: 10,
        }),
    });

    composition
}

/// Composes a dialog UI from layout data.
pub fn compose_dialog_ui(layout: &MenuDialogLayout, appearance: &MenuAppearance) -> UiComposition {
    let border_color =
        menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]);
    let opacity_alpha = (appearance.opacity_percent.clamp(0, 100) as f32) / 100.0;
    let text_color = apply_menu_opacity(
        menu_hex_color_rgba(&appearance.text_color_hex).unwrap_or([1.0, 1.0, 1.0, 1.0]),
        appearance.opacity_percent,
    );
    let title_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32 + 4.0,
        weight: TextWeight::Bold,
        color: text_color,
        ..TextStyle::default()
    };
    let body_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32,
        weight: TextWeight::Normal,
        color: text_color,
        ..TextStyle::default()
    };
    let button_style = body_style.clone();
    let selected_button_style = TextStyle {
        weight: TextWeight::Bold,
        ..body_style.clone()
    };

    let mut composition = UiComposition::default();

    // Panel background
    composition.push(UiBlock {
        rect: layout.panel,
        fill_color: menu_fill_color_rgba(
            &appearance.menu_background_color_hex,
            appearance.menu_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: menu_border_color(appearance.border_style, border_color, opacity_alpha),
        text: None,
    });

    // Title and body blocks
    for block in [&layout.title, &layout.body] {
        composition.push(UiBlock {
            rect: block.rect,
            fill_color: menu_fill_color_rgba(
                &appearance.title_background_color_hex,
                appearance.title_background_transparent,
                appearance.opacity_percent,
            ),
            border_color: menu_border_color(block.border_style, border_color, opacity_alpha),
            text: Some(UiTextBlock {
                content: block.text.clone(),
                position: glam::Vec2::new(block.rect.center_x(), block.rect.y + 10.0),
                anchor: TextAnchor::TopCenter,
                style: if block.rect == layout.title.rect {
                    title_style.clone()
                } else {
                    body_style.clone()
                },
                layer: 11,
            }),
        });
    }

    // Buttons
    for button in [&layout.confirm_button, &layout.cancel_button] {
        composition.push(UiBlock {
            rect: button.rect,
            fill_color: menu_fill_color_rgba(
                &appearance.entry_background_color_hex,
                appearance.entry_background_transparent,
                appearance.opacity_percent,
            ),
            border_color: menu_border_color(button.border_style, border_color, opacity_alpha),
            text: Some(UiTextBlock {
                content: if button.selected {
                    format!("> {}", button.text)
                } else {
                    format!("  {}", button.text)
                },
                position: glam::Vec2::new(button.rect.center_x(), button.rect.y + 6.0),
                anchor: TextAnchor::TopCenter,
                style: if button.selected {
                    selected_button_style.clone()
                } else {
                    button_style.clone()
                },
                layer: 11,
            }),
        });
    }

    composition
}
