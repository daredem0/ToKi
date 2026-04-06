use super::*;
use crate::ui::sprite_editor::{canonical_indexed_color_for_size, indexed_slot_for_authored_color};
use toki_core::assets::atlas::ColorMode;
use toki_core::palette::{validate_indexed_rgba8, PaletteSize};

impl InspectorSystem {
    pub(crate) fn render_sprite_editor_toolbox(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
    ) {
        ui.heading("Sprite Toolbox");
        ui.separator();

        render_tool_palette(ui, ui_state);
        ui.separator();

        render_tool_options(ui, ui_state);

        if crate::ui::editor_context::sprite_state(ui_state).has_floating() {
            ui.separator();
            render_floating_controls(ui, ui_state);
        }
    }

    pub(crate) fn render_sprite_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
    ) {
        ui.heading("Sprite Inspector");
        ui.separator();

        if !crate::ui::editor_context::sprite_state(ui_state).has_canvas() {
            ui.label("No sprite selected.");
            ui.label("Open or create a sprite asset to inspect it.");
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Tool:");
            ui.label(tool_label(
                crate::ui::editor_context::sprite_state(ui_state).tool,
            ));
        });

        if let Some(path) = &crate::ui::editor_context::sprite_state(ui_state)
            .active()
            .active_sprite
        {
            ui.horizontal(|ui| {
                ui.label("File:");
                ui.label(path);
            });
        }

        if let Some((w, h)) = crate::ui::editor_context::sprite_state(ui_state).canvas_dimensions()
        {
            ui.horizontal(|ui| {
                ui.label("Canvas:");
                ui.label(format!("{}x{}", w, h));
            });
        }

        if let Some(pos) = crate::ui::editor_context::sprite_state(ui_state)
            .active()
            .cursor_canvas_pos
        {
            ui.horizontal(|ui| {
                ui.label("Cursor:");
                ui.label(format!("{}, {}", pos.x, pos.y));
            });
        }

        if let Some(cell_idx) = crate::ui::editor_context::sprite_state(ui_state)
            .active()
            .selected_cell
        {
            ui.horizontal(|ui| {
                ui.label("Selected Cell:");
                ui.label(cell_idx.to_string());
            });
        }

        if crate::ui::editor_context::sprite_state(ui_state)
            .active()
            .dirty
        {
            ui.separator();
            ui.label("Canvas has unsaved changes.");
        }
    }
}

