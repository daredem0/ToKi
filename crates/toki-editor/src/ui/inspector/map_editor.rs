use super::*;
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
        renderer: Option<&mut crate::rendering::WindowRenderer>,
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
                if let Some(brush_source) = Self::load_map_editor_brush_source(ui_state, config)
                {
                    let selected_entry = crate::ui::editor_ui::selected_map_editor_brush_entry(
                        &brush_source.brush_entries,
                        crate::ui::editor_context::map_state(ui_state)
                            .selected_tile
                            .as_deref(),
                    )
                    .cloned();
                    let selected_atlas = selected_entry
                        .as_ref()
                        .and_then(|entry| {
                            crate::ui::editor_ui::map_editor_brush_entry_atlas_name(&entry.id)
                        })
                        .and_then(|atlas_name| brush_source.atlases.get(atlas_name));

                    ui.horizontal(|ui| {
                        ui.label("Tileset Entries:");
                        ui.label(brush_source.tileset.entries.len().to_string());
                    });
                    if let Some(atlas) = selected_atlas.map(|source| &source.meta) {
                        ui.horizontal(|ui| {
                            ui.label("Selected Source Mode:");
                            ui.label(match atlas.color_mode {
                                toki_core::assets::atlas::ColorMode::TrueColor => "TrueColor",
                                toki_core::assets::atlas::ColorMode::PaletteIndexed => {
                                    "PaletteIndexed"
                                }
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
                    }
                    crate::ui::editor_ui::sync_map_editor_brush_selection(
                        ui_state,
                        &brush_source.brush_entries,
                    );
                    ui.horizontal(|ui| {
                        ui.label("Tile:");
                        let selected_label = crate::ui::editor_ui::selected_map_editor_brush_entry(
                            &brush_source.brush_entries,
                            crate::ui::editor_context::map_state(ui_state)
                                .selected_tile
                                .as_deref(),
                        )
                        .map(|entry| entry.display_label.as_str())
                        .unwrap_or("No tile selected");
                        egui::ComboBox::from_id_salt("inspector_map_editor_brush_tile_selector")
                            .selected_text(selected_label)
                            .show_ui(ui, |ui| {
                                for entry in &brush_source.brush_entries {
                                    let is_selected =
                                        crate::ui::editor_context::map_state_mut(ui_state)
                                            .selected_tile
                                            .as_deref()
                                            == Some(entry.id.as_str());
                                    if ui
                                        .selectable_label(is_selected, &entry.display_label)
                                        .clicked()
                                    {
                                        crate::ui::editor_context::map_state_mut(ui_state)
                                            .selected_tile = Some(entry.id.clone());
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
                    Self::render_collision_stamp_toggle(ui_state, ui);

                    if let (Some(selected_entry), Some(atlas_source)) = (selected_entry, selected_atlas)
                    {
                        let texture_path = atlas_source
                            .path
                            .parent()
                            .map(|parent| parent.join(&atlas_source.meta.image));
                        ui.horizontal(|ui| {
                            ui.label(format!("Selected Tile: {}", selected_entry.display_label));
                            if let Some(texture_path) = texture_path.as_ref() {
                                Self::render_map_editor_selected_tile_preview(
                                    ui_state,
                                    ui,
                                    ctx,
                                    &atlas_source.meta,
                                    texture_path,
                                    &selected_entry,
                                    renderer,
                                );
                            }
                        });
                    } else {
                        ui.label("Selected Tile: none");
                    }
                } else {
                    ui.label("No tileset entries available for the current map.");
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

        ui.separator();
        Self::render_auto_tile_groups_section(ui_state, ui, config);

        if crate::ui::editor_ui::has_unsaved_map_editor_changes(ui_state) {
            ui.separator();
            ui.label("Map editor has unsaved changes.");
        }
    }

    fn render_auto_tile_groups_section(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        config: Option<&EditorConfig>,
    ) {
        let Some(brush_source) = Self::load_map_editor_brush_source(ui_state, config) else {
            return;
        };
        let mut auto_tile_entries = brush_source
            .brush_entries
            .iter()
            .filter(|entry| entry.kind == crate::ui::editor_ui::MapEditorBrushKind::AutoTileGroup)
            .collect::<Vec<_>>();
        auto_tile_entries.sort_by(|left, right| left.display_label.cmp(&right.display_label));
        if auto_tile_entries.is_empty() {
            ui.label("Auto-Tile Groups: none");
            return;
        }
        ui.label("Auto-Tile Groups");
        for entry in auto_tile_entries {
            ui.horizontal(|ui| {
                ui.label(&entry.display_label);
                if let Some(atlas_name) =
                    crate::ui::editor_ui::map_editor_brush_entry_atlas_name(&entry.id)
                {
                    ui.small(format!("source: {atlas_name}"));
                }
            });
        }
        ui.small("Auto-tiles are linked through the map tileset; atlas merge import is not used here.");
    }

    fn render_import_auto_tile_button(
        _ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        _config: Option<&EditorConfig>,
    ) {
        ui.small("Use tileset composition to link auto-tiles from source atlases.");
    }

    fn discover_auto_tile_atlases(
        project_path: &std::path::Path,
    ) -> Vec<(String, std::path::PathBuf)> {
        let sprites_dir = project_path.join("assets").join("sprites");
        let Ok(entries) = std::fs::read_dir(&sprites_dir) else {
            return Vec::new();
        };
        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|e| {
                let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(e.path()).ok()?;
                if atlas.auto_tile_groups.is_empty() {
                    return None;
                }
                let name = e
                    .path()
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                Some((name, e.path()))
            })
            .collect()
    }

    fn execute_auto_tile_import(
        _ui_state: &mut EditorUI,
        _config: Option<&EditorConfig>,
        _project_path: &std::path::Path,
        _source_path: &std::path::Path,
    ) {
        tracing::warn!("Auto-tile atlas import via atlas merge is not used in tileset-based maps");
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
        mut renderer: Option<&mut crate::rendering::WindowRenderer>,
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

            if let Some(brush_source) = Self::load_map_editor_brush_source(ui_state, config)
            {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Preview:");
                    if let Some(brush_entry) =
                        crate::ui::editor_ui::selected_map_editor_brush_entry(
                            &brush_source.brush_entries,
                            Some(tile_info.tile_name.as_str()),
                        )
                    {
                        if let Some(atlas_name) =
                            crate::ui::editor_ui::map_editor_brush_entry_atlas_name(
                                &brush_entry.id,
                            )
                        {
                            if let Some(atlas_source) = brush_source.atlases.get(atlas_name) {
                                if let Some(texture_path) = atlas_source
                                    .path
                                    .parent()
                                    .map(|parent| parent.join(&atlas_source.meta.image))
                                {
                                    Self::render_map_editor_selected_tile_preview(
                                        ui_state,
                                        ui,
                                        ctx,
                                        &atlas_source.meta,
                                        &texture_path,
                                        brush_entry,
                                        renderer.as_deref_mut(),
                                    );
                                }
                            }
                        }
                    }
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

    fn render_collision_stamp_toggle(ui_state: &mut EditorUI, ui: &mut egui::Ui) {
        let state = crate::ui::editor_context::map_state_mut(ui_state);
        let mut enabled = state.brush_stamp_solid.is_some();
        let mut solid = state.brush_stamp_solid.unwrap_or(true);
        ui.horizontal(|ui| {
            if ui.checkbox(&mut enabled, "Stamp solid").changed() {
                let state = crate::ui::editor_context::map_state_mut(ui_state);
                state.brush_stamp_solid = if enabled { Some(solid) } else { None };
            }
            if enabled {
                let label = if solid { "Solid" } else { "Passable" };
                if ui.selectable_label(solid, label).clicked() {
                    solid = !solid;
                    crate::ui::editor_context::map_state_mut(ui_state).brush_stamp_solid =
                        Some(solid);
                }
            }
        });
    }

    fn update_tile_property(
        _ui_state: &mut EditorUI,
        _config: Option<&EditorConfig>,
        _tile_name: &str,
        _solid: bool,
        _trigger: bool,
    ) {
        tracing::warn!("Editing tile collision properties from the map inspector is disabled in tileset-based mode");
    }

    pub(super) fn load_map_editor_brush_source(
        ui_state: &EditorUI,
        config: Option<&EditorConfig>,
    ) -> Option<crate::ui::editor_ui::LoadedMapEditorBrushSource> {
        crate::ui::editor_ui::load_map_editor_brush_source(ui_state, config)
    }

    pub(super) fn render_map_editor_selected_tile_preview(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        atlas: &toki_core::assets::atlas::AtlasMeta,
        texture_path: &std::path::Path,
        brush_entry: &crate::ui::editor_ui::MapEditorBrushEntry,
        renderer: Option<&mut crate::rendering::WindowRenderer>,
    ) {
        let Some(texture) = Self::ensure_map_editor_brush_preview_texture(
            ui_state,
            ctx,
            atlas,
            texture_path,
            renderer,
        ) else {
            return;
        };
        let Some(texture_size) = atlas.image_size() else {
            return;
        };
        let Some(preview_tile_id) = brush_entry.preview_tile_id.as_deref() else {
            return;
        };
        let Some(rect_px) = atlas.get_tile_rect(preview_tile_id) else {
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
            texture,
            rect.shrink(2.0),
            uv_rect,
            egui::Color32::WHITE,
        );
        response.on_hover_text(&brush_entry.display_label);
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
        _ctx: &egui::Context,
        atlas: &toki_core::assets::atlas::AtlasMeta,
        texture_path: &std::path::Path,
        renderer: Option<&mut crate::rendering::WindowRenderer>,
    ) -> Option<egui::TextureId> {
        let renderer = renderer?;
        let presentation_settings = ui_state.project.indexed_presentation_settings();
        let resolved_palette_id = toki_core::indexed_presentation::resolve_indexed_palette(
            atlas.color_mode,
            &ui_state.project.available_palettes,
            &presentation_settings,
            None,
            atlas.palette.as_deref(),
        )
        .ok()
        .flatten()
        .map(|(palette_id, _)| palette_id);
        let cache_key = toki_core::indexed_presentation::texture_preview_cache_key(
            &texture_path.display().to_string(),
            atlas.color_mode,
            resolved_palette_id.as_deref(),
            &presentation_settings.resolve_post_process(&ui_state.project.available_palettes),
        );

        if crate::ui::editor_context::map_state(ui_state)
            .brush_preview_cache_key
            .as_deref()
            == Some(cache_key.as_str())
            && crate::ui::editor_context::map_state(ui_state)
                .brush_preview_texture
                .is_some()
        {
            return crate::ui::editor_context::map_state(ui_state).brush_preview_texture;
        }

        let texture = renderer
            .preview_texture_from_path(
            texture_path,
            atlas.color_mode,
            &ui_state.project.available_palettes,
            &presentation_settings,
            None,
            atlas.palette.as_deref(),
        )
        .ok()?;
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_cache_key =
            Some(cache_key);
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_image_path =
            Some(texture_path.to_path_buf());
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture =
            Some(texture.texture_id);
        Some(texture.texture_id)
    }
}
