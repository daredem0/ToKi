//! Sprite editor panel - split into focused submodules.

mod canvas;
mod dialogs;
mod layout;
mod shortcuts;
mod toolbar;
mod tools;

use crate::project::Project;
use crate::ui::editor_ui::DualCanvasLayout;
use crate::ui::EditorUI;

/// Renders the sprite editor panel
pub fn render_sprite_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project: Option<&mut Project>,
    mut renderer: Option<&mut crate::rendering::WindowRenderer>,
) {
    // Get sprites directory from project (needed for multiple operations)
    let sprites_dir = project
        .as_ref()
        .map(|p| p.path.join("assets").join("sprites"));

    // Handle dialogs
    dialogs::render_dialogs(ui_state, ctx, sprites_dir.as_deref());

    // Toolbar (simplified - tools are in inspector panel)
    toolbar::render_toolbar(ui, ui_state, sprites_dir.as_deref());
    ui.separator();

    // Main content area - render based on layout mode
    match crate::ui::editor_context::sprite_state_mut(ui_state).layout {
        DualCanvasLayout::Single => {
            if crate::ui::editor_context::sprite_state_mut(ui_state).has_canvas() {
                canvas::render_canvas_viewport(ui, ui_state, ctx, None, renderer.as_deref_mut());
            } else {
                layout::render_no_canvas_message(ui, ui_state, sprites_dir.as_deref());
            }
        }
        DualCanvasLayout::Horizontal => {
            layout::render_dual_viewports_horizontal(
                ui,
                ui_state,
                ctx,
                sprites_dir.as_deref(),
                renderer.as_deref_mut(),
            );
        }
        DualCanvasLayout::Vertical => {
            layout::render_dual_viewports_vertical(
                ui,
                ui_state,
                ctx,
                sprites_dir.as_deref(),
                renderer.as_deref_mut(),
            );
        }
    }

    // Handle copy/paste shortcuts (after viewports so cursor positions are updated)
    shortcuts::handle_copy_paste_shortcuts(ui_state, ctx);
}
