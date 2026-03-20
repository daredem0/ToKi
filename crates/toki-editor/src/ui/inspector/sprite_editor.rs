use super::*;

impl InspectorSystem {
    pub(super) fn render_sprite_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
    ) {
        ui.heading("Sprite Tools");
        ui.separator();

        render_tool_palette(ui, ui_state);
        ui.separator();

        render_tool_options(ui, ui_state);
    }
}

fn render_tool_palette(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    use super::super::editor_ui::SpriteEditorTool;

    ui.label("Tool:");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.sprite.tool, SpriteEditorTool::Drag, "Drag");
        ui.selectable_value(&mut ui_state.sprite.tool, SpriteEditorTool::Brush, "Brush");
        ui.selectable_value(&mut ui_state.sprite.tool, SpriteEditorTool::Eraser, "Eraser");
        ui.selectable_value(&mut ui_state.sprite.tool, SpriteEditorTool::Fill, "Fill");
    });
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.sprite.tool, SpriteEditorTool::Eyedropper, "Eyedrop");
        ui.selectable_value(&mut ui_state.sprite.tool, SpriteEditorTool::Select, "Select");
        ui.selectable_value(&mut ui_state.sprite.tool, SpriteEditorTool::Line, "Line");
    });
}

fn render_tool_options(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    use super::super::editor_ui::SpriteEditorTool;

    match ui_state.sprite.tool {
        SpriteEditorTool::Drag => {
            ui.label("Primary drag pans the canvas.");
        }
        SpriteEditorTool::Brush => {
            ui.label("Click/drag to draw pixels.");
            render_brush_size(ui, ui_state);
        }
        SpriteEditorTool::Eraser => {
            ui.label("Click/drag to erase pixels.");
            render_brush_size(ui, ui_state);
        }
        SpriteEditorTool::Fill => {
            ui.label("Click to fill connected area.");
        }
        SpriteEditorTool::Eyedropper => {
            ui.label("Click to pick a color from canvas.");
        }
        SpriteEditorTool::Select => {
            ui.label("Click/drag to select a region.");
        }
        SpriteEditorTool::Line => {
            ui.label("Click and drag to draw a line.");
            render_brush_size(ui, ui_state);
        }
    }

    ui.separator();
    render_viewport_controls(ui, ui_state);

    if ui_state.sprite.dirty {
        ui.separator();
        ui.label("Canvas has unsaved changes.");
    }
}

fn render_brush_size(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.label("Brush Size:");
        ui.add(
            egui::DragValue::new(&mut ui_state.sprite.brush_size)
                .range(1..=32)
                .speed(0.1),
        );
        ui.label("px");
    });
}

fn render_viewport_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.label("Viewport:");

    ui.horizontal(|ui| {
        ui.label(format!("Zoom: {}x", ui_state.sprite.viewport.zoom as i32));
        if ui.button("-").clicked() {
            ui_state.sprite.viewport.zoom_out();
        }
        if ui.button("+").clicked() {
            ui_state.sprite.viewport.zoom_in();
        }
    });

    ui.checkbox(&mut ui_state.sprite.show_grid, "Show Grid");

    if let Some(pos) = ui_state.sprite.cursor_canvas_pos {
        ui.label(format!("Cursor: {}, {}", pos.x, pos.y));
    }

    if let Some((w, h)) = ui_state.sprite.canvas_dimensions() {
        ui.label(format!("Canvas: {}x{}", w, h));
    }
}