fn tool_label(tool: crate::ui::editor_ui::SpriteEditorTool) -> &'static str {
    use crate::ui::editor_ui::SpriteEditorTool;

    match tool {
        SpriteEditorTool::Drag => "Drag",
        SpriteEditorTool::Brush => "Brush",
        SpriteEditorTool::Eraser => "Eraser",
        SpriteEditorTool::Gradient => "Gradient",
        SpriteEditorTool::Fill => "Fill",
        SpriteEditorTool::Eyedropper => "Eyedropper",
        SpriteEditorTool::Select => "Select",
        SpriteEditorTool::Line => "Line",
        SpriteEditorTool::MagicWand => "Magic Wand",
        SpriteEditorTool::MagicErase => "Magic Erase",
        SpriteEditorTool::AddOutline => "Add Outline",
        SpriteEditorTool::AddShadow => "Add Shadow",
        SpriteEditorTool::Rectangle => "Rectangle",
        SpriteEditorTool::Ellipse => "Ellipse",
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
            (SpriteEditorTool::Gradient, "Gradient"),
            (SpriteEditorTool::Fill, "Fill"),
        ],
        &[
            (SpriteEditorTool::Eyedropper, "Eyedrop"),
            (SpriteEditorTool::Select, "Select"),
            (SpriteEditorTool::MagicWand, "Magic Wand"),
        ],
        &[(SpriteEditorTool::MagicErase, "Magic Erase")],
        &[
            (SpriteEditorTool::Rectangle, "Rect"),
            (SpriteEditorTool::Ellipse, "Ellipse"),
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
                    ui.selectable_value(
                        &mut crate::ui::editor_context::sprite_state_mut(ui_state).tool,
                        tool,
                        label,
                    );
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

    match crate::ui::editor_context::sprite_state_mut(ui_state).tool {
        SpriteEditorTool::Drag => {
            ui.label("Primary drag pans the canvas.");
        }
        SpriteEditorTool::Brush => {
            ui.label("Click/drag to draw pixels.");
            render_brush_size(ui, ui_state);
            render_dither_selector(ui, ui_state);
            let pixel_perfect_enabled =
                crate::ui::editor_context::sprite_state(ui_state).brush_size == 1;
            ui.add_enabled_ui(pixel_perfect_enabled, |ui| {
                ui.checkbox(
                    &mut crate::ui::editor_context::sprite_state_mut(ui_state).pixel_perfect,
                    "Pixel Perfect",
                );
            });
        }
        SpriteEditorTool::Eraser => {
            ui.label("Click/drag to erase pixels.");
            render_brush_size(ui, ui_state);
        }
        SpriteEditorTool::Gradient => {
            ui.label("Click and drag to preview and apply a gradient.");
            render_gradient_options(ui, ui_state);
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
        SpriteEditorTool::Rectangle => {
            ui.label("Click and drag to draw a rectangle.");
            render_brush_size(ui, ui_state);
            ui.checkbox(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).shape_filled,
                "Filled",
            );
        }
        SpriteEditorTool::Ellipse => {
            ui.label("Click and drag to draw an ellipse.");
            render_brush_size(ui, ui_state);
            ui.checkbox(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).shape_filled,
                "Filled",
            );
        }
    }

    // Symmetry controls for painting tools
    if matches!(
        crate::ui::editor_context::sprite_state_mut(ui_state).tool,
        SpriteEditorTool::Brush
            | SpriteEditorTool::Eraser
            | SpriteEditorTool::Fill
            | SpriteEditorTool::Line
            | SpriteEditorTool::Rectangle
            | SpriteEditorTool::Ellipse
    ) {
        ui.separator();
        ui.label("Symmetry:");
        ui.horizontal(|ui| {
            ui.toggle_value(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).symmetry_horizontal,
                "Horizontal",
            );
            ui.toggle_value(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).symmetry_vertical,
                "Vertical",
            );
        });
        if crate::ui::editor_context::sprite_state_mut(ui_state).is_sheet() {
            ui.checkbox(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).symmetry_per_tile,
                "Per tile",
            );
        }
    }

    ui.separator();
    render_color_mode_controls(ui, ui_state);

    ui.separator();
    render_color_picker(ui, ui_state);

    ui.separator();
    render_viewport_controls(ui, ui_state);

    // Save controls
    if crate::ui::editor_context::sprite_state_mut(ui_state).has_canvas() {
        ui.separator();
        render_save_controls(ui, ui_state);
    }

    if crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .dirty
    {
        ui.separator();
        ui.label("Canvas has unsaved changes.");
    }
}

fn render_color_mode_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let previous_mode = crate::ui::editor_context::sprite_state_mut(ui_state).color_mode;
    ui.label("Color Mode:");
    egui::ComboBox::from_id_salt("sprite_editor_color_mode")
        .selected_text(
            match crate::ui::editor_context::sprite_state_mut(ui_state).color_mode {
                ColorMode::TrueColor => "TrueColor",
                ColorMode::PaletteIndexed => "PaletteIndexed",
            },
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).color_mode,
                ColorMode::TrueColor,
                "TrueColor",
            );
            ui.selectable_value(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).color_mode,
                ColorMode::PaletteIndexed,
                "PaletteIndexed",
            );
        });

    if previous_mode != crate::ui::editor_context::sprite_state_mut(ui_state).color_mode {
        if crate::ui::editor_context::sprite_state_mut(ui_state).color_mode
            == ColorMode::PaletteIndexed
        {
            ensure_valid_indexed_foreground_color(ui_state);
            ensure_valid_indexed_gradient_end_color(ui_state);
        }
        crate::ui::editor_context::sprite_state_mut(ui_state).invalidate_all_canvas_textures();
    }

    if crate::ui::editor_context::sprite_state_mut(ui_state).color_mode == ColorMode::PaletteIndexed
    {
        let available_palettes = ui_state.project.available_palettes.clone();
        ui_state
            .sprite_editor_context_mut()
            .sprite
            .sync_palette_selection(&available_palettes);
        let palette_ids = ui_state
            .project
            .available_palettes
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        let previous_palette_id = crate::ui::editor_context::sprite_state_mut(ui_state)
            .authored_palette_id
            .clone();
        ui.horizontal(|ui| {
            ui.label("Authored Palette:");
            egui::ComboBox::from_id_salt("sprite_editor_palette_id")
                .selected_text(
                    ui_state
                        .sprite_editor_context()
                        .sprite
                        .authored_palette_id
                        .as_deref()
                        .unwrap_or("No palette"),
                )
                .show_ui(ui, |ui| {
                    for palette_id in &palette_ids {
                        ui.selectable_value(
                            &mut crate::ui::editor_context::sprite_state_mut(ui_state)
                                .authored_palette_id,
                            Some(palette_id.clone()),
                            palette_id,
                        );
                    }
                });
        });

        ensure_valid_indexed_foreground_color(ui_state);
        ensure_valid_indexed_gradient_end_color(ui_state);
        if previous_palette_id
            != crate::ui::editor_context::sprite_state_mut(ui_state).authored_palette_id
        {
            crate::ui::editor_context::sprite_state_mut(ui_state).invalidate_all_canvas_textures();
        }

        if crate::ui::editor_context::sprite_state_mut(ui_state).has_canvas() {
            let can_convert = ui_state
                .sprite_editor_context()
                .sprite
                .authored_palette_id
                .as_ref()
                .and_then(|palette_id| ui_state.project.available_palettes.get(palette_id))
                .is_some();
            let response = ui
                .add_enabled(can_convert, egui::Button::new("Convert To Palette"))
                .on_hover_text(
                    "Maps all non-transparent pixels to the nearest color slot of the selected palette.",
                );
            if response.clicked() {
                if let Some(palette) = ui_state
                    .sprite_editor_context()
                    .sprite
                    .authored_palette_id
                    .as_ref()
                    .and_then(|palette_id| ui_state.project.available_palettes.get(palette_id))
                    .cloned()
                {
                    crate::ui::editor_context::sprite_state_mut(ui_state)
                        .convert_active_canvas_to_palette(&palette);
                }
            }
        }
    }
}

