use crate::project::Project;
use crate::ui::editor_ui::{
    CanvasSide, DualCanvasLayout, SpriteCanvas, SpriteCanvasViewport, SpriteEditorTool,
};
use crate::ui::EditorUI;

/// Renders the sprite editor panel
pub fn render_sprite_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project: Option<&mut Project>,
) {
    // Get sprites directory from project (needed for multiple operations)
    let sprites_dir = project
        .as_ref()
        .map(|p| p.path.join("assets").join("sprites"));

    // Handle new canvas dialog
    if ui_state.sprite.show_new_canvas_dialog {
        render_new_canvas_dialog(ui_state, ctx);
    }

    // Handle save dialog
    if ui_state.sprite.show_save_dialog {
        render_save_dialog(ui_state, ctx, sprites_dir.as_deref());
    }

    // Handle load dialog
    if ui_state.sprite.show_load_dialog {
        render_load_dialog(ui_state, ctx);
    }

    // Handle merge dialog
    if ui_state.sprite.show_merge_dialog {
        render_merge_dialog(ui_state, ctx);
    }

    // Handle resize dialog
    if ui_state.sprite.show_resize_dialog {
        render_resize_dialog(ui_state, ctx);
    }

    // Handle rename dialog
    if ui_state.sprite.show_rename_dialog {
        render_rename_dialog(ui_state, ctx, sprites_dir.as_deref());
    }

    // Handle delete confirmation
    if ui_state.sprite.show_delete_confirm {
        render_delete_confirm_dialog(ui_state, ctx, sprites_dir.as_deref());
    }

    // Handle warning dialog
    if ui_state.sprite.show_warning_dialog {
        render_warning_dialog(ui_state, ctx);
    }

    // Toolbar (simplified - tools are in inspector panel)
    render_toolbar(ui, ui_state, sprites_dir.as_deref());
    ui.separator();

    // Main content area - render based on layout mode
    match ui_state.sprite.layout {
        DualCanvasLayout::Single => {
            if ui_state.sprite.has_canvas() {
                render_canvas_viewport(ui, ui_state, ctx, None);
            } else {
                render_no_canvas_message(ui, ui_state, sprites_dir.as_deref());
            }
        }
        DualCanvasLayout::Horizontal => {
            render_dual_viewports_horizontal(ui, ui_state, ctx, sprites_dir.as_deref());
        }
        DualCanvasLayout::Vertical => {
            render_dual_viewports_vertical(ui, ui_state, ctx, sprites_dir.as_deref());
        }
    }

    // Handle copy/paste shortcuts (after viewports so cursor positions are updated)
    // Use ctx.input() directly to avoid potential filtering by ui
    handle_copy_paste_shortcuts(ui_state, ctx);
}

fn render_toolbar(
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
            crate::ui::editor_ui::DualCanvasLayout::Single => "Single",
            crate::ui::editor_ui::DualCanvasLayout::Horizontal => "Side-by-Side",
            crate::ui::editor_ui::DualCanvasLayout::Vertical => "Stacked",
        };
        if ui
            .button(format!("Layout: {}", layout_label))
            .on_hover_text("Click to cycle between Single, Side-by-Side, and Stacked layouts")
            .clicked()
        {
            ui_state.sprite.cycle_layout();
        }

        // Show active canvas indicator when in dual mode
        if ui_state.sprite.layout != crate::ui::editor_ui::DualCanvasLayout::Single {
            ui.separator();
            let active_label = match ui_state.sprite.active_canvas {
                crate::ui::editor_ui::CanvasSide::Left => "Left",
                crate::ui::editor_ui::CanvasSide::Right => "Right",
            };
            ui.label(format!("Active: {}", active_label));
            if ui.button("Switch").on_hover_text("Switch active canvas").clicked() {
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
    }
}

fn render_new_canvas_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    egui::Window::new("New Canvas")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Sprite Width:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.new_sprite_width)
                        .range(1..=512)
                        .speed(1),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Sprite Height:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.new_sprite_height)
                        .range(1..=512)
                        .speed(1),
                );
            });

            ui.separator();

            ui.checkbox(
                &mut ui_state.sprite.new_canvas_is_sheet,
                "Create as sprite sheet",
            );

            if ui_state.sprite.new_canvas_is_sheet {
                ui.horizontal(|ui| {
                    ui.label("Columns:");
                    ui.add(
                        egui::DragValue::new(&mut ui_state.sprite.new_sheet_cols)
                            .range(1..=64)
                            .speed(1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Rows:");
                    ui.add(
                        egui::DragValue::new(&mut ui_state.sprite.new_sheet_rows)
                            .range(1..=64)
                            .speed(1),
                    );
                });

                // Show calculated canvas size
                let cols = ui_state.sprite.new_sheet_cols;
                let rows = ui_state.sprite.new_sheet_rows;
                let canvas_w = ui_state.sprite.new_sprite_width * cols;
                let canvas_h = ui_state.sprite.new_sprite_height * rows;
                ui.label(format!(
                    "Canvas: {}x{} ({} cells)",
                    canvas_w,
                    canvas_h,
                    cols * rows
                ));
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    submit_new_canvas(ui_state);
                }
                if ui.button("Cancel").clicked() {
                    ui_state.cancel_new_sprite_canvas_dialog();
                }
            });
        });
}

fn submit_new_canvas(ui_state: &mut EditorUI) {
    let sprite_w = ui_state.sprite.new_sprite_width.max(1);
    let sprite_h = ui_state.sprite.new_sprite_height.max(1);

    if ui_state.sprite.new_canvas_is_sheet {
        let cols = ui_state.sprite.new_sheet_cols.max(1);
        let rows = ui_state.sprite.new_sheet_rows.max(1);
        let canvas_w = sprite_w * cols;
        let canvas_h = sprite_h * rows;
        ui_state
            .sprite
            .new_sheet(canvas_w, canvas_h, sprite_w, sprite_h);
    } else {
        ui_state.sprite.new_canvas(sprite_w, sprite_h);
        ui_state.sprite.active_mut().show_cell_grid = false;
    }
    ui_state.sprite.show_new_canvas_dialog = false;
}

