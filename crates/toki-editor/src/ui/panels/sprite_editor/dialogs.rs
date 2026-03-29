//! Sprite editor dialog rendering.

use crate::ui::editor_ui::{ResizeAnchor, SpriteAssetKind, SpriteEditorState};
use crate::ui::EditorUI;

/// Render all active dialogs.
pub fn render_dialogs(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    sprites_dir: Option<&std::path::Path>,
) {
    if crate::ui::editor_context::sprite_state_mut(ui_state).show_new_canvas_dialog {
        render_new_canvas_dialog(ui_state, ctx);
    }
    if crate::ui::editor_context::sprite_state_mut(ui_state).show_save_dialog {
        render_save_dialog(ui_state, ctx, sprites_dir);
    }
    if crate::ui::editor_context::sprite_state_mut(ui_state).show_load_dialog {
        render_load_dialog(ui_state, ctx);
    }
    if crate::ui::editor_context::sprite_state_mut(ui_state).show_merge_dialog {
        render_merge_dialog(ui_state, ctx);
    }
    if crate::ui::editor_context::sprite_state_mut(ui_state).show_resize_dialog {
        render_resize_dialog(ui_state, ctx);
    }
    if crate::ui::editor_context::sprite_state_mut(ui_state).show_rename_dialog {
        render_rename_dialog(ui_state, ctx, sprites_dir);
    }
    if crate::ui::editor_context::sprite_state_mut(ui_state).show_delete_confirm {
        render_delete_confirm_dialog(ui_state, ctx, sprites_dir);
    }
}

fn render_new_canvas_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    let source_image = crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image.clone();
    let source_image_size = crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image_size;
    let source_image_validation = source_image_size.map(|size| {
        let sprite_w = crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_width.max(1);
        let sprite_h = crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_height.max(1);
        if size.x % sprite_w == 0 && size.y % sprite_h == 0 {
            Ok((size.x / sprite_w, size.y / sprite_h))
        } else {
            Err(format!(
                "Image size {}x{} does not divide evenly by configured tile size {}x{}.",
                size.x, size.y, sprite_w, sprite_h
            ))
        }
    });

    egui::Window::new("New Canvas")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some(path) = source_image.as_ref() {
                ui.label(format!("Source Image: {}", path.display()));
                if let Some(size) = source_image_size {
                    ui.label(format!("Image Size: {}x{}", size.x, size.y));
                }
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                ui.label("Sprite Width:");
                ui.add(
                    egui::DragValue::new(&mut crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_width)
                        .range(1..=512)
                        .speed(1),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Sprite Height:");
                ui.add(
                    egui::DragValue::new(&mut crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_height)
                        .range(1..=512)
                        .speed(1),
                );
            });

            ui.separator();

            if source_image.is_some() {
                crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_is_sheet = true;
                let mut forced_sheet = true;
                ui.add_enabled(
                    false,
                    egui::Checkbox::new(&mut forced_sheet, "Create as sprite sheet"),
                );
            } else {
                ui.checkbox(
                    &mut crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_is_sheet,
                    "Create as sprite sheet",
                );
            }

            if crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_is_sheet {
                if source_image.is_some() {
                    match source_image_validation.as_ref() {
                        Some(Ok((cols, rows))) => {
                            ui.label(format!(
                                "Canvas: {}x{} ({}x{} tiles, {} cells)",
                                source_image_size.unwrap().x,
                                source_image_size.unwrap().y,
                                cols,
                                rows,
                                cols * rows
                            ));
                        }
                        Some(Err(error)) => {
                            ui.colored_label(egui::Color32::RED, error);
                        }
                        None => {}
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Columns:");
                        ui.add(
                            egui::DragValue::new(&mut crate::ui::editor_context::sprite_state_mut(ui_state).new_sheet_cols)
                                .range(1..=64)
                                .speed(1),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rows:");
                        ui.add(
                            egui::DragValue::new(&mut crate::ui::editor_context::sprite_state_mut(ui_state).new_sheet_rows)
                                .range(1..=64)
                                .speed(1),
                        );
                    });

                    // Show calculated canvas size
                    let cols = crate::ui::editor_context::sprite_state_mut(ui_state).new_sheet_cols;
                    let rows = crate::ui::editor_context::sprite_state_mut(ui_state).new_sheet_rows;
                    let canvas_w = crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_width * cols;
                    let canvas_h = crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_height * rows;
                    ui.label(format!(
                        "Canvas: {}x{} ({} cells)",
                        canvas_w,
                        canvas_h,
                        cols * rows
                    ));
                }
            }

            ui.separator();

            if let Some(error) = crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_error.as_ref() {
                ui.colored_label(egui::Color32::RED, error);
                ui.separator();
            }

            ui.horizontal(|ui| {
                let create_enabled = !matches!(source_image_validation.as_ref(), Some(Err(_)));
                if ui
                    .add_enabled(create_enabled, egui::Button::new("Create"))
                    .clicked()
                {
                    submit_new_canvas(ui_state);
                }
                if ui.button("Cancel").clicked() {
                    ui_state.cancel_new_sprite_canvas_dialog();
                }
            });
        });
}