fn render_save_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.label("Asset:");

    if crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .active_sprite
        .is_some()
        && ui.button("Save").clicked()
    {
        if let Err(e) = crate::ui::editor_context::sprite_state_mut(ui_state).save_current_asset() {
            tracing::error!("Failed to save sprite: {}", e);
        }
    }

    if ui.button("Save As...").clicked() {
        crate::ui::editor_context::sprite_state_mut(ui_state).begin_save_dialog();
    }

    // Show current asset path if known
    if let Some(path) = &crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .active_sprite
    {
        ui.label(format!("File: {}", path));
    }
}

fn render_brush_size(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.label("Brush Size:");
        ui.add(
            egui::DragValue::new(
                &mut crate::ui::editor_context::sprite_state_mut(ui_state).brush_size,
            )
            .range(1..=32)
            .speed(0.1),
        );
        ui.label("px");
    });
}

fn render_dither_selector(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    use crate::ui::sprite_editor::DitherPattern;

    ui.horizontal(|ui| {
        ui.label("Dither:");
        egui::ComboBox::from_id_salt("dither_pattern")
            .selected_text(
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .dither_pattern
                    .label(),
            )
            .show_ui(ui, |ui| {
                for pattern in DitherPattern::ALL {
                    ui.selectable_value(
                        &mut crate::ui::editor_context::sprite_state_mut(ui_state).dither_pattern,
                        pattern,
                        pattern.label(),
                    );
                }
            });
    });
}

fn render_gradient_options(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    use crate::ui::editor_ui::{GradientMode, GradientStyle};

    ui.label("Mode:");
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut crate::ui::editor_context::sprite_state_mut(ui_state).gradient_mode,
            GradientMode::Linear,
            "Linear",
        );
        ui.selectable_value(
            &mut crate::ui::editor_context::sprite_state_mut(ui_state).gradient_mode,
            GradientMode::Radial,
            "Radial",
        );
    });

    ui.label("Style:");
    ui.horizontal(|ui| {
        ui.selectable_value(
            &mut crate::ui::editor_context::sprite_state_mut(ui_state).gradient_style,
            GradientStyle::Smooth,
            "Smooth",
        );
        ui.selectable_value(
            &mut crate::ui::editor_context::sprite_state_mut(ui_state).gradient_style,
            GradientStyle::Dithered,
            "Dithered",
        );
    });

    render_gradient_end_color_picker(ui, ui_state);
}