fn render_save_dialog(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    use crate::ui::editor_ui::SpriteAssetKind;

    egui::Window::new("Save Sprite Asset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Check if we have a project to save to
            let Some(sprites_dir) = sprites_dir else {
                ui.label("No project open. Cannot save sprite.");
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_save_dialog = false;
                }
                return;
            };

            ui.horizontal(|ui| {
                ui.label("Asset Name:");
                ui.text_edit_singleline(&mut ui_state.sprite.active_mut().save_asset_name);
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Save as:");
                ui.selectable_value(
                    &mut ui_state.sprite.active_mut().save_asset_kind,
                    SpriteAssetKind::ObjectSheet,
                    "Object Sheet",
                );
                ui.selectable_value(
                    &mut ui_state.sprite.active_mut().save_asset_kind,
                    SpriteAssetKind::TileAtlas,
                    "Tile Atlas",
                );
            });

            // Show info about what will be created
            ui.add_space(4.0);
            if ui_state.sprite.is_sheet() {
                if let Some((cols, rows)) = ui_state.sprite.sheet_cell_count() {
                    ui.label(format!(
                        "Will create {}x{} grid ({} items)",
                        cols,
                        rows,
                        cols * rows
                    ));
                }
            } else {
                ui.label("Will create single sprite asset");
            }

            // Show target path
            ui.label(format!("Target: {}", sprites_dir.display()));

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if let Err(e) = ui_state.sprite.save_as_asset(sprites_dir) {
                        tracing::error!("Failed to save sprite: {}", e);
                    }
                }
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_save_dialog = false;
                }
            });
        });
}

fn render_warning_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    use crate::ui::editor_ui::WarningAction;

    egui::Window::new("Warning")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(&ui_state.sprite.warning_message);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Confirm").clicked() {
                    // Execute the pending action
                    if let Some(action) = ui_state.sprite.pending_warning_action.take() {
                        match action {
                            WarningAction::ClearCell(cell_idx) => {
                                ui_state.sprite.active_mut().selected_cell = Some(cell_idx);
                                ui_state.sprite.clear_selected_cell();
                            }
                            WarningAction::ChangeCellSize {
                                new_width,
                                new_height,
                            } => {
                                ui_state.sprite.active_mut().cell_size.x = new_width;
                                ui_state.sprite.active_mut().cell_size.y = new_height;
                                ui_state.sprite.active_mut().selected_cell = None;
                            }
                        }
                    }
                    ui_state.sprite.show_warning_dialog = false;
                }
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.pending_warning_action = None;
                    ui_state.sprite.show_warning_dialog = false;
                }
            });
        });
}

fn render_load_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    use crate::ui::editor_ui::SpriteAssetKind;

    egui::Window::new("Load Sprite Asset")
        .collapsible(false)
        .resizable(true)
        .default_size([400.0, 300.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if ui_state.sprite.discovered_assets.is_empty() {
                ui.label("No sprite assets found in project.");
                ui.add_space(8.0);
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_load_dialog = false;
                }
                return;
            }

            ui.label(format!(
                "Found {} sprite assets:",
                ui_state.sprite.discovered_assets.len()
            ));
            ui.separator();

            // Scrollable list of assets
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, asset) in ui_state.sprite.discovered_assets.iter().enumerate() {
                        let is_selected = ui_state.sprite.selected_asset_index == Some(i);
                        let kind_label = match asset.kind {
                            SpriteAssetKind::TileAtlas => "Atlas",
                            SpriteAssetKind::ObjectSheet => "Object",
                        };

                        let label = format!("{} [{}]", asset.name, kind_label);
                        if ui.selectable_label(is_selected, label).clicked() {
                            ui_state.sprite.selected_asset_index = Some(i);
                        }
                    }
                });

            ui.separator();

            // Show selected asset info
            if let Some(idx) = ui_state.sprite.selected_asset_index {
                if let Some(asset) = ui_state.sprite.discovered_assets.get(idx) {
                    ui.label(format!("Selected: {}", asset.name));
                    ui.label(format!("Path: {}", asset.png_path.display()));
                }
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let can_load = ui_state.sprite.selected_asset_index.is_some();
                if ui
                    .add_enabled(can_load, egui::Button::new("Load"))
                    .clicked()
                {
                    if let Some(idx) = ui_state.sprite.selected_asset_index {
                        // Clone the asset to avoid borrow issues
                        let asset = ui_state.sprite.discovered_assets[idx].clone();
                        if let Err(e) = ui_state.sprite.load_sprite_asset(&asset) {
                            tracing::error!("Failed to load sprite: {}", e);
                        }
                    }
                }
                if ui
                    .add_enabled(can_load, egui::Button::new("Rename"))
                    .clicked()
                {
                    if let Some(idx) = ui_state.sprite.selected_asset_index {
                        if let Some(asset) = ui_state.sprite.discovered_assets.get(idx) {
                            ui_state.sprite.rename_new_name = asset.name.clone();
                            ui_state.sprite.show_rename_dialog = true;
                        }
                    }
                }
                if ui
                    .add_enabled(can_load, egui::Button::new("Delete"))
                    .clicked()
                {
                    if let Some(idx) = ui_state.sprite.selected_asset_index {
                        if let Some(asset) = ui_state.sprite.discovered_assets.get(idx) {
                            ui_state.sprite.delete_asset_name = asset.name.clone();
                            ui_state.sprite.show_delete_confirm = true;
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_load_dialog = false;
                }
            });
        });
}

fn render_merge_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    use crate::ui::editor_ui::SpriteAssetKind;

    egui::Window::new("Merge Sprites into Sheet")
        .collapsible(false)
        .resizable(true)
        .default_size([450.0, 350.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if ui_state.sprite.discovered_assets.is_empty() {
                ui.label("No sprite assets found in project.");
                ui.add_space(8.0);
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_merge_dialog = false;
                }
                return;
            }

            ui.label("Select sprites to merge (click to toggle):");
            ui.separator();

            // Build asset display info first to avoid borrow conflicts
            let asset_info: Vec<_> = ui_state
                .sprite
                .discovered_assets
                .iter()
                .enumerate()
                .map(|(i, asset)| {
                    let is_selected = ui_state.sprite.merge_selected_indices.contains(&i);
                    let kind_label = match asset.kind {
                        SpriteAssetKind::TileAtlas => "Atlas",
                        SpriteAssetKind::ObjectSheet => "Object",
                    };
                    (i, format!("{} [{}]", asset.name, kind_label), is_selected)
                })
                .collect();

            // Scrollable list of assets with checkboxes
            let mut toggle_index = None;
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    for (i, label, is_selected) in &asset_info {
                        let mut selected = *is_selected;
                        if ui.checkbox(&mut selected, label.as_str()).changed() {
                            toggle_index = Some(*i);
                        }
                    }
                });

            // Apply toggle after the loop
            if let Some(idx) = toggle_index {
                ui_state.sprite.toggle_merge_selection(idx);
            }

            ui.separator();

            // Show selected count
            let count = ui_state.sprite.merge_selected_indices.len();
            ui.label(format!("Selected: {} sprites", count));

            // Target columns setting
            ui.horizontal(|ui| {
                ui.label("Columns:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.merge_target_cols)
                        .range(1..=16)
                        .speed(1),
                );
            });

            // Show calculated grid
            if count > 0 {
                let cols = ui_state.sprite.merge_target_cols.max(1);
                let rows = (count as u32).div_ceil(cols);
                ui.label(format!("Result: {}x{} grid ({} cells)", cols, rows, count));
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let can_merge = count >= 2;
                if ui
                    .add_enabled(can_merge, egui::Button::new("Merge"))
                    .clicked()
                {
                    if let Err(e) = ui_state.sprite.merge_sprites_into_sheet() {
                        tracing::error!("Failed to merge sprites: {}", e);
                    }
                }
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_merge_dialog = false;
                }
            });

            if count < 2 {
                ui.label("Select at least 2 sprites to merge.");
            }
        });
}