fn submit_new_canvas(ui_state: &mut EditorUI) {
    let sprite_w = crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_width.max(1);
    let sprite_h = crate::ui::editor_context::sprite_state_mut(ui_state).new_sprite_height.max(1);
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_error = None;

    if let Some(path) = crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image.clone() {
        match ui_state
            .sprite_editor_context_mut()
            .sprite
            .import_external_image_as_sheet(&path, sprite_w, sprite_h)
        {
            Ok(()) => {
                crate::ui::editor_context::sprite_state_mut(ui_state).show_new_canvas_dialog = false;
                crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image = None;
                crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image_size = None;
            }
            Err(error) => {
                crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_error = Some(error);
            }
        }
        return;
    }

    if crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_is_sheet {
        let cols = crate::ui::editor_context::sprite_state_mut(ui_state).new_sheet_cols.max(1);
        let rows = crate::ui::editor_context::sprite_state_mut(ui_state).new_sheet_rows.max(1);
        let canvas_w = sprite_w * cols;
        let canvas_h = sprite_h * rows;
        ui_state
            .sprite_editor_context_mut()
            .sprite
            .new_sheet(canvas_w, canvas_h, sprite_w, sprite_h);
    } else {
        crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas(sprite_w, sprite_h);
        crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().show_cell_grid = false;
    }
    crate::ui::editor_context::sprite_state_mut(ui_state).show_new_canvas_dialog = false;
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image = None;
    crate::ui::editor_context::sprite_state_mut(ui_state).new_canvas_source_image_size = None;
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
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_save_dialog = false;
                }
                return;
            };

            ui.horizontal(|ui| {
                ui.label("Asset Name:");
                ui.text_edit_singleline(&mut crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().save_asset_name);
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Save as:");
                ui.selectable_value(
                    &mut crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().save_asset_kind,
                    SpriteAssetKind::ObjectSheet,
                    "Object Sheet",
                );
                ui.selectable_value(
                    &mut crate::ui::editor_context::sprite_state_mut(ui_state).active_mut().save_asset_kind,
                    SpriteAssetKind::TileAtlas,
                    "Tile Atlas",
                );
            });

            ui.add_space(4.0);
            if crate::ui::editor_context::sprite_state_mut(ui_state).is_sheet() {
                if let Some((cols, rows)) = crate::ui::editor_context::sprite_state_mut(ui_state).sheet_cell_count() {
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
                    if let Err(e) = crate::ui::editor_context::sprite_state_mut(ui_state).save_as_asset(sprites_dir) {
                        tracing::error!("Failed to save sprite: {}", e);
                    }
                }
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_save_dialog = false;
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
            if crate::ui::editor_context::sprite_state(ui_state)
                .discovered_assets
                .is_empty()
            {
                ui.label("No sprite assets found in project.");
                ui.add_space(8.0);
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_load_dialog = false;
                }
                return;
            }

            ui.label(format!(
                "Found {} sprite assets:",
                crate::ui::editor_context::sprite_state(ui_state)
                    .discovered_assets
                    .len()
            ));
            ui.separator();

            let discovered_assets = crate::ui::editor_context::sprite_state(ui_state)
                .discovered_assets
                .clone();
            let selected_asset_index = crate::ui::editor_context::sprite_state(ui_state).selected_asset_index;
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, asset) in discovered_assets.iter().enumerate() {
                        let is_selected = selected_asset_index == Some(i);
                        let kind_label = match asset.kind {
                            SpriteAssetKind::TileAtlas => "Atlas",
                            SpriteAssetKind::ObjectSheet => "Object",
                        };

                        let label = format!("{} [{}]", asset.name, kind_label);
                        if ui.selectable_label(is_selected, label).clicked() {
                            crate::ui::editor_context::sprite_state_mut(ui_state).selected_asset_index = Some(i);
                        }
                    }
                });

            ui.separator();

            if let Some(idx) = crate::ui::editor_context::sprite_state(ui_state).selected_asset_index {
                if let Some(asset) = crate::ui::editor_context::sprite_state(ui_state).discovered_assets.get(idx) {
                    ui.label(format!("Selected: {}", asset.name));
                    ui.label(format!("Path: {}", asset.png_path.display()));
                }
            }

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let can_load = crate::ui::editor_context::sprite_state(ui_state)
                    .selected_asset_index
                    .is_some();
                if ui
                    .add_enabled(can_load, egui::Button::new("Load"))
                    .clicked()
                {
                    if let Some(idx) = crate::ui::editor_context::sprite_state(ui_state).selected_asset_index {
                        let asset = crate::ui::editor_context::sprite_state(ui_state).discovered_assets[idx].clone();
                        if let Err(e) = crate::ui::editor_context::sprite_state_mut(ui_state).load_sprite_asset(&asset) {
                            tracing::error!("Failed to load sprite: {}", e);
                        } else {
                            let available_palettes = ui_state.project.available_palettes.clone();
                            ui_state.sprite_editor_context_mut().sprite.sync_palette_selection(&available_palettes);
                        }
                    }
                }
                if ui
                    .add_enabled(can_load, egui::Button::new("Rename"))
                    .clicked()
                {
                    if let Some(idx) = crate::ui::editor_context::sprite_state(ui_state).selected_asset_index {
                        if let Some(asset) = crate::ui::editor_context::sprite_state(ui_state).discovered_assets.get(idx) {
                            crate::ui::editor_context::sprite_state_mut(ui_state).rename_new_name = asset.name.clone();
                            crate::ui::editor_context::sprite_state_mut(ui_state).show_rename_dialog = true;
                        }
                    }
                }
                if ui
                    .add_enabled(can_load, egui::Button::new("Delete"))
                    .clicked()
                {
                    if let Some(idx) = crate::ui::editor_context::sprite_state(ui_state).selected_asset_index {
                        if let Some(asset) = crate::ui::editor_context::sprite_state(ui_state).discovered_assets.get(idx) {
                            crate::ui::editor_context::sprite_state_mut(ui_state).delete_asset_name = asset.name.clone();
                            crate::ui::editor_context::sprite_state_mut(ui_state).show_delete_confirm = true;
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_load_dialog = false;
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
            if crate::ui::editor_context::sprite_state(ui_state)
                .discovered_assets
                .is_empty()
            {
                ui.label("No sprite assets found in project.");
                ui.add_space(8.0);
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_merge_dialog = false;
                }
                return;
            }

            ui.label("Select sprites to merge (click to toggle):");
            ui.separator();

            let merge_selected_indices = crate::ui::editor_context::sprite_state(ui_state)
                .merge_selected_indices
                .clone();
            let asset_info: Vec<_> = ui_state
                .sprite_editor_context()
                .sprite
                .discovered_assets
                .iter()
                .enumerate()
                .map(|(i, asset)| {
                    let is_selected = merge_selected_indices.contains(&i);
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
                crate::ui::editor_context::sprite_state_mut(ui_state).toggle_merge_selection(idx);
            }

            ui.separator();

            let count = crate::ui::editor_context::sprite_state_mut(ui_state).merge_selected_indices.len();
            ui.label(format!("Selected: {} sprites", count));

            ui.horizontal(|ui| {
                ui.label("Columns:");
                ui.add(
                    egui::DragValue::new(&mut crate::ui::editor_context::sprite_state_mut(ui_state).merge_target_cols)
                        .range(1..=16)
                        .speed(1),
                );
            });

            if count > 0 {
                let cols = crate::ui::editor_context::sprite_state_mut(ui_state).merge_target_cols.max(1);
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
                    if let Err(e) = crate::ui::editor_context::sprite_state_mut(ui_state).merge_sprites_into_sheet() {
                        tracing::error!("Failed to merge sprites: {}", e);
                    }
                }
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_merge_dialog = false;
                }
            });

            if count < 2 {
                ui.label("Select at least 2 sprites to merge.");
            }
        });
}

