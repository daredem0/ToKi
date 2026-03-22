//! Menu layout building.

use super::types::{
    MenuAppearance, MenuBorderStyle, MenuDialogLayout, MenuDialogView, MenuEntryLayout, MenuLayout,
    MenuLayoutBlock, MenuRect, MenuView, MenuVisualMetrics,
};
use super::utilities::menu_visual_metrics;

/// Builds a menu layout from view data.
pub fn build_menu_layout(
    view: &MenuView,
    appearance: &MenuAppearance,
    viewport: glam::Vec2,
) -> MenuLayout {
    let metrics = menu_visual_metrics();
    let panel = menu_panel_rect(view, appearance, viewport, &metrics);
    let content_x = panel.x + metrics.panel_inner_margin_px;
    let content_width = (panel.width - metrics.panel_inner_margin_px * 2.0).max(1.0);
    let title_height = appearance.font_size_px as f32
        + metrics.title_size_delta_px
        + metrics.title_padding_px.y * 2.0;
    let title_rect = MenuRect {
        x: content_x,
        y: metrics.title_top_y_px,
        width: content_width,
        height: title_height,
    };
    let entries_start_y = title_rect.y + title_rect.height + appearance.title_spacing_px as f32;
    let entry_height = appearance.font_size_px as f32 + metrics.entry_padding_px.y * 2.0;
    let button_spacing = appearance.button_spacing_px as f32;
    let entries: Vec<MenuEntryLayout> = view
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| MenuEntryLayout {
            rect: MenuRect {
                x: content_x,
                y: entries_start_y + index as f32 * (entry_height + button_spacing),
                width: content_width,
                height: entry_height,
            },
            text: entry.text.clone(),
            selected: entry.selected,
            selectable: entry.selectable,
            border_style: entry
                .border_style_override
                .unwrap_or(appearance.border_style),
        })
        .collect();
    let hint_font_size = (appearance.font_size_px as f32 - 2.0).max(10.0);
    let hint_height = hint_font_size + metrics.hint_padding_px.y * 2.0;
    let footer_y = entries
        .last()
        .map(|entry| entry.rect.y + entry.rect.height)
        .unwrap_or(title_rect.y + title_rect.height)
        + appearance.footer_spacing_px as f32;
    let hint_rect = MenuRect {
        x: content_x,
        y: footer_y,
        width: content_width,
        height: hint_height,
    };

    MenuLayout {
        panel,
        title: MenuLayoutBlock {
            rect: title_rect,
            text: view.title.clone(),
            border_style: view
                .title_border_style_override
                .unwrap_or(appearance.border_style),
        },
        entries,
        hint: MenuLayoutBlock {
            rect: hint_rect,
            text: appearance.footer_text.clone(),
            border_style: MenuBorderStyle::None,
        },
    }
}

/// Builds a dialog layout from view data.
pub fn build_dialog_layout(
    view: &MenuDialogView,
    appearance: &MenuAppearance,
    viewport: glam::Vec2,
) -> MenuDialogLayout {
    let metrics = menu_visual_metrics();
    let panel_width = (viewport.x * (appearance.menu_width_percent.clamp(20, 100) as f32 / 100.0))
        .clamp(160.0, (viewport.x - 16.0).max(160.0));
    let content_x = (viewport.x - panel_width) * 0.5 + metrics.panel_inner_margin_px;
    let content_width = (panel_width - metrics.panel_inner_margin_px * 2.0).max(1.0);
    let title_height = appearance.font_size_px as f32
        + metrics.title_size_delta_px
        + metrics.title_padding_px.y * 2.0;
    let title_rect = MenuRect {
        x: content_x,
        y: viewport.y * 0.18,
        width: content_width,
        height: title_height,
    };
    let body_height =
        ((appearance.font_size_px as f32 * 3.2).max(40.0)) + metrics.entry_padding_px.y * 2.0;
    let body_rect = MenuRect {
        x: content_x,
        y: title_rect.y + title_rect.height + appearance.title_spacing_px as f32,
        width: content_width,
        height: body_height,
    };
    let button_height = appearance.font_size_px as f32 + metrics.entry_padding_px.y * 2.0;
    let button_width =
        ((content_width - appearance.button_spacing_px as f32).max(2.0) * 0.5).floor();
    let confirm_rect = MenuRect {
        x: content_x,
        y: body_rect.y + body_rect.height + appearance.footer_spacing_px as f32,
        width: button_width,
        height: button_height,
    };
    let cancel_rect = MenuRect {
        x: content_x + button_width + appearance.button_spacing_px as f32,
        y: confirm_rect.y,
        width: button_width,
        height: button_height,
    };
    let panel = MenuRect {
        x: (viewport.x - panel_width) * 0.5,
        y: (title_rect.y - metrics.panel_inner_margin_px).max(8.0),
        width: panel_width,
        height: (cancel_rect.y + cancel_rect.height - title_rect.y)
            + metrics.panel_inner_margin_px * 2.0
            + appearance.title_spacing_px as f32,
    };

    MenuDialogLayout {
        panel,
        title: MenuLayoutBlock {
            rect: title_rect,
            text: view.title.clone(),
            border_style: appearance.border_style,
        },
        body: MenuLayoutBlock {
            rect: body_rect,
            text: view.body.clone(),
            border_style: appearance.border_style,
        },
        confirm_button: MenuEntryLayout {
            rect: confirm_rect,
            text: view.confirm_text.clone(),
            selected: view.confirm_selected,
            selectable: true,
            border_style: appearance.border_style,
        },
        cancel_button: MenuEntryLayout {
            rect: cancel_rect,
            text: view.cancel_text.clone(),
            selected: !view.confirm_selected,
            selectable: true,
            border_style: appearance.border_style,
        },
    }
}

fn menu_panel_rect(
    view: &MenuView,
    appearance: &MenuAppearance,
    viewport: glam::Vec2,
    metrics: &MenuVisualMetrics,
) -> MenuRect {
    let font_size_px = appearance.font_size_px as f32;
    let title_height =
        font_size_px + metrics.title_size_delta_px + metrics.title_padding_px.y * 2.0;
    let entries_start_y =
        metrics.title_top_y_px + title_height + appearance.title_spacing_px as f32;
    let entry_height = font_size_px + metrics.entry_padding_px.y * 2.0;
    let button_spacing = appearance.button_spacing_px as f32;
    let last_entry_bottom = if view.entries.is_empty() {
        metrics.title_top_y_px + title_height
    } else {
        entries_start_y
            + (view.entries.len() - 1) as f32 * (entry_height + button_spacing)
            + entry_height
    };
    let hint_size_px = (font_size_px - 2.0).max(10.0);
    let hint_height = hint_size_px + metrics.hint_padding_px.y * 2.0;
    let content_bottom = last_entry_bottom + appearance.footer_spacing_px as f32 + hint_height;
    let requested_panel_width =
        viewport.x * (appearance.menu_width_percent.clamp(20, 100) as f32 / 100.0);
    let requested_panel_height =
        viewport.y * (appearance.menu_height_percent.clamp(20, 100) as f32 / 100.0);
    let max_panel_width = (viewport.x - 16.0).max(40.0);
    let panel_width = requested_panel_width.clamp(40.0, max_panel_width);
    let x = (viewport.x - panel_width) * 0.5;
    let y = (metrics.title_top_y_px - metrics.panel_inner_margin_px).max(8.0);
    let content_height = (content_bottom - y + metrics.panel_inner_margin_px).max(80.0);
    MenuRect {
        x,
        y,
        width: panel_width,
        height: content_height.max(requested_panel_height),
    }
}
