use super::EditorUI;
use crate::ui::hierarchy::HierarchySystem;

impl EditorUI {
    pub(super) fn render_standalone_maps_section(
        &mut self,
        ui: &mut egui::Ui,
        config: Option<&crate::config::EditorConfig>,
    ) {
        ui.add_space(10.0);
        egui::CollapsingHeader::new("Maps")
            .id_salt("asset_palette_maps_section")
            .default_open(false)
            .show(ui, |ui| {
                ui.separator();

                let Some(config) = config else {
                    return;
                };
                let Some(project_path) = config.current_project_path() else {
                    return;
                };

                let tilemaps_path = project_path.join("assets").join("tilemaps");
                if !tilemaps_path.exists() {
                    return;
                }

                let Ok(entries) = std::fs::read_dir(&tilemaps_path) else {
                    tracing::warn!("Could not read tilemaps directory");
                    return;
                };

                let mut map_selections: Vec<String> = Vec::new();
                let mut scene_map_additions: Vec<(String, String)> = Vec::new();

                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let Some(name) = file_name.to_str() else {
                        continue;
                    };
                    if !name.ends_with(".json") {
                        continue;
                    }

                    let map_name = name.trim_end_matches(".json").to_string();
                    let is_selected = matches!(
                        &self.selection,
                        Some(super::Selection::StandaloneMap(name)) if name == &map_name
                    );

                    let response = ui.selectable_label(is_selected, &map_name);

                    if response.clicked() {
                        tracing::info!("Map selected: {}", map_name);
                        map_selections.push(map_name.clone());
                    }

                    response.context_menu(|ui| {
                        ui.label("Add to Scene:");
                        ui.separator();

                        let scene_names: Vec<(String, bool)> = self
                            .scenes
                            .iter()
                            .map(|s| (s.name.clone(), s.maps.contains(&map_name)))
                            .collect();

                        for (scene_name, already_added) in scene_names {
                            if !already_added {
                                if ui.button(&scene_name).clicked() {
                                    scene_map_additions
                                        .push((scene_name.clone(), map_name.clone()));
                                    ui.close();
                                }
                            } else {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(format!("{} (already added)", scene_name)),
                                );
                            }
                        }

                        if self.scenes.is_empty() {
                            ui.label("No scenes available");
                        }
                    });
                }

                for map_name in map_selections {
                    self.set_selection(super::Selection::StandaloneMap(map_name));
                }

                for (scene_name, map_name) in scene_map_additions {
                    if let Some(target_scene) =
                        self.scenes.iter_mut().find(|s| s.name == scene_name)
                    {
                        target_scene.maps.push(map_name.clone());
                        tracing::info!("Added map '{}' to scene '{}'", map_name, scene_name);
                        self.scene_content_changed = true;
                    }
                }
            });
    }

    pub(super) fn render_entity_palette_section(
        &mut self,
        ui: &mut egui::Ui,
        config: Option<&crate::config::EditorConfig>,
    ) {
        ui.add_space(10.0);
        egui::CollapsingHeader::new("Entities")
            .id_salt("asset_palette_entities_section")
            .default_open(false)
            .show(ui, |ui| {
                ui.separator();

                let Some(config) = config else {
                    ui.label("No project configuration available for Entity palette");
                    return;
                };
                let Some(project_path) = config.current_project_path() else {
                    ui.label("No project loaded for Entity palette");
                    return;
                };

                let selected_entity =
                    HierarchySystem::render_entity_palette(ui, project_path, &self.selection);

                if let Some(selected_entity) = selected_entity {
                    self.set_selection(super::Selection::EntityDefinition(selected_entity));
                }
            });
    }

    pub(super) fn render_palette_assets_section(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        egui::CollapsingHeader::new("Palettes")
            .id_salt("asset_palette_palettes_section")
            .default_open(false)
            .show(ui, |ui| {
                ui.separator();
                self.render_palette_list(ui);
            });
    }

    fn render_palette_list(&mut self, ui: &mut egui::Ui) {
        let all_palettes = collect_all_palettes(&self.project.available_palettes);

        if all_palettes.is_empty() {
            ui.label("No palettes available.");
            return;
        }

        let grouped = group_palettes_by_size(&all_palettes);
        let mut selected_palette_id = None;

        for (size_label, entries) in &grouped {
            egui::CollapsingHeader::new(size_label.as_str())
                .id_salt(format!("palette_group_{size_label}"))
                .default_open(true)
                .show(ui, |ui| {
                    for (palette_id, is_builtin) in entries {
                        let label = if *is_builtin {
                            format!("{palette_id} (built-in)")
                        } else {
                            palette_id.clone()
                        };
                        let is_selected = matches!(
                            &self.selection,
                            Some(super::Selection::Palette(id)) if id == palette_id
                        );
                        if ui.selectable_label(is_selected, label).clicked() {
                            selected_palette_id = Some(palette_id.clone());
                        }
                    }
                });
            ui.add_space(5.0);
        }

        if let Some(palette_id) = selected_palette_id {
            self.set_selection(super::Selection::Palette(palette_id));
        }
    }
}

/// Collects all palettes (builtins + project) into a unified list with a
/// builtin flag.
fn collect_all_palettes(
    project_palettes: &std::collections::BTreeMap<String, toki_core::palette::Palette>,
) -> Vec<(String, toki_core::palette::Palette, bool)> {
    let builtins = toki_core::palette::builtin_palettes();
    let mut all: Vec<(String, toki_core::palette::Palette, bool)> = Vec::new();

    for (id, palette) in &builtins {
        all.push((id.clone(), palette.clone(), true));
    }
    for (id, palette) in project_palettes {
        if !builtins.contains_key(id) {
            all.push((id.clone(), palette.clone(), false));
        }
    }
    all.sort_by(|a, b| a.0.cmp(&b.0));
    all
}

/// Groups palettes by color count, returning sorted groups with a label like
/// "4 Colors", "16 Colors".
fn group_palettes_by_size(
    palettes: &[(String, toki_core::palette::Palette, bool)],
) -> Vec<(String, Vec<(String, bool)>)> {
    let mut groups: std::collections::BTreeMap<usize, Vec<(String, bool)>> =
        std::collections::BTreeMap::new();

    for (id, palette, is_builtin) in palettes {
        groups
            .entry(palette.size().color_count())
            .or_default()
            .push((id.clone(), *is_builtin));
    }

    groups
        .into_iter()
        .map(|(count, entries)| (format!("{count} Colors"), entries))
        .collect()
}