fn render_resize_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    use crate::ui::editor_ui::ResizeAnchor;

    let cell_w = ui_state.sprite.active().cell_size.x.max(1);
    let cell_h = ui_state.sprite.active().cell_size.y.max(1);

    egui::Window::new("Resize Canvas")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Current size info
            if let Some((w, h)) = ui_state.sprite.canvas_dimensions() {
                let tiles_x = w.div_ceil(cell_w);
                let tiles_y = h.div_ceil(cell_h);
                ui.label(format!(
                    "Current: {}x{} tiles ({}x{} px)",
                    tiles_x, tiles_y, w, h
                ));
            }

            ui.separator();

            // New size inputs in tiles
            ui.horizontal(|ui| {
                ui.label("Tiles X:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.resize_tiles_x)
                        .range(1..=128)
                        .speed(1),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Tiles Y:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.resize_tiles_y)
                        .range(1..=128)
                        .speed(1),
                );
            });

            // Show calculated pixel dimensions
            let new_w = ui_state.sprite.resize_tiles_x * cell_w;
            let new_h = ui_state.sprite.resize_tiles_y * cell_h;
            ui.label(format!("Result: {}x{} px", new_w, new_h));

            ui.separator();

            // Anchor grid
            ui.label("Anchor:");
            egui::Grid::new("resize_anchor_grid")
                .spacing([2.0, 2.0])
                .show(ui, |ui| {
                    for (i, anchor) in ResizeAnchor::all().iter().enumerate() {
                        let is_selected = ui_state.sprite.resize_anchor == *anchor;
                        if ui.selectable_label(is_selected, anchor.label()).clicked() {
                            ui_state.sprite.resize_anchor = *anchor;
                        }
                        if (i + 1) % 3 == 0 {
                            ui.end_row();
                        }
                    }
                });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Resize").clicked() {
                    let w = ui_state.sprite.resize_tiles_x * cell_w;
                    let h = ui_state.sprite.resize_tiles_y * cell_h;
                    let anchor = ui_state.sprite.resize_anchor;
                    ui_state.sprite.resize_canvas(w, h, anchor);
                    ui_state.sprite.show_resize_dialog = false;
                }
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_resize_dialog = false;
                }
            });
        });
}

fn render_rename_dialog(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    use crate::ui::editor_ui::SpriteEditorState;

    // Extract asset name before mutable borrow
    let old_name = ui_state
        .sprite
        .selected_asset_index
        .and_then(|idx| ui_state.sprite.discovered_assets.get(idx))
        .map(|a| a.name.clone());

    let Some(old_name) = old_name else {
        ui_state.sprite.show_rename_dialog = false;
        return;
    };

    egui::Window::new("Rename Asset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("Current name: {}", old_name));

            ui.horizontal(|ui| {
                ui.label("New name:");
                ui.text_edit_singleline(&mut ui_state.sprite.rename_new_name);
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let new_name = ui_state.sprite.rename_new_name.clone();
                let can_rename = !new_name.is_empty() && new_name != old_name;

                if ui
                    .add_enabled(can_rename, egui::Button::new("Rename"))
                    .clicked()
                {
                    if let Some(dir) = sprites_dir {
                        match SpriteEditorState::rename_asset(dir, &old_name, &new_name) {
                            Ok(()) => {
                                // Refresh the asset list
                                ui_state.sprite.discovered_assets =
                                    SpriteEditorState::scan_sprite_assets(dir);
                                ui_state.sprite.selected_asset_index = None;
                                ui_state.sprite.show_rename_dialog = false;
                            }
                            Err(e) => {
                                tracing::error!("Failed to rename asset: {}", e);
                            }
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_rename_dialog = false;
                }
            });
        });
}

fn render_delete_confirm_dialog(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    use crate::ui::editor_ui::SpriteEditorState;

    egui::Window::new("Delete Asset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let name = &ui_state.sprite.delete_asset_name;

            ui.label(format!("Are you sure you want to delete \"{}\"?", name));
            ui.label("This will remove the PNG and JSON files.");
            ui.label("This action cannot be undone.");

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    if let Some(dir) = sprites_dir {
                        let name_clone = name.clone();
                        match SpriteEditorState::delete_asset(dir, &name_clone) {
                            Ok(()) => {
                                // Refresh the asset list
                                ui_state.sprite.discovered_assets =
                                    SpriteEditorState::scan_sprite_assets(dir);
                                ui_state.sprite.selected_asset_index = None;
                                ui_state.sprite.show_delete_confirm = false;
                            }
                            Err(e) => {
                                tracing::error!("Failed to delete asset: {}", e);
                            }
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    ui_state.sprite.show_delete_confirm = false;
                }
            });
        });
}

fn render_no_canvas_message(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    sprites_dir: Option<&std::path::Path>,
) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("No canvas open");
            ui.add_space(10.0);
            if ui.button("Create New Canvas").clicked() {
                ui_state.begin_new_sprite_canvas_dialog();
            }
            ui.add_space(5.0);
            let load_enabled = sprites_dir.is_some();
            if ui
                .add_enabled(load_enabled, egui::Button::new("Load Existing Sprite"))
                .clicked()
            {
                if let Some(dir) = sprites_dir {
                    ui_state.sprite.begin_load_dialog(dir);
                }
            }
        });
    });
}

