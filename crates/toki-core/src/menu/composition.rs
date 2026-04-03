//! Menu UI composition through the generic widget system.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::entity::EntityManager;
use crate::flags::{FlagValue, GameFlags};
use crate::ids::{UiLayoutId, UiWidgetId};
use crate::rules::TriggerContext;
use crate::text::{TextAnchor, TextSlant, TextWeight};
use crate::ui::UiComposition;
use crate::ui_layout::{
    UiAnchor, UiBindingContext, UiLayoutAsset, UiLayoutEngine, UiLayoutOutput, UiLayoutSpec,
    UiSpacing, UiSurfaceState, UiTextSegment, UiTextTemplate, UiTheme, UiTypography, UiWidgetKind,
    UiWidgetNode, UiWidgetStyle,
};
use crate::value_path::ValuePathContext;

use super::types::{MenuAppearance, MenuDialogLayout, MenuLayout, MenuTextAppearance};
use super::utilities::{
    apply_menu_opacity, menu_border_color, menu_fill_color_rgba, menu_hex_color_rgba,
};

const MENU_LAYOUT_ID: &str = "__runtime_menu__";
const DIALOG_LAYOUT_ID: &str = "__runtime_dialog__";
const MENU_TITLE_ID: &str = "__menu_title";
const MENU_HINT_ID: &str = "__menu_hint";
const DIALOG_TITLE_ID: &str = "__dialog_title";
const DIALOG_BODY_ID: &str = "__dialog_body";

/// Composes a menu UI from layout data through the generic widget engine.
pub fn compose_menu_ui(layout: &MenuLayout, appearance: &MenuAppearance) -> UiComposition {
    compose_menu_output(layout, appearance).composition
}

/// Composes a dialog UI from layout data through the generic widget engine.
pub fn compose_dialog_ui(layout: &MenuDialogLayout, appearance: &MenuAppearance) -> UiComposition {
    compose_dialog_output(layout, appearance).composition
}

/// Returns the full generic-widget output for a runtime menu surface.
pub fn compose_menu_output(layout: &MenuLayout, appearance: &MenuAppearance) -> UiLayoutOutput {
    let layout_asset = build_menu_widget_tree(layout, appearance);
    UiLayoutEngine::compose(
        &layout_asset,
        &theme_from_menu_appearance(appearance),
        glam::Vec2::new(
            (layout.panel.x + layout.panel.width + 8.0).max(1.0),
            (layout.panel.y + layout.panel.height + 8.0).max(1.0),
        ),
        empty_binding_context(),
        Some(&surface_state_for_selected_entry(
            MENU_LAYOUT_ID,
            layout
                .entries
                .iter()
                .position(|entry| entry.selected && entry.selectable),
            "menu_entry_",
        )),
    )
}

/// Returns the full generic-widget output for a runtime dialog surface.
pub fn compose_dialog_output(
    layout: &MenuDialogLayout,
    appearance: &MenuAppearance,
) -> UiLayoutOutput {
    let layout_asset = build_dialog_widget_tree(layout, appearance);
    UiLayoutEngine::compose(
        &layout_asset,
        &theme_from_menu_appearance(appearance),
        glam::Vec2::new(
            (layout.panel.x + layout.panel.width + 8.0).max(1.0),
            (layout.panel.y + layout.panel.height + 8.0).max(1.0),
        ),
        empty_binding_context(),
        Some(&surface_state_for_selected_entry(
            DIALOG_LAYOUT_ID,
            layout
                .entries
                .iter()
                .position(|entry| entry.selected && entry.selectable),
            "dialog_entry_",
        )),
    )
}

pub fn build_menu_widget_tree(layout: &MenuLayout, appearance: &MenuAppearance) -> UiLayoutAsset {
    let mut root = UiWidgetNode {
        layout: stretch_layout(),
        children: vec![
            background_panel_widget("menu_panel", layout.panel, appearance),
            text_label_widget(
                MENU_TITLE_ID,
                layout.title.rect,
                &layout.title.text,
                title_style(appearance),
            ),
        ],
        ..UiWidgetNode::default()
    };
    root.children
        .extend(layout.entries.iter().enumerate().map(|(index, entry)| {
            if entry.selectable {
                button_widget(
                    &format!("menu_entry_{index}"),
                    entry.rect,
                    &entry.text,
                    entry_style(appearance),
                )
            } else {
                text_label_widget(
                    &format!("menu_entry_{index}"),
                    entry.rect,
                    &entry.text,
                    entry_style(appearance),
                )
            }
        }));
    root.children.push(text_label_widget(
        MENU_HINT_ID,
        layout.hint.rect,
        &layout.hint.text,
        hint_style(appearance),
    ));

    UiLayoutAsset {
        id: UiLayoutId::new(MENU_LAYOUT_ID),
        title: "Runtime Menu".to_string(),
        startup_visible: true,
        z_order: 0,
        root,
    }
}

