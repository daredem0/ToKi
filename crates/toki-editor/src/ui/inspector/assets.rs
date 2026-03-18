use super::*;
use crate::ui::editor_ui::MapLoadRequest;

impl InspectorSystem {
    pub(super) fn render_map_details(
        ui: &mut egui::Ui,
        map_name: &str,
        config: Option<&EditorConfig>,
        scene_name: Option<&str>,
        map_load_requested: &mut Option<MapLoadRequest>,
    ) {
        // Try to load and show map details
        if let Some(config) = config {
            if let Some(project_path) = config.current_project_path() {
                let map_file = project_path
                    .join("assets")
                    .join("tilemaps")
                    .join(format!("{}.json", map_name));

                if map_file.exists() {
                    // Try to read the tilemap file
                    match std::fs::read_to_string(&map_file) {
                        Ok(content) => {
                            // Try to parse as JSON to show basic info
                            match serde_json::from_str::<serde_json::Value>(&content) {
                                Ok(json) => {
                                    // Show file info
                                    ui.horizontal(|ui| {
                                        ui.label("File:");
                                        ui.label(format!("{}.json", map_name));
                                    });

                                    // Show file size
                                    ui.horizontal(|ui| {
                                        ui.label("Size:");
                                        ui.label(format!("{} bytes", content.len()));
                                    });

                                    // Show JSON properties and values
                                    if let Some(obj) = json.as_object() {
                                        ui.horizontal(|ui| {
                                            ui.label("Properties:");
                                            ui.label(format!("{}", obj.keys().count()));
                                        });

                                        ui.separator();
                                        ui.label("Map Properties:");

                                        egui::ScrollArea::vertical()
                                            .id_salt("map_properties_scroll")
                                            .max_height(200.0)
                                            .show(ui, |ui| {
                                                for (key, value) in obj {
                                                    ui.horizontal(|ui| {
                                                        ui.label(format!("{}:", key));

                                                        // Format value based on type
                                                        let value_str = match value {
                                                            serde_json::Value::String(s) => {
                                                                format!("\"{}\"", s)
                                                            }
                                                            serde_json::Value::Number(n) => {
                                                                n.to_string()
                                                            }
                                                            serde_json::Value::Bool(b) => {
                                                                b.to_string()
                                                            }
                                                            serde_json::Value::Array(arr) => {
                                                                format!("[{} items]", arr.len())
                                                            }
                                                            serde_json::Value::Object(obj) => {
                                                                format!(
                                                                    "{{{}}} properties",
                                                                    obj.keys().count()
                                                                )
                                                            }
                                                            serde_json::Value::Null => {
                                                                "null".to_string()
                                                            }
                                                        };

                                                        ui.label(value_str);
                                                    });
                                                }
                                            });
                                    }

                                    ui.separator();
                                    ui.label("Map Actions:");

                                    if let Some(scene_name) = scene_name {
                                        if ui.button("📂 Load in Viewport").clicked() {
                                            tracing::info!(
                                                "Load Map '{}' from scene '{}' clicked",
                                                map_name,
                                                scene_name
                                            );
                                            *map_load_requested = Some(MapLoadRequest {
                                                scene_name: scene_name.to_string(),
                                                map_name: map_name.to_string(),
                                            });
                                        }
                                    } else {
                                        ui.label("(Not associated with a scene)");
                                    }
                                }
                                Err(e) => {
                                    ui.label("❌ Invalid JSON file");
                                    ui.label(format!("Error: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            ui.label("❌ Could not read map file");
                            ui.label(format!("Error: {}", e));
                        }
                    }
                } else {
                    ui.label("❌ Map file not found");
                }
            }
        }
    }

    /// Renders detailed information about an entity definition
    pub(super) fn render_entity_definition_details(
        ui: &mut egui::Ui,
        entity_name: &str,
        config: Option<&EditorConfig>,
    ) {
        // Try to load and show entity definition details
        if let Some(config) = config {
            if let Some(project_path) = config.current_project_path() {
                let entity_file = project_path
                    .join("entities")
                    .join(format!("{}.json", entity_name));

                if entity_file.exists() {
                    // Try to read the entity definition file
                    match std::fs::read_to_string(&entity_file) {
                        Ok(content) => {
                            if let Ok(mut definition) = serde_json::from_str::<
                                toki_core::entity::EntityDefinition,
                            >(&content)
                            {
                                ui.separator();
                                ui.label("Stats:");

                                ui.horizontal(|ui| {
                                    ui.label("Health:");
                                    let mut changed = false;
                                    let mut health_enabled = definition.attributes.health.is_some();
                                    let mut health_value =
                                        definition.attributes.health.unwrap_or(0) as i64;
                                    changed |=
                                        ui.checkbox(&mut health_enabled, "Enabled").changed();
                                    if health_enabled {
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(&mut health_value)
                                                    .speed(1.0)
                                                    .range(0..=i64::MAX),
                                            )
                                            .changed();
                                    }
                                    if changed {
                                        let new_health = if health_enabled {
                                            Some(health_value.clamp(0, u32::MAX as i64) as u32)
                                        } else {
                                            None
                                        };
                                        definition.attributes.health = new_health;
                                        Self::set_optional_definition_stat(
                                            &mut definition.attributes,
                                            HEALTH_STAT_ID,
                                            new_health.map(|value| value as i32),
                                        );
                                        if let Err(err) =
                                            Self::save_entity_definition(&definition, &entity_file)
                                        {
                                            tracing::error!("{}", err);
                                            ui.colored_label(egui::Color32::RED, err);
                                        }
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Attack Power:");
                                    let mut changed = false;
                                    let mut attack_power =
                                        definition.attributes.stats.get(ATTACK_POWER_STAT_ID).copied();
                                    let mut attack_power_enabled = attack_power.is_some();
                                    let mut attack_power_value = attack_power.unwrap_or(0) as i64;
                                    changed |= ui
                                        .checkbox(&mut attack_power_enabled, "Enabled")
                                        .changed();
                                    if attack_power_enabled {
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(&mut attack_power_value)
                                                    .speed(1.0)
                                                    .range(0..=i64::MAX),
                                            )
                                            .changed();
                                    }
                                    if changed {
                                        attack_power = if attack_power_enabled {
                                            Some(
                                                attack_power_value
                                                    .clamp(0, i32::MAX as i64)
                                                    as i32,
                                            )
                                        } else {
                                            None
                                        };
                                        Self::set_optional_definition_stat(
                                            &mut definition.attributes,
                                            ATTACK_POWER_STAT_ID,
                                            attack_power,
                                        );
                                        if let Err(err) =
                                            Self::save_entity_definition(&definition, &entity_file)
                                        {
                                            tracing::error!("{}", err);
                                            ui.colored_label(egui::Color32::RED, err);
                                        }
                                    }
                                });

                                let is_static_item = definition.category == "item"
                                    && definition.rendering.static_object.is_some();

                                if is_static_item {
                                    if let Some(static_object) = &definition.rendering.static_object
                                    {
                                        ui.separator();
                                        ui.label("Static Render");
                                        ui.horizontal(|ui| {
                                            ui.label("Sheet:");
                                            ui.label(&static_object.sheet);
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Object:");
                                            ui.label(&static_object.object_name);
                                        });
                                    }
                                } else {
                                    ui.separator();
                                    ui.label("Audio Settings:");

                                    ui.horizontal(|ui| {
                                        ui.label("Movement Trigger:");
                                        let mut changed = false;
                                        egui::ComboBox::from_id_salt(format!(
                                            "entity_def_movement_trigger_{}",
                                            entity_name
                                        ))
                                        .selected_text(movement_sound_trigger_label(
                                            definition.audio.movement_sound_trigger,
                                        ))
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                changed |= ui
                                                    .selectable_value(
                                                        &mut definition
                                                            .audio
                                                            .movement_sound_trigger,
                                                        MovementSoundTrigger::Distance,
                                                        "Distance",
                                                    )
                                                    .changed();
                                                changed |= ui
                                                    .selectable_value(
                                                        &mut definition
                                                            .audio
                                                            .movement_sound_trigger,
                                                        MovementSoundTrigger::AnimationLoop,
                                                        "Animation Loop",
                                                    )
                                                    .changed();
                                            },
                                        );
                                        if changed {
                                            if let Err(err) = Self::save_entity_definition(
                                                &definition,
                                                &entity_file,
                                            ) {
                                                tracing::error!("{}", err);
                                                ui.colored_label(egui::Color32::RED, err);
                                            }
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Footstep Distance:");
                                        let mut changed = false;
                                        ui.add_enabled_ui(
                                            matches!(
                                                definition.audio.movement_sound_trigger,
                                                MovementSoundTrigger::Distance
                                            ),
                                            |ui| {
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(
                                                            &mut definition
                                                                .audio
                                                                .footstep_trigger_distance,
                                                        )
                                                        .speed(0.5)
                                                        .range(0.0..=f32::MAX),
                                                    )
                                                    .changed();
                                            },
                                        );
                                        if changed {
                                            definition.audio.footstep_trigger_distance =
                                                definition.audio.footstep_trigger_distance.max(0.0);
                                            if let Err(err) = Self::save_entity_definition(
                                                &definition,
                                                &entity_file,
                                            ) {
                                                tracing::error!("{}", err);
                                                ui.colored_label(egui::Color32::RED, err);
                                            }
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Hearing Radius:");
                                        let mut changed = false;
                                        changed |= ui
                                            .add(
                                                egui::DragValue::new(
                                                    &mut definition.audio.hearing_radius,
                                                )
                                                .speed(1.0)
                                                .range(0..=u32::MAX),
                                            )
                                            .changed();
                                        if changed {
                                            if let Err(err) = Self::save_entity_definition(
                                                &definition,
                                                &entity_file,
                                            ) {
                                                tracing::error!("{}", err);
                                                ui.colored_label(egui::Color32::RED, err);
                                            }
                                        }
                                    });

                                    let mut movement_sound_options =
                                        Self::discover_audio_asset_names(
                                            project_path.join("assets/audio/sfx").as_path(),
                                        );
                                    if !definition.audio.movement_sound.trim().is_empty()
                                        && !movement_sound_options
                                            .iter()
                                            .any(|name| name == &definition.audio.movement_sound)
                                    {
                                        movement_sound_options
                                            .push(definition.audio.movement_sound.clone());
                                        movement_sound_options.sort();
                                        movement_sound_options.dedup();
                                    }

                                    ui.horizontal(|ui| {
                                        ui.label("Movement Sound:");
                                        let selected_text =
                                            if definition.audio.movement_sound.trim().is_empty() {
                                                "None".to_string()
                                            } else {
                                                definition.audio.movement_sound.clone()
                                            };
                                        let mut changed = false;
                                        egui::ComboBox::from_id_salt(format!(
                                            "entity_def_movement_sound_{}",
                                            entity_name
                                        ))
                                        .selected_text(selected_text)
                                        .show_ui(
                                            ui,
                                            |ui| {
                                                changed |= ui
                                                    .selectable_value(
                                                        &mut definition.audio.movement_sound,
                                                        String::new(),
                                                        "None",
                                                    )
                                                    .changed();
                                                for sound_name in &movement_sound_options {
                                                    changed |= ui
                                                        .selectable_value(
                                                            &mut definition.audio.movement_sound,
                                                            sound_name.clone(),
                                                            sound_name,
                                                        )
                                                        .changed();
                                                }
                                            },
                                        );
                                        if changed {
                                            if let Err(err) = Self::save_entity_definition(
                                                &definition,
                                                &entity_file,
                                            ) {
                                                tracing::error!("{}", err);
                                                ui.colored_label(egui::Color32::RED, err);
                                            }
                                        }
                                    });
                                }
                            }

                            // Try to parse as JSON to show detailed info
                            match serde_json::from_str::<serde_json::Value>(&content) {
                                Ok(json) => {
                                    // Show file info
                                    ui.horizontal(|ui| {
                                        ui.label("File:");
                                        ui.label(format!("{}.json", entity_name));
                                    });

                                    if let Some(obj) = json.as_object() {
                                        let is_static_item = obj
                                            .get("category")
                                            .and_then(|v| v.as_str())
                                            .is_some_and(|category| category == "item")
                                            && obj
                                                .get("rendering")
                                                .and_then(|v| v.as_object())
                                                .and_then(|rendering| {
                                                    rendering.get("static_object")
                                                })
                                                .is_some();
                                        // Show basic entity information
                                        if let Some(display_name) =
                                            obj.get("display_name").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Display Name:");
                                                ui.label(display_name);
                                            });
                                        }

                                        if let Some(description) =
                                            obj.get("description").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Description:");
                                                ui.label(description);
                                            });
                                        }

                                        if let Some(entity_type) =
                                            obj.get("entity_type").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Type:");
                                                ui.label(entity_type);
                                            });
                                        }

                                        if let Some(category) =
                                            obj.get("category").and_then(|v| v.as_str())
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label("Category:");
                                                ui.label(category);
                                            });
                                        }

                                        // Show rendering properties
                                        if let Some(rendering) =
                                            obj.get("rendering").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Rendering:");

                                            if let Some(size) =
                                                rendering.get("size").and_then(|v| v.as_array())
                                            {
                                                if size.len() == 2 {
                                                    if let (Some(w), Some(h)) =
                                                        (size[0].as_u64(), size[1].as_u64())
                                                    {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Size:");
                                                            ui.label(format!("{}x{}", w, h));
                                                        });
                                                    }
                                                }
                                            }

                                            if let Some(visible) =
                                                rendering.get("visible").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Visible:");
                                                    ui.label(format!("{}", visible));
                                                });
                                            }

                                            if let Some(render_layer) = rendering
                                                .get("render_layer")
                                                .and_then(|v| v.as_u64())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Render Layer:");
                                                    ui.label(format!("{}", render_layer));
                                                });
                                            }
                                        }

                                        // Show attributes
                                        if let Some(attributes) =
                                            obj.get("attributes").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Attributes:");

                                            if let Some(health) =
                                                attributes.get("health").and_then(|v| v.as_u64())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Health:");
                                                    ui.label(format!("{}", health));
                                                });
                                            }

                                            if let Some(attack_power) = attributes
                                                .get("stats")
                                                .and_then(|v| v.as_object())
                                                .and_then(|stats| {
                                                    stats
                                                        .get(ATTACK_POWER_STAT_ID)
                                                        .and_then(|v| v.as_i64())
                                                })
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Attack Power:");
                                                    ui.label(format!("{}", attack_power));
                                                });
                                            }

                                            if let Some(speed) =
                                                attributes.get("speed").and_then(|v| v.as_u64())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Speed:");
                                                    ui.label(format!("{}", speed));
                                                });
                                            }

                                            if let Some(solid) =
                                                attributes.get("solid").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Solid:");
                                                    ui.label(format!("{}", solid));
                                                });
                                            }

                                            if let Some(active) =
                                                attributes.get("active").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Active:");
                                                    ui.label(format!("{}", active));
                                                });
                                            }

                                            if let Some(can_move) =
                                                attributes.get("can_move").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Can Move:");
                                                    ui.label(format!("{}", can_move));
                                                });
                                            }

                                            if let Some(has_inventory) = attributes
                                                .get("has_inventory")
                                                .and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Has Inventory:");
                                                    ui.label(format!("{}", has_inventory));
                                                });
                                            }
                                        }

                                        // Show collision information
                                        if let Some(collision) =
                                            obj.get("collision").and_then(|v| v.as_object())
                                        {
                                            ui.separator();
                                            ui.label("Collision:");

                                            if let Some(enabled) =
                                                collision.get("enabled").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Enabled:");
                                                    ui.label(format!("{}", enabled));
                                                });
                                            }

                                            if let Some(offset) =
                                                collision.get("offset").and_then(|v| v.as_array())
                                            {
                                                if offset.len() == 2 {
                                                    if let (Some(x), Some(y)) =
                                                        (offset[0].as_i64(), offset[1].as_i64())
                                                    {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Offset:");
                                                            ui.label(format!("({}, {})", x, y));
                                                        });
                                                    }
                                                }
                                            }

                                            if let Some(size) =
                                                collision.get("size").and_then(|v| v.as_array())
                                            {
                                                if size.len() == 2 {
                                                    if let (Some(w), Some(h)) =
                                                        (size[0].as_u64(), size[1].as_u64())
                                                    {
                                                        ui.horizontal(|ui| {
                                                            ui.label("Size:");
                                                            ui.label(format!("{}x{}", w, h));
                                                        });
                                                    }
                                                }
                                            }

                                            if let Some(trigger) =
                                                collision.get("trigger").and_then(|v| v.as_bool())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Trigger:");
                                                    ui.label(format!("{}", trigger));
                                                });
                                            }
                                        }

                                        // Show audio information
                                        if !is_static_item {
                                            if let Some(audio) =
                                                obj.get("audio").and_then(|v| v.as_object())
                                            {
                                                ui.separator();
                                                ui.label("Audio:");

                                                if let Some(distance) = audio
                                                    .get("footstep_trigger_distance")
                                                    .and_then(|v| v.as_f64())
                                                {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Footstep Distance:");
                                                        ui.label(format!("{:.1}", distance));
                                                    });
                                                }

                                                if let Some(movement_sound) = audio
                                                    .get("movement_sound")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Movement Sound:");
                                                        ui.label(movement_sound);
                                                    });
                                                }

                                                if let Some(collision_sound) = audio
                                                    .get("collision_sound")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Collision Sound:");
                                                        ui.label(collision_sound);
                                                    });
                                                }
                                            }
                                        }

                                        // Show animation information
                                        if !is_static_item {
                                            if let Some(animations) =
                                                obj.get("animations").and_then(|v| v.as_object())
                                            {
                                                ui.separator();
                                                ui.label("Animations:");

                                                if let Some(atlas_name) = animations
                                                    .get("atlas_name")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Atlas:");
                                                        ui.label(atlas_name);
                                                    });
                                                }

                                                if let Some(default_state) = animations
                                                    .get("default_state")
                                                    .and_then(|v| v.as_str())
                                                {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Default State:");
                                                        ui.label(default_state);
                                                    });
                                                }

                                                if let Some(clips) = animations
                                                    .get("clips")
                                                    .and_then(|v| v.as_array())
                                                {
                                                    ui.horizontal(|ui| {
                                                        ui.label("Available Clips:");
                                                        ui.label(format!("{}", clips.len()));
                                                    });

                                                    ui.indent("animation_clips", |ui| {
                                                        for clip in clips.iter() {
                                                            if let Some(clip_obj) = clip.as_object()
                                                            {
                                                                let state = clip_obj
                                                                    .get("state")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("unknown");
                                                                let loop_mode = clip_obj
                                                                    .get("loop_mode")
                                                                    .and_then(|v| v.as_str())
                                                                    .unwrap_or("unknown");
                                                                let frame_duration = clip_obj
                                                                    .get("frame_duration_ms")
                                                                    .and_then(|v| v.as_f64())
                                                                    .unwrap_or(0.0);
                                                                let frame_count = clip_obj
                                                                    .get("frame_tiles")
                                                                    .and_then(|v| v.as_array())
                                                                    .map(|arr| arr.len())
                                                                    .unwrap_or(0);

                                                                ui.horizontal(|ui| {
                                                                    ui.label(format!(
                                                                        "• {}: {} frames, {:.0}ms, {}",
                                                                        state,
                                                                        frame_count,
                                                                        frame_duration,
                                                                        loop_mode
                                                                    ));
                                                                });
                                                            }
                                                        }
                                                    });
                                                }
                                            }
                                        }

                                        ui.separator();
                                        ui.label("Entity Actions:");

                                        if ui.button("🎮 Place in Scene").clicked() {
                                            tracing::info!(
                                                "Place entity '{}' button clicked",
                                                entity_name
                                            );
                                            // TODO: Implement entity placement functionality
                                        }
                                    }
                                }
                                Err(e) => {
                                    ui.label("❌ Invalid JSON file");
                                    ui.label(format!("Error: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            ui.label("❌ Could not read entity definition file");
                            ui.label(format!("Error: {}", e));
                        }
                    }
                } else {
                    ui.label("❌ Entity definition file not found");
                }
            }
        }
    }
}