/// Render dual viewports side-by-side (horizontal layout)
fn render_dual_viewports_horizontal(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    let available = ui.available_size();
    let splitter_width = 8.0;
    let usable_width = (available.x - splitter_width).max(200.0);

    // Calculate widths based on split ratio
    let split_ratio = ui_state.sprite.split_ratio.clamp(0.1, 0.9);
    let left_width = (usable_width * split_ratio).max(100.0);
    let right_width = (usable_width * (1.0 - split_ratio)).max(100.0);

    ui.horizontal(|ui| {
        // Left canvas
        ui.vertical(|ui| {
            ui.set_width(left_width);
            render_canvas_panel_header(ui, ui_state, CanvasSide::Left);
            if ui_state.sprite.canvas_state(CanvasSide::Left).canvas.is_some() {
                render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Left));
            } else {
                render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Left);
            }
        });

        // Draggable splitter
        let splitter_response = render_vertical_splitter(ui, available.y);
        if splitter_response.dragged() {
            let delta = splitter_response.drag_delta().x;
            let new_ratio = ui_state.sprite.split_ratio + delta / usable_width;
            ui_state.sprite.split_ratio = new_ratio.clamp(0.1, 0.9);
        }

        // Right canvas
        ui.vertical(|ui| {
            ui.set_width(right_width);
            render_canvas_panel_header(ui, ui_state, CanvasSide::Right);
            if ui_state.sprite.canvas_state(CanvasSide::Right).canvas.is_some() {
                render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Right));
            } else {
                render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Right);
            }
        });
    });
}

/// Render dual viewports stacked (vertical layout)
fn render_dual_viewports_vertical(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    let available = ui.available_size();
    let splitter_height = 8.0;
    let usable_height = (available.y - splitter_height - 48.0).max(200.0); // Reserve space for headers

    // Calculate heights based on split ratio
    let split_ratio = ui_state.sprite.split_ratio.clamp(0.1, 0.9);
    let top_height = (usable_height * split_ratio).max(100.0);
    let bottom_height = (usable_height * (1.0 - split_ratio)).max(100.0);

    // Top canvas
    ui.vertical(|ui| {
        ui.set_height(top_height);
        render_canvas_panel_header(ui, ui_state, CanvasSide::Left);
        if ui_state.sprite.canvas_state(CanvasSide::Left).canvas.is_some() {
            render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Left));
        } else {
            render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Left);
        }
    });

    // Draggable splitter
    let splitter_response = render_horizontal_splitter(ui, available.x);
    if splitter_response.dragged() {
        let delta = splitter_response.drag_delta().y;
        let new_ratio = ui_state.sprite.split_ratio + delta / usable_height;
        ui_state.sprite.split_ratio = new_ratio.clamp(0.1, 0.9);
    }

    // Bottom canvas
    ui.vertical(|ui| {
        ui.set_height(bottom_height);
        render_canvas_panel_header(ui, ui_state, CanvasSide::Right);
        if ui_state.sprite.canvas_state(CanvasSide::Right).canvas.is_some() {
            render_canvas_viewport(ui, ui_state, ctx, Some(CanvasSide::Right));
        } else {
            render_empty_canvas_slot(ui, ui_state, sprites_dir, CanvasSide::Right);
        }
    });
}

/// Render a vertical splitter (for horizontal layout - splits left/right)
fn render_vertical_splitter(ui: &mut egui::Ui, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(8.0, height),
        egui::Sense::click_and_drag(),
    );

    // Change cursor on hover
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    // Draw the splitter
    let painter = ui.painter();
    let color = if response.hovered() || response.dragged() {
        egui::Color32::from_gray(120)
    } else {
        egui::Color32::from_gray(80)
    };

    // Draw a thin line in the center
    let center_x = rect.center().x;
    painter.line_segment(
        [egui::pos2(center_x, rect.top()), egui::pos2(center_x, rect.bottom())],
        egui::Stroke::new(2.0, color),
    );

    response
}

/// Render a horizontal splitter (for vertical layout - splits top/bottom)
fn render_horizontal_splitter(ui: &mut egui::Ui, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, 8.0),
        egui::Sense::click_and_drag(),
    );

    // Change cursor on hover
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }

    // Draw the splitter
    let painter = ui.painter();
    let color = if response.hovered() || response.dragged() {
        egui::Color32::from_gray(120)
    } else {
        egui::Color32::from_gray(80)
    };

    // Draw a thin line in the center
    let center_y = rect.center().y;
    painter.line_segment(
        [egui::pos2(rect.left(), center_y), egui::pos2(rect.right(), center_y)],
        egui::Stroke::new(2.0, color),
    );

    response
}

/// Render a header for a canvas panel showing its side and active state
fn render_canvas_panel_header(ui: &mut egui::Ui, ui_state: &mut EditorUI, side: CanvasSide) {
    let is_active = ui_state.sprite.active_canvas == side;
    let label = side.label();

    ui.horizontal(|ui| {
        if is_active {
            ui.label(egui::RichText::new(format!("● {}", label)).strong());
        } else if ui.button(label).clicked() {
            ui_state.sprite.set_active_canvas(side);
        }

        // Show dirty indicator
        if ui_state.sprite.canvas_state(side).dirty {
            ui.label("*");
        }
    });
}

/// Render an empty canvas slot with options to create/load
fn render_empty_canvas_slot(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    sprites_dir: Option<&std::path::Path>,
    side: CanvasSide,
) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label("Empty");
            if ui.button("New").clicked() {
                ui_state.sprite.set_active_canvas(side);
                ui_state.begin_new_sprite_canvas_dialog();
            }
            let load_enabled = sprites_dir.is_some();
            if ui.add_enabled(load_enabled, egui::Button::new("Load")).clicked() {
                if let Some(dir) = sprites_dir {
                    ui_state.sprite.set_active_canvas(side);
                    ui_state.sprite.begin_load_dialog(dir);
                }
            }
        });
    });
}