pub fn build_dialog_widget_tree(
    layout: &MenuDialogLayout,
    appearance: &MenuAppearance,
) -> UiLayoutAsset {
    let mut root = UiWidgetNode {
        layout: stretch_layout(),
        children: vec![
            background_panel_widget("dialog_panel", layout.panel, appearance),
            text_label_widget(
                DIALOG_TITLE_ID,
                layout.title.rect,
                &layout.title.text,
                dialog_speaker_style(appearance),
            ),
            text_label_widget(
                DIALOG_BODY_ID,
                layout.body.rect,
                &layout.body.text,
                dialog_body_style(appearance),
            ),
        ],
        ..UiWidgetNode::default()
    };
    root.children
        .extend(layout.entries.iter().enumerate().map(|(index, entry)| {
            if entry.selectable {
                button_widget(
                    &format!("dialog_entry_{index}"),
                    entry.rect,
                    &entry.text,
                    entry_style(appearance),
                )
            } else {
                text_label_widget(
                    &format!("dialog_entry_{index}"),
                    entry.rect,
                    &entry.text,
                    entry_style(appearance),
                )
            }
        }));

    UiLayoutAsset {
        id: UiLayoutId::new(DIALOG_LAYOUT_ID),
        title: "Runtime Dialog".to_string(),
        startup_visible: true,
        z_order: 0,
        root,
    }
}

pub fn menu_entry_index_from_widget_id(widget_id: &UiWidgetId) -> Option<usize> {
    widget_id
        .as_str()
        .strip_prefix("menu_entry_")
        .and_then(|value| value.parse().ok())
}

pub fn dialog_entry_index_from_widget_id(widget_id: &UiWidgetId) -> Option<usize> {
    widget_id
        .as_str()
        .strip_prefix("dialog_entry_")
        .and_then(|value| value.parse().ok())
}

fn background_panel_widget(
    id: &str,
    rect: crate::ui::UiRect,
    appearance: &MenuAppearance,
) -> UiWidgetNode {
    UiWidgetNode {
        id: id.into(),
        title: id.to_string(),
        layout: absolute_layout(rect),
        style: panel_style(appearance),
        event_id: None,
        focusable: false,
        visible_if: None,
        enabled_if: None,
        kind: UiWidgetKind::Label {
            content: literal_text(""),
        },
        children: Vec::new(),
    }
}

fn text_label_widget(
    id: &str,
    rect: crate::ui::UiRect,
    text: &str,
    style: UiWidgetStyle,
) -> UiWidgetNode {
    UiWidgetNode {
        id: id.into(),
        title: id.to_string(),
        layout: absolute_layout(rect),
        style,
        event_id: None,
        focusable: false,
        visible_if: None,
        enabled_if: None,
        kind: UiWidgetKind::Label {
            content: literal_text(text),
        },
        children: Vec::new(),
    }
}

fn button_widget(
    id: &str,
    rect: crate::ui::UiRect,
    text: &str,
    style: UiWidgetStyle,
) -> UiWidgetNode {
    UiWidgetNode {
        id: id.into(),
        title: id.to_string(),
        layout: absolute_layout(rect),
        style,
        event_id: None,
        focusable: true,
        visible_if: None,
        enabled_if: None,
        kind: UiWidgetKind::Button {
            label: literal_text(text),
        },
        children: Vec::new(),
    }
}

fn literal_text(text: &str) -> UiTextTemplate {
    UiTextTemplate {
        segments: vec![UiTextSegment::Literal {
            text: text.to_string(),
        }],
    }
}

