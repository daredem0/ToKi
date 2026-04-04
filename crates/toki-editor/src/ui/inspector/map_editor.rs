use super::*;

fn tile_display_label(name: &str, atlas: &toki_core::assets::atlas::AtlasMeta) -> String {
    if atlas.is_auto_tile_group(name) {
        format!("[A] {name}")
    } else if atlas.is_animated_tile(name) {
        format!("[~] {name}")
    } else {
        name.to_string()
    }
}

enum LayerPanelAction {
    ToggleVisibility(usize),
    ToggleAboveEntities(usize),
    Select(usize),
    MoveUp(usize),
    MoveDown(usize),
    Remove(usize),
    Add(String),
}

struct LayerRowData<'a> {
    index: usize,
    name: &'a str,
    visible: bool,
    above_entities: bool,
    is_active: bool,
    layer_count: usize,
}

impl InspectorSystem {
    pub(crate) fn render_map_editor_toolbox(
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
        Self::render_layer_panel(ui_state, ui);
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
                    let mut solid = tile_info.solid;
                    let mut trigger = tile_info.trigger;
                    let tile_name_for_edit = tile_info.tile_name.clone();
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        changed |= ui.checkbox(&mut solid, "Solid").changed();
                        changed |= ui.checkbox(&mut trigger, "Trigger").changed();
                    });
                    if changed {
                        Self::update_tile_property(
                            ui_state,
                            config,
                            &tile_name_for_edit,
                            solid,
                            trigger,
                        );
                    }
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
                                    let display = tile_display_label(tile_name, &atlas);
                                    if ui.selectable_label(is_selected, display).clicked() {
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
                        if let Some(props) = atlas.get_tile_properties(&tile_name).cloned() {
                            let mut solid = props.solid;
                            let mut trigger = props.trigger;
                            let mut changed = false;
                            ui.horizontal(|ui| {
                                changed |= ui.checkbox(&mut solid, "Solid").changed();
                                changed |= ui.checkbox(&mut trigger, "Trigger").changed();
                            });
                            if changed {
                                let mut atlas_mut = atlas.clone();
                                if let Some(tile_info) = atlas_mut.tiles.get_mut(&tile_name) {
                                    tile_info.properties.solid = solid;
                                    tile_info.properties.trigger = trigger;
                                }
                                crate::ui::editor_context::map_state_mut(ui_state).modified_atlas =
                                    Some(atlas_mut);
                            }
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

    fn render_layer_panel(ui_state: &mut EditorUI, ui: &mut egui::Ui) {
        let Some(draft) = &crate::ui::editor_context::map_state(ui_state).draft else {
            return;
        };
        let layer_count = draft.tilemap.layers.len();
        if layer_count == 0 {
            return;
        }
        let active_layer = crate::ui::editor_context::map_state(ui_state).active_layer;

        ui.label("Layers");
        let mut action: Option<LayerPanelAction> = None;

        // Render layers in reverse order (topmost first, like Photoshop)
        for display_i in 0..layer_count {
            let layer_index = layer_count - 1 - display_i;
            let layer = &draft.tilemap.layers[layer_index];
            let row = LayerRowData {
                index: layer_index,
                name: &layer.name,
                visible: layer.visible,
                above_entities: layer.above_entities,
                is_active: layer_index == active_layer,
                layer_count,
            };
            Self::render_layer_row(ui, &row, &mut action);
            if row.is_active {
                Self::render_active_layer_properties(ui, &row, &mut action);
            }
        }

        ui.horizontal(|ui| {
            if ui.small_button("+ Add Layer").clicked() {
                let name = format!("Layer {}", layer_count);
                action = Some(LayerPanelAction::Add(name));
            }
        });

        Self::apply_layer_action(ui_state, action);
    }

    fn render_layer_row(
        ui: &mut egui::Ui,
        row: &LayerRowData<'_>,
        action: &mut Option<LayerPanelAction>,
    ) {
        ui.horizontal(|ui| {
            let eye_label = if row.visible { "\u{1F441}" } else { "--" };
            if ui.small_button(eye_label).clicked() {
                *action = Some(LayerPanelAction::ToggleVisibility(row.index));
            }

            let label = if row.is_active {
                egui::RichText::new(row.name).strong()
            } else if !row.visible {
                egui::RichText::new(row.name).weak()
            } else {
                egui::RichText::new(row.name)
            };
            if ui.selectable_label(row.is_active, label).clicked() {
                *action = Some(LayerPanelAction::Select(row.index));
            }

            if row.index + 1 < row.layer_count && ui.small_button("Up").clicked() {
                *action = Some(LayerPanelAction::MoveUp(row.index));
            }
            if row.index > 0 && ui.small_button("Down").clicked() {
                *action = Some(LayerPanelAction::MoveDown(row.index));
            }
            if row.layer_count > 1 && ui.small_button("🗑").clicked() {
                *action = Some(LayerPanelAction::Remove(row.index));
            }
        });
    }

    fn render_active_layer_properties(
        ui: &mut egui::Ui,
        row: &LayerRowData<'_>,
        action: &mut Option<LayerPanelAction>,
    ) {
        ui.indent(format!("layer_props_{}", row.index), |ui| {
            let mut above = row.above_entities;
            if ui.checkbox(&mut above, "Above entities").changed() {
                *action = Some(LayerPanelAction::ToggleAboveEntities(row.index));
            }
        });
    }

    fn apply_layer_action(ui_state: &mut EditorUI, action: Option<LayerPanelAction>) {
        let Some(action) = action else {
            return;
        };
        match action {
            LayerPanelAction::ToggleVisibility(i) => {
                crate::ui::editor_ui::toggle_layer_visibility(ui_state, i);
            }
            LayerPanelAction::ToggleAboveEntities(i) => {
                crate::ui::editor_ui::toggle_layer_above_entities(ui_state, i);
            }
            LayerPanelAction::Select(i) => {
                crate::ui::editor_ui::set_active_layer(ui_state, i);
            }
            LayerPanelAction::MoveUp(i) => {
                crate::ui::editor_ui::move_layer(ui_state, i, i + 1);
            }
            LayerPanelAction::MoveDown(i) => {
                crate::ui::editor_ui::move_layer(ui_state, i, i - 1);
            }
            LayerPanelAction::Remove(i) => {
                crate::ui::editor_ui::remove_layer_from_map(ui_state, i);
            }
            LayerPanelAction::Add(name) => {
                crate::ui::editor_ui::add_layer_to_map(ui_state, &name);
            }
        }
    }

    pub(crate) fn render_map_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        config: Option<&EditorConfig>,
    ) {
        ui.heading("Map Inspector");
        ui.separator();

        let selected_tile_info = crate::ui::editor_context::map_state(ui_state)
            .selected_tile_info
            .clone();
        if let Some(tile_info) = selected_tile_info {
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

            if let Some((_, atlas, texture_path)) =
                Self::load_map_editor_brush_source(ui_state, config)
            {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Preview:");
                    Self::render_map_editor_selected_tile_preview(
                        ui_state,
                        ui,
                        ctx,
                        &atlas,
                        &texture_path,
                        &tile_info.tile_name,
                    );
                });
            }
        } else {
            ui.label("No tile selected.");
            ui.label("Click a tile in the map editor to inspect it.");
        }

        if crate::ui::editor_ui::has_unsaved_map_editor_changes(ui_state) {
            ui.separator();
            ui.label("Map editor has unsaved changes.");
        }
    }

    fn update_tile_property(
        ui_state: &mut EditorUI,
        config: Option<&EditorConfig>,
        tile_name: &str,
        solid: bool,
        trigger: bool,
    ) {
        let Some((_, atlas, _)) = Self::load_map_editor_brush_source(ui_state, config) else {
            return;
        };
        let mut atlas_mut = atlas;
        if let Some(tile_info) = atlas_mut.tiles.get_mut(tile_name) {
            tile_info.properties.solid = solid;
            tile_info.properties.trigger = trigger;
        }
        let map_state = crate::ui::editor_context::map_state_mut(ui_state);
        map_state.modified_atlas = Some(atlas_mut);
        // Update cached tile info to reflect the change immediately
        if let Some(info) = &mut map_state.selected_tile_info {
            if info.tile_name == tile_name {
                info.solid = solid;
                info.trigger = trigger;
            }
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
        let atlas =
            if let Some(cached) = &crate::ui::editor_context::map_state(ui_state).modified_atlas {
                cached.clone()
            } else {
                toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_path).ok()?
            };
        let texture_path = atlas_path.parent()?.join(&atlas.image);
        let mut tile_names: Vec<String> = atlas.tiles.keys().cloned().collect();
        for group_name in atlas.auto_tile_groups.keys() {
            if !tile_names.contains(group_name) {
                tile_names.push(group_name.clone());
            }
        }
        for anim_name in atlas.animated_tiles.keys() {
            if !tile_names.contains(anim_name) {
                tile_names.push(anim_name.clone());
            }
        }
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


    pub(super) fn set_optional_runtime_stat(
        combat: &mut toki_core::entity::CombatComponent,
        stat_id: &str,
        value: Option<i32>,
    ) -> bool {
        let previous_base = combat.stats.base.get(stat_id).copied();
        let previous_current = combat.stats.current.get(stat_id).copied();
        let mut changed = false;

        match value {
            Some(value) => {
                if previous_base != Some(value) {
                    combat.stats.base.insert(stat_id.to_string(), value);
                    changed = true;
                }
                if previous_current != Some(value) {
                    combat.stats.current.insert(stat_id.to_string(), value);
                    changed = true;
                }
            }
            None => {
                changed |= combat.stats.base.remove(stat_id).is_some();
                changed |= combat.stats.current.remove(stat_id).is_some();
            }
        }

        changed
    }

    pub(super) fn set_optional_definition_stat(
        components: &mut toki_core::entity::ComponentsDef,
        stat_id: &str,
        value: Option<i32>,
    ) -> bool {
        let combat = components
            .combat
            .get_or_insert_with(toki_core::entity::CombatComponent::default);
        let previous = combat.stats.base.get(stat_id).copied();
        match value {
            Some(value) if previous != Some(value) => {
                combat.stats.base.insert(stat_id.to_string(), value);
                combat.stats.current.insert(stat_id.to_string(), value);
                true
            }
            Some(_) => false,
            None => {
                let mut changed = combat.stats.base.remove(stat_id).is_some();
                changed |= combat.stats.current.remove(stat_id).is_some();
                changed
            }
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
