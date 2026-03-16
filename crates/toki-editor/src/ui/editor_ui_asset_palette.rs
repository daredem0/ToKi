use super::EditorUI;
use crate::ui::hierarchy::HierarchySystem;
use crate::ui::undo_redo::EditorCommand;

impl EditorUI {
    pub(super) fn render_standalone_maps_section(
        &mut self,
        ui: &mut egui::Ui,
        config: Option<&crate::config::EditorConfig>,
    ) {
        ui.add_space(10.0);
        ui.heading("🗺️ Maps");
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
                            scene_map_additions.push((scene_name.clone(), map_name.clone()));
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
            if let Some(target_scene) = self.scenes.iter_mut().find(|s| s.name == scene_name) {
                target_scene.maps.push(map_name.clone());
                tracing::info!("Added map '{}' to scene '{}'", map_name, scene_name);
                self.scene_content_changed = true;
            }
        }
    }

    pub(super) fn render_entity_palette_section(
        &mut self,
        ui: &mut egui::Ui,
        config: Option<&crate::config::EditorConfig>,
    ) {
        ui.add_space(10.0);
        ui.heading("🧙 Entities");
        ui.separator();

        let Some(config) = config else {
            ui.label("No project configuration available for Entity palette");
            return;
        };
        let Some(project_path) = config.current_project_path() else {
            ui.label("No project loaded for Entity palette");
            return;
        };

        let (selected_entity, entity_additions, placement_request) =
            HierarchySystem::render_entity_palette(ui, project_path, &self.selection, &self.scenes);

        if let Some(selected_entity) = selected_entity {
            self.set_selection(super::Selection::EntityDefinition(selected_entity));
        }

        if let Some(entity_definition) = placement_request {
            self.enter_placement_mode(entity_definition);
        }

        for (scene_name, entity_name) in entity_additions {
            self.add_entity_definition_to_scene(config, &scene_name, &entity_name);
        }
    }

    fn add_entity_definition_to_scene(
        &mut self,
        config: &crate::config::EditorConfig,
        scene_name: &str,
        entity_name: &str,
    ) {
        let Some(project_path) = config.current_project_path() else {
            tracing::error!("No project path available for entity creation");
            return;
        };

        let entity_file = project_path
            .join("entities")
            .join(format!("{}.json", entity_name));
        if !entity_file.exists() {
            tracing::error!("Entity definition file not found: {:?}", entity_file);
            return;
        }

        let content = match std::fs::read_to_string(&entity_file) {
            Ok(content) => content,
            Err(error) => {
                tracing::error!("Failed to read entity file '{}': {}", entity_name, error);
                return;
            }
        };

        let entity_def = match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content)
        {
            Ok(entity_def) => entity_def,
            Err(error) => {
                tracing::error!(
                    "Failed to parse entity definition '{}': {}",
                    entity_name,
                    error
                );
                return;
            }
        };

        let Some(scene_index) = self
            .scenes
            .iter()
            .position(|scene| scene.name == scene_name)
        else {
            return;
        };

        let new_id = self.scenes[scene_index]
            .entities
            .iter()
            .map(|entity| entity.id)
            .max()
            .unwrap_or(0)
            + 1;

        let default_position = glam::IVec2::new(100, 100);

        match entity_def.create_entity(default_position, new_id) {
            Ok(entity) => {
                if self.execute_command(EditorCommand::add_entity(scene_name.to_string(), entity)) {
                    tracing::info!(
                        "Successfully added entity '{}' (ID: {}) to scene '{}' at position ({}, {})",
                        entity_name,
                        new_id,
                        scene_name,
                        default_position.x,
                        default_position.y
                    );
                }
            }
            Err(error) => {
                tracing::error!("Failed to create entity '{}': {}", entity_name, error);
            }
        }
    }
}
