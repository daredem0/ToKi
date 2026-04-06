//! Animation editor panel.
//!
//! Provides a dedicated tab for editing entity animations with preview.
//! Entities are loaded by selecting them in the hierarchy panel.
//!
//! # Module Structure
//!
//! - `toolbar`: Toolbar with save and entity info
//! - `preview`: Preview area and playback controls
//! - `atlas_grid`: Atlas grid for frame selection
//! - `clip_list`: Clip list panel
//! - `frame_sequence`: Frame sequence panel
//! - `io`: Entity and atlas loading/saving
//! - `dialogs`: New clip dialog
mod atlas_grid;
mod clip_list;
mod dialogs;
mod frame_sequence;
mod io;
mod preview;
mod toolbar;

use crate::project::Project;
use crate::ui::editor_ui::Selection;
use crate::ui::widgets::separators::{render_horizontal_separator, render_vertical_separator};
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::indexed_presentation::resolve_indexed_palette;

/// Renders the animation editor panel
#[allow(clippy::needless_option_as_deref)]
pub fn render_animation_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project: Option<&mut Project>,
    renderer: Option<&mut crate::rendering::WindowRenderer>,
) {
    let project_path = project.as_ref().map(|p| p.path.clone());

    // Check if an entity definition is selected and load it
    sync_with_selection(ui_state, project_path.as_deref());
    sync_preview_palette(ui_state);

    // Handle new clip dialog
    if crate::ui::editor_context::animation_state_mut(ui_state).show_new_clip_dialog {
        dialogs::render_new_clip_dialog(ui_state, ctx);
    }

    // Toolbar
    toolbar::render_toolbar(ui, ui_state);
    ui.separator();

    // Main content
    if crate::ui::editor_context::animation_state_mut(ui_state).has_entity() {
        render_editor_content(ui, ui_state, ctx, renderer);
    } else {
        render_no_entity_message(ui);
    }
}

/// Sync animation editor state with current selection
fn sync_with_selection(ui_state: &mut EditorUI, project_path: Option<&Path>) {
    let Some(project_path) = project_path else {
        return;
    };

    // Check if an EntityDefinition is selected
    let selected_entity = match &ui_state.selection {
        Some(Selection::EntityDefinition(name)) => Some(name.clone()),
        _ => None,
    };

    let Some(entity_name) = selected_entity else {
        return;
    };

    // Check if we already have this entity loaded
    if crate::ui::editor_context::animation_state_mut(ui_state)
        .active_entity
        .as_ref()
        == Some(&entity_name)
    {
        return;
    }

    // Load the entity
    io::load_entity(ui_state, project_path, &entity_name);
}

fn sync_preview_palette(ui_state: &mut EditorUI) {
    let atlas_color_mode = crate::ui::editor_context::animation_state(ui_state).atlas_color_mode;
    let atlas_entity_palette_override = crate::ui::editor_context::animation_state(ui_state)
        .atlas_entity_palette_override
        .clone();
    let atlas_default_palette = crate::ui::editor_context::animation_state(ui_state)
        .atlas_default_palette
        .clone();
    let presentation_settings = ui_state.project.indexed_presentation_settings();
    let resolved = resolve_indexed_palette(
        atlas_color_mode,
        &ui_state.project.available_palettes,
        &presentation_settings,
        atlas_entity_palette_override.as_deref(),
        atlas_default_palette.as_deref(),
    )
    .ok()
    .flatten()
    .map(|(palette_id, _)| palette_id);

    if crate::ui::editor_context::animation_state_mut(ui_state).atlas_palette_id != resolved {
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_palette_id = resolved;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_texture = None;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_texture_cache_key = None;
    }
}

fn render_no_entity_message(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("No entity selected");
            ui.add_space(8.0);
            ui.label("Select an entity in the hierarchy panel to edit its animations.");
        });
    });
}

fn render_editor_content(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    mut renderer: Option<&mut crate::rendering::WindowRenderer>,
) {
    // Left: Clip list. Center: Atlas grid + Preview. Right: Frame sequence
    let available_width = ui.available_width();
    let available_height = ui.available_height();

    // Get panel widths from state
    let clip_list_width = crate::ui::editor_context::animation_state_mut(ui_state).clip_list_width;
    let frame_sequence_width =
        crate::ui::editor_context::animation_state_mut(ui_state).frame_sequence_width;
    let separator_width = 8.0; // Width of draggable separator
    let center_width =
        (available_width - clip_list_width - frame_sequence_width - separator_width * 2.0)
            .max(200.0);

    ui.horizontal(|ui| {
        // Left panel: Clip list
        ui.allocate_ui_with_layout(
            egui::vec2(clip_list_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                clip_list::render_clip_list(ui, ui_state);
            },
        );

        // Draggable separator between clip list and center
        let sep_response = render_vertical_separator(ui, available_height);
        if sep_response.dragged() {
            crate::ui::editor_context::animation_state_mut(ui_state).clip_list_width =
                (crate::ui::editor_context::animation_state_mut(ui_state).clip_list_width
                    + sep_response.drag_delta().x)
                    .clamp(120.0, available_width * 0.4);
        }

        // Center panel: Atlas grid and preview
        ui.allocate_ui_with_layout(
            egui::vec2(center_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                #[allow(clippy::needless_option_as_deref)]
                render_center_panel(ui, ui_state, ctx, renderer.as_deref_mut());
            },
        );

        // Draggable separator between center and frame sequence
        let sep_response = render_vertical_separator(ui, available_height);
        if sep_response.dragged() {
            // Dragging right makes frame panel smaller
            crate::ui::editor_context::animation_state_mut(ui_state).frame_sequence_width =
                (crate::ui::editor_context::animation_state_mut(ui_state).frame_sequence_width
                    - sep_response.drag_delta().x)
                    .clamp(150.0, available_width * 0.4);
        }

        // Right panel: Frame sequence
        ui.allocate_ui_with_layout(
            egui::vec2(frame_sequence_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                frame_sequence::render_frame_sequence(ui, ui_state);
            },
        );
    });
}

fn render_center_panel(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    renderer: Option<&mut crate::rendering::WindowRenderer>,
) {
    let available_width = ui.available_width();
    let available_height = ui.available_height();

    // Preview controls at top (fixed height)
    preview::render_preview_controls(ui, ui_state, ctx);

    // Get preview height from state
    let preview_height = crate::ui::editor_context::animation_state_mut(ui_state).preview_height;

    // Preview area with stored height
    ui.group(|ui| {
        ui.set_min_height(preview_height);
        ui.set_max_height(preview_height);
        preview::render_preview_area(ui, ui_state);
    });

    // Draggable separator between preview and atlas
    let sep_response = render_horizontal_separator(ui, available_width);
    if sep_response.dragged() {
        crate::ui::editor_context::animation_state_mut(ui_state).preview_height =
            (crate::ui::editor_context::animation_state_mut(ui_state).preview_height
                + sep_response.drag_delta().y)
                .clamp(100.0, available_height - 150.0);
    }

    // Atlas grid (for selecting frames)
    ui.heading("Atlas");

    // Use all remaining space for atlas grid
    atlas_grid::render_atlas_grid(ui, ui_state, ctx, renderer);
}
