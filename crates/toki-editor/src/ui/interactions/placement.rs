use super::GridInteraction;
use crate::config::EditorConfig;
use crate::editor_services::commands as editor_commands;
use crate::scene::SceneViewport;
use crate::ui::editor_ui::{DecorationPlacementDraft, SceneAnchorPlacementDraft};
use crate::ui::interactions::MapObjectInteraction;
use crate::ui::undo_redo::EditorCommand;
use crate::ui::EditorUI;
use std::path::{Path, PathBuf};
use toki_core::assets::{tilemap::TileMap, tileset::TileSetResolver};
use toki_core::entity::{
    build_decoration_entity, decoration_collision_box, DecorationSpec, Entity, EntityDefinition,
    StoredEntity,
};
use toki_core::scene::{SceneAnchor, SceneAnchorKind};

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
        if let Some(drag_state) = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .scene_anchor_move_drag
            .as_ref()
        {
            if let Some(hover_pos) = response.hover_pos() {
                let display_rect = viewport.display_rect_in(rect);
                let cursor_world = viewport.screen_to_world_pos_raw(hover_pos, display_rect);
                let world_pos = GridInteraction::drag_target_world_position(
                    cursor_world,
                    drag_state.grab_offset,
                    viewport.tilemap(),
                    config,
                );
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_position = Some(world_pos);
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_valid = Some(true);
                viewport.mark_dirty();
            } else {
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_position = None;
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_valid = None;
                viewport.mark_dirty();
            }
            return;
        }

        if crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .is_in_placement_mode()
        {
            if let Some(hover_pos) = response.hover_pos() {
                let display_rect = viewport.display_rect_in(rect);
                let cursor_world = viewport.screen_to_world_pos_raw(hover_pos, display_rect);
                let grab_offset = crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .entity_move_drag
                    .as_ref()
                    .map(|drag| drag.grab_offset)
                    .unwrap_or(glam::Vec2::ZERO);
                let mut world_pos = GridInteraction::drag_target_world_position(
                    cursor_world,
                    grab_offset,
                    viewport.tilemap(),
                    config,
                );
                if crate::ui::editor_context::scene_viewport_context(ui_state)
                    .placement
                    .decoration_draft()
                    .is_some()
                {
                    world_pos =
                        Self::decoration_anchor_world_position(viewport.tilemap(), world_pos);
                }
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_position = Some(world_pos);

                let is_valid =
                    Self::check_placement_validity(ui_state, viewport, world_pos, config);
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_valid = Some(is_valid);
                viewport.mark_dirty();
            } else {
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_position = None;
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .preview_valid = None;
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

        let display_rect = viewport.display_rect_in(rect);
        let mut world_pos = GridInteraction::placement_pose(
            viewport.screen_to_world_pos_raw(click_pos, display_rect),
            viewport.tilemap(),
            config,
        )
        .world_origin;
        if crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .decoration_draft()
            .is_some()
        {
            world_pos = Self::decoration_anchor_world_position(viewport.tilemap(), world_pos);
        }
        if let Some(entity_def_name) = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .entity_definition()
            .map(str::to_string)
        {
            tracing::info!(
                "Placing entity '{}' at world coordinates ({}, {}) [converted from screen ({}, {})]",
                entity_def_name,
                world_pos.x,
                world_pos.y,
                click_pos.x,
                click_pos.y
            );

            if Self::try_place_entity(ui_state, &entity_def_name, world_pos, config, viewport) {
                viewport.mark_dirty();
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .exit_placement_mode();
            }
            return;
        }

        if let Some(decoration_draft) = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .decoration_draft()
            .cloned()
        {
            tracing::info!(
                "Placing decoration '{}:{}' at world coordinates ({}, {})",
                decoration_draft.sheet,
                decoration_draft.object_name,
                world_pos.x,
                world_pos.y
            );
            if Self::try_place_decoration(ui_state, decoration_draft, world_pos, viewport) {
                viewport.mark_dirty();
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .exit_placement_mode();
            }
            return;
        }

        if let Some(anchor_draft) = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .scene_anchor_draft()
            .cloned()
        {
            tracing::info!(
                "Placing scene anchor '{}' ({:?}) at world coordinates ({}, {})",
                anchor_draft.suggested_id,
                anchor_draft.kind,
                world_pos.x,
                world_pos.y
            );
            if Self::try_place_scene_anchor(ui_state, anchor_draft, world_pos) {
                viewport.mark_dirty();
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .exit_placement_mode();
            }
        }
    }

    fn decoration_anchor_world_position(
        tilemap: Option<&TileMap>,
        world_pos: glam::Vec2,
    ) -> glam::Vec2 {
        tilemap
            .and_then(|tilemap| MapObjectInteraction::object_anchor_at_world(tilemap, world_pos))
            .map(|anchor| anchor.as_vec2())
            .unwrap_or(world_pos)
    }

    fn try_place_scene_anchor(
        ui_state: &mut EditorUI,
        anchor_draft: SceneAnchorPlacementDraft,
        world_pos: glam::Vec2,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            tracing::error!("No active scene for scene anchor placement");
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
            return false;
        };
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            tracing::error!("Active scene '{}' not found", active_scene_name);
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
            return false;
        };

        let before_scene = ui_state.scenes[scene_index].clone();
        if before_scene
            .anchors
            .iter()
            .any(|anchor| anchor.id == anchor_draft.suggested_id)
        {
            tracing::warn!(
                "Cannot place scene anchor '{}' in scene '{}': id already exists",
                anchor_draft.suggested_id,
                active_scene_name
            );
            return false;
        }

        let mut after_scene = before_scene.clone();
        after_scene.anchors.push(SceneAnchor {
            id: anchor_draft.suggested_id.clone(),
            kind: anchor_draft.kind,
            position: Self::placement_world_position_to_entity_position(world_pos),
            facing: None,
        });

        let changed = editor_commands::execute(
            ui_state,
            EditorCommand::update_scene(active_scene_name.clone(), before_scene, after_scene),
        );
        if changed {
            ui_state.set_selection(crate::ui::editor_ui::Selection::SceneAnchor {
                scene_name: active_scene_name,
                anchor_id: anchor_draft.suggested_id,
            });
        }
        changed
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
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
            return false;
        };

        let Some(project_path) = config.current_project_path() else {
            tracing::error!("No project path available for entity creation");
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
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
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .exit_placement_mode();
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

    fn try_place_decoration(
        ui_state: &mut EditorUI,
        draft: DecorationPlacementDraft,
        world_pos: glam::Vec2,
        viewport: &SceneViewport,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            tracing::error!("No active scene for decoration placement");
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
            return false;
        };
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|s| s.name == active_scene_name)
        else {
            tracing::error!("Active scene '{}' not found", active_scene_name);
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
            return false;
        };

        let world_pos_i32 = Self::placement_world_position_to_entity_position(world_pos);
        let new_id = Self::next_entity_id(ui_state.scenes[scene_index].entities());
        let entity = build_decoration_entity(
            new_id,
            DecorationSpec {
                position: world_pos_i32,
                size: draft.size_px,
                sheet: draft.sheet.clone(),
                object_name: draft.object_name.clone(),
                grounding: draft.grounding.clone(),
                visible: draft.visible,
                solid: draft.solid,
            },
        );

        let tilemap = viewport.tilemap();
        let tileset = viewport.tileset_resolver();
        let can_place = Self::can_place_entity(&entity, world_pos_i32, tilemap, tileset.as_ref());
        if !can_place {
            tracing::warn!(
                "Cannot place decoration '{}:{}' at position ({}, {}) - collision detected with solid terrain",
                draft.sheet,
                draft.object_name,
                world_pos_i32.x,
                world_pos_i32.y
            );
            return false;
        }

        let changed = editor_commands::execute(
            ui_state,
            EditorCommand::add_entity(active_scene_name, entity),
        );
        if changed {
            ui_state.set_selection(crate::ui::editor_ui::Selection::Entity(new_id));
        }
        changed
    }

    /// Create entity in the active scene, returns true if successful
    fn create_entity_in_scene(
        ui_state: &mut EditorUI,
        entity_def: EntityDefinition,
        entity_def_name: &str,
        world_pos_i32: glam::IVec2,
        viewport: &SceneViewport,
    ) -> bool {
        let tilemap = viewport.tilemap();
        let tileset = viewport.tileset_resolver();
        Self::create_entity_in_scene_with_collision_context(
            ui_state,
            entity_def,
            entity_def_name,
            world_pos_i32,
            tilemap,
            tileset.as_ref(),
        )
    }

    fn create_entity_in_scene_with_collision_context(
        ui_state: &mut EditorUI,
        entity_def: EntityDefinition,
        entity_def_name: &str,
        world_pos_i32: glam::IVec2,
        tilemap: Option<&TileMap>,
        tileset: Option<&TileSetResolver<'_>>,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            tracing::error!("No active scene for entity placement");
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
            return false;
        };

        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|s| s.name == active_scene_name)
        else {
            tracing::error!("Active scene '{}' not found", active_scene_name);
            crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                .placement
                .exit_placement_mode();
            return false;
        };

        let new_id = Self::next_entity_id(ui_state.scenes[scene_index].entities());

        let spawn_bundle = match entity_def.create_spawn_bundle(world_pos_i32, new_id) {
            Ok(bundle) => bundle,
            Err(e) => {
                tracing::error!("Failed to create entity '{}': {}", entity_def_name, e);
                crate::ui::editor_context::scene_viewport_context_mut(ui_state)
                    .placement
                    .exit_placement_mode();
                return false;
            }
        };
        let entity = spawn_bundle.entity.clone();
        let stored = StoredEntity::new(spawn_bundle.entity, spawn_bundle.optional_components);

        let can_place = Self::can_place_entity(&entity, world_pos_i32, tilemap, tileset);

        if can_place {
            let add_command = EditorCommand::add_stored_entity(active_scene_name.clone(), stored);
            let added = editor_commands::execute(ui_state, add_command);
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
        if crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .scene_anchor_draft()
            .is_some()
        {
            return true;
        }

        let world_pos_i32 = Self::placement_world_position_to_entity_position(world_pos);
        if let Some(decoration_draft) = crate::ui::editor_context::scene_viewport_context(ui_state)
            .placement
            .decoration_draft()
        {
            let collision_box = decoration_collision_box(
                decoration_draft.size_px,
                &decoration_draft.grounding,
                decoration_draft.solid,
            );
            return if let Some(tilemap) = viewport.tilemap() {
                let tileset = viewport.tileset_resolver();
                tileset.as_ref().is_none_or(|tileset| {
                    toki_core::collision::can_place_collision_box_at_position(
                        collision_box.as_ref(),
                        world_pos_i32,
                        tilemap,
                        tileset,
                    )
                })
            } else {
                true
            };
        }

        let placement = &crate::ui::editor_context::scene_viewport_context(ui_state).placement;
        let Some(entity_def_name) = placement.entity_definition() else {
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

        let collision_box = entity_def.get_collision_box();
        if let Some(tilemap) = viewport.tilemap() {
            let tileset = viewport.tileset_resolver();
            tileset.as_ref().is_none_or(|tileset| {
                toki_core::collision::can_place_collision_box_at_position(
                    collision_box.as_ref(),
                    world_pos_i32,
                    tilemap,
                    tileset,
                )
            })
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

    pub fn next_scene_anchor_id(anchors: &[SceneAnchor], kind: SceneAnchorKind) -> String {
        let base = match kind {
            SceneAnchorKind::SpawnPoint => "spawn_point",
        };
        let mut index = 1usize;
        loop {
            let candidate = format!("{base}_{index}");
            if anchors.iter().all(|anchor| anchor.id != candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    fn can_place_entity(
        entity: &Entity,
        world_pos_i32: glam::IVec2,
        tilemap: Option<&TileMap>,
        tileset: Option<&TileSetResolver<'_>>,
    ) -> bool {
        match (tilemap, tileset) {
            (Some(tilemap), Some(tileset)) => toki_core::collision::can_entity_move_to_position(
                entity,
                world_pos_i32,
                tilemap,
                tileset,
            ),
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