fn render_gradient_end_color_picker(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    use super::super::editor_ui::PixelColor;

    ui.label("End Color:");

    if crate::ui::editor_context::sprite_state_mut(ui_state).color_mode == ColorMode::PaletteIndexed
    {
        ensure_valid_indexed_gradient_end_color(ui_state);
        if let Some(palette_id) = crate::ui::editor_context::sprite_state_mut(ui_state)
            .authored_palette_id
            .clone()
        {
            if let Some(palette) = ui_state
                .project
                .available_palettes
                .get(&palette_id)
                .cloned()
            {
                let selected_slot = indexed_slot_for_authored_color(
                    crate::ui::editor_context::sprite_state(ui_state).gradient_end_color,
                    Some(&palette),
                );
                ui.horizontal_wrapped(|ui| {
                    for (slot, color) in palette.colors().iter().copied().enumerate() {
                        let pixel_color = PixelColor::from_rgba_array(color);
                        let is_selected = selected_slot == Some(slot);
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                        ui.painter()
                            .rect_filled(rect, 3.0, pixel_color.to_color32());
                        ui.painter().rect_stroke(
                            rect,
                            3.0,
                            egui::Stroke::new(
                                if is_selected { 2.0 } else { 1.0 },
                                if is_selected {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::GRAY
                                },
                            ),
                            egui::StrokeKind::Outside,
                        );
                        if response.clicked() {
                            crate::ui::editor_context::sprite_state_mut(ui_state)
                                .gradient_end_color =
                                canonical_indexed_color_for_size(slot, palette.size());
                        }
                    }
                });
                return;
            }
        }

        ui.label("No palette available for indexed editing.");
        return;
    }

    let mut color = crate::ui::editor_context::sprite_state_mut(ui_state)
        .gradient_end_color
        .to_color32();

    ui.horizontal(|ui| {
        if ui.color_edit_button_srgba(&mut color).changed() {
            crate::ui::editor_context::sprite_state_mut(ui_state).gradient_end_color =
                PixelColor::from_color32(color);
        }
        let hex = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        ui.label(hex);
    });
}

fn render_color_picker(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    use super::super::editor_ui::PixelColor;

    ui.label("Color:");

    if crate::ui::editor_context::sprite_state_mut(ui_state).color_mode == ColorMode::PaletteIndexed
    {
        ensure_valid_indexed_foreground_color(ui_state);
        if let Some(palette_id) = crate::ui::editor_context::sprite_state_mut(ui_state)
            .authored_palette_id
            .clone()
        {
            if let Some(palette) = ui_state
                .project
                .available_palettes
                .get(&palette_id)
                .cloned()
            {
                let selected_slot = indexed_slot_for_authored_color(
                    crate::ui::editor_context::sprite_state(ui_state).foreground_color,
                    Some(&palette),
                );
                ui.horizontal_wrapped(|ui| {
                    for (slot, color) in palette.colors().iter().copied().enumerate() {
                        let pixel_color = PixelColor::from_rgba_array(color);
                        let is_selected = selected_slot == Some(slot);
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::click());
                        ui.painter()
                            .rect_filled(rect, 3.0, pixel_color.to_color32());
                        ui.painter().rect_stroke(
                            rect,
                            3.0,
                            egui::Stroke::new(
                                if is_selected { 2.0 } else { 1.0 },
                                if is_selected {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::GRAY
                                },
                            ),
                            egui::StrokeKind::Outside,
                        );
                        if response.clicked() {
                            crate::ui::editor_context::sprite_state_mut(ui_state)
                                .foreground_color =
                                canonical_indexed_color_for_size(slot, palette.size());
                        }
                    }
                });
                render_indexed_validation(ui, ui_state);
                return;
            }
        }

        ui.label("No palette available for indexed editing.");
        render_indexed_validation(ui, ui_state);
        return;
    }

    // Convert to Color32 for egui color picker
    let mut color = crate::ui::editor_context::sprite_state_mut(ui_state)
        .foreground_color
        .to_color32();

    ui.horizontal(|ui| {
        if ui.color_edit_button_srgba(&mut color).changed() {
            crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color =
                PixelColor::from_color32(color);
        }

        // Show hex value
        let hex = format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b());
        ui.label(hex);
    });

    // Recent colors palette
    if !crate::ui::editor_context::sprite_state_mut(ui_state)
        .recent_colors
        .is_empty()
    {
        ui.add_space(4.0);
        ui.label("Recent:");
        render_recent_colors(ui, ui_state);
    }
}