fn render_canvas_viewport(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    target_side: Option<CanvasSide>,
) {
    let available_size = ui.available_size();

    // Allocate the viewport area (ensure non-negative dimensions)
    let viewport_height = (available_size.y - 24.0).max(50.0);
    let viewport_width = available_size.x.max(50.0);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(viewport_width, viewport_height),
        egui::Sense::click_and_drag(),
    );

    // Determine which canvas side we're rendering
    let render_side = target_side.unwrap_or(ui_state.sprite.active_canvas);

    // If a target side is specified, set it as active when clicked (not just hovered)
    if let Some(side) = target_side {
        if response.clicked() || response.dragged() {
            ui_state.sprite.set_active_canvas(side);
        }
    }

    // Check if this viewport is the one being interacted with
    let is_interactive = target_side.is_none() || target_side == Some(ui_state.sprite.active_canvas);

    // Handle pan with right-click drag or middle-click drag (only for this canvas)
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        let delta = response.drag_delta();
        ui_state
            .sprite
            .canvas_state_mut(render_side)
            .viewport
            .pan_by(glam::Vec2::new(delta.x, delta.y));
    }

    // Handle scroll zoom (only for this canvas)
    if response.hovered() {
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if scroll_delta > 0.0 {
                ui_state.sprite.canvas_state_mut(render_side).viewport.zoom_in();
            } else {
                ui_state.sprite.canvas_state_mut(render_side).viewport.zoom_out();
            }
        }
    }

    // Handle keyboard shortcuts (only for active canvas, and only once per frame)
    if is_interactive && !ui.ctx().wants_keyboard_input() {
        // Zoom (+/- keys)
        if ui.input(|input| {
            input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals)
        }) {
            ui_state.sprite.active_mut().viewport.zoom_in();
        }
        if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
            ui_state.sprite.active_mut().viewport.zoom_out();
        }

        // Tool shortcuts
        handle_tool_shortcuts(ui_state, ui);

        // Undo/Redo (Ctrl+Z / Ctrl+Y)
        handle_undo_redo_shortcuts(ui_state, ui);
    }

    // Update cursor position for this canvas
    if let Some(hover_pos) = response.hover_pos() {
        let canvas_pos = ui_state
            .sprite
            .canvas_state(render_side)
            .viewport
            .screen_to_canvas(glam::Vec2::new(hover_pos.x, hover_pos.y), rect);
        ui_state.sprite.canvas_state_mut(render_side).cursor_canvas_pos = Some(glam::IVec2::new(
            canvas_pos.x.floor() as i32,
            canvas_pos.y.floor() as i32,
        ));
    } else {
        ui_state.sprite.canvas_state_mut(render_side).cursor_canvas_pos = None;
    }

    // Handle tool interactions (only for active canvas)
    if is_interactive {
        handle_tool_interaction(ui_state, &response, rect, ctx);
    }

    // Draw canvas background
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::from_gray(40));

    // Get references to this canvas's state for drawing
    let canvas_state = ui_state.sprite.canvas_state(render_side);

    // Ensure canvas texture is created before drawing
    if canvas_state.canvas.is_some() {
        ensure_canvas_texture_for_side(ui_state, ctx, render_side);
    }

    // Draw checkerboard transparency pattern and canvas
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if let Some(canvas) = &canvas_state.canvas {
        let viewport = canvas_state.viewport.clone();
        let texture = canvas_state.canvas_texture.as_ref();
        draw_canvas_with_checkerboard(&painter, rect, &viewport, canvas, texture);
    }

    // Draw pixel grid overlay
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if canvas_state.show_grid && canvas_state.viewport.zoom >= 4.0 {
        if let Some(canvas) = &canvas_state.canvas {
            draw_pixel_grid(&painter, rect, &canvas_state.viewport, canvas);
        }
    }

    // Draw cell grid overlay for sprite sheets
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if canvas_state.show_cell_grid {
        if let Some(canvas) = &canvas_state.canvas {
            draw_cell_grid(
                &painter,
                rect,
                &canvas_state.viewport,
                canvas,
                canvas_state.cell_size,
                canvas_state.selected_cell,
            );
        }
    }

    // Draw selection rectangle
    let canvas_state = ui_state.sprite.canvas_state(render_side);
    if let Some(selection) = &canvas_state.selection {
        draw_selection_rect(&painter, rect, &canvas_state.viewport, selection);
    }

    // Status bar (only in single mode or when this is the active canvas)
    if target_side.is_none() || target_side == Some(ui_state.sprite.active_canvas) {
        render_status_bar(ui, ui_state);
    }
}

fn draw_canvas_with_checkerboard(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
    texture: Option<&egui::TextureHandle>,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    // Calculate canvas screen rect
    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));
    let canvas_screen_max = egui::pos2(
        canvas_screen_min.x + canvas.width as f32 * zoom,
        canvas_screen_min.y + canvas.height as f32 * zoom,
    );
    let canvas_screen_rect = egui::Rect::from_min_max(canvas_screen_min, canvas_screen_max);

    // Clip to viewport
    let visible_rect = canvas_screen_rect.intersect(rect);
    if visible_rect.is_positive() {
        // Draw checkerboard pattern for transparency
        draw_checkerboard(painter, rect, visible_rect, viewport, canvas);

        // Draw canvas texture
        if let Some(tex) = texture {
            painter.image(
                tex.id(),
                canvas_screen_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }

    // Draw canvas border
    painter.rect_stroke(
        canvas_screen_rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
        egui::StrokeKind::Outside,
    );
}

fn draw_checkerboard(
    painter: &egui::Painter,
    viewport_rect: egui::Rect,
    visible_rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    // Each checkerboard square = 1 pixel, aligned with the pixel grid
    let pixel_size = zoom;
    let color1 = egui::Color32::from_gray(180);
    let color2 = egui::Color32::from_gray(220);

    // Calculate where pixel (0,0) appears on screen (using original viewport rect, not clipped)
    let canvas_screen_min = egui::pos2(
        viewport_rect.left() + (-pan.x * zoom),
        viewport_rect.top() + (-pan.y * zoom),
    );

    // Find the range of visible pixels (use visible_rect for bounds)
    let first_visible_x = ((visible_rect.left() - canvas_screen_min.x) / pixel_size).floor() as i32;
    let first_visible_y = ((visible_rect.top() - canvas_screen_min.y) / pixel_size).floor() as i32;
    let last_visible_x = ((visible_rect.right() - canvas_screen_min.x) / pixel_size).ceil() as i32;
    let last_visible_y = ((visible_rect.bottom() - canvas_screen_min.y) / pixel_size).ceil() as i32;

    // Clamp to canvas bounds
    let start_x = first_visible_x.max(0) as u32;
    let start_y = first_visible_y.max(0) as u32;
    let end_x = (last_visible_x as u32).min(canvas.width);
    let end_y = (last_visible_y as u32).min(canvas.height);

    for py in start_y..end_y {
        for px in start_x..end_x {
            let color = if (px + py) % 2 == 0 { color1 } else { color2 };
            let screen_x = canvas_screen_min.x + px as f32 * pixel_size;
            let screen_y = canvas_screen_min.y + py as f32 * pixel_size;
            let check_rect = egui::Rect::from_min_size(
                egui::pos2(screen_x, screen_y),
                egui::vec2(pixel_size, pixel_size),
            );
            // Clip to the visible rect
            let clipped = check_rect.intersect(visible_rect);
            if clipped.width() > 0.0 && clipped.height() > 0.0 {
                painter.rect_filled(clipped, 0.0, color);
            }
        }
    }
}

fn ensure_canvas_texture_for_side(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    side: CanvasSide,
) {
    // Check if we already have a valid texture for this side
    if ui_state.sprite.canvas_state(side).canvas_texture.is_some() {
        return;
    }

    let Some(canvas) = &ui_state.sprite.canvas_state(side).canvas else {
        return;
    };

    // Create texture from canvas pixels
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [canvas.width as usize, canvas.height as usize],
        canvas.pixels(),
    );

    let texture_name = match side {
        CanvasSide::Left => "sprite_editor_canvas_left",
        CanvasSide::Right => "sprite_editor_canvas_right",
    };

    let texture = ctx.load_texture(texture_name, color_image, egui::TextureOptions::NEAREST);

    ui_state.sprite.canvas_state_mut(side).canvas_texture = Some(texture);
}

