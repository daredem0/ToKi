use super::EditorUI;
use crate::ui::hierarchy::HierarchySystem;
use crate::ui::undo_redo::{EditorCommand, IndexedEntity};

impl EditorUI {
    pub fn render_hierarchy_and_maps_combined_panel(
        &mut self,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        config: Option<&crate::config::EditorConfig>,
    ) {
        egui::SidePanel::left("hierarchy_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("📋 Scene Hierarchy");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("hierarchy_scroll")
                    .show(ui, |ui| {
                        let mut selection_changes = Vec::new();
                        let mut active_scene_change = None;
                        let mut map_removals: Vec<(usize, usize)> = Vec::new();
                        let mut entity_removals: Vec<(String, u32)> = Vec::new();

                        for (scene_index, scene) in self.scenes.iter().enumerate() {
                            let is_active_scene = self.active_scene.as_ref() == Some(&scene.name);
                            let scene_header = if is_active_scene {
                                egui::RichText::new(format!("🎬 {}", scene.name))
                                    .color(egui::Color32::from_rgb(132, 211, 132))
                            } else {
                                egui::RichText::new(format!("🎬 {}", scene.name))
                            };
                            let scene_header_response = egui::CollapsingHeader::new(scene_header)
                                .id_salt(format!("scene_{}", scene.name))
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{} entities", scene.entities.len()));
                                        ui.label(format!("{} maps", scene.maps.len()));
                                    });

                                    if !scene.maps.is_empty() {
                                        ui.label("Maps:");
                                        ui.indent(format!("maps_{}", scene.name), |ui| {
                                            for (map_index, map_name) in scene.maps.iter().enumerate() {
                                                let is_selected = matches!(
                                                    &self.selection,
                                                    Some(super::Selection::Map(scene_name, selected_map))
                                                        if scene_name == &scene.name && selected_map == map_name
                                                );

                                                ui.horizontal(|ui| {
                                                    let response = ui.selectable_label(
                                                        is_selected,
                                                        format!("🗺️ {}", map_name),
                                                    );

                                                    if response.clicked() {
                                                        selection_changes.push(super::Selection::Map(
                                                            scene.name.clone(),
                                                            map_name.clone(),
                                                        ));
                                                        tracing::info!(
                                                            "Selected map: {} in scene: {}",
                                                            map_name,
                                                            scene.name
                                                        );
                                                    }

                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            if ui
                                                                .small_button("🗑️")
                                                                .on_hover_text("Remove map from scene")
                                                                .clicked()
                                                            {
                                                                map_removals.push((scene_index, map_index));
                                                            }
                                                        },
                                                    );
                                                });
                                            }
                                        });
                                    }

                                    if !scene.entities.is_empty() {
                                        ui.label("Scene Entities:");
                                        ui.indent(format!("entities_{}", scene.name), |ui| {
                                            for entity in &scene.entities {
                                                let is_selected = matches!(
                                                    &self.selection,
                                                    Some(super::Selection::Entity(id)) if id == &entity.id
                                                );

                                                ui.horizontal(|ui| {
                                                    let kind_label = entity
                                                        .definition_name
                                                        .clone()
                                                        .or_else(|| {
                                                            if entity.category.is_empty() {
                                                                None
                                                            } else {
                                                                Some(entity.category.clone())
                                                            }
                                                        })
                                                        .unwrap_or_else(|| format!("{:?}", entity.entity_kind));
                                                    let entity_display = if matches!(
                                                        entity.effective_control_role(),
                                                        toki_core::entity::ControlRole::PlayerCharacter
                                                    ) {
                                                        format!(
                                                            "👤 {} (Player Character, ID: {})",
                                                            kind_label, entity.id
                                                        )
                                                    } else {
                                                        format!("🧩 {} (ID: {})", kind_label, entity.id)
                                                    };

                                                    let response = ui.selectable_label(is_selected, entity_display);

                                                    if response.clicked() {
                                                        selection_changes.push(super::Selection::Entity(entity.id));
                                                        tracing::info!("Selected scene entity ID: {}", entity.id);
                                                    }

                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(egui::Align::Center),
                                                        |ui| {
                                                            ui.label(format!(
                                                                "({}, {})",
                                                                entity.position.x, entity.position.y
                                                            ));
                                                        },
                                                    );
                                                });

                                                ui.horizontal(|ui| {
                                                    ui.add_space(20.0);
                                                    if ui
                                                        .small_button("🗑️")
                                                        .on_hover_text("Remove from scene")
                                                        .clicked()
                                                    {
                                                        entity_removals.push((scene.name.clone(), entity.id));
                                                    }
                                                });
                                            }
                                        });
                                    }

