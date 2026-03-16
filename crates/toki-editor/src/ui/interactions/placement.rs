use super::GridInteraction;
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use crate::ui::undo_redo::EditorCommand;
use crate::ui::EditorUI;
use std::path::{Path, PathBuf};
use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::entity::{Entity, EntityDefinition};

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
                let cursor_world = viewport.screen_to_world_pos_raw(hover_pos, rect);
                let grab_offset = ui_state
                    .entity_move_drag
                    .as_ref()
                    .map(|drag| drag.grab_offset)
                    .unwrap_or(glam::Vec2::ZERO);
                let world_pos = GridInteraction::drag_target_world_position(
                    cursor_world,
                    grab_offset,
                    viewport.scene_manager().tilemap(),
                    config,
                );
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

        let world_pos = GridInteraction::maybe_snap_world_position(
            viewport.screen_to_world_pos_raw(click_pos, rect),
            viewport.scene_manager().tilemap(),
            config,
        );
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

        let entity_def = match Self::load_entity_definition(project_path, entity_def_name) {
            Ok(entity_def) => entity_def,
            Err(msg) => {
                tracing::error!(
                    "Failed to load entity definition '{}': {}",
                    entity_def_name,
                    msg
                );
                ui_state.exit_placement_mode();
                return false;
            }
        };

        let world_pos_i32 = Self::placement_world_position_to_entity_position(world_pos);

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
        entity_def: EntityDefinition,
        entity_def_name: &str,
        world_pos_i32: glam::IVec2,
        viewport: &SceneViewport,
    ) -> bool {
        let tilemap = viewport.scene_manager().tilemap();
        let terrain_atlas =
            tilemap.map(|_| viewport.scene_manager().resources().get_terrain_atlas());
        Self::create_entity_in_scene_with_collision_context(
            ui_state,
            entity_def,
            entity_def_name,
            world_pos_i32,
            tilemap,
            terrain_atlas,
        )
    }

    fn create_entity_in_scene_with_collision_context(
        ui_state: &mut EditorUI,
        entity_def: EntityDefinition,
        entity_def_name: &str,
        world_pos_i32: glam::IVec2,
        tilemap: Option<&TileMap>,
        terrain_atlas: Option<&AtlasMeta>,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            tracing::error!("No active scene for entity placement");
            ui_state.exit_placement_mode();
            return false;
        };

        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|s| s.name == active_scene_name)
        else {
            tracing::error!("Active scene '{}' not found", active_scene_name);
            ui_state.exit_placement_mode();
            return false;
        };

        let new_id = Self::next_entity_id(&ui_state.scenes[scene_index].entities);

        let entity = match entity_def.create_entity(world_pos_i32, new_id) {
            Ok(entity) => entity,
            Err(e) => {
                tracing::error!("Failed to create entity '{}': {}", entity_def_name, e);
                ui_state.exit_placement_mode();
                return false;
            }
        };

        let can_place = Self::can_place_entity(&entity, world_pos_i32, tilemap, terrain_atlas);

        if can_place {
            let add_command = EditorCommand::add_entity(active_scene_name.clone(), entity);
            let added = ui_state.execute_command(add_command);
            if !added {
                tracing::warn!(
                    "Skipping placement for entity '{}' in scene '{}' because command application failed",
                    entity_def_name,
                    active_scene_name
                );
                return false;
            }
            tracing::info!(
                "Successfully placed entity '{}' (ID: {}) in scene '{}' at world position ({}, {})",
                entity_def_name,
                new_id,
                active_scene_name,
                world_pos_i32.x,
                world_pos_i32.y
            );
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

        let entity_def = match Self::load_entity_definition(project_path, entity_def_name) {
            Ok(entity_def) => entity_def,
            Err(_) => return false,
        };

        let world_pos_i32 = Self::placement_world_position_to_entity_position(world_pos);

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

    fn placement_world_position_to_entity_position(world_pos: glam::Vec2) -> glam::IVec2 {
        glam::IVec2::new(world_pos.x.floor() as i32, world_pos.y.floor() as i32)
    }

    fn next_entity_id(entities: &[Entity]) -> toki_core::entity::EntityId {
        entities.iter().map(|e| e.id).max().unwrap_or(0) + 1
    }

    fn can_place_entity(
        entity: &Entity,
        world_pos_i32: glam::IVec2,
        tilemap: Option<&TileMap>,
        terrain_atlas: Option<&AtlasMeta>,
    ) -> bool {
        match (tilemap, terrain_atlas) {
            (Some(tilemap), Some(terrain_atlas)) => {
                toki_core::collision::can_entity_move_to_position(
                    entity,
                    world_pos_i32,
                    tilemap,
                    terrain_atlas,
                )
            }
            _ => true,
        }
    }

    fn entity_definition_path(project_path: &Path, entity_def_name: &str) -> PathBuf {
        project_path
            .join("entities")
            .join(format!("{}.json", entity_def_name))
    }

    fn load_entity_definition(
        project_path: &Path,
        entity_def_name: &str,
    ) -> Result<EntityDefinition, String> {
        let entity_file = Self::entity_definition_path(project_path, entity_def_name);
        if !entity_file.exists() {
            return Err(format!(
                "Entity definition file not found: {}",
                entity_file.display()
            ));
        }

        let content = std::fs::read_to_string(&entity_file)
            .map_err(|e| format!("Failed to read entity file '{}': {}", entity_def_name, e))?;

        serde_json::from_str::<EntityDefinition>(&content).map_err(|e| {
            format!(
                "Failed to parse entity definition '{}': {}",
                entity_def_name, e
            )
        })
    }
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