fn draw_pixel_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));

    // Use a dark gray that contrasts with the light checkerboard squares
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(80, 80, 80, 180));

    // Vertical lines
    for x in 0..=canvas.width {
        let screen_x = canvas_screen_min.x + x as f32 * zoom;
        if screen_x >= rect.left() && screen_x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(screen_x, rect.top().max(canvas_screen_min.y)),
                    egui::pos2(
                        screen_x,
                        rect.bottom()
                            .min(canvas_screen_min.y + canvas.height as f32 * zoom),
                    ),
                ],
                stroke,
            );
        }
    }

    // Horizontal lines
    for y in 0..=canvas.height {
        let screen_y = canvas_screen_min.y + y as f32 * zoom;
        if screen_y >= rect.top() && screen_y <= rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(rect.left().max(canvas_screen_min.x), screen_y),
                    egui::pos2(
                        rect.right()
                            .min(canvas_screen_min.x + canvas.width as f32 * zoom),
                        screen_y,
                    ),
                ],
                stroke,
            );
        }
    }
}

fn draw_cell_grid(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    canvas: &SpriteCanvas,
    cell_size: glam::UVec2,
    selected_cell: Option<usize>,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));

    let stroke = egui::Stroke::new(
        2.0,
        egui::Color32::from_rgba_unmultiplied(255, 200, 50, 180),
    );

    let cols = canvas.width / cell_size.x.max(1);
    let rows = canvas.height / cell_size.y.max(1);

    // Vertical cell lines
    for x in 0..=cols {
        let pixel_x = x * cell_size.x;
        let screen_x = canvas_screen_min.x + pixel_x as f32 * zoom;
        if screen_x >= rect.left() && screen_x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(screen_x, rect.top().max(canvas_screen_min.y)),
                    egui::pos2(
                        screen_x,
                        rect.bottom()
                            .min(canvas_screen_min.y + canvas.height as f32 * zoom),
                    ),
                ],
                stroke,
            );
        }
    }

    // Horizontal cell lines
    for y in 0..=rows {
        let pixel_y = y * cell_size.y;
        let screen_y = canvas_screen_min.y + pixel_y as f32 * zoom;
        if screen_y >= rect.top() && screen_y <= rect.bottom() {
            painter.line_segment(
                [
                    egui::pos2(rect.left().max(canvas_screen_min.x), screen_y),
                    egui::pos2(
                        rect.right()
                            .min(canvas_screen_min.x + canvas.width as f32 * zoom),
                        screen_y,
                    ),
                ],
                stroke,
            );
        }
    }

    // Highlight selected cell
    if let Some(cell_idx) = selected_cell {
        let col = cell_idx as u32 % cols;
        let row = cell_idx as u32 / cols;
        if row < rows {
            let cell_min = egui::pos2(
                canvas_screen_min.x + (col * cell_size.x) as f32 * zoom,
                canvas_screen_min.y + (row * cell_size.y) as f32 * zoom,
            );
            let cell_max = egui::pos2(
                cell_min.x + cell_size.x as f32 * zoom,
                cell_min.y + cell_size.y as f32 * zoom,
            );
            let cell_rect = egui::Rect::from_min_max(cell_min, cell_max);

            let fill = egui::Color32::from_rgba_unmultiplied(255, 200, 50, 40);
            let highlight_stroke = egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 200, 50));

            painter.rect_filled(cell_rect, 0.0, fill);
            painter.rect_stroke(cell_rect, 0.0, highlight_stroke, egui::StrokeKind::Inside);
        }
    }
}

fn draw_selection_rect(
    painter: &egui::Painter,
    rect: egui::Rect,
    viewport: &SpriteCanvasViewport,
    selection: &crate::ui::editor_ui::SpriteSelection,
) {
    let zoom = viewport.zoom;
    let pan = viewport.pan;

    let canvas_screen_min = egui::pos2(rect.left() + (-pan.x * zoom), rect.top() + (-pan.y * zoom));

    // Calculate selection screen rect
    let sel_min = egui::pos2(
        canvas_screen_min.x + selection.x as f32 * zoom,
        canvas_screen_min.y + selection.y as f32 * zoom,
    );
    let sel_max = egui::pos2(
        sel_min.x + selection.width as f32 * zoom,
        sel_min.y + selection.height as f32 * zoom,
    );
    let sel_rect = egui::Rect::from_min_max(sel_min, sel_max);

    // Draw selection with dashed border and semi-transparent fill
    let fill = egui::Color32::from_rgba_unmultiplied(100, 150, 255, 50);
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 150, 255));

    painter.rect_filled(sel_rect, 0.0, fill);
    painter.rect_stroke(sel_rect, 0.0, stroke, egui::StrokeKind::Outside);
}

fn render_status_bar(ui: &mut egui::Ui, ui_state: &EditorUI) {
    ui.horizontal(|ui| {
        // Cursor position
        if let Some(pos) = ui_state.sprite.active().cursor_canvas_pos {
            ui.label(format!("Cursor: {}, {}", pos.x, pos.y));
        } else {
            ui.label("Cursor: -, -");
        }

        ui.separator();

        // Canvas dimensions
        if let Some((w, h)) = ui_state.sprite.canvas_dimensions() {
            ui.label(format!("Canvas: {}x{}", w, h));
        }

        ui.separator();

        // Zoom level
        ui.label(format!("Zoom: {}x", ui_state.sprite.active().viewport.zoom as i32));

        ui.separator();

        // Dirty indicator
        if ui_state.sprite.active().dirty {
            ui.label("*Modified");
        }
    });
}

fn handle_tool_interaction(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    _rect: egui::Rect,
    _ctx: &egui::Context,
) {
    use crate::ui::editor_ui::SpriteEditorTool;

    let Some(canvas_pos) = ui_state.sprite.active().cursor_canvas_pos else {
        return;
    };

    match ui_state.sprite.tool {
        SpriteEditorTool::Drag => handle_drag_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Brush => handle_brush_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Eraser => handle_eraser_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Fill => handle_fill_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Eyedropper => handle_eyedropper_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Line => handle_line_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::Select => handle_select_tool(ui_state, response, canvas_pos),
        SpriteEditorTool::MagicWand => handle_magic_wand_tool(ui_state, response, canvas_pos),
    }
}

