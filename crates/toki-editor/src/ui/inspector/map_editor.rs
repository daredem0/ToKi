use super::*;

impl InspectorSystem {
    pub(crate) fn render_map_editor_command_palette(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        config: Option<&EditorConfig>,
    ) {
        ui.heading("Map Tools");
        ui.separator();
        ui.label("Command Palette");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut crate::ui::editor_context::map_state_mut(ui_state).tool, MapEditorTool::Drag, "Drag");
            ui.selectable_value(&mut crate::ui::editor_context::map_state_mut(ui_state).tool, MapEditorTool::Brush, "Brush");
            ui.selectable_value(&mut crate::ui::editor_context::map_state_mut(ui_state).tool, MapEditorTool::Fill, "Fill");
            ui.selectable_value(&mut crate::ui::editor_context::map_state_mut(ui_state).tool, MapEditorTool::PickTile, "Pick Tile");
            ui.selectable_value(
                &mut crate::ui::editor_context::map_state_mut(ui_state).tool,
                MapEditorTool::PlaceObject,
                "Place Object",
            );
            ui.selectable_value(
                &mut crate::ui::editor_context::map_state_mut(ui_state).tool,
                MapEditorTool::DeleteObject,
                "Delete",
            );
        });
        ui.separator();

        match crate::ui::editor_context::map_state_mut(ui_state).tool {
            MapEditorTool::Drag => {
                ui.label("Primary drag pans the map editor camera.");
                if let Some(selected_object) = crate::ui::editor_context::map_state_mut(ui_state).selected_object_info.clone() {
                    ui.separator();
                    ui.label("Object Info");
                    ui.horizontal(|ui| {
                        ui.label("Sheet:");
                        ui.label(selected_object.sheet.display().to_string());
                    });
                    ui.horizontal(|ui| {
                        ui.label("Object:");
                        ui.label(&selected_object.object_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        ui.label(format!(
                            "{}, {}",
                            selected_object.position.x, selected_object.position.y
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        ui.label(format!(
                            "{}x{} px",
                            selected_object.size_px.x, selected_object.size_px.y
                        ));
                    });

                    let mut grounding = selected_object.grounding.clone();
                    let origin = grounding.origin.get_or_insert([
                        (selected_object.size_px.x / 2) as i32,
                        selected_object.size_px.y.saturating_sub(1) as i32,
                    ]);
                    ui.separator();
                    ui.label("Grounding");
                    let mut grounding_changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Origin:");
                        grounding_changed |= ui
                            .add(egui::DragValue::new(&mut origin[0]).speed(1))
                            .changed();
                        grounding_changed |= ui
                            .add(egui::DragValue::new(&mut origin[1]).speed(1))
                            .changed();
                    });
                    let footprint = grounding.footprint.get_or_insert_with(|| {
                        toki_core::entity::EntityFootprint::new(
                            [0, 0],
                            [selected_object.size_px.x, selected_object.size_px.y],
                        )
                    });
                    ui.horizontal(|ui| {
                        ui.label("Footprint Offset:");
                        grounding_changed |= ui
                            .add(egui::DragValue::new(&mut footprint.offset[0]).speed(1))
                            .changed();
                        grounding_changed |= ui
                            .add(egui::DragValue::new(&mut footprint.offset[1]).speed(1))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Footprint Size:");
                        grounding_changed |= ui
                            .add(egui::DragValue::new(&mut footprint.size[0]).range(1..=u32::MAX))
                            .changed();
                        grounding_changed |= ui
                            .add(egui::DragValue::new(&mut footprint.size[1]).range(1..=u32::MAX))
                            .changed();
                    });

                    let mut visible = selected_object.visible;
                    let mut solid = selected_object.solid;
                    let visible_changed = ui.checkbox(&mut visible, "Visible").changed();
                    let solid_changed = ui.checkbox(&mut solid, "Solid").changed();
                    if grounding_changed || visible_changed || solid_changed {
                        crate::ui::editor_ui::queue_map_editor_object_property_edit(
                            ui_state,
                            selected_object.index,
                            grounding,
                            visible,
                            solid,
                        );
                    }
                } else if let Some(tile_info) = &crate::ui::editor_context::map_state_mut(ui_state).selected_tile_info {
                    ui.separator();
                    ui.label("Tile Info");
                    ui.horizontal(|ui| {
                        ui.label("Tile:");
                        ui.label(&tile_info.tile_name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                        ui.label(format!("{}, {}", tile_info.tile_x, tile_info.tile_y));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Solid:");
                        ui.label(if tile_info.solid { "Yes" } else { "No" });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Trigger:");
                        ui.label(if tile_info.trigger { "Yes" } else { "No" });
                    });
                } else {
                    ui.label("Click a tile or object to inspect it.");
                }
            }
            MapEditorTool::Brush | MapEditorTool::Fill => {
                ui.label(match crate::ui::editor_context::map_state_mut(ui_state).tool {
                    MapEditorTool::Brush => "Primary click/drag paints tiles.",
                    MapEditorTool::Fill => "Primary click fills the whole map.",
                    MapEditorTool::Drag
                    | MapEditorTool::PickTile
                    | MapEditorTool::PlaceObject
                    | MapEditorTool::DeleteObject => unreachable!(),
                });
                if let Some((tile_names, atlas, texture_path)) =
                    Self::load_map_editor_brush_source(ui_state, config)
                {
                    ui.horizontal(|ui| {
                        ui.label("Atlas Mode:");
                        ui.label(match atlas.color_mode {
                            toki_core::assets::atlas::ColorMode::TrueColor => "TrueColor",
                            toki_core::assets::atlas::ColorMode::PaletteIndexed => "PaletteIndexed",
                        });
                    });
                    if atlas.is_palette_indexed() {
                        ui.horizontal(|ui| {
                            ui.label("Default Palette:");
                            ui.label(atlas.palette.as_deref().unwrap_or("gb_default"));
                        });
                        ui.label(
                            "Indexed atlas palette selection is controlled by atlas metadata or the project-wide indexed override.",
                        );
                    }
                    crate::ui::editor_ui::sync_map_editor_brush_selection(ui_state, &tile_names);
                    ui.horizontal(|ui| {
                        ui.label("Tile:");
                        egui::ComboBox::from_id_salt("inspector_map_editor_brush_tile_selector")
                            .selected_text(
                                crate::ui::editor_context::map_state(ui_state)
                                    .selected_tile
                                    .as_deref()
                                    .unwrap_or("No tile selected"),
                            )
                            .show_ui(ui, |ui| {
                                for tile_name in &tile_names {
                                    let is_selected = crate::ui::editor_context::map_state_mut(ui_state).selected_tile.as_deref()
                                        == Some(tile_name.as_str());
                                    if ui.selectable_label(is_selected, tile_name).clicked() {
                                        crate::ui::editor_context::map_state_mut(ui_state).selected_tile = Some(tile_name.clone());
                                    }
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        if crate::ui::editor_context::map_state_mut(ui_state).tool == MapEditorTool::Brush {
                            ui.label("Brush Size:");
                            ui.add(
                                egui::DragValue::new(&mut crate::ui::editor_context::map_state_mut(ui_state).brush_size_tiles)
                                    .range(1..=32)
                                    .speed(1),
                            );
                            ui.label("tiles");
                        }
                    });

                    if let Some(tile_name) = crate::ui::editor_context::map_state_mut(ui_state).selected_tile.clone() {
                        ui.horizontal(|ui| {
                            ui.label(format!("Selected Tile: {}", tile_name));
                            Self::render_map_editor_selected_tile_preview(
                                ui_state,
                                ui,
                                ctx,
                                &atlas,
                                &texture_path,
                                &tile_name,
                            );
                        });
                        if let Some((solid, trigger)) =
                            Self::selected_map_editor_tile_metadata(&atlas, &tile_name)
                        {
                            ui.horizontal(|ui| {
                                ui.label("Solid:");
                                ui.label(if solid { "Yes" } else { "No" });
                                ui.separator();
                                ui.label("Trigger:");
                                ui.label(if trigger { "Yes" } else { "No" });
                            });
                        }
                    } else {
                        ui.label("Selected Tile: none");
                    }
                } else {
                    ui.label("No atlas tiles available for the current map.");
                }
                ui.label(match crate::ui::editor_context::map_state_mut(ui_state).tool {
                    MapEditorTool::Brush => "Secondary drag pans the camera.",
                    MapEditorTool::Fill => "Secondary drag pans the camera.",
                    MapEditorTool::Drag
                    | MapEditorTool::PickTile
                    | MapEditorTool::PlaceObject
                    | MapEditorTool::DeleteObject => unreachable!(),
                });
            }
            MapEditorTool::PickTile => {
                ui.label("Click a tile in the map to pick it.");
                ui.label("After picking, the tool switches back to Brush automatically.");
                if let Some(tile_name) = crate::ui::editor_context::map_state_mut(ui_state).selected_tile.as_deref() {
                    ui.separator();
                    ui.label(format!("Current Brush Tile: {}", tile_name));
                }
            }
            MapEditorTool::PlaceObject => {
                ui.label("Primary click places the selected object on the map.");
                ui.label("Secondary drag pans the camera.");
                if let Some((sheet_names, object_names, object_sheet, texture_path)) =
                    Self::load_map_editor_object_sheet_source(ui_state, config)
                {
                    crate::ui::editor_ui::sync_map_editor_object_sheet_selection(
                        ui_state,
                        &sheet_names,
                    );
                    ui.horizontal(|ui| {
                        ui.label("Object Sheet:");
                        egui::ComboBox::from_id_salt("inspector_map_editor_object_sheet_selector")
                            .selected_text(
                                crate::ui::editor_context::map_state(ui_state)
                                    .selected_object_sheet
                                    .as_deref()
                                    .unwrap_or("No object sheet selected"),
                            )
                            .show_ui(ui, |ui| {
                                for sheet_name in &sheet_names {
                                    let is_selected = crate::ui::editor_context::map_state_mut(ui_state).selected_object_sheet.as_deref()
                                        == Some(sheet_name.as_str());
                                    if ui.selectable_label(is_selected, sheet_name).clicked() {
                                        crate::ui::editor_context::map_state_mut(ui_state).selected_object_sheet =
                                            Some(sheet_name.clone());
                                        crate::ui::editor_context::map_state_mut(ui_state).selected_object_name = None;
                                    }
                                }
                            });
                    });
                    crate::ui::editor_ui::sync_map_editor_object_selection(ui_state, &object_names);
                    ui.separator();
                    ui.label("Objects");
                    Self::render_map_editor_object_sheet_gallery(
                        ui_state,
                        ui,
                        ctx,
                        &object_sheet,
                        &texture_path,
                        &object_names,
                    );

                    if let Some(object_name) = crate::ui::editor_context::map_state_mut(ui_state).selected_object_name.clone() {
                        ui.separator();
                        ui.label(format!("Selected Object: {}", object_name));
                        if let Some(object_info) = object_sheet.objects.get(&object_name) {
                            ui.horizontal(|ui| {
                                ui.label("Size:");
                                ui.label(format!(
                                    "{}x{} tiles",
                                    object_info.size_tiles.x, object_info.size_tiles.y
                                ));
                                ui.separator();
                                ui.label(format!(
                                    "{}x{} px",
                                    object_info.size_tiles.x * object_sheet.tile_size.x,
                                    object_info.size_tiles.y * object_sheet.tile_size.y
                                ));
                            });
                        }
                    }
                } else {
                    ui.label("No object sheets available in assets/sprites.");
                }
            }
            MapEditorTool::DeleteObject => {
                ui.label("Primary click deletes the clicked visible object from the map.");
                ui.label("Secondary drag pans the camera.");
            }
        }

        if crate::ui::editor_ui::has_unsaved_map_editor_changes(ui_state) {
            ui.separator();
            ui.label("Map editor has unsaved changes.");
        }
    }

    pub(super) fn load_map_editor_brush_source(
        ui_state: &EditorUI,
        config: Option<&EditorConfig>,
    ) -> Option<(
        Vec<String>,
        toki_core::assets::atlas::AtlasMeta,
        std::path::PathBuf,
    )> {
        let project_path = config?.current_project_path()?;

        let tilemap = if let Some(draft) = &crate::ui::editor_context::map_state(ui_state).draft {
            draft.tilemap.clone()
        } else {
            let active_map = crate::ui::editor_context::map_state(ui_state).active_map.as_ref()?;
            toki_core::assets::tilemap::TileMap::load_from_file(
                project_path
                    .join("assets")
                    .join("tilemaps")
                    .join(format!("{}.json", active_map)),
            )
            .ok()?
        };

        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(&tilemap.atlas);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path
                    .join("assets")
                    .join("sprites")
                    .join(&tilemap.atlas)
            }
        };
        let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_path).ok()?;
        let texture_path = atlas_path.parent()?.join(&atlas.image);
        let mut tile_names = atlas.tiles.keys().cloned().collect::<Vec<_>>();
        tile_names.sort();
        Some((tile_names, atlas, texture_path))
    }

    pub(super) fn load_map_editor_object_sheet_source(
        ui_state: &EditorUI,
        config: Option<&EditorConfig>,
    ) -> Option<(
        Vec<String>,
        Vec<String>,
        ObjectSheetMeta,
        std::path::PathBuf,
    )> {
        let project_path = config?.current_project_path()?;
        let sprites_dir = project_path.join("assets").join("sprites");
        let mut object_sheets = Vec::new();

        for entry in std::fs::read_dir(&sprites_dir).ok()? {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if !path.is_file() || path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let Ok(object_sheet) = ObjectSheetMeta::load_from_file(&path) else {
                continue;
            };
            let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };
            object_sheets.push((stem.to_string(), path, object_sheet));
        }

        if object_sheets.is_empty() {
            return None;
        }

        object_sheets.sort_by(|left, right| left.0.cmp(&right.0));
        let sheet_names = object_sheets
            .iter()
            .map(|(name, _, _)| name.clone())
            .collect::<Vec<_>>();
        let selected_sheet_name = crate::ui::editor_context::map_state(ui_state)
            .selected_object_sheet
            .clone()
            .unwrap_or_else(|| sheet_names[0].clone());

        let (_, object_sheet_path, object_sheet) = object_sheets
            .into_iter()
            .find(|(name, _, _)| name == &selected_sheet_name)
            .or_else(|| {
                let fallback = sheet_names[0].clone();
                std::fs::read_dir(&sprites_dir)
                    .ok()?
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.is_file() && path.extension().is_some_and(|ext| ext == "json")
                    })
                    .filter_map(|path| {
                        let object_sheet = ObjectSheetMeta::load_from_file(&path).ok()?;
                        let stem = path.file_stem()?.to_str()?.to_string();
                        Some((stem, path, object_sheet))
                    })
                    .find(|(name, _, _)| name == &fallback)
            })?;

        let mut object_names = object_sheet.objects.keys().cloned().collect::<Vec<_>>();
        object_names.sort_by(|left, right| {
            let left_info = object_sheet.objects.get(left);
            let right_info = object_sheet.objects.get(right);

            match (left_info, right_info) {
                (Some(left_info), Some(right_info)) => left_info
                    .position
                    .y
                    .cmp(&right_info.position.y)
                    .then_with(|| left_info.position.x.cmp(&right_info.position.x))
                    .then_with(|| left.cmp(right)),
                _ => left.cmp(right),
            }
        });
        let texture_path = object_sheet_path.parent()?.join(&object_sheet.image);

        Some((sheet_names, object_names, object_sheet, texture_path))
    }

    pub(super) fn render_map_editor_selected_tile_preview(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        atlas: &toki_core::assets::atlas::AtlasMeta,
        texture_path: &std::path::Path,
        tile_name: &str,
    ) {
        let Some(texture) =
            Self::ensure_map_editor_brush_preview_texture(ui_state, ctx, texture_path)
        else {
            return;
        };
        let Some(texture_size) = atlas.image_size() else {
            return;
        };
        let Some(rect_px) = atlas.get_tile_rect(tile_name) else {
            return;
        };

        let uv_rect = egui::Rect::from_min_max(
            egui::pos2(
                rect_px[0] as f32 / texture_size.x as f32,
                rect_px[1] as f32 / texture_size.y as f32,
            ),
            egui::pos2(
                (rect_px[0] + rect_px[2]) as f32 / texture_size.x as f32,
                (rect_px[1] + rect_px[3]) as f32 / texture_size.y as f32,
            ),
        );

        let preview_size = egui::vec2(48.0, 48.0);
        let (rect, response) = ui.allocate_exact_size(preview_size, egui::Sense::hover());
        ui.painter().rect_stroke(
            rect,
            2.0,
            egui::Stroke::new(1.0, egui::Color32::GRAY),
            egui::StrokeKind::Outside,
        );
        ui.painter().image(
            texture.id(),
            rect.shrink(2.0),
            uv_rect,
            egui::Color32::WHITE,
        );
        response.on_hover_text(tile_name);
    }

    fn render_map_editor_object_sheet_gallery(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        object_sheet: &ObjectSheetMeta,
        texture_path: &std::path::Path,
        object_names: &[String],
    ) {
        let Some(texture) =
            Self::ensure_map_editor_brush_preview_texture(ui_state, ctx, texture_path)
        else {
            ui.label("Preview texture unavailable.");
            return;
        };
        let Some(texture_size) = object_sheet.image_size() else {
            ui.label("Preview image size unavailable.");
            return;
        };

        const COLUMNS: usize = 4;
        const SLOT_SIZE: f32 = 64.0;

        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                egui::Grid::new("inspector_map_editor_object_gallery")
                    .num_columns(COLUMNS)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (index, object_name) in object_names.iter().enumerate() {
                            ui.vertical(|ui| {
                                let is_selected = crate::ui::editor_context::map_state_mut(ui_state).selected_object_name.as_deref()
                                    == Some(object_name.as_str());
                                if Self::render_map_editor_object_gallery_item(
                                    ui,
                                    texture.id(),
                                    texture_size,
                                    object_sheet,
                                    object_name,
                                    is_selected,
                                    SLOT_SIZE,
                                )
                                .clicked()
                                {
                                    crate::ui::editor_context::map_state_mut(ui_state).selected_object_name = Some(object_name.clone());
                                }
                                ui.add_sized(
                                    [SLOT_SIZE, 16.0],
                                    egui::Label::new(
                                        egui::RichText::new(object_name.as_str()).small(),
                                    )
                                    .truncate(),
                                );
                            });

                            if (index + 1) % COLUMNS == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }

    fn render_map_editor_object_gallery_item(
        ui: &mut egui::Ui,
        texture_id: egui::TextureId,
        texture_size: glam::UVec2,
        object_sheet: &ObjectSheetMeta,
        object_name: &str,
        selected: bool,
        slot_size: f32,
    ) -> egui::Response {
        let desired_size = egui::vec2(slot_size, slot_size);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        let frame_stroke = if selected {
            egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE)
        } else {
            egui::Stroke::new(1.0, egui::Color32::GRAY)
        };
        let frame_fill = if selected {
            egui::Color32::from_rgb(35, 55, 75)
        } else {
            egui::Color32::from_gray(24)
        };
        ui.painter().rect(
            rect,
            4.0,
            frame_fill,
            frame_stroke,
            egui::StrokeKind::Outside,
        );

        if let Some(rect_px) = object_sheet.get_object_rect(object_name) {
            let uv_rect = egui::Rect::from_min_max(
                egui::pos2(
                    rect_px[0] as f32 / texture_size.x as f32,
                    rect_px[1] as f32 / texture_size.y as f32,
                ),
                egui::pos2(
                    (rect_px[0] + rect_px[2]) as f32 / texture_size.x as f32,
                    (rect_px[1] + rect_px[3]) as f32 / texture_size.y as f32,
                ),
            );
            let max_dimension = rect_px[2].max(rect_px[3]) as f32;
            let preview_scale = if max_dimension > 0.0 {
                (slot_size - 8.0) / max_dimension
            } else {
                1.0
            };
            let preview_size = egui::vec2(
                rect_px[2] as f32 * preview_scale,
                rect_px[3] as f32 * preview_scale,
            );
            let preview_rect = egui::Rect::from_center_size(rect.center(), preview_size);
            ui.painter()
                .image(texture_id, preview_rect, uv_rect, egui::Color32::WHITE);
        }

        response.on_hover_text(object_name)
    }

    pub(super) fn selected_map_editor_tile_metadata(
        atlas: &toki_core::assets::atlas::AtlasMeta,
        tile_name: &str,
    ) -> Option<(bool, bool)> {
        let properties = atlas.get_tile_properties(tile_name)?;
        Some((properties.solid, properties.trigger))
    }

    pub(super) fn set_optional_runtime_stat(
        attributes: &mut toki_core::entity::EntityAttributes,
        stat_id: &str,
        value: Option<i32>,
    ) -> bool {
        let previous_base = attributes.gameplay.stats.base.get(stat_id).copied();
        let previous_current = attributes.gameplay.stats.current.get(stat_id).copied();
        let mut changed = false;

        match value {
            Some(value) => {
                if previous_base != Some(value) {
                    attributes.gameplay.stats.base.insert(stat_id.to_string(), value);
                    changed = true;
                }
                if previous_current != Some(value) {
                    attributes
                        .gameplay
                        .stats
                        .current
                        .insert(stat_id.to_string(), value);
                    changed = true;
                }
            }
            None => {
                changed |= attributes.gameplay.stats.base.remove(stat_id).is_some();
                changed |= attributes.gameplay.stats.current.remove(stat_id).is_some();
            }
        }

        changed
    }

    pub(super) fn set_optional_definition_stat(
        attributes: &mut toki_core::entity::AttributesDef,
        stat_id: &str,
        value: Option<i32>,
    ) -> bool {
        let previous = attributes.stats.get(stat_id).copied();
        match value {
            Some(value) if previous != Some(value) => {
                attributes.stats.insert(stat_id.to_string(), value);
                true
            }
            Some(_) => false,
            None => attributes.stats.remove(stat_id).is_some(),
        }
    }

    pub(super) fn ensure_map_editor_brush_preview_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if crate::ui::editor_context::map_state_mut(ui_state).brush_preview_image_path.as_deref() == Some(texture_path)
            && crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture.is_some()
        {
            return crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_image_path = Some(texture_path.to_path_buf());
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture = Some(texture.clone());
        Some(texture)
    }
}