fn stretch_layout() -> UiLayoutSpec {
    UiLayoutSpec {
        anchor: UiAnchor::Stretch,
        offset: [0.0, 0.0],
        size: [160.0, 144.0],
        margin: UiSpacing {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        padding: UiSpacing {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
    }
}

fn absolute_layout(rect: crate::ui::UiRect) -> UiLayoutSpec {
    UiLayoutSpec {
        anchor: UiAnchor::TopLeft,
        offset: [rect.x, rect.y],
        size: [rect.width, rect.height],
        margin: UiSpacing {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        padding: UiSpacing {
            left: 6,
            top: 4,
            right: 6,
            bottom: 4,
        },
    }
}

pub fn theme_from_menu_appearance(appearance: &MenuAppearance) -> UiTheme {
    UiTheme {
        font_family: appearance.font_family.clone(),
        base_font_size_px: appearance.font_size_px,
        menu_font_size_px: appearance.font_size_px,
        dialog_speaker_font_size_px: appearance.dialog_speaker_text.font_size_px,
        dialog_body_font_size_px: appearance.dialog_body_text.font_size_px,
        foreground_color: rgba_to_u8(
            menu_hex_color_rgba(&appearance.text_color_hex).unwrap_or([1.0, 1.0, 1.0, 1.0]),
        ),
        background_color: rgba_to_u8(
            menu_fill_color_rgba(
                &appearance.menu_background_color_hex,
                appearance.menu_background_transparent,
                appearance.opacity_percent,
            )
            .unwrap_or([0.08, 0.16, 0.08, 1.0]),
        ),
        accent_color: rgba_to_u8(
            menu_fill_color_rgba(
                &appearance.title_background_color_hex,
                appearance.title_background_transparent,
                appearance.opacity_percent,
            )
            .unwrap_or([0.08, 0.21, 0.08, 1.0]),
        ),
        border_color: rgba_to_u8(
            menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]),
        ),
        border_thickness_px: appearance.border_thickness_px,
        default_spacing: UiSpacing::default(),
        progress_fill_color: rgba_to_u8(
            menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]),
        ),
        progress_empty_color: rgba_to_u8(
            menu_fill_color_rgba(
                &appearance.entry_background_color_hex,
                appearance.entry_background_transparent,
                appearance.opacity_percent,
            )
            .unwrap_or([0.06, 0.12, 0.06, 1.0]),
        ),
        selection_color: rgba_to_u8(apply_menu_opacity(
            menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]),
            appearance.opacity_percent,
        )),
    }
}