fn handle_drag_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    // Click to select cell in sheet mode
    if response.clicked() && ui_state.sprite.is_sheet() && canvas_pos.x >= 0 && canvas_pos.y >= 0 {
        let cell = ui_state
            .sprite
            .cell_at_position(canvas_pos.x as u32, canvas_pos.y as u32);
        ui_state.sprite.active_mut().selected_cell = cell;
    }

    // Primary drag for panning (same as secondary/middle)
    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        ui_state
            .sprite
            .active_mut()
            .viewport
            .pan_by(glam::Vec2::new(delta.x, delta.y));
    }
}

fn handle_brush_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.drag_started_by(egui::PointerButton::Primary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        let color = ui_state.sprite.foreground_color;
        let brush_size = ui_state.sprite.brush_size;
        if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
            if SpritePaintInteraction::paint_brush(canvas, canvas_pos, color, brush_size) {
                ui_state.sprite.active_mut().dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        finish_paint_stroke(ui_state);
    }
}

fn handle_eraser_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.drag_started_by(egui::PointerButton::Primary) {
        start_paint_stroke(ui_state);
    }

    if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
        let brush_size = ui_state.sprite.brush_size;
        if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
            if SpritePaintInteraction::erase_brush(canvas, canvas_pos, brush_size) {
                ui_state.sprite.active_mut().dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        finish_paint_stroke(ui_state);
    }
}

fn handle_fill_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.clicked() {
        start_paint_stroke(ui_state);
        let color = ui_state.sprite.foreground_color;
        if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
            if SpritePaintInteraction::flood_fill(canvas, canvas_pos, color) {
                ui_state.sprite.active_mut().dirty = true;
                invalidate_canvas_texture(ui_state);
            }
        }
        finish_paint_stroke(ui_state);
    }
}

fn handle_eyedropper_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.clicked() {
        if let Some(canvas) = &ui_state.sprite.active().canvas {
            if let Some(color) = SpritePaintInteraction::pick_color(canvas, canvas_pos) {
                ui_state.sprite.foreground_color = color;
                ui_state.sprite.add_recent_color(color);
            }
        }
    }
}

fn handle_line_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    use crate::ui::interactions::SpritePaintInteraction;

    if response.drag_started_by(egui::PointerButton::Primary) {
        ui_state.sprite.active_mut().line_start_pos = Some(canvas_pos);
        start_paint_stroke(ui_state);
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        let color = ui_state.sprite.foreground_color;
        let brush_size = ui_state.sprite.brush_size;
        if let Some(start) = ui_state.sprite.active_mut().line_start_pos.take() {
            if let Some(canvas) = &mut ui_state.sprite.active_mut().canvas {
                if SpritePaintInteraction::draw_line(canvas, start, canvas_pos, color, brush_size) {
                    ui_state.sprite.active_mut().dirty = true;
                    invalidate_canvas_texture(ui_state);
                }
            }
        }
        finish_paint_stroke(ui_state);
    }
}

fn handle_select_tool(ui_state: &mut EditorUI, response: &egui::Response, canvas_pos: glam::IVec2) {
    if response.drag_started_by(egui::PointerButton::Primary) {
        tracing::info!("Select tool: drag started at {:?}", canvas_pos);
        ui_state.sprite.active_mut().selection_start_pos = Some(canvas_pos);
        ui_state.sprite.active_mut().selection = None;
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.active().selection_start_pos {
            ui_state.sprite.active_mut().selection = Some(create_selection(start, canvas_pos));
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if let Some(start) = ui_state.sprite.active_mut().selection_start_pos.take() {
            let selection = create_selection(start, canvas_pos);
            // Only keep selection if it has non-zero size
            if selection.width > 0 && selection.height > 0 {
                tracing::info!(
                    "Select tool: created selection x={}, y={}, w={}, h={}",
                    selection.x,
                    selection.y,
                    selection.width,
                    selection.height
                );
                ui_state.sprite.active_mut().selection = Some(selection);
            } else {
                tracing::info!("Select tool: selection too small, discarded");
                ui_state.sprite.active_mut().selection = None;
            }
        }
    }

    // Clear selection with right-click
    if response.clicked_by(egui::PointerButton::Secondary) {
        tracing::info!("Select tool: selection cleared by right-click");
        ui_state.sprite.active_mut().selection = None;
    }
}

fn handle_magic_wand_tool(
    ui_state: &mut EditorUI,
    response: &egui::Response,
    canvas_pos: glam::IVec2,
) {
    use crate::ui::editor_ui::SpriteSelection;

    if response.clicked() && canvas_pos.x >= 0 && canvas_pos.y >= 0 {
        if let Some(canvas) = &ui_state.sprite.active().canvas {
            let x = canvas_pos.x as u32;
            let y = canvas_pos.y as u32;

            if let Some((sel_x, sel_y, sel_w, sel_h)) = canvas.find_connected_sprite(x, y) {
                tracing::info!(
                    "Magic wand: selected sprite at ({}, {}) with size {}x{}",
                    sel_x,
                    sel_y,
                    sel_w,
                    sel_h
                );
                ui_state.sprite.active_mut().selection =
                    Some(SpriteSelection::new(sel_x, sel_y, sel_w, sel_h));
            } else {
                tracing::info!("Magic wand: clicked on transparent pixel, clearing selection");
                ui_state.sprite.active_mut().selection = None;
            }
        }
    }

    // Clear selection with right-click
    if response.clicked_by(egui::PointerButton::Secondary) {
        ui_state.sprite.active_mut().selection = None;
    }
}

fn create_selection(start: glam::IVec2, end: glam::IVec2) -> crate::ui::editor_ui::SpriteSelection {
    let x = start.x.min(end.x).max(0) as u32;
    let y = start.y.min(end.y).max(0) as u32;
    let w = (start.x - end.x).unsigned_abs();
    let h = (start.y - end.y).unsigned_abs();
    crate::ui::editor_ui::SpriteSelection::new(x, y, w, h)
}

fn start_paint_stroke(ui_state: &mut EditorUI) {
    if !ui_state.sprite.active().is_painting {
        ui_state.sprite.active_mut().is_painting = true;
        ui_state.sprite.active_mut().canvas_before_stroke =
            ui_state.sprite.active().canvas.clone();
    }
}

fn finish_paint_stroke(ui_state: &mut EditorUI) {
    if ui_state.sprite.active().is_painting {
        ui_state.sprite.active_mut().is_painting = false;
        if let Some(before) = ui_state.sprite.active_mut().canvas_before_stroke.take() {
            ui_state.sprite.push_undo_state(before);
        }
        // Add the used color to recent colors
        ui_state
            .sprite
            .add_recent_color(ui_state.sprite.foreground_color);
    }
}

fn invalidate_canvas_texture(ui_state: &mut EditorUI) {
    ui_state.sprite.active_mut().canvas_texture = None;
}

fn handle_tool_shortcuts(ui_state: &mut EditorUI, ui: &egui::Ui) {
    use SpriteEditorTool::*;

    // Tool shortcuts: B=Brush, E=Eraser, G=Fill, I=Eyedropper, M=Select, D=Drag, L=Line, W=MagicWand
    if ui.input(|i| i.key_pressed(egui::Key::B)) {
        ui_state.sprite.tool = Brush;
    }
    if ui.input(|i| i.key_pressed(egui::Key::E)) {
        ui_state.sprite.tool = Eraser;
    }
    if ui.input(|i| i.key_pressed(egui::Key::G)) {
        ui_state.sprite.tool = Fill;
    }
    if ui.input(|i| i.key_pressed(egui::Key::I)) {
        ui_state.sprite.tool = Eyedropper;
    }
    if ui.input(|i| i.key_pressed(egui::Key::M)) {
        ui_state.sprite.tool = Select;
    }
    if ui.input(|i| i.key_pressed(egui::Key::D)) {
        ui_state.sprite.tool = Drag;
    }
    if ui.input(|i| i.key_pressed(egui::Key::L)) {
        ui_state.sprite.tool = Line;
    }
    if ui.input(|i| i.key_pressed(egui::Key::W)) {
        ui_state.sprite.tool = MagicWand;
    }

    // Brush size: [ and ] to decrease/increase
    if ui.input(|i| i.key_pressed(egui::Key::OpenBracket)) {
        ui_state.sprite.brush_size = ui_state.sprite.brush_size.saturating_sub(1).max(1);
    }
    if ui.input(|i| i.key_pressed(egui::Key::CloseBracket)) {
        ui_state.sprite.brush_size = (ui_state.sprite.brush_size + 1).min(32);
    }
}

fn handle_undo_redo_shortcuts(ui_state: &mut EditorUI, ui: &egui::Ui) {
    let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.mac_cmd);
    let shift = ui.input(|i| i.modifiers.shift);

    // Ctrl+Z for undo (without shift)
    if ctrl && !shift && ui.input(|i| i.key_pressed(egui::Key::Z)) && ui_state.sprite.undo() {
        invalidate_canvas_texture(ui_state);
    }

    // Ctrl+Y or Ctrl+Shift+Z for redo
    let redo_pressed = ui.input(|i| i.key_pressed(egui::Key::Y))
        || (shift && ui.input(|i| i.key_pressed(egui::Key::Z)));
    if ctrl && redo_pressed && ui_state.sprite.redo() {
        invalidate_canvas_texture(ui_state);
    }
}

