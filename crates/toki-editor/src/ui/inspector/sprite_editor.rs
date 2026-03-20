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
    render_color_picker(ui, ui_state);

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

fn render_color_picker(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    use super::super::editor_ui::PixelColor;

    ui.label("Color:");

    // Convert to Color32 for egui color picker
    let mut color = ui_state.sprite.foreground_color.to_color32();

    ui.horizontal(|ui| {
        if ui.color_edit_button_srgba(&mut color).changed() {
            ui_state.sprite.foreground_color = PixelColor::from_color32(color);
        }

        // Show hex value
        let hex = format!(
            "#{:02X}{:02X}{:02X}",
            color.r(),
            color.g(),
            color.b()
        );
        ui.label(hex);
    });

    // Recent colors palette
    if !ui_state.sprite.recent_colors.is_empty() {
        ui.add_space(4.0);
        ui.label("Recent:");
        render_recent_colors(ui, ui_state);
    }
}

fn render_recent_colors(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let size = egui::vec2(16.0, 16.0);
    let colors_per_row = 8;

    ui.horizontal_wrapped(|ui| {
        for (i, &color) in ui_state.sprite.recent_colors.iter().enumerate() {
            if i > 0 && i % colors_per_row == 0 {
                ui.end_row();
            }

            let color32 = color.to_color32();
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

            // Draw color swatch
            ui.painter().rect_filled(rect, 2.0, color32);
            ui.painter().rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::GRAY),
                egui::StrokeKind::Outside,
            );

            if response.clicked() {
                ui_state.sprite.foreground_color = color;
            }

            if response.hovered() {
                let hex = format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b);
                response.on_hover_text(hex);
            }
        }
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
