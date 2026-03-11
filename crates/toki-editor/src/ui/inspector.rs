use super::editor_ui::{EditorUI, Selection};
use crate::config::EditorConfig;

/// Handles inspector panel rendering for assets and entities
pub struct InspectorSystem;

#[derive(Debug, Clone)]
struct EntityPropertyDraft {
    position_x: i32,
    position_y: i32,
    size_x: i64,
    size_y: i64,
    visible: bool,
    active: bool,
    solid: bool,
    can_move: bool,
    has_inventory: bool,
    speed: i64,
    render_layer: i32,
    health_enabled: bool,
    health_value: i64,
    collision_enabled: bool,
    collision_offset_x: i32,
    collision_offset_y: i32,
    collision_size_x: i64,
    collision_size_y: i64,
    collision_trigger: bool,
}

impl EntityPropertyDraft {
    fn from_entity(entity: &toki_core::entity::Entity) -> Self {
        let (
            collision_enabled,
            collision_offset_x,
            collision_offset_y,
            collision_size_x,
            collision_size_y,
            collision_trigger,
        ) = if let Some(collision_box) = &entity.collision_box {
            (
                true,
                collision_box.offset.x,
                collision_box.offset.y,
                collision_box.size.x as i64,
                collision_box.size.y as i64,
                collision_box.trigger,
            )
        } else {
            (
                false,
                0,
                0,
                entity.size.x as i64,
                entity.size.y as i64,
                false,
            )
        };

        let (health_enabled, health_value) = match entity.attributes.health {
            Some(value) => (true, value as i64),
            None => (false, 0),
        };

        Self {
            position_x: entity.position.x,
            position_y: entity.position.y,
            size_x: entity.size.x as i64,
            size_y: entity.size.y as i64,
            visible: entity.attributes.visible,
            active: entity.attributes.active,
            solid: entity.attributes.solid,
            can_move: entity.attributes.can_move,
            has_inventory: entity.attributes.has_inventory,
            speed: entity.attributes.speed as i64,
            render_layer: entity.attributes.render_layer,
            health_enabled,
            health_value,
            collision_enabled,
            collision_offset_x,
            collision_offset_y,
            collision_size_x,
            collision_size_y,
            collision_trigger,
        }
    }
}

