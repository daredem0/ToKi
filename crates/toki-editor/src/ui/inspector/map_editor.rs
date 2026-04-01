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
            ui.selectable_value(
                &mut crate::ui::editor_context::map_state_mut(ui_state).tool,
                MapEditorTool::Drag,
                "Drag",
            );
            ui.selectable_value(
                &mut crate::ui::editor_context::map_state_mut(ui_state).tool,
                MapEditorTool::Brush,
                "Brush",
            );
            ui.selectable_value(
                &mut crate::ui::editor_context::map_state_mut(ui_state).tool,
                MapEditorTool::Fill,
                "Fill",
            );
            ui.selectable_value(
                &mut crate::ui::editor_context::map_state_mut(ui_state).tool,
                MapEditorTool::PickTile,
                "Pick Tile",
            );
        });
        ui.separator();

        match crate::ui::editor_context::map_state_mut(ui_state).tool {
            MapEditorTool::Drag => {
                ui.label("Primary drag pans the map editor camera.");
                if let Some(tile_info) =
                    &crate::ui::editor_context::map_state_mut(ui_state).selected_tile_info
                {
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
                ui.label(
                    match crate::ui::editor_context::map_state_mut(ui_state).tool {
                        MapEditorTool::Brush => "Primary click/drag paints tiles.",
                        MapEditorTool::Fill => "Primary click fills the whole map.",
                        MapEditorTool::Drag | MapEditorTool::PickTile => unreachable!(),
                    },
                );
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
                                    let is_selected =
                                        crate::ui::editor_context::map_state_mut(ui_state)
                                            .selected_tile
                                            .as_deref()
                                            == Some(tile_name.as_str());
                                    if ui.selectable_label(is_selected, tile_name).clicked() {
                                        crate::ui::editor_context::map_state_mut(ui_state)
                                            .selected_tile = Some(tile_name.clone());
                                    }
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        if crate::ui::editor_context::map_state_mut(ui_state).tool
                            == MapEditorTool::Brush
                        {
                            ui.label("Brush Size:");
                            ui.add(
                                egui::DragValue::new(
                                    &mut crate::ui::editor_context::map_state_mut(ui_state)
                                        .brush_size_tiles,
                                )
                                .range(1..=32)
                                .speed(1),
                            );
                            ui.label("tiles");
                        }
                    });

                    if let Some(tile_name) = crate::ui::editor_context::map_state_mut(ui_state)
                        .selected_tile
                        .clone()
                    {
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
                ui.label(
                    match crate::ui::editor_context::map_state_mut(ui_state).tool {
                        MapEditorTool::Brush => "Secondary drag pans the camera.",
                        MapEditorTool::Fill => "Secondary drag pans the camera.",
                        MapEditorTool::Drag | MapEditorTool::PickTile => unreachable!(),
                    },
                );
            }
            MapEditorTool::PickTile => {
                ui.label("Click a tile in the map to pick it.");
                ui.label("After picking, the tool switches back to Brush automatically.");
                if let Some(tile_name) = crate::ui::editor_context::map_state_mut(ui_state)
                    .selected_tile
                    .as_deref()
                {
                    ui.separator();
                    ui.label(format!("Current Brush Tile: {}", tile_name));
                }
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
            let active_map = crate::ui::editor_context::map_state(ui_state)
                .active_map
                .as_ref()?;
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
                    attributes
                        .gameplay
                        .stats
                        .base
                        .insert(stat_id.to_string(), value);
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
        if crate::ui::editor_context::map_state_mut(ui_state)
            .brush_preview_image_path
            .as_deref()
            == Some(texture_path)
            && crate::ui::editor_context::map_state_mut(ui_state)
                .brush_preview_texture
                .is_some()
        {
            return crate::ui::editor_context::map_state_mut(ui_state)
                .brush_preview_texture
                .clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_image_path =
            Some(texture_path.to_path_buf());
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture =
            Some(texture.clone());
        Some(texture)
    }
}
