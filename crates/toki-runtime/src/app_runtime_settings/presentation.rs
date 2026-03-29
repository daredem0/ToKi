use toki_core::menu::{
    build_menu_layout, compose_menu_ui, menu_hex_color_rgba, MenuAppearance, MenuEntryLayout,
    MenuLayout, MenuView, MenuViewEntry,
};
use toki_core::ui::{UiBlock, UiComposition, UiRect};

use super::{App, RuntimeMenuOverlay, RuntimeOverlayEntry};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RuntimeOverlayPresentation {
    pub(super) layout: MenuLayout,
    pub(super) entries: Vec<RuntimeOverlayEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeOverlayHitTarget {
    Entry(usize),
    Slider { entry_index: usize, percent: u8 },
}

impl App {
    pub(super) fn runtime_overlay_presentation(
        &self,
        appearance: &MenuAppearance,
        viewport: glam::Vec2,
    ) -> Option<RuntimeOverlayPresentation> {
        let overlay = self.runtime_overlay.clone()?;
        let (title, entries): (String, Vec<RuntimeOverlayEntry>) = match overlay {
            RuntimeMenuOverlay::Audio { selected_index } => (
                "Audio Settings".to_string(),
                self.audio_overlay_entries(selected_index),
            ),
            RuntimeMenuOverlay::Display { selected_index } => (
                "Display Settings".to_string(),
                self.display_overlay_entries(selected_index),
            ),
            RuntimeMenuOverlay::Graphics { selected_index } => (
                "Graphics Settings".to_string(),
                self.graphics_overlay_entries(selected_index),
            ),
        };

        let view = MenuView {
            screen_id: "__runtime_settings__".to_string(),
            title,
            title_border_style_override: None,
            entries: entries
                .iter()
                .map(|entry| MenuViewEntry {
                    text: format!("{}: {}", entry.label, entry.value_text),
                    selected: entry.selected,
                    selectable: true,
                    border_style_override: None,
                })
                .collect(),
        };

        Some(RuntimeOverlayPresentation {
            layout: build_menu_layout(&view, appearance, viewport),
            entries,
        })
    }
}

pub(super) fn compose_runtime_settings_ui(
    layout_entries: &[MenuEntryLayout],
    overlay_entries: &[RuntimeOverlayEntry],
    layout: &MenuLayout,
    appearance: &MenuAppearance,
) -> UiComposition {
    let mut composition = compose_menu_ui(layout, appearance);
    let accent =
        menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]);
    let track = [0.12, 0.18, 0.12, 0.85];

    for (layout_entry, overlay_entry) in layout_entries.iter().zip(overlay_entries.iter()) {
        let Some(slider_percent) = overlay_entry.slider_percent else {
            continue;
        };
        let track_x = layout_entry.rect.x + layout_entry.rect.width * 0.56;
        let track_width = layout_entry.rect.width * 0.28;
        let track_y = layout_entry.rect.y + layout_entry.rect.height - 7.0;
        let track_height = 3.0;
        composition.push(UiBlock {
            rect: UiRect {
                x: track_x,
                y: track_y,
                width: track_width,
                height: track_height,
            },
            fill_color: Some(track),
            border_color: None,
            border_thickness: 0.0,
            text: None,
        });
        composition.push(UiBlock {
            rect: UiRect {
                x: track_x,
                y: track_y,
                width: track_width * (slider_percent.min(100) as f32 / 100.0),
                height: track_height,
            },
            fill_color: Some(accent),
            border_color: None,
            border_thickness: 0.0,
            text: None,
        });
    }

    composition
}

pub(super) fn runtime_overlay_hit_target_at_position(
    layout_entries: &[MenuEntryLayout],
    overlay_entries: &[RuntimeOverlayEntry],
    position: glam::Vec2,
) -> Option<RuntimeOverlayHitTarget> {
    for (entry_index, (layout_entry, overlay_entry)) in layout_entries
        .iter()
        .zip(overlay_entries.iter())
        .enumerate()
    {
        if let Some(slider_rect) = runtime_overlay_slider_rect(layout_entry, overlay_entry) {
            if rect_contains(slider_rect, position) {
                return Some(RuntimeOverlayHitTarget::Slider {
                    entry_index,
                    percent: slider_percent_from_position(slider_rect, position.x),
                });
            }
        }
        if rect_contains(layout_entry.rect, position) {
            return Some(RuntimeOverlayHitTarget::Entry(entry_index));
        }
    }
    None
}

pub(super) fn runtime_overlay_slider_rect(
    layout_entry: &MenuEntryLayout,
    overlay_entry: &RuntimeOverlayEntry,
) -> Option<UiRect> {
    overlay_entry.slider_percent?;
    Some(UiRect {
        x: layout_entry.rect.x + layout_entry.rect.width * 0.56,
        y: layout_entry.rect.y + layout_entry.rect.height - 10.0,
        width: layout_entry.rect.width * 0.28,
        height: 8.0,
    })
}

pub(super) fn rect_contains(rect: UiRect, position: glam::Vec2) -> bool {
    position.x >= rect.x
        && position.x <= rect.x + rect.width
        && position.y >= rect.y
        && position.y <= rect.y + rect.height
}

pub(super) fn slider_percent_from_position(rect: UiRect, x: f32) -> u8 {
    (((x - rect.x) / rect.width.max(1.0)).clamp(0.0, 1.0) * 100.0).round() as u8
}