impl InspectorSystem {
    /// Renders the main inspector panel on the right side of the screen
    pub fn render_inspector_panel(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        config: Option<&EditorConfig>,
    ) {
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("🔍 Inspector");
                ui.separator();

                // Wrap all inspector content in a scrollable area
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        let current_selection = ui_state.selection.clone();
                        match current_selection.as_ref() {
                            Some(Selection::Scene(scene_name)) => {
                                ui.heading(format!("🎬 {}", scene_name));
                                ui.separator();

                                if let Some(scene) = ui_state.get_scene(scene_name) {
                                    ui.horizontal(|ui| {
                                        ui.label("Maps:");
                                        ui.label(format!("{}", scene.maps.len()));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Entities:");
                                        ui.label(format!("{}", scene.entities.len()));
                                    });

                                    ui.separator();
                                    ui.label("Scene Actions:");

                                    if ui.button("🗺️ Add Map").clicked() {
                                        tracing::info!("Add Map to scene: {}", scene_name);
                                        // Maps are added via the hierarchy panel, this could open a dialog
                                    }

                                    if ui.button("👤 Add Entity").clicked() {
                                        tracing::info!("Add Entity to scene: {}", scene_name);
                                        // TODO: Entity creation
                                    }
                                }
                            }

                            Some(Selection::Map(scene_name, map_name)) => {
                                ui.heading(format!("🗺️ {}", map_name));
                                ui.label(format!("Scene: {}", scene_name));
                                ui.separator();

                                Self::render_map_details(
                                    ui,
                                    map_name,
                                    config,
                                    Some(scene_name),
                                    &mut ui_state.map_load_requested,
                                );
                            }

                            Some(Selection::Entity(entity_id)) => {
                                ui.heading(format!("👤 Entity {}", entity_id));
                                ui.separator();
                                let mut entity_changed = false;
                                if let Some(scene_entity) =
                                    Self::find_selected_scene_entity_mut(ui_state, *entity_id)
                                {
                                    let mut draft = EntityPropertyDraft::from_entity(scene_entity);
                                    if Self::render_scene_entity_editor(ui, &mut draft) {
                                        entity_changed =
                                            Self::apply_entity_property_draft(scene_entity, &draft);
                                    }
                                } else {
                                    ui.label("Runtime-only entity (read-only)");
                                    ui.separator();
                                    Self::render_runtime_entity_read_only(
                                        ui, game_state, *entity_id,
                                    );
                                }

                                if entity_changed {
                                    ui_state.scene_content_changed = true;
                                }
                            }

                            Some(Selection::StandaloneMap(map_name)) => {
                                ui.heading(format!("🗺️ {}", map_name));
                                ui.label("(Standalone map - not in scene)");
                                ui.separator();

                                Self::render_map_details(
                                    ui,
                                    map_name,
                                    config,
                                    None,
                                    &mut ui_state.map_load_requested,
                                );
                            }

                            Some(Selection::EntityDefinition(entity_name)) => {
                                ui.heading(format!("🤖 {}", entity_name));
                                ui.label("Entity Definition");
                                ui.separator();

                                Self::render_entity_definition_details(ui, entity_name, config);
                            }

                            None => {
                                ui.label("No selection");
                                ui.separator();
                                ui.label("Click on an item in the hierarchy to inspect it.");
                            }
                        }
                    });
            });
    }

    fn render_scene_entity_editor(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
        let mut changed = false;

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

        changed |= ui.checkbox(&mut draft.visible, "Visible").changed();
        changed |= ui.checkbox(&mut draft.active, "Active").changed();
        changed |= ui.checkbox(&mut draft.solid, "Solid").changed();
        changed |= ui.checkbox(&mut draft.can_move, "Can Move").changed();
        changed |= ui
            .checkbox(&mut draft.has_inventory, "Has Inventory")
            .changed();

        ui.separator();
        ui.label("Health");
        changed |= ui.checkbox(&mut draft.health_enabled, "Enabled").changed();
        if draft.health_enabled {
            ui.horizontal(|ui| {
                ui.label("Value:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.health_value)
                            .speed(1.0)
                            .range(0..=i64::MAX),
                    )
                    .changed();
            });
        }

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

    fn render_runtime_entity_read_only(
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
                    ui.label(format!("{:?}", entity.entity_type));
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

                if entity.attributes.has_inventory {
                    ui.horizontal(|ui| {
                        ui.label("Has Inventory:");
                        ui.label("Yes");
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

    fn find_selected_scene_entity_mut(
        ui_state: &mut EditorUI,
        entity_id: toki_core::entity::EntityId,
    ) -> Option<&mut toki_core::entity::Entity> {
        let active_scene_name = ui_state.active_scene.clone()?;
        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == active_scene_name)?;
        scene
            .entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
    }

    fn apply_entity_property_draft(
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

    /// Renders detailed information about a specific map
    pub fn render_map_details(
        ui: &mut egui::Ui,
        map_name: &str,
        config: Option<&EditorConfig>,
        scene_name: Option<&str>,
        map_load_requested: &mut Option<(String, String)>,
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
                                            *map_load_requested = Some((
                                                scene_name.to_string(),
                                                map_name.to_string(),
                                            ));
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
    pub fn render_entity_definition_details(
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
                            // Try to parse as JSON to show detailed info
                            match serde_json::from_str::<serde_json::Value>(&content) {
                                Ok(json) => {
                                    // Show file info
                                    ui.horizontal(|ui| {
                                        ui.label("File:");
                                        ui.label(format!("{}.json", entity_name));
                                    });

                                    if let Some(obj) = json.as_object() {
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

                                            if let Some(movement_sound) =
                                                audio.get("movement_sound").and_then(|v| v.as_str())
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

                                        // Show animation information
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

                                            if let Some(clips) =
                                                animations.get("clips").and_then(|v| v.as_array())
                                            {
                                                ui.horizontal(|ui| {
                                                    ui.label("Available Clips:");
                                                    ui.label(format!("{}", clips.len()));
                                                });

                                                ui.indent("animation_clips", |ui| {
                                                    for clip in clips.iter() {
                                                        if let Some(clip_obj) = clip.as_object() {
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

#[cfg(test)]
mod tests {
    use super::{EntityPropertyDraft, InspectorSystem};
    use crate::ui::EditorUI;
    use glam::{IVec2, UVec2};
    use toki_core::collision::CollisionBox;
    use toki_core::entity::{EntityAttributes, EntityManager, EntityType};
    use toki_core::Scene;

    fn sample_entity_with_id(id: u32) -> toki_core::entity::Entity {
        let mut manager = EntityManager::new();
        let spawned_id = manager.spawn_entity(
            EntityType::Npc,
            IVec2::new(10, 20),
            UVec2::new(16, 16),
            EntityAttributes {
                health: Some(25),
                speed: 3,
                solid: true,
                visible: true,
                animation_controller: None,
                render_layer: 1,
                active: true,
                can_move: true,
                has_inventory: false,
            },
        );
        let mut entity = manager
            .get_entity(spawned_id)
            .expect("missing spawned entity")
            .clone();
        entity.id = id;
        entity.collision_box = Some(CollisionBox::new(
            IVec2::new(0, 0),
            UVec2::new(16, 16),
            false,
        ));
        entity
    }

    #[test]
    fn apply_entity_property_draft_clamps_and_sets_values() {
        let mut entity = sample_entity_with_id(1);
        let mut draft = EntityPropertyDraft::from_entity(&entity);
        draft.position_x = 100;
        draft.position_y = 200;
        draft.size_x = 0;
        draft.size_y = -5;
        draft.visible = false;
        draft.active = false;
        draft.solid = false;
        draft.can_move = false;
        draft.has_inventory = true;
        draft.speed = -10;
        draft.render_layer = 8;
        draft.health_enabled = true;
        draft.health_value = -4;
        draft.collision_enabled = true;
        draft.collision_offset_x = 3;
        draft.collision_offset_y = -2;
        draft.collision_size_x = 0;
        draft.collision_size_y = -7;
        draft.collision_trigger = true;

        let changed = InspectorSystem::apply_entity_property_draft(&mut entity, &draft);

        assert!(changed);
        assert_eq!(entity.position, IVec2::new(100, 200));
        assert_eq!(entity.size, UVec2::new(1, 1));
        assert!(!entity.attributes.visible);
        assert!(!entity.attributes.active);
        assert!(!entity.attributes.solid);
        assert!(!entity.attributes.can_move);
        assert!(entity.attributes.has_inventory);
        assert_eq!(entity.attributes.speed, 0);
        assert_eq!(entity.attributes.render_layer, 8);
        assert_eq!(entity.attributes.health, Some(0));

        let collision = entity
            .collision_box
            .as_ref()
            .expect("collision should be enabled");
        assert_eq!(collision.offset, IVec2::new(3, -2));
        assert_eq!(collision.size, UVec2::new(1, 1));
        assert!(collision.trigger);
    }

    #[test]
    fn apply_entity_property_draft_disables_health_and_collision() {
        let mut entity = sample_entity_with_id(1);
        let mut draft = EntityPropertyDraft::from_entity(&entity);
        draft.health_enabled = false;
        draft.collision_enabled = false;

        let changed = InspectorSystem::apply_entity_property_draft(&mut entity, &draft);

        assert!(changed);
        assert_eq!(entity.attributes.health, None);
        assert!(entity.collision_box.is_none());
    }

    #[test]
    fn find_selected_scene_entity_mut_returns_entity_from_active_scene() {
        let mut ui_state = EditorUI::new();
        let entity = sample_entity_with_id(7);
        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(entity);

        let selected_entity = InspectorSystem::find_selected_scene_entity_mut(&mut ui_state, 7)
            .expect("entity should be found");
        selected_entity.position = IVec2::new(50, 60);

        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        let entity = scene
            .entities
            .iter()
            .find(|entity| entity.id == 7)
            .expect("entity should still exist");
        assert_eq!(entity.position, IVec2::new(50, 60));
    }

    #[test]
    fn find_selected_scene_entity_mut_returns_none_for_inactive_scene() {
        let mut ui_state = EditorUI::new();
        ui_state.scenes.push(Scene::new("Other".to_string()));
        ui_state.active_scene = Some("Other".to_string());

        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(sample_entity_with_id(42));

        assert!(InspectorSystem::find_selected_scene_entity_mut(&mut ui_state, 42).is_none());
    }
}
