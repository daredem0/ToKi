//! Sprite editor dialog rendering.

use crate::ui::editor_ui::{ResizeAnchor, SpriteAssetKind, SpriteEditorState, WarningAction};
use crate::ui::EditorUI;

/// Render all active dialogs.
pub fn render_dialogs(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    if ui_state.sprite.show_new_canvas_dialog {
        render_new_canvas_dialog(ui_state, ctx);
    }
    if ui_state.sprite.show_save_dialog {
        render_save_dialog(ui_state, ctx, sprites_dir);
    }
    if ui_state.sprite.show_load_dialog {
        render_load_dialog(ui_state, ctx);
    }
    if ui_state.sprite.show_merge_dialog {
        render_merge_dialog(ui_state, ctx);
    }
    if ui_state.sprite.show_resize_dialog {
        render_resize_dialog(ui_state, ctx);
    }
    if ui_state.sprite.show_rename_dialog {
        render_rename_dialog(ui_state, ctx, sprites_dir);
    }
    if ui_state.sprite.show_delete_confirm {
        render_delete_confirm_dialog(ui_state, ctx, sprites_dir);
    }
    if ui_state.sprite.show_warning_dialog {
        render_warning_dialog(ui_state, ctx);
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
    egui::Window::new("Save Sprite Asset")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
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
    egui::Window::new("Warning")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(&ui_state.sprite.warning_message);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Confirm").clicked() {
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

            if let Some(idx) = toggle_index {
                ui_state.sprite.toggle_merge_selection(idx);
            }

            ui.separator();

            let count = ui_state.sprite.merge_selected_indices.len();
            ui.label(format!("Selected: {} sprites", count));

            ui.horizontal(|ui| {
                ui.label("Columns:");
                ui.add(
                    egui::DragValue::new(&mut ui_state.sprite.merge_target_cols)
                        .range(1..=16)
                        .speed(1),
                );
            });

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
    let cell_w = ui_state.sprite.active().cell_size.x.max(1);
    let cell_h = ui_state.sprite.active().cell_size.y.max(1);

    egui::Window::new("Resize Canvas")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some((w, h)) = ui_state.sprite.canvas_dimensions() {
                let tiles_x = w.div_ceil(cell_w);
                let tiles_y = h.div_ceil(cell_h);
                ui.label(format!(
                    "Current: {}x{} tiles ({}x{} px)",
                    tiles_x, tiles_y, w, h
                ));
            }

            ui.separator();

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

            let new_w = ui_state.sprite.resize_tiles_x * cell_w;
            let new_h = ui_state.sprite.resize_tiles_y * cell_h;
            ui.label(format!("Result: {}x{} px", new_w, new_h));

            ui.separator();

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
