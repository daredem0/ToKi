use super::*;

impl InspectorSystem {
    pub(super) fn render_scene_entity_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        config: Option<&EditorConfig>,
    ) -> bool {
        let mut changed = false;
        let is_static_item = draft.category == "item" && draft.static_object_sheet.is_some();

        ui.label("Scene Entity Properties");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Position:");
            changed |= ui
                .add(egui::DragValue::new(&mut draft.position_x).speed(1.0))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut draft.position_y).speed(1.0))
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.size_x)
                        .speed(1.0)
                        .range(1..=i64::MAX),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.size_y)
                        .speed(1.0)
                        .range(1..=i64::MAX),
                )
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Render Layer:");
            changed |= ui
                .add(egui::DragValue::new(&mut draft.render_layer).speed(1.0))
                .changed();
        });

        if let (Some(sheet), Some(object_name)) =
            (&draft.static_object_sheet, &draft.static_object_name)
        {
            ui.horizontal(|ui| {
                ui.label("Static Render:");
                ui.label(format!("{sheet}/{object_name}"));
            });
        }

        if !is_static_item {
            ui.horizontal(|ui| {
                ui.label("Speed:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.speed)
                            .speed(1.0)
                            .range(0..=i64::MAX),
                    )
                    .changed();
            });
        }

        changed |= ui.checkbox(&mut draft.visible, "Visible").changed();
        changed |= ui.checkbox(&mut draft.active, "Active").changed();
        changed |= ui.checkbox(&mut draft.solid, "Solid").changed();
        if !is_static_item {
            changed |= ui.checkbox(&mut draft.can_move, "Can Move").changed();
            ui.horizontal(|ui| {
                ui.label("Control Role:");
                egui::ComboBox::from_id_salt("entity_control_role")
                    .selected_text(control_role_label(draft.control_role))
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(&mut draft.control_role, ControlRole::None, "None")
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut draft.control_role,
                                ControlRole::PlayerCharacter,
                                "Player Character",
                            )
                            .changed();
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Movement:");
                egui::ComboBox::from_id_salt("entity_movement_profile")
                    .selected_text(movement_profile_label(
                        draft.control_role,
                        draft.movement_profile,
                    ))
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut draft.movement_profile,
                                MovementProfile::None,
                                "None",
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut draft.movement_profile,
                                MovementProfile::PlayerWasd,
                                "Player WASD",
                            )
                            .changed();
                    });
            });
            ui.horizontal(|ui| {
                ui.label("AI:");
                egui::ComboBox::from_id_salt("entity_ai_behavior")
                    .selected_text(ai_behavior_label(draft.ai_behavior))
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(&mut draft.ai_behavior, AiBehavior::None, "None")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut draft.ai_behavior, AiBehavior::Wander, "Wander")
                            .changed();
                    });
            });
        }
        changed |= ui
            .checkbox(&mut draft.has_inventory, "Has Inventory")
            .changed();

        if !is_static_item {
            ui.separator();
            ui.label("Audio");
            ui.horizontal(|ui| {
                ui.label("Movement Trigger:");
                egui::ComboBox::from_id_salt("entity_movement_sound_trigger")
                    .selected_text(movement_sound_trigger_label(draft.movement_sound_trigger))
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut draft.movement_sound_trigger,
                                MovementSoundTrigger::Distance,
                                "Distance",
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut draft.movement_sound_trigger,
                                MovementSoundTrigger::AnimationLoop,
                                "Animation Loop",
                            )
                            .changed();
                    });
            });
            let uses_distance_trigger =
                matches!(draft.movement_sound_trigger, MovementSoundTrigger::Distance);
            ui.horizontal(|ui| {
                ui.label("Footstep Distance:");
                ui.add_enabled_ui(uses_distance_trigger, |ui| {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut draft.footstep_trigger_distance)
                                .speed(0.5)
                                .range(0.0..=f32::MAX),
                        )
                        .changed();
                });
            });
            ui.horizontal(|ui| {
                ui.label("Movement Sound:");
                let mut sfx_names = config
                    .and_then(|cfg| cfg.current_project_path())
                    .map(|project_path| {
                        Self::discover_audio_asset_names(
                            project_path.join("assets/audio/sfx").as_path(),
                        )
                    })
                    .unwrap_or_default();
                if !draft.movement_sound.trim().is_empty()
                    && !sfx_names.iter().any(|name| name == &draft.movement_sound)
                {
                    sfx_names.push(draft.movement_sound.clone());
                    sfx_names.sort();
                    sfx_names.dedup();
                }
                egui::ComboBox::from_id_salt("entity_movement_sound")
                    .selected_text(if draft.movement_sound.trim().is_empty() {
                        "None".to_string()
                    } else {
                        draft.movement_sound.clone()
                    })
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(&mut draft.movement_sound, String::new(), "None")
                            .changed();
                        for sound_name in &sfx_names {
                            changed |= ui
                                .selectable_value(
                                    &mut draft.movement_sound,
                                    sound_name.clone(),
                                    sound_name,
                                )
                                .changed();
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Hearing Radius:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.hearing_radius)
                            .speed(1.0)
                            .range(0..=u32::MAX),
                    )
                    .changed();
            });
        }

        ui.separator();
        ui.label("Stats");
        ui.horizontal(|ui| {
            ui.label("Health:");
            changed |= ui.checkbox(&mut draft.health_enabled, "Enabled").changed();
            if draft.health_enabled {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.health_value)
                            .speed(1.0)
                            .range(0..=i64::MAX),
                    )
                    .changed();
            }
        });
        ui.horizontal(|ui| {
            ui.label("Attack Power:");
            changed |= ui
                .checkbox(&mut draft.attack_power_enabled, "Enabled")
                .changed();
            if draft.attack_power_enabled {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.attack_power_value)
                            .speed(1.0)
                            .range(0..=i64::MAX),
                    )
                    .changed();
            }
        });

        ui.separator();
        ui.label("Collision");
        changed |= ui
            .checkbox(&mut draft.collision_enabled, "Enabled")
            .changed();
        if draft.collision_enabled {
            ui.horizontal(|ui| {
                ui.label("Offset:");
                changed |= ui
                    .add(egui::DragValue::new(&mut draft.collision_offset_x).speed(1.0))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut draft.collision_offset_y).speed(1.0))
                    .changed();
            });

            ui.horizontal(|ui| {
                ui.label("Size:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.collision_size_x)
                            .speed(1.0)
                            .range(1..=i64::MAX),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.collision_size_y)
                            .speed(1.0)
                            .range(1..=i64::MAX),
                    )
                    .changed();
            });

            changed |= ui
                .checkbox(&mut draft.collision_trigger, "Trigger")
                .changed();
        }

        changed
    }

    pub(super) fn render_multi_scene_entity_editor(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            ui.label("No active scene");
            return false;
        };

        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            ui.label("Active scene not found");
            return false;
        };

        let selected_ids = ui_state.selected_entity_ids.clone();
        let selected_set: HashSet<_> = selected_ids.iter().copied().collect();
        let selected_entities = {
            let scene = &ui_state.scenes[scene_index];
            scene
                .entities
                .iter()
                .filter(|entity| selected_set.contains(&entity.id))
                .collect::<Vec<_>>()
        };

        if selected_entities.len() < 2 {
            ui.label("Select at least two scene entities for batch editing.");
            return false;
        }

        let common = Self::collect_multi_entity_common_state(&selected_entities);
        if ui_state.multi_entity_inspector_selection_signature != selected_ids {
            ui_state.multi_entity_inspector_selection_signature = selected_ids;
            ui_state.multi_entity_render_layer_input = common.render_layer.unwrap_or(0) as i64;
            ui_state.multi_entity_delta_x_input = 0;
            ui_state.multi_entity_delta_y_input = 0;
        }

        ui.label("Batch edit selected scene entities");
        ui.horizontal(|ui| {
            ui.label("Entities:");
            ui.label(selected_entities.len().to_string());
        });
        ui.separator();

        let mut edit = MultiEntityBatchEdit::default();
        Self::render_multi_entity_bool_row(
            ui,
            "Visible",
            common.visible,
            &mut edit.set_visible,
            "Set Visible",
            "Set Hidden",
        );
        Self::render_multi_entity_bool_row(
            ui,
            "Active",
            common.active,
            &mut edit.set_active,
            "Set Active",
            "Set Inactive",
        );
        Self::render_multi_entity_bool_row(
            ui,
            "Collision",
            common.collision_enabled,
            &mut edit.set_collision_enabled,
            "Enable Collision",
            "Disable Collision",
        );

        ui.horizontal(|ui| {
            ui.label(format!(
                "Render Layer: {}",
                common
                    .render_layer
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "Mixed".to_string())
            ));
            ui.add(egui::DragValue::new(&mut ui_state.multi_entity_render_layer_input).speed(1.0));
            if ui.button("Apply Layer").clicked() {
                edit.set_render_layer = Some(
                    ui_state
                        .multi_entity_render_layer_input
                        .clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                );
            }
        });

        ui.horizontal(|ui| {
            ui.label("Position Delta:");
            ui.add(egui::DragValue::new(&mut ui_state.multi_entity_delta_x_input).speed(1.0));
            ui.add(egui::DragValue::new(&mut ui_state.multi_entity_delta_y_input).speed(1.0));
            if ui.button("Apply Delta").clicked() {
                let delta = glam::IVec2::new(
                    ui_state.multi_entity_delta_x_input,
                    ui_state.multi_entity_delta_y_input,
                );
                if delta != glam::IVec2::ZERO {
                    edit.position_delta = Some(delta);
                }
                ui_state.multi_entity_delta_x_input = 0;
                ui_state.multi_entity_delta_y_input = 0;
            }
        });

        if edit.is_noop() {
            return false;
        }

        Self::apply_multi_entity_batch_edit_with_undo(
            ui_state,
            &active_scene_name,
            &selected_set,
            edit,
        )
    }

    pub(super) fn render_multi_entity_bool_row(
        ui: &mut egui::Ui,
        label: &str,
        common_value: Option<bool>,
        out_edit: &mut Option<bool>,
        true_button: &str,
        false_button: &str,
    ) {
        ui.horizontal(|ui| {
            let state_text = match common_value {
                Some(true) => "true",
                Some(false) => "false",
                None => "mixed",
            };
            ui.label(format!("{label}: {state_text}"));
            if ui.button(true_button).clicked() {
                *out_edit = Some(true);
            }
            if ui.button(false_button).clicked() {
                *out_edit = Some(false);
            }
        });
    }

    pub(super) fn collect_multi_entity_common_state(
        entities: &[&toki_core::entity::Entity],
    ) -> MultiEntityCommonState {
        fn common_bool(
            entities: &[&toki_core::entity::Entity],
            accessor: impl Fn(&toki_core::entity::Entity) -> bool,
        ) -> Option<bool> {
            let first = entities.first().map(|entity| accessor(entity))?;
            if entities.iter().all(|entity| accessor(entity) == first) {
                Some(first)
            } else {
                None
            }
        }

        fn common_i32(
            entities: &[&toki_core::entity::Entity],
            accessor: impl Fn(&toki_core::entity::Entity) -> i32,
        ) -> Option<i32> {
            let first = entities.first().map(|entity| accessor(entity))?;
            if entities.iter().all(|entity| accessor(entity) == first) {
                Some(first)
            } else {
                None
            }
        }

        MultiEntityCommonState {
            visible: common_bool(entities, |entity| entity.attributes.visible),
            active: common_bool(entities, |entity| entity.attributes.active),
            collision_enabled: common_bool(entities, |entity| entity.collision_box.is_some()),
            render_layer: common_i32(entities, |entity| entity.attributes.render_layer),
        }
    }

    pub(super) fn apply_multi_entity_batch_edit_with_undo(
        ui_state: &mut EditorUI,
        scene_name: &str,
        selected_set: &HashSet<toki_core::entity::EntityId>,
        edit: MultiEntityBatchEdit,
    ) -> bool {
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == scene_name)
        else {
            return false;
        };

        let before_entities = ui_state.scenes[scene_index]
            .entities
            .iter()
            .filter(|entity| selected_set.contains(&entity.id))
            .cloned()
            .collect::<Vec<_>>();

        if before_entities.is_empty() {
            return false;
        }

        let mut changed = false;
        let mut after_entities = Vec::with_capacity(before_entities.len());
        for before_entity in &before_entities {
            let mut after_entity = before_entity.clone();
            changed |= Self::apply_multi_entity_batch_edit_to_entity(&mut after_entity, edit);
            after_entities.push(after_entity);
        }

        if !changed {
            return false;
        }

        ui_state.execute_command(EditorCommand::update_entities(
            scene_name.to_string(),
            before_entities,
            after_entities,
        ))
    }

    pub(super) fn apply_multi_entity_batch_edit_to_entity(
        entity: &mut toki_core::entity::Entity,
        edit: MultiEntityBatchEdit,
    ) -> bool {
        let mut changed = false;

        if let Some(visible) = edit.set_visible {
            if entity.attributes.visible != visible {
                entity.attributes.visible = visible;
                changed = true;
            }
        }

        if let Some(active) = edit.set_active {
            if entity.attributes.active != active {
                entity.attributes.active = active;
                changed = true;
            }
        }

        if let Some(render_layer) = edit.set_render_layer {
            if entity.attributes.render_layer != render_layer {
                entity.attributes.render_layer = render_layer;
                changed = true;
            }
        }

        if let Some(delta) = edit.position_delta {
            let new_position = entity.position + delta;
            if entity.position != new_position {
                entity.position = new_position;
                changed = true;
            }
        }

        if let Some(collision_enabled) = edit.set_collision_enabled {
            if collision_enabled {
                if entity.collision_box.is_none() {
                    entity.collision_box =
                        Some(toki_core::collision::CollisionBox::solid_box(entity.size));
                    changed = true;
                }
            } else if entity.collision_box.is_some() {
                entity.collision_box = None;
                changed = true;
            }
        }

        changed
    }

    pub(super) fn render_runtime_entity_read_only(
        ui: &mut egui::Ui,
        game_state: Option<&toki_core::GameState>,
        entity_id: toki_core::entity::EntityId,
    ) {
        if let Some(game_state) = game_state {
            if let Some(entity) = game_state.entity_manager().get_entity(entity_id) {
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.label(format!("({}, {})", entity.position.x, entity.position.y));
                });

                ui.horizontal(|ui| {
                    ui.label("Size:");
                    ui.label(format!("{}x{}", entity.size.x, entity.size.y));
                });

                ui.horizontal(|ui| {
                    ui.label("Type:");
                    if entity.category.is_empty() {
                        ui.label(format!("{:?}", entity.entity_kind));
                    } else {
                        ui.label(entity.category.as_str());
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Control Role:");
                    ui.label(control_role_label(entity.control_role));
                });

                ui.horizontal(|ui| {
                    ui.label("Visible:");
                    ui.label(format!("{}", entity.attributes.visible));
                });

                ui.horizontal(|ui| {
                    ui.label("Active:");
                    ui.label(format!("{}", entity.attributes.active));
                });

                if let Some(health) = entity.attributes.health {
                    ui.horizontal(|ui| {
                        ui.label("Health:");
                        ui.label(format!("{}", health));
                    });
                }
                if let Some(attack_power) = entity.attributes.current_stat(ATTACK_POWER_STAT_ID) {
                    ui.horizontal(|ui| {
                        ui.label("Attack Power:");
                        ui.label(format!("{}", attack_power));
                    });
                }

                if entity.attributes.has_inventory {
                    ui.horizontal(|ui| {
                        ui.label("Has Inventory:");
                        ui.label("Yes");
                    });
                }
                let is_static_item =
                    entity.category == "item" && entity.attributes.static_object_render.is_some();
                if let Some(static_render) = &entity.attributes.static_object_render {
                    ui.horizontal(|ui| {
                        ui.label("Static Render:");
                        ui.label(format!(
                            "{}/{}",
                            static_render.sheet, static_render.object_name
                        ));
                    });
                }
                if !is_static_item {
                    ui.horizontal(|ui| {
                        ui.label("AI:");
                        ui.label(ai_behavior_label(entity.attributes.ai_behavior));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Movement:");
                        ui.label(movement_profile_label(
                            entity.control_role,
                            entity.attributes.movement_profile,
                        ));
                    });
                }

                if let Some(collision_box) = &entity.collision_box {
                    ui.separator();
                    ui.label("Collision Box:");
                    ui.horizontal(|ui| {
                        ui.label("Offset:");
                        ui.label(format!(
                            "({}, {})",
                            collision_box.offset.x, collision_box.offset.y
                        ));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        ui.label(format!("{}x{}", collision_box.size.x, collision_box.size.y));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Trigger:");
                        ui.label(format!("{}", collision_box.trigger));
                    });
                }

                if let Some(animation_controller) = &entity.attributes.animation_controller {
                    ui.separator();
                    ui.label("Animation:");
                    ui.horizontal(|ui| {
                        ui.label("Current State:");
                        ui.label(format!("{:?}", animation_controller.current_clip_state));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Frame:");
                        ui.label(format!("{}", animation_controller.current_frame_index));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Finished:");
                        ui.label(format!("{}", animation_controller.is_finished));
                    });
                }
            } else {
                ui.label("❌ Entity not found in game state");
            }
        } else {
            ui.label("❌ No game state available");
        }
    }

    pub(super) fn find_selected_scene_entity(
        ui_state: &EditorUI,
        entity_id: toki_core::entity::EntityId,
    ) -> Option<toki_core::entity::Entity> {
        let active_scene_name = ui_state.active_scene.clone()?;
        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == active_scene_name)?;
        scene
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
            .cloned()
    }

    pub(super) fn apply_entity_property_draft_with_undo(
        ui_state: &mut EditorUI,
        entity_id: toki_core::entity::EntityId,
        draft: &EntityPropertyDraft,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            return false;
        };
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            return false;
        };
        let Some(entity_index) = ui_state.scenes[scene_index]
            .entities
            .iter()
            .position(|entity| entity.id == entity_id)
        else {
            return false;
        };

        let before = ui_state.scenes[scene_index].entities[entity_index].clone();
        let mut after = before.clone();
        let mut changed = Self::apply_entity_property_draft(&mut after, draft);

        let mut before_entities = vec![before];
        let mut after_entities = vec![after.clone()];

        if matches!(after.control_role, ControlRole::PlayerCharacter) {
            for other in ui_state.scenes[scene_index].entities.iter() {
                if other.id == entity_id {
                    continue;
                }
                if matches!(
                    other.effective_control_role(),
                    toki_core::entity::ControlRole::PlayerCharacter
                ) {
                    let mut demoted = other.clone();
                    demoted.control_role = ControlRole::None;
                    before_entities.push(other.clone());
                    after_entities.push(demoted);
                    changed = true;
                }
            }
        }

        if !changed {
            return false;
        }

        ui_state.execute_command(EditorCommand::update_entities(
            active_scene_name,
            before_entities,
            after_entities,
        ))
    }

    pub(super) fn apply_entity_property_draft(
        entity: &mut toki_core::entity::Entity,
        draft: &EntityPropertyDraft,
    ) -> bool {
        fn set_if_changed<T: PartialEq>(target: &mut T, value: T) -> bool {
            if *target != value {
                *target = value;
                true
            } else {
                false
            }
        }

        fn clamp_to_non_negative_u32(value: i64) -> u32 {
            value.clamp(0, u32::MAX as i64) as u32
        }

        fn clamp_to_min_one_u32(value: i64) -> u32 {
            value.clamp(1, u32::MAX as i64) as u32
        }

        let mut changed = false;

        let new_position = glam::IVec2::new(draft.position_x, draft.position_y);
        changed |= set_if_changed(&mut entity.position, new_position);

        let new_size = glam::UVec2::new(
            clamp_to_min_one_u32(draft.size_x),
            clamp_to_min_one_u32(draft.size_y),
        );
        changed |= set_if_changed(&mut entity.size, new_size);

        changed |= set_if_changed(&mut entity.attributes.visible, draft.visible);
        changed |= set_if_changed(&mut entity.attributes.active, draft.active);
        changed |= set_if_changed(&mut entity.attributes.solid, draft.solid);
        changed |= set_if_changed(&mut entity.attributes.can_move, draft.can_move);
        changed |= set_if_changed(&mut entity.control_role, draft.control_role);
        changed |= set_if_changed(&mut entity.attributes.ai_behavior, draft.ai_behavior);
        changed |= set_if_changed(
            &mut entity.attributes.movement_profile,
            draft.movement_profile,
        );
        changed |= set_if_changed(
            &mut entity.audio.movement_sound_trigger,
            draft.movement_sound_trigger,
        );
        changed |= set_if_changed(
            &mut entity.audio.footstep_trigger_distance,
            draft.footstep_trigger_distance.max(0.0),
        );
        changed |= set_if_changed(&mut entity.audio.hearing_radius, draft.hearing_radius);
        let new_movement_sound = {
            let trimmed = draft.movement_sound.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        changed |= set_if_changed(&mut entity.audio.movement_sound, new_movement_sound);
        changed |= set_if_changed(&mut entity.attributes.has_inventory, draft.has_inventory);
        changed |= set_if_changed(
            &mut entity.attributes.speed,
            clamp_to_non_negative_u32(draft.speed),
        );
        changed |= set_if_changed(&mut entity.attributes.render_layer, draft.render_layer);

        let new_health = if draft.health_enabled {
            Some(clamp_to_non_negative_u32(draft.health_value))
        } else {
            None
        };
        changed |= set_if_changed(&mut entity.attributes.health, new_health);
        changed |= Self::set_optional_runtime_stat(
            &mut entity.attributes,
            HEALTH_STAT_ID,
            new_health.map(|value| value as i32),
        );

        let new_attack_power = if draft.attack_power_enabled {
            Some(draft.attack_power_value.clamp(0, i32::MAX as i64) as i32)
        } else {
            None
        };
        changed |= Self::set_optional_runtime_stat(
            &mut entity.attributes,
            ATTACK_POWER_STAT_ID,
            new_attack_power,
        );

        if draft.collision_enabled {
            if entity.collision_box.is_none() {
                entity.collision_box =
                    Some(toki_core::collision::CollisionBox::solid_box(entity.size));
                changed = true;
            }

            if let Some(collision_box) = entity.collision_box.as_mut() {
                changed |= set_if_changed(
                    &mut collision_box.offset,
                    glam::IVec2::new(draft.collision_offset_x, draft.collision_offset_y),
                );
                changed |= set_if_changed(
                    &mut collision_box.size,
                    glam::UVec2::new(
                        clamp_to_min_one_u32(draft.collision_size_x),
                        clamp_to_min_one_u32(draft.collision_size_y),
                    ),
                );
                changed |= set_if_changed(&mut collision_box.trigger, draft.collision_trigger);
            }
        } else if entity.collision_box.is_some() {
            entity.collision_box = None;
            changed = true;
        }

        changed
    }
}
