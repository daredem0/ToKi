//! Menu UI composition.

use crate::text::{TextAnchor, TextSlant, TextStyle, TextWeight};
use crate::ui::{UiBlock, UiComposition, UiTextBlock};

use super::types::{MenuAppearance, MenuDialogLayout, MenuLayout, MenuTextAppearance};
use super::utilities::{
    apply_menu_opacity, menu_border_color, menu_fill_color_rgba, menu_hex_color_rgba,
};

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
        size_px: appearance.font_size_px as f32 + 4.0,
        weight: TextWeight::Bold,
        ..menu_text_style(
            &MenuTextAppearance {
                font_family: appearance.font_family.clone(),
                ..MenuTextAppearance::default()
            },
            text_color,
        )
    };
    let entry_style = menu_text_style(
        &MenuTextAppearance {
            font_family: appearance.font_family.clone(),
            font_size_px: appearance.font_size_px,
            ..MenuTextAppearance::default()
        },
        text_color,
    );
    let selected_style = TextStyle {
        color: text_color,
        weight: TextWeight::Bold,
        ..entry_style.clone()
    };
    let hint_style = TextStyle {
        size_px: (appearance.font_size_px as f32 - 2.0).max(10.0),
        ..menu_text_style(
            &MenuTextAppearance {
                font_family: appearance.font_family.clone(),
                ..MenuTextAppearance::default()
            },
            text_color,
        )
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
        border_thickness: appearance.border_thickness_px as f32,
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
        border_thickness: appearance.border_thickness_px as f32,
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
            border_thickness: appearance.border_thickness_px as f32,
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
        border_thickness: 0.0,
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
    let title_style = menu_text_style(&appearance.dialog_speaker_text, text_color);
    let body_style = menu_text_style(&appearance.dialog_body_text, text_color);
    let button_style = menu_text_style(
        &MenuTextAppearance {
            font_family: appearance.font_family.clone(),
            font_size_px: appearance.font_size_px,
            ..MenuTextAppearance::default()
        },
        text_color,
    );
    let selected_button_style = TextStyle {
        weight: TextWeight::Bold,
        ..button_style.clone()
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
        border_thickness: appearance.border_thickness_px as f32,
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
            border_thickness: appearance.border_thickness_px as f32,
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

    // Entries
    for entry in &layout.entries {
        composition.push(UiBlock {
            rect: entry.rect,
            fill_color: menu_fill_color_rgba(
                &appearance.entry_background_color_hex,
                appearance.entry_background_transparent,
                appearance.opacity_percent,
            ),
            border_color: menu_border_color(entry.border_style, border_color, opacity_alpha),
            border_thickness: appearance.border_thickness_px as f32,
            text: Some(UiTextBlock {
                content: if entry.selected {
                    format!("> {}", entry.text)
                } else {
                    format!("  {}", entry.text)
                },
                position: glam::Vec2::new(entry.rect.center_x(), entry.rect.y + 6.0),
                anchor: TextAnchor::TopCenter,
                style: if entry.selected {
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

fn menu_text_style(appearance: &MenuTextAppearance, color: [f32; 4]) -> TextStyle {
    TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32,
        weight: if appearance.bold {
            TextWeight::Bold
        } else {
            TextWeight::Normal
        },
        slant: if appearance.cursive {
            TextSlant::Italic
        } else {
            TextSlant::Normal
        },
        color,
    }
}