fn handle_copy_paste_shortcuts(ui_state: &mut EditorUI, ctx: &egui::Context) {
    // Use raw event inspection to avoid egui's built-in clipboard handling consuming events
    let (ctrl, c_pressed, v_pressed) = ctx.input(|i| {
        let ctrl = i.modifiers.ctrl || i.modifiers.mac_cmd;
        let mut c_pressed = false;
        let mut v_pressed = false;

        // Check raw events to bypass consumption
        for event in &i.events {
            if let egui::Event::Key {
                key,
                pressed,
                modifiers,
                ..
            } = event
            {
                if *pressed && (modifiers.ctrl || modifiers.mac_cmd) {
                    match key {
                        egui::Key::C => c_pressed = true,
                        egui::Key::V => v_pressed = true,
                        _ => {}
                    }
                }
            }
        }

        (ctrl, c_pressed, v_pressed)
    });

    // Log key state for debugging
    if c_pressed || v_pressed {
        tracing::info!(
            "Key pressed (via events): C={}, V={}, Ctrl={}",
            c_pressed,
            v_pressed,
            ctrl
        );
    }

    // Ctrl+C for copy (from active canvas selection)
    if ctrl && c_pressed {
        let has_selection = ui_state.sprite.active().selection.is_some();
        let has_canvas = ui_state.sprite.active().canvas.is_some();
        tracing::info!(
            "Copy attempt: has_selection={}, has_canvas={}",
            has_selection,
            has_canvas
        );

        if let Some(sel) = &ui_state.sprite.active().selection {
            tracing::info!(
                "Selection: x={}, y={}, w={}, h={}",
                sel.x,
                sel.y,
                sel.width,
                sel.height
            );
        }

        if ui_state.sprite.copy_selection() {
            tracing::info!("Copy successful - clipboard has content");
        } else {
            tracing::warn!("Copy failed - no selection or no canvas");
        }
    }

    // Ctrl+V for paste (at cursor position on hovered canvas)
    if ctrl && v_pressed {
        let has_clipboard = ui_state.sprite.clipboard.is_some();
        let hovered = find_hovered_canvas(ui_state);
        let paste_side = hovered.unwrap_or(ui_state.sprite.active_canvas);
        let cursor_pos = ui_state.sprite.canvas_state(paste_side).cursor_canvas_pos;
        let has_canvas = ui_state.sprite.canvas_state(paste_side).canvas.is_some();

        tracing::info!(
            "Paste attempt: has_clipboard={}, hovered={:?}, paste_side={:?}, cursor_pos={:?}, has_canvas={}",
            has_clipboard,
            hovered,
            paste_side,
            cursor_pos,
            has_canvas
        );

        // If no cursor position on the target canvas, use (0, 0) as fallback
        if cursor_pos.is_none() {
            tracing::info!("No cursor position, using (0, 0) fallback");
            ui_state.sprite.canvas_state_mut(paste_side).cursor_canvas_pos =
                Some(glam::IVec2::new(0, 0));
        }

        if ui_state.sprite.paste_at_cursor(paste_side) {
            invalidate_canvas_texture_for_side(ui_state, paste_side);
            tracing::info!("Paste successful to {:?}", paste_side);
        } else {
            tracing::warn!("Paste failed - check clipboard and canvas state");
        }
    }
}

/// Find which canvas the cursor is currently hovering over
fn find_hovered_canvas(ui_state: &EditorUI) -> Option<CanvasSide> {
    // Check left canvas first, then right
    if ui_state
        .sprite
        .canvas_state(CanvasSide::Left)
        .cursor_canvas_pos
        .is_some()
    {
        return Some(CanvasSide::Left);
    }
    if ui_state
        .sprite
        .canvas_state(CanvasSide::Right)
        .cursor_canvas_pos
        .is_some()
    {
        return Some(CanvasSide::Right);
    }
    None
}

fn invalidate_canvas_texture_for_side(ui_state: &mut EditorUI, side: CanvasSide) {
    ui_state.sprite.canvas_state_mut(side).canvas_texture = None;
}