fn panel_style(appearance: &MenuAppearance) -> UiWidgetStyle {
    UiWidgetStyle {
        fill_color: rgba_fill(
            &appearance.menu_background_color_hex,
            appearance.menu_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: rgba_border(appearance.border_style, appearance),
        text_color: None,
        accent_color: None,
        text_wrap: false,
        typography: UiTypography::default(),
    }
}

fn title_style(appearance: &MenuAppearance) -> UiWidgetStyle {
    UiWidgetStyle {
        fill_color: rgba_fill(
            &appearance.title_background_color_hex,
            appearance.title_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: rgba_border(appearance.border_style, appearance),
        text_color: Some(theme_from_menu_appearance(appearance).foreground_color),
        accent_color: None,
        text_wrap: false,
        typography: typography_from_text_appearance(
            &MenuTextAppearance {
                font_family: appearance.font_family.clone(),
                font_size_px: appearance.font_size_px.saturating_add(4),
                bold: true,
                cursive: false,
            },
            Some(TextAnchor::TopCenter),
        ),
    }
}

fn hint_style(appearance: &MenuAppearance) -> UiWidgetStyle {
    UiWidgetStyle {
        fill_color: None,
        border_color: None,
        text_color: Some(theme_from_menu_appearance(appearance).foreground_color),
        accent_color: None,
        text_wrap: false,
        typography: typography_from_text_appearance(
            &MenuTextAppearance {
                font_family: appearance.font_family.clone(),
                font_size_px: appearance.font_size_px.saturating_sub(2).max(10),
                bold: false,
                cursive: false,
            },
            Some(TextAnchor::BottomCenter),
        ),
    }
}

fn entry_style(appearance: &MenuAppearance) -> UiWidgetStyle {
    UiWidgetStyle {
        fill_color: rgba_fill(
            &appearance.entry_background_color_hex,
            appearance.entry_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: rgba_border(appearance.border_style, appearance),
        text_color: Some(theme_from_menu_appearance(appearance).foreground_color),
        accent_color: None,
        text_wrap: false,
        typography: typography_from_text_appearance(
            &MenuTextAppearance {
                font_family: appearance.font_family.clone(),
                font_size_px: appearance.font_size_px,
                bold: false,
                cursive: false,
            },
            Some(TextAnchor::TopCenter),
        ),
    }
}

fn dialog_speaker_style(appearance: &MenuAppearance) -> UiWidgetStyle {
    UiWidgetStyle {
        fill_color: rgba_fill(
            &appearance.title_background_color_hex,
            appearance.title_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: rgba_border(appearance.border_style, appearance),
        text_color: Some(theme_from_menu_appearance(appearance).foreground_color),
        accent_color: None,
        text_wrap: false,
        typography: typography_from_text_appearance(
            &appearance.dialog_speaker_text,
            Some(TextAnchor::TopCenter),
        ),
    }
}

fn dialog_body_style(appearance: &MenuAppearance) -> UiWidgetStyle {
    UiWidgetStyle {
        fill_color: rgba_fill(
            &appearance.title_background_color_hex,
            appearance.title_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: rgba_border(appearance.border_style, appearance),
        text_color: Some(theme_from_menu_appearance(appearance).foreground_color),
        accent_color: None,
        text_wrap: true,
        typography: typography_from_text_appearance(
            &appearance.dialog_body_text,
            Some(TextAnchor::TopCenter),
        ),
    }
}

fn typography_from_text_appearance(
    appearance: &MenuTextAppearance,
    anchor: Option<TextAnchor>,
) -> UiTypography {
    UiTypography {
        font_family: Some(appearance.font_family.clone()),
        font_size_px: Some(appearance.font_size_px),
        weight: Some(if appearance.bold {
            TextWeight::Bold
        } else {
            TextWeight::Normal
        }),
        slant: Some(if appearance.cursive {
            TextSlant::Italic
        } else {
            TextSlant::Normal
        }),
        anchor,
    }
}

fn rgba_fill(hex: &str, transparent: bool, opacity_percent: u16) -> Option<[u8; 4]> {
    menu_fill_color_rgba(hex, transparent, opacity_percent).map(rgba_to_u8)
}

fn rgba_border(
    style: super::types::MenuBorderStyle,
    appearance: &MenuAppearance,
) -> Option<[u8; 4]> {
    let border =
        menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]);
    let opacity_alpha = appearance.opacity_percent.clamp(0, 100) as f32 / 100.0;
    menu_border_color(style, border, opacity_alpha).map(rgba_to_u8)
}

fn rgba_to_u8(rgba: [f32; 4]) -> [u8; 4] {
    [
        (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

fn surface_state_for_selected_entry(
    _layout_id: &str,
    selected_index: Option<usize>,
    prefix: &str,
) -> UiSurfaceState {
    UiSurfaceState {
        visible: true,
        z_order: 0,
        startup_visible: true,
        binding_overrides: HashMap::new(),
        focused_widget_id: selected_index.map(|index| UiWidgetId::new(format!("{prefix}{index}"))),
        scroll_offsets: HashMap::new(),
    }
}

fn empty_binding_context() -> UiBindingContext<'static, 'static, 'static> {
    static ENTITY_MANAGER: OnceLock<EntityManager> = OnceLock::new();
    static FLAGS: OnceLock<GameFlags> = OnceLock::new();
    static OVERRIDES: OnceLock<HashMap<String, FlagValue>> = OnceLock::new();
    static DECLARED_FLAGS: OnceLock<Vec<crate::project_runtime::ProjectFlagDefinition>> =
        OnceLock::new();
    static EMPTY_TRIGGER_CONTEXT: TriggerContext = TriggerContext {
        trigger_self: None,
        trigger_other: None,
    };

    UiBindingContext {
        value_paths: ValuePathContext {
            entity_manager: ENTITY_MANAGER.get_or_init(EntityManager::default),
            game_flags: FLAGS.get_or_init(GameFlags::default),
            player_id: None,
            trigger_context: &EMPTY_TRIGGER_CONTEXT,
        },
        binding_overrides: OVERRIDES.get_or_init(HashMap::new),
        declared_flags: DECLARED_FLAGS.get_or_init(Vec::new),
    }
}
