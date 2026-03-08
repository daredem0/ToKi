use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use crate::ui::EditorUI;

/// Handles entity placement interactions
pub struct PlacementInteraction;

impl PlacementInteraction {
    /// Handle placement mode hover logic for preview updates
    pub fn handle_hover(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        response: &egui::Response,
        rect: egui::Rect,
        config: Option<&EditorConfig>,
    ) {
        if ui_state.is_in_placement_mode() {
            if let Some(hover_pos) = response.hover_pos() {
                let world_pos = viewport.screen_to_world_pos_raw(hover_pos, rect);
                ui_state.placement_preview_position = Some(world_pos);

                let is_valid =
                    Self::check_placement_validity(ui_state, viewport, world_pos, config);
                ui_state.placement_preview_valid = Some(is_valid);
                viewport.mark_dirty();
            } else {
                ui_state.placement_preview_position = None;
                ui_state.placement_preview_valid = None;
                viewport.mark_dirty();
            }
        }
    }

    /// Handle placement click - creates entity at clicked position
    pub fn handle_click(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        click_pos: egui::Pos2,
        rect: egui::Rect,
        config: Option<&EditorConfig>,
    ) {
        tracing::info!("Placement click detected at screen pos: {:?}", click_pos);

        let Some(entity_def_name) = &ui_state.placement_entity_definition.clone() else {
            tracing::warn!("No entity definition for placement");
            return;
        };

        let world_pos = viewport.screen_to_world_pos(click_pos, rect);
        tracing::info!(
            "Placing entity '{}' at world coordinates ({}, {}) [converted from screen ({}, {})]",
            entity_def_name,
            world_pos.x,
            world_pos.y,
            click_pos.x,
            click_pos.y
        );

        if Self::try_place_entity(ui_state, entity_def_name, world_pos, config, viewport) {
            ui_state.exit_placement_mode();
        }
    }

    /// Try to place entity at given world position, returns true if successful
    fn try_place_entity(
        ui_state: &mut EditorUI,
        entity_def_name: &str,
        world_pos: glam::Vec2,
        config: Option<&EditorConfig>,
        viewport: &SceneViewport,
    ) -> bool {
        let Some(config) = config else {
            tracing::error!("No config available for entity creation");
            ui_state.exit_placement_mode();
            return false;
        };

        let Some(project_path) = config.current_project_path() else {
            tracing::error!("No project path available for entity creation");
            ui_state.exit_placement_mode();
            return false;
        };

        let entity_file = project_path
            .join("entities")
            .join(format!("{}.json", entity_def_name));
        if !entity_file.exists() {
            tracing::error!("Entity definition file not found: {:?}", entity_file);
            ui_state.exit_placement_mode();
            return false;
        }

        let content = match std::fs::read_to_string(&entity_file) {
            Ok(content) => content,
            Err(e) => {
                tracing::error!("Failed to read entity file '{}': {}", entity_def_name, e);
                ui_state.exit_placement_mode();
                return false;
            }
        };

        let entity_def = match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content)
        {
            Ok(entity_def) => entity_def,
            Err(e) => {
                tracing::error!(
                    "Failed to parse entity definition '{}': {}",
                    entity_def_name,
                    e
                );
                ui_state.exit_placement_mode();
                return false;
            }
        };

        let sprite_size =
            glam::UVec2::new(entity_def.rendering.size[0], entity_def.rendering.size[1]);
        let half_size = glam::Vec2::new(sprite_size.x as f32 / 2.0, sprite_size.y as f32 / 2.0);
        let centered_world_pos = world_pos - half_size;
        let world_pos_i32 =
            glam::IVec2::new(centered_world_pos.x as i32, centered_world_pos.y as i32);

        Self::create_entity_in_scene(
            ui_state,
            entity_def,
            entity_def_name,
            world_pos_i32,
            viewport,
        )
    }

    /// Create entity in the active scene, returns true if successful
    fn create_entity_in_scene(
        ui_state: &mut EditorUI,
        entity_def: toki_core::entity::EntityDefinition,
        entity_def_name: &str,
        world_pos_i32: glam::IVec2,
        viewport: &SceneViewport,
    ) -> bool {
        let Some(active_scene_name) = &ui_state.active_scene else {
            tracing::error!("No active scene for entity placement");
            ui_state.exit_placement_mode();
            return false;
        };

        let Some(target_scene) = ui_state
            .scenes
            .iter_mut()
            .find(|s| s.name == *active_scene_name)
        else {
            tracing::error!("Active scene '{}' not found", active_scene_name);
            ui_state.exit_placement_mode();
            return false;
        };

        let new_id = target_scene
            .entities
            .iter()
            .map(|e| e.id)
            .max()
            .unwrap_or(0)
            + 1;

        let entity = match entity_def.create_entity(world_pos_i32, new_id) {
            Ok(entity) => entity,
            Err(e) => {
                tracing::error!("Failed to create entity '{}': {}", entity_def_name, e);
                ui_state.exit_placement_mode();
                return false;
            }
        };

        let can_place = if let Some(tilemap) = viewport.scene_manager().tilemap() {
            let terrain_atlas = viewport.scene_manager().resources().get_terrain_atlas();
            toki_core::collision::can_entity_move_to_position(
                &entity,
                world_pos_i32,
                tilemap,
                terrain_atlas,
            )
        } else {
            true
        };

        if can_place {
            target_scene.entities.push(entity);
            tracing::info!(
                "Successfully placed entity '{}' (ID: {}) in scene '{}' at world position ({}, {})",
                entity_def_name,
                new_id,
                active_scene_name,
                world_pos_i32.x,
                world_pos_i32.y
            );
            ui_state.scene_content_changed = true;
            true
        } else {
            tracing::warn!("Cannot place entity '{}' at position ({}, {}) - collision detected with solid terrain (staying in placement mode)",
                entity_def_name, world_pos_i32.x, world_pos_i32.y);
            false
        }
    }

    /// Check if placement is valid at given world position
    fn check_placement_validity(
        ui_state: &EditorUI,
        viewport: &mut SceneViewport,
        world_pos: glam::Vec2,
        config: Option<&EditorConfig>,
    ) -> bool {
        let Some(entity_def_name) = &ui_state.placement_entity_definition else {
            return false;
        };

        let Some(config) = config else {
            return false;
        };

        let Some(project_path) = config.current_project_path() else {
            return false;
        };

        let entity_file = project_path
            .join("entities")
            .join(format!("{}.json", entity_def_name));
        if !entity_file.exists() {
            return false;
        }

        let content = match std::fs::read_to_string(&entity_file) {
            Ok(content) => content,
            Err(_) => return false,
        };

        let entity_def = match serde_json::from_str::<toki_core::entity::EntityDefinition>(&content)
        {
            Ok(entity_def) => entity_def,
            Err(_) => return false,
        };

        let sprite_size =
            glam::UVec2::new(entity_def.rendering.size[0], entity_def.rendering.size[1]);
        let half_size = glam::Vec2::new(sprite_size.x as f32 / 2.0, sprite_size.y as f32 / 2.0);
        let centered_world_pos = world_pos - half_size;
        let world_pos_i32 =
            glam::IVec2::new(centered_world_pos.x as i32, centered_world_pos.y as i32);

        let collision_box = entity_def.get_collision_box();
        if let Some(tilemap) = viewport.scene_manager().tilemap() {
            let terrain_atlas = viewport.scene_manager().resources().get_terrain_atlas();
            toki_core::collision::can_place_collision_box_at_position(
                collision_box.as_ref(),
                world_pos_i32,
                tilemap,
                terrain_atlas,
            )
        } else {
            true
        }
    }
}