                                    ui.label("Runtime Entities:");
                                    ui.indent("scene_runtime_entities", |ui| {
                                        if let Some(game_state) = game_state {
                                            let entity_ids = game_state.entity_manager().active_entities();

                                            if entity_ids.is_empty() {
                                                ui.label("No runtime entities");
                                            } else {
                                                for entity_id in &entity_ids {
                                                    if let Some(entity) =
                                                        game_state.entity_manager().get_entity(*entity_id)
                                                    {
                                                        let is_selected = matches!(
                                                            &self.selection,
                                                            Some(super::Selection::Entity(id)) if id == entity_id
                                                        );

                                                        ui.horizontal(|ui| {
                                                            let response = ui.selectable_label(
                                                                is_selected,
                                                                format!("⚙️ Runtime Entity {}", entity_id),
                                                            );

                                                            if response.clicked() {
                                                                selection_changes.push(super::Selection::Entity(*entity_id));
                                                                self.selected_entity_id = Some(*entity_id);
                                                            }

                                                            ui.with_layout(
                                                                egui::Layout::right_to_left(
                                                                    egui::Align::Center,
                                                                ),
                                                                |ui| {
                                                                    ui.label(format!(
                                                                        "({}, {})",
                                                                        entity.position.x,
                                                                        entity.position.y
                                                                    ));
                                                                },
                                                            );
                                                        });
                                                    }
                                                }
                                            }
                                        } else {
                                            ui.label("No game state available");
                                        }
                                    });
                                });

                            if scene_header_response.header_response.clicked() {
                                selection_changes.push(super::Selection::Scene(scene.name.clone()));
                                tracing::info!("Selected scene: {}", scene.name);
                            }

                            scene_header_response.header_response.context_menu(|ui| {
                                ui.horizontal(|ui| {
                                    if is_active_scene {
                                        ui.label("✅ Active Scene");
                                    } else if ui.button("🎯 Set as Active Scene").clicked() {
                                        active_scene_change = Some(scene.name.clone());
                                        tracing::info!("Setting {} as active scene", scene.name);
                                        ui.close();
                                    }
                                });
                            });
                        }

                        map_removals.sort_by(|a, b| b.1.cmp(&a.1));
                        for (scene_index, map_index) in map_removals {
                            if let Some(scene) = self.scenes.get_mut(scene_index) {
                                if map_index < scene.maps.len() {
                                    let removed_map = scene.maps.remove(map_index);
                                    if matches!(&self.selection, Some(super::Selection::Map(s, m)) if s == &scene.name && m == &removed_map) {
                                        self.clear_selection();
                                    }
                                }
                            }
                        }

                        for (scene_name, entity_id) in entity_removals {
                            let Some(scene_index) =
                                self.scenes.iter().position(|scene| scene.name == scene_name)
                            else {
                                continue;
                            };
                            let Some((index, entity)) = self.scenes[scene_index]
                                .entities
                                .iter()
                                .enumerate()
                                .find(|(_, entity)| entity.id == entity_id)
                                .map(|(index, entity)| (index, entity.clone()))
                            else {
                                continue;
                            };

                            let removed = self.execute_command(EditorCommand::remove_entities(
                                scene_name.clone(),
                                vec![IndexedEntity { index, entity }],
                            ));
                            if removed {
                                tracing::info!("Removed entity {} from scene {}", entity_id, scene_name);

                                if matches!(&self.selection, Some(super::Selection::Entity(id)) if id == &entity_id) {
                                    self.clear_selection();
                                }
                            }
                        }

                        if let Some(selection) = selection_changes.last() {
                            self.set_selection(selection.clone());
                        }

                        if let Some(new_active_scene) = active_scene_change {
                            self.active_scene = Some(new_active_scene);
                        }

                        ui.separator();

                        if ui.button("+ Add Scene").clicked() {
                            let new_scene_name = format!("Scene {}", self.scenes.len() + 1);
                            self.add_scene(new_scene_name.clone());
                            tracing::info!("Created new scene: {}", new_scene_name);
                        }

                        if self.show_maps {
                            ui.add_space(10.0);
                            ui.heading("🗺️ Maps");
                            ui.separator();

                            if let Some(config) = config {
                                if let Some(project_path) = config.current_project_path() {
                                    let tilemaps_path = project_path.join("assets").join("tilemaps");

                                    if tilemaps_path.exists() {
                                        if let Ok(entries) = std::fs::read_dir(&tilemaps_path) {
                                            let mut map_selections: Vec<String> = Vec::new();
                                            let mut scene_map_additions: Vec<(String, String)> = Vec::new();

                                            for entry in entries.flatten() {
                                                if let Some(name) = entry.file_name().to_str() {
                                                    if name.ends_with(".json") {
                                                        let map_name =
                                                            name.trim_end_matches(".json").to_string();

                                                        let is_selected = matches!(
                                                            &self.selection,
                                                            Some(super::Selection::StandaloneMap(name)) if name == &map_name
                                                        );

                                                        let response =
                                                            ui.selectable_label(is_selected, &map_name);

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
                                                                .map(|s| {
                                                                    (
                                                                        s.name.clone(),
                                                                        s.maps.contains(&map_name),
                                                                    )
                                                                })
                                                                .collect();

                                                            for (scene_name, already_added) in scene_names {
                                                                if !already_added {
                                                                    if ui.button(&scene_name).clicked() {
                                                                        scene_map_additions.push((
                                                                            scene_name.clone(),
                                                                            map_name.clone(),
                                                                        ));
                                                                        ui.close();
                                                                    }
                                                                } else {
                                                                    ui.add_enabled(
                                                                        false,
                                                                        egui::Button::new(format!(
                                                                            "{} (already added)",
                                                                            scene_name
                                                                        )),
                                                                    );
                                                                }
                                                            }

                                                            if self.scenes.is_empty() {
                                                                ui.label("No scenes available");
                                                            }
                                                        });
                                                    }
                                                }
                                            }

                                            for map_name in map_selections {
                                                self.set_selection(super::Selection::StandaloneMap(map_name));
                                            }

                                            for (scene_name, map_name) in scene_map_additions {
                                                if let Some(target_scene) =
                                                    self.scenes.iter_mut().find(|s| s.name == scene_name)
                                                {
                                                    target_scene.maps.push(map_name.clone());
                                                    tracing::info!(
                                                        "Added map '{}' to scene '{}'",
                                                        map_name,
                                                        scene_name
                                                    );
                                                    self.scene_content_changed = true;
                                                }
                                            }
                                        } else {
                                            tracing::warn!("Could not read tilemaps directory");
                                        }
                                    }
                                }
                            }
                        }

                        ui.add_space(10.0);
                        ui.heading("🧙 Entities");
                        ui.separator();

                        if let Some(config) = config {
                            if let Some(project_path) = config.current_project_path() {
                                let (selected_entity, entity_additions, placement_request) =
                                    HierarchySystem::render_entity_palette(
                                        ui,
                                        project_path,
                                        &self.selection,
                                        &self.scenes,
                                    );

                                if let Some(selected_entity) = selected_entity {
                                    self.set_selection(super::Selection::EntityDefinition(selected_entity));
                                }

                                if let Some(entity_definition) = placement_request {
                                    self.enter_placement_mode(entity_definition);
                                }

                                for (scene_name, entity_name) in entity_additions {
                                    if let Some(project_path) = config.current_project_path() {
                                        let entity_file = project_path
                                            .join("entities")
                                            .join(format!("{}.json", entity_name));

                                        if entity_file.exists() {
                                            match std::fs::read_to_string(&entity_file) {
                                                Ok(content) => {
                                                    match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content) {
                                                        Ok(entity_def) => {
                                                            let Some(scene_index) = self
                                                                .scenes
                                                                .iter()
                                                                .position(|scene| scene.name == scene_name)
                                                            else {
                                                                continue;
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
                                                                    if self.execute_command(
                                                                        EditorCommand::add_entity(
                                                                            scene_name.clone(),
                                                                            entity,
                                                                        ),
                                                                    ) {
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
                                                                    tracing::error!(
                                                                        "Failed to create entity '{}': {}",
                                                                        entity_name,
                                                                        error
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Err(error) => {
                                                            tracing::error!(
                                                                "Failed to parse entity definition '{}': {}",
                                                                entity_name,
                                                                error
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(error) => {
                                                    tracing::error!(
                                                        "Failed to read entity file '{}': {}",
                                                        entity_name,
                                                        error
                                                    );
                                                }
                                            }
                                        } else {
                                            tracing::error!(
                                                "Entity definition file not found: {:?}",
                                                entity_file
                                            );
                                        }
                                    } else {
                                        tracing::error!("No project path available for entity creation");
                                    }
                                }
                            } else {
                                ui.label("No project loaded for Entity palette");
                            }
                        } else {
                            ui.label("No project configuration available for Entity palette");
                        }
                    });
            });
    }
}
