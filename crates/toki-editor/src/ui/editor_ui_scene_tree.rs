use super::EditorUI;
use crate::ui::undo_redo::{EditorCommand, IndexedEntity};

impl EditorUI {
    fn is_scene_item_entity(entity: &toki_core::entity::Entity) -> bool {
        entity.category == "item" || entity.attributes.pickup.is_some()
    }

    fn render_scene_entity_group(
        ui: &mut egui::Ui,
        scene_name: &str,
        label: &str,
        entities: &[&toki_core::entity::Entity],
        current_selection: &Option<super::Selection>,
        selection_changes: &mut Vec<super::Selection>,
        entity_removals: &mut Vec<(String, u32)>,
    ) {
        if entities.is_empty() {
            return;
        }

        egui::CollapsingHeader::new(label)
            .id_salt(format!(
                "{}_{}",
                label.replace(' ', "_").to_lowercase(),
                scene_name
            ))
            .default_open(false)
            .show(ui, |ui| {
                for entity in entities {
                    let is_selected = matches!(
                        current_selection,
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
                            format!("Player: {} (ID: {})", kind_label, entity.id)
                        } else if Self::is_scene_item_entity(entity) {
                            format!("Item: {} (ID: {})", kind_label, entity.id)
                        } else {
                            format!("Entity: {} (ID: {})", kind_label, entity.id)
                        };

                        let response = ui.selectable_label(is_selected, entity_display);

                        if response.clicked() {
                            selection_changes.push(super::Selection::Entity(entity.id));
                            tracing::info!("Selected scene entity ID: {}", entity.id);
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button("🗑")
                                .on_hover_text("Remove from scene")
                                .clicked()
                            {
                                entity_removals.push((scene_name.to_string(), entity.id));
                            }
                            ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                        });
                    });
                }
            });
    }

    pub(super) fn render_scene_hierarchy_section(
        &mut self,
        ui: &mut egui::Ui,
        game_state: Option<&toki_core::GameState>,
    ) {
        let mut selection_changes = Vec::new();
        let mut active_scene_change = None;
        let mut map_removals: Vec<(usize, usize)> = Vec::new();
        let mut entity_removals: Vec<(String, u32)> = Vec::new();

        for (scene_index, scene) in self.scenes.iter().enumerate() {
            let is_active_scene = self.active_scene.as_ref() == Some(&scene.name);
            let scene_header = if is_active_scene {
                egui::RichText::new(format!("Scene: {}", scene.name))
                    .color(egui::Color32::from_rgb(132, 211, 132))
            } else {
                egui::RichText::new(format!("Scene: {}", scene.name))
            };
            let scene_header_response = egui::CollapsingHeader::new(scene_header)
                .id_salt(format!("scene_{}", scene.name))
                .default_open(false)
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
                                    let response =
                                        ui.selectable_label(is_selected, format!("🗺 {}", map_name));

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
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui
                                                .small_button("🗑")
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
                        let (scene_items, scene_entities): (Vec<_>, Vec<_>) = scene
                            .entities
                            .iter()
                            .partition(|entity| Self::is_scene_item_entity(entity));

                        Self::render_scene_entity_group(
                            ui,
                            &scene.name,
                            "Scene Entities:",
                            &scene_entities,
                            &self.selection,
                            &mut selection_changes,
                            &mut entity_removals,
                        );
                        Self::render_scene_entity_group(
                            ui,
                            &scene.name,
                            "Scene Items:",
                            &scene_items,
                            &self.selection,
                            &mut selection_changes,
                            &mut entity_removals,
                        );
                    }

                    if self.show_runtime_entities {
                        egui::CollapsingHeader::new("Runtime Entities:")
                            .id_salt(format!("runtime_entities_{}", scene.name))
                            .default_open(false)
                            .show(ui, |ui| {
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
                                                    Some(super::Selection::Entity(id))
                                                        if id == entity_id
                                                );

                                                ui.horizontal(|ui| {
                                                    let response = ui.selectable_label(
                                                        is_selected,
                                                        format!("Runtime: Entity {}", entity_id),
                                                    );

                                                    if response.clicked() {
                                                        selection_changes.push(
                                                            super::Selection::Entity(*entity_id),
                                                        );
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
                    }
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
                    if matches!(&self.selection, Some(super::Selection::Map(s, m)) if s == &scene.name && m == &removed_map)
                    {
                        self.clear_selection();
                    }
                }
            }
        }

        for (scene_name, entity_id) in entity_removals {
            let Some(scene_index) = self
                .scenes
                .iter()
                .position(|scene| scene.name == scene_name)
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

                if matches!(&self.selection, Some(super::Selection::Entity(id)) if id == &entity_id)
                {
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
    }
}
