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

    const TOOL_ROWS: &[&[(SpriteEditorTool, &str)]] = &[
        &[
            (SpriteEditorTool::Drag, "Drag"),
            (SpriteEditorTool::Brush, "Brush"),
            (SpriteEditorTool::Eraser, "Eraser"),
        ],
        &[
            (SpriteEditorTool::Line, "Line"),
            (SpriteEditorTool::Fill, "Fill"),
            (SpriteEditorTool::Eyedropper, "Eyedrop"),
        ],
        &[
            (SpriteEditorTool::Select, "Select"),
            (SpriteEditorTool::MagicWand, "Magic Wand"),
            (SpriteEditorTool::MagicErase, "Magic Erase"),
        ],
        &[
            (SpriteEditorTool::AddOutline, "Add Outline"),
            (SpriteEditorTool::AddShadow, "Add Shadow"),
        ],
    ];

    ui.label("Tool:");
    egui::Grid::new("sprite_tool_palette_grid")
        .num_columns(3)
        .spacing([6.0, 6.0])
        .show(ui, |ui| {
            for row in TOOL_ROWS {
                for &(tool, label) in *row {
                    ui.selectable_value(&mut ui_state.sprite.tool, tool, label);
                }
                for _ in row.len()..3 {
                    ui.label("");
                }
                ui.end_row();
            }
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
        SpriteEditorTool::MagicWand => {
            ui.label("Click to select connected sprite pixels.");
        }
        SpriteEditorTool::MagicErase => {
            ui.label("Click to erase the connected color region inside the clicked tile.");
        }
        SpriteEditorTool::AddOutline => {
            ui.label(
                "Click a sprite to add an outline of the current color inside the clicked tile.",
            );
        }
        SpriteEditorTool::AddShadow => {
            ui.label("Click a sprite to add a ground shadow of the current color inside the clicked tile.");
        }
    }

    ui.separator();
    render_color_picker(ui, ui_state);

    ui.separator();
    render_viewport_controls(ui, ui_state);

    // Save controls
    if ui_state.sprite.has_canvas() {
        ui.separator();
        render_save_controls(ui, ui_state);
    }

    if ui_state.sprite.active().dirty {
        ui.separator();
        ui.label("Canvas has unsaved changes.");
    }
}

fn render_save_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.label("Asset:");

    if ui.button("Save As...").clicked() {
        ui_state.sprite.begin_save_dialog();
    }

    // Show current asset path if known
    if let Some(path) = &ui_state.sprite.active().active_sprite {
        ui.label(format!("File: {}", path));
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
        let hex = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
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

    let zoom = ui_state.sprite.active().viewport.zoom;
    ui.horizontal(|ui| {
        ui.label(format!("Zoom: {}x", zoom as i32));
        if ui.button("-").clicked() {
            ui_state.sprite.active_mut().viewport.zoom_out();
        }
        if ui.button("+").clicked() {
            ui_state.sprite.active_mut().viewport.zoom_in();
        }
    });

    ui.checkbox(
        &mut ui_state.sprite.active_mut().show_grid,
        "Show Pixel Grid",
    );

    if let Some(pos) = ui_state.sprite.active().cursor_canvas_pos {
        ui.label(format!("Cursor: {}, {}", pos.x, pos.y));
    }

    if let Some((w, h)) = ui_state.sprite.canvas_dimensions() {
        ui.label(format!("Canvas: {}x{}", w, h));
    }

    // Canvas transforms
    ui.separator();
    render_canvas_transforms(ui, ui_state);

    // Sheet controls
    ui.separator();
    render_sheet_controls(ui, ui_state);
}

fn render_canvas_transforms(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.label("Transform:");

    ui.horizontal(|ui| {
        if ui
            .button("Flip H")
            .on_hover_text("Flip horizontally")
            .clicked()
        {
            ui_state.sprite.flip_horizontal();
        }
        if ui
            .button("Flip V")
            .on_hover_text("Flip vertically")
            .clicked()
        {
            ui_state.sprite.flip_vertical();
        }
    });

    ui.horizontal(|ui| {
        if ui
            .button("Rot CW")
            .on_hover_text("Rotate 90° clockwise")
            .clicked()
        {
            ui_state.sprite.rotate_clockwise();
        }
        if ui
            .button("Rot CCW")
            .on_hover_text("Rotate 90° counter-clockwise")
            .clicked()
        {
            ui_state.sprite.rotate_counter_clockwise();
        }
    });

    if ui.button("Resize...").clicked() {
        ui_state.sprite.begin_resize_dialog();
    }
}

fn render_sheet_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.label("Sprite Sheet:");

    ui.checkbox(
        &mut ui_state.sprite.active_mut().show_cell_grid,
        "Show Cell Grid",
    );

    let show_cell_grid = ui_state.sprite.active().show_cell_grid;
    if show_cell_grid {
        ui.horizontal(|ui| {
            ui.label("Cell Width:");
            if ui
                .add(
                    egui::DragValue::new(&mut ui_state.sprite.active_mut().cell_size.x)
                        .range(1..=512)
                        .speed(1),
                )
                .changed()
            {
                // Deselect cell if grid changed
                ui_state.sprite.active_mut().selected_cell = None;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Cell Height:");
            if ui
                .add(
                    egui::DragValue::new(&mut ui_state.sprite.active_mut().cell_size.y)
                        .range(1..=512)
                        .speed(1),
                )
                .changed()
            {
                ui_state.sprite.active_mut().selected_cell = None;
            }
        });

        // Show cell count
        if let Some((cols, rows)) = ui_state.sprite.sheet_cell_count() {
            ui.label(format!("Grid: {}x{} ({} cells)", cols, rows, cols * rows));
        }

        // Sheet expansion controls
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("+ Row").clicked() {
                ui_state.sprite.append_row();
            }
            if ui.button("+ Column").clicked() {
                ui_state.sprite.append_column();
            }
        });

        // Show selected cell info and operations
        let selected_cell = ui_state.sprite.active().selected_cell;
        if let Some(cell_idx) = selected_cell {
            if let Some((cols, rows)) = ui_state.sprite.sheet_cell_count() {
                let col = cell_idx as u32 % cols;
                let row = cell_idx as u32 / cols;
                ui.label(format!(
                    "Selected: Cell {} (col {}, row {})",
                    cell_idx, col, row
                ));

                ui.add_space(4.0);

                // Cell operations
                ui.horizontal(|ui| {
                    if ui.button("Clear Cell").clicked() {
                        ui_state.sprite.clear_selected_cell();
                    }
                    if ui
                        .button("Delete & Collapse")
                        .on_hover_text("Delete cell and shift remaining cells to fill the gap")
                        .clicked()
                    {
                        ui_state.sprite.delete_cell_with_collapse();
                    }
                });

                // Swap with another cell
                let total_cells = cols * rows;
                ui.horizontal(|ui| {
                    ui.label("Swap with:");
                    ui.add(
                        egui::DragValue::new(&mut ui_state.sprite.active_mut().swap_target_cell)
                            .range(0..=(total_cells.saturating_sub(1)))
                            .speed(1),
                    );
                    let target = ui_state.sprite.active().swap_target_cell as usize;
                    if ui.button("Swap").clicked() && target != cell_idx {
                        ui_state.sprite.swap_cells(cell_idx, target);
                    }
                });
            }
        } else {
            ui.label("Selected: None (click to select)");
        }
    }
}
