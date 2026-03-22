//! Sprite editor toolbar rendering.

use crate::ui::editor_ui::{DualCanvasLayout, SpriteEditorTool};
use crate::ui::EditorUI;

pub fn render_toolbar(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    sprites_dir: Option<&std::path::Path>,
) {
    ui.horizontal(|ui| {
        ui.heading("Sprite Editor");
        ui.separator();

        if ui.button("New Canvas").clicked() {
            ui_state.begin_new_sprite_canvas_dialog();
        }

        // Load button - only enabled if we have a project
        let load_enabled = sprites_dir.is_some();
        if ui
            .add_enabled(load_enabled, egui::Button::new("Load Sprite"))
            .clicked()
        {
            if let Some(dir) = sprites_dir {
                ui_state.sprite.begin_load_dialog(dir);
            }
        }

        // Merge sprites into sheet
        if ui
            .add_enabled(load_enabled, egui::Button::new("Merge..."))
            .on_hover_text("Merge multiple sprites into a single sheet")
            .clicked()
        {
            if let Some(dir) = sprites_dir {
                ui_state.sprite.begin_merge_dialog(dir);
            }
        }

        // Import external image
        if ui.button("Import...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Import Image")
                .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                .pick_file()
            {
                if let Err(e) = ui_state.sprite.import_external_image(&path) {
                    tracing::error!("Failed to import image: {}", e);
                }
            }
        }

        // Export as PNG
        let has_canvas = ui_state.sprite.has_canvas();
        if ui
            .add_enabled(has_canvas, egui::Button::new("Export PNG..."))
            .clicked()
        {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Export PNG")
                .add_filter("PNG Image", &["png"])
                .set_file_name("sprite.png")
                .save_file()
            {
                if let Err(e) = ui_state.sprite.export_as_png(&path) {
                    tracing::error!("Failed to export image: {}", e);
                }
            }
        }

        if ui_state.sprite.has_canvas() && ui_state.sprite.active().dirty {
            ui.label("Unsaved changes");
        }

        ui.separator();

        // Layout toggle button
        let layout_label = match ui_state.sprite.layout {
            DualCanvasLayout::Single => "Single",
            DualCanvasLayout::Horizontal => "Side-by-Side",
            DualCanvasLayout::Vertical => "Stacked",
        };
        if ui
            .button(format!("Layout: {}", layout_label))
            .on_hover_text("Click to cycle between Single, Side-by-Side, and Stacked layouts")
            .clicked()
        {
            ui_state.sprite.cycle_layout();
        }

        // Show active canvas indicator when in dual mode
        if ui_state.sprite.layout != DualCanvasLayout::Single {
            ui.separator();
            let active_label = match ui_state.sprite.active_canvas {
                crate::ui::editor_ui::CanvasSide::Left => "Left",
                crate::ui::editor_ui::CanvasSide::Right => "Right",
            };
            ui.label(format!("Active: {}", active_label));
            if ui
                .button("Switch")
                .on_hover_text("Switch active canvas")
                .clicked()
            {
                ui_state.sprite.switch_active_canvas();
            }
        }
    });

    // Show current tool (like map editor)
    if ui_state.sprite.has_canvas() {
        ui.horizontal(|ui| {
            ui.label("Tool:");
            ui.label(tool_label(ui_state.sprite.tool));
        });
    }
}

fn tool_label(tool: SpriteEditorTool) -> &'static str {
    match tool {
        SpriteEditorTool::Drag => "Drag",
        SpriteEditorTool::Brush => "Brush",
        SpriteEditorTool::Eraser => "Eraser",
        SpriteEditorTool::Fill => "Fill",
        SpriteEditorTool::Eyedropper => "Eyedropper",
        SpriteEditorTool::Select => "Select",
        SpriteEditorTool::Line => "Line",
        SpriteEditorTool::MagicWand => "Magic Wand",
        SpriteEditorTool::MagicErase => "Magic Erase",
        SpriteEditorTool::AddOutline => "Add Outline",
    }
}