fn render_indexed_validation(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let palette_size = ui_state
        .sprite_editor_context()
        .sprite
        .authored_palette_id
        .as_ref()
        .and_then(|id| ui_state.project.available_palettes.get(id))
        .map_or(PaletteSize::Pal4, |p| p.size());
    let Some(canvas) = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .canvas
        .as_ref()
    else {
        return;
    };
    let validation = validate_indexed_rgba8(canvas.pixels(), palette_size);
    ui.add_space(4.0);
    if validation.invalid_colors.is_empty() {
        ui.label(format!(
            "Indexed validation: OK ({} unique colors)",
            validation.unique_color_count
        ));
    } else {
        ui.colored_label(
            egui::Color32::YELLOW,
            format!(
                "Indexed validation: {} invalid colors ({} unique colors)",
                validation.invalid_colors.len(),
                validation.unique_color_count
            ),
        );
    }
}

fn ensure_valid_indexed_foreground_color(ui_state: &mut EditorUI) {
    if crate::ui::editor_context::sprite_state_mut(ui_state).color_mode != ColorMode::PaletteIndexed
    {
        return;
    }

    let foreground_color = crate::ui::editor_context::sprite_state(ui_state).foreground_color;
    let selected_palette = ui_state
        .sprite_editor_context()
        .sprite
        .authored_palette_id
        .as_ref()
        .and_then(|palette_id| ui_state.project.available_palettes.get(palette_id));

    if indexed_slot_for_authored_color(foreground_color, selected_palette).is_none() {
        let size = selected_palette.map_or(PaletteSize::Pal4, |p| p.size());
        crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color =
            canonical_indexed_color_for_size(size.color_count() - 1, size);
    }
}

fn ensure_valid_indexed_gradient_end_color(ui_state: &mut EditorUI) {
    if crate::ui::editor_context::sprite_state_mut(ui_state).color_mode != ColorMode::PaletteIndexed
    {
        return;
    }

    let gradient_end_color = crate::ui::editor_context::sprite_state(ui_state).gradient_end_color;
    let selected_palette = ui_state
        .sprite_editor_context()
        .sprite
        .authored_palette_id
        .as_ref()
        .and_then(|palette_id| ui_state.project.available_palettes.get(palette_id));

    if indexed_slot_for_authored_color(gradient_end_color, selected_palette).is_none() {
        let size = selected_palette.map_or(PaletteSize::Pal4, |p| p.size());
        crate::ui::editor_context::sprite_state_mut(ui_state).gradient_end_color =
            canonical_indexed_color_for_size(size.color_count() - 1, size);
    }
}