fn render_resize_dialog(ui_state: &mut EditorUI, ctx: &egui::Context) {
    let cell_w = crate::ui::editor_context::sprite_state_mut(ui_state).active().cell_size.x.max(1);
    let cell_h = crate::ui::editor_context::sprite_state_mut(ui_state).active().cell_size.y.max(1);

    egui::Window::new("Resize Canvas")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            if let Some((w, h)) = crate::ui::editor_context::sprite_state_mut(ui_state).canvas_dimensions() {
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
                    egui::DragValue::new(&mut crate::ui::editor_context::sprite_state_mut(ui_state).resize_tiles_x)
                        .range(1..=128)
                        .speed(1),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Tiles Y:");
                ui.add(
                    egui::DragValue::new(&mut crate::ui::editor_context::sprite_state_mut(ui_state).resize_tiles_y)
                        .range(1..=128)
                        .speed(1),
                );
            });

            let new_w = crate::ui::editor_context::sprite_state_mut(ui_state).resize_tiles_x * cell_w;
            let new_h = crate::ui::editor_context::sprite_state_mut(ui_state).resize_tiles_y * cell_h;
            ui.label(format!("Result: {}x{} px", new_w, new_h));

            ui.separator();

            ui.label("Anchor:");
            egui::Grid::new("resize_anchor_grid")
                .spacing([2.0, 2.0])
                .show(ui, |ui| {
                    for (i, anchor) in ResizeAnchor::all().iter().enumerate() {
                        let is_selected = crate::ui::editor_context::sprite_state_mut(ui_state).resize_anchor == *anchor;
                        if ui.selectable_label(is_selected, anchor.label()).clicked() {
                            crate::ui::editor_context::sprite_state_mut(ui_state).resize_anchor = *anchor;
                        }
                        if (i + 1) % 3 == 0 {
                            ui.end_row();
                        }
                    }
                });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Resize").clicked() {
                    let w = crate::ui::editor_context::sprite_state_mut(ui_state).resize_tiles_x * cell_w;
                    let h = crate::ui::editor_context::sprite_state_mut(ui_state).resize_tiles_y * cell_h;
                    let anchor = crate::ui::editor_context::sprite_state_mut(ui_state).resize_anchor;
                    crate::ui::editor_context::sprite_state_mut(ui_state).resize_canvas(w, h, anchor);
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_resize_dialog = false;
                }
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_resize_dialog = false;
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
        .sprite_editor_context()
        .sprite
        .selected_asset_index
        .and_then(|idx| crate::ui::editor_context::sprite_state(ui_state).discovered_assets.get(idx))
        .map(|a| a.name.clone());

    let Some(old_name) = old_name else {
        crate::ui::editor_context::sprite_state_mut(ui_state).show_rename_dialog = false;
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
                ui.text_edit_singleline(&mut crate::ui::editor_context::sprite_state_mut(ui_state).rename_new_name);
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let new_name = crate::ui::editor_context::sprite_state_mut(ui_state).rename_new_name.clone();
                let can_rename = !new_name.is_empty() && new_name != old_name;

                if ui
                    .add_enabled(can_rename, egui::Button::new("Rename"))
                    .clicked()
                {
                    if let Some(dir) = sprites_dir {
                        match SpriteEditorState::rename_asset(dir, &old_name, &new_name) {
                            Ok(()) => {
                                crate::ui::editor_context::sprite_state_mut(ui_state).discovered_assets =
                                    SpriteEditorState::scan_sprite_assets(dir);
                                crate::ui::editor_context::sprite_state_mut(ui_state).selected_asset_index = None;
                                crate::ui::editor_context::sprite_state_mut(ui_state).show_rename_dialog = false;
                            }
                            Err(e) => {
                                tracing::error!("Failed to rename asset: {}", e);
                            }
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_rename_dialog = false;
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
            let name = crate::ui::editor_context::sprite_state(ui_state)
                .delete_asset_name
                .clone();

            ui.label(format!("Are you sure you want to delete \"{}\"?", name));
            ui.label("This will remove the PNG and JSON files.");
            ui.label("This action cannot be undone.");

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    if let Some(dir) = sprites_dir {
                        match SpriteEditorState::delete_asset(dir, &name) {
                            Ok(()) => {
                                crate::ui::editor_context::sprite_state_mut(ui_state).discovered_assets =
                                    SpriteEditorState::scan_sprite_assets(dir);
                                crate::ui::editor_context::sprite_state_mut(ui_state).selected_asset_index = None;
                                crate::ui::editor_context::sprite_state_mut(ui_state).show_delete_confirm = false;
                            }
                            Err(e) => {
                                tracing::error!("Failed to delete asset: {}", e);
                            }
                        }
                    }
                }
                if ui.button("Cancel").clicked() {
                    crate::ui::editor_context::sprite_state_mut(ui_state).show_delete_confirm = false;
                }
            });
        });
}
