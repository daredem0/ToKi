//! Entity editor panel - dedicated tab for creating and editing entity definitions.

mod browser;
mod components;
mod components_core;
mod details;
mod dialogs;
mod io;
mod toolbar;
mod widgets;

use crate::project::Project;
use crate::ui::widgets::separators::render_vertical_separator;
use crate::ui::EditorUI;

/// Renders the entity editor panel
pub fn render_entity_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project: Option<&mut Project>,
) {
    let project_path = project.as_ref().map(|p| p.path.clone());

    // Refresh entity list if needed
    if crate::ui::editor_context::entity_editor_state_mut(ui_state).needs_refresh {
        io::refresh_entity_list(ui_state, project_path.as_deref());
        crate::ui::editor_context::entity_editor_state_mut(ui_state).needs_refresh = false;
    }

    // Auto-load entities on first view if we have a project
    if crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .entities
        .is_empty()
        && project_path.is_some()
    {
        io::refresh_entity_list(ui_state, project_path.as_deref());
    }

    // Handle dialogs
    if crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .new_entity_dialog
        .is_open
    {
        dialogs::render_new_entity_dialog(ui_state, ctx, project_path.as_deref());
    }
    if crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .delete_confirmation
        .is_open
    {
        dialogs::render_delete_confirmation_dialog(ui_state, ctx, project_path.as_deref());
    }

    // Toolbar
    toolbar::render_toolbar(ui, ui_state);
    ui.separator();

    // Main content: Browser + Editor split
    render_main_content(ui, ui_state, project_path.as_deref());
}

fn render_main_content(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    project_path: Option<&std::path::Path>,
) {
    let available_width = ui.available_width();
    let available_height = ui.available_height();
    let browser_width =
        crate::ui::editor_context::entity_editor_state_mut(ui_state).browser_panel_width;
    let separator_width = 8.0;
    let editor_width = (available_width - browser_width - separator_width).max(200.0);

    ui.horizontal(|ui| {
        // Left panel: Entity browser
        ui.allocate_ui_with_layout(
            egui::vec2(browser_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                browser::render_entity_browser(ui, ui_state, project_path);
            },
        );

        // Draggable separator
        let sep_response = render_vertical_separator(ui, available_height);
        if sep_response.dragged() {
            crate::ui::editor_context::entity_editor_state_mut(ui_state).browser_panel_width =
                (crate::ui::editor_context::entity_editor_state_mut(ui_state).browser_panel_width
                    + sep_response.drag_delta().x)
                    .clamp(150.0, available_width * 0.4);
        }

        // Right panel: Entity details
        ui.allocate_ui_with_layout(
            egui::vec2(editor_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                details::render_entity_details(ui, ui_state);
            },
        );
    });
}