fn render_recent_colors(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let size = egui::vec2(16.0, 16.0);
    let colors_per_row = 8;
    let recent_colors = crate::ui::editor_context::sprite_state(ui_state)
        .recent_colors
        .clone();

    ui.horizontal_wrapped(|ui| {
        for (i, &color) in recent_colors.iter().enumerate() {
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
                crate::ui::editor_context::sprite_state_mut(ui_state).foreground_color = color;
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

    let zoom = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .viewport
        .zoom;
    ui.horizontal(|ui| {
        ui.label(format!("Zoom: {}x", zoom as i32));
        if ui.button("-").clicked() {
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .viewport
                .zoom_out();
        }
        if ui.button("+").clicked() {
            crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .viewport
                .zoom_in();
        }
    });

    ui.checkbox(
        &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .show_grid,
        "Show Pixel Grid",
    );
    ui.checkbox(
        &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .tile_preview,
        "Tile Preview",
    );

    if let Some(pos) = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .cursor_canvas_pos
    {
        ui.label(format!("Cursor: {}, {}", pos.x, pos.y));
    }

    if let Some((w, h)) = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_dimensions()
    {
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
            crate::ui::editor_context::sprite_state_mut(ui_state).flip_horizontal();
        }
        if ui
            .button("Flip V")
            .on_hover_text("Flip vertically")
            .clicked()
        {
            crate::ui::editor_context::sprite_state_mut(ui_state).flip_vertical();
        }
    });

    ui.horizontal(|ui| {
        if ui
            .button("Rot CW")
            .on_hover_text("Rotate 90° clockwise")
            .clicked()
        {
            crate::ui::editor_context::sprite_state_mut(ui_state).rotate_clockwise();
        }
        if ui
            .button("Rot CCW")
            .on_hover_text("Rotate 90° counter-clockwise")
            .clicked()
        {
            crate::ui::editor_context::sprite_state_mut(ui_state).rotate_counter_clockwise();
        }
    });

    if ui.button("Resize...").clicked() {
        crate::ui::editor_context::sprite_state_mut(ui_state).begin_resize_dialog();
    }
}

fn render_sheet_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.label("Sprite Sheet:");

    ui.checkbox(
        &mut crate::ui::editor_context::sprite_state_mut(ui_state)
            .active_mut()
            .show_cell_grid,
        "Show Cell Grid",
    );

    let show_cell_grid = crate::ui::editor_context::sprite_state_mut(ui_state)
        .active()
        .show_cell_grid;
    if show_cell_grid {
        ui.checkbox(
            &mut crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .show_cell_cross,
            "Show Cell Cross",
        );

        ui.horizontal(|ui| {
            ui.label("Cell Width:");
            if ui
                .add(
                    egui::DragValue::new(
                        &mut crate::ui::editor_context::sprite_state_mut(ui_state)
                            .active_mut()
                            .cell_size
                            .x,
                    )
                    .range(1..=512)
                    .speed(1),
                )
                .changed()
            {
                // Deselect cell if grid changed
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .selected_cell = None;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Cell Height:");
            if ui
                .add(
                    egui::DragValue::new(
                        &mut crate::ui::editor_context::sprite_state_mut(ui_state)
                            .active_mut()
                            .cell_size
                            .y,
                    )
                    .range(1..=512)
                    .speed(1),
                )
                .changed()
            {
                crate::ui::editor_context::sprite_state_mut(ui_state)
                    .active_mut()
                    .selected_cell = None;
            }
        });

        // Show cell count
        if let Some((cols, rows)) =
            crate::ui::editor_context::sprite_state_mut(ui_state).sheet_cell_count()
        {
            ui.label(format!("Grid: {}x{} ({} cells)", cols, rows, cols * rows));
        }

        // Sheet expansion controls
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("+ Row").clicked() {
                crate::ui::editor_context::sprite_state_mut(ui_state).append_row();
            }
            if ui.button("+ Column").clicked() {
                crate::ui::editor_context::sprite_state_mut(ui_state).append_column();
            }
        });

        // Show selected cell info and operations
        let selected_cell = crate::ui::editor_context::sprite_state_mut(ui_state)
            .active()
            .selected_cell;
        if let Some(cell_idx) = selected_cell {
            if let Some((cols, rows)) =
                crate::ui::editor_context::sprite_state_mut(ui_state).sheet_cell_count()
            {
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
                        crate::ui::editor_context::sprite_state_mut(ui_state).clear_selected_cell();
                    }
                    if ui
                        .button("Delete & Collapse")
                        .on_hover_text("Delete cell and shift remaining cells to fill the gap")
                        .clicked()
                    {
                        crate::ui::editor_context::sprite_state_mut(ui_state)
                            .delete_cell_with_collapse();
                    }
                });

                // Swap with another cell
                let total_cells = cols * rows;
                ui.horizontal(|ui| {
                    ui.label("Swap with:");
                    ui.add(
                        egui::DragValue::new(
                            &mut crate::ui::editor_context::sprite_state_mut(ui_state)
                                .active_mut()
                                .swap_target_cell,
                        )
                        .range(0..=(total_cells.saturating_sub(1)))
                        .speed(1),
                    );
                    let target = crate::ui::editor_context::sprite_state_mut(ui_state)
                        .active()
                        .swap_target_cell as usize;
                    if ui.button("Swap").clicked() && target != cell_idx {
                        crate::ui::editor_context::sprite_state_mut(ui_state)
                            .swap_cells(cell_idx, target);
                    }
                });
            }
        } else {
            ui.label("Selected: None (click to select)");
        }
    }

    if crate::ui::editor_context::sprite_state(ui_state)
        .autotile_info
        .is_some()
    {
        ui.separator();
        ui.label("Autotile Overlay:");
        ui.checkbox(
            &mut crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .show_autotile_labels,
            "Show Autotile Labels",
        );
        ui.checkbox(
            &mut crate::ui::editor_context::sprite_state_mut(ui_state)
                .active_mut()
                .show_autotile_guides,
            "Show Autotile Guides",
        );
    }
}

fn render_floating_controls(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let is_sheet = crate::ui::editor_context::sprite_state(ui_state).is_sheet();
    let label = if is_sheet {
        "Center to tile"
    } else {
        "Center to canvas"
    };
    ui.label("Floating Selection:");
    if ui.button(label).clicked() {
        crate::ui::editor_context::sprite_state_mut(ui_state).center_floating_on_tile();
        crate::ui::editor_context::sprite_state_mut(ui_state).invalidate_all_canvas_textures();
    }
}
