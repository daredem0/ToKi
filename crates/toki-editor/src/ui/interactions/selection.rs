use super::GridInteraction;
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use crate::ui::editor_ui::EntityMoveDragState;
use crate::ui::undo_redo::{EditorCommand, EntityPosition};
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::entity::{Entity, EntityDefinition, EntityKind};

/// Handles entity selection and drag operations
pub struct SelectionInteraction;

impl SelectionInteraction {
    /// Handle selection click (single click): select clicked entity and update inspector state.
    pub fn handle_click(
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        click_pos: egui::Pos2,
        rect: egui::Rect,
        ctrl_pressed: bool,
    ) {
        // Ignore plain click-selection while an explicit move drag operation is active.
        if ui_state.is_entity_move_drag_active() {
            return;
        }

        let world_pos = viewport.screen_to_world_pos(click_pos, rect);
        let clicked_entity = viewport.get_entity_at_world_pos(world_pos);
        Self::apply_click_selection(ui_state, clicked_entity, ctrl_pressed);
    }

    fn apply_click_selection(
        ui_state: &mut EditorUI,
        clicked_entity: Option<toki_core::entity::EntityId>,
        ctrl_pressed: bool,
    ) {
        if let Some(entity_id) = clicked_entity {
            tracing::info!("Selected entity {} via viewport click", entity_id);
            if ctrl_pressed {
                ui_state.toggle_entity_selection(entity_id);
            } else {
                ui_state.set_single_entity_selection(entity_id);
            }
            return;
        }

        if ctrl_pressed {
            return;
        }

        tracing::info!("Clearing selection - no entity under viewport click");
        ui_state.clear_entity_selection();
    }

    /// Handle drag start (click+hold+drag): begin move operation if drag started over an entity.
    pub fn handle_drag_start(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        drag_start_pos: egui::Pos2,
        rect: egui::Rect,
        config: Option<&EditorConfig>,
        ctrl_pressed: bool,
    ) {
        if ui_state.is_in_placement_mode() || ui_state.is_entity_move_drag_active() || ctrl_pressed
        {
            return;
        }

        let world_pos = viewport.screen_to_world_pos(drag_start_pos, rect);
        let Some(entity_id) = viewport.get_entity_at_world_pos(world_pos) else {
            return;
        };

        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            tracing::warn!("Cannot start entity move drag: no active scene");
            return;
        };

        let Some(entity) = Self::find_scene_entity(ui_state, &active_scene_name, entity_id) else {
            tracing::warn!(
                "Cannot start entity move drag: entity {} not found in active scene '{}'",
                entity_id,
                active_scene_name
            );
            return;
        };

        let project_path = config.and_then(|cfg| cfg.current_project_path().map(|p| p.as_path()));
        let entity_def_name = Self::resolve_entity_definition_name(&entity, project_path)
            .unwrap_or_else(|| Self::entity_kind_name(&entity.entity_kind).to_string());

        tracing::info!(
            "Starting move drag for entity {} using definition '{}'",
            entity_id,
            entity_def_name
        );

        let dragged_entities =
            Self::drag_entities_for_start(ui_state, &active_scene_name, &entity, entity_id);
        if dragged_entities.len() == 1 {
            ui_state.set_single_entity_selection(entity_id);
        } else {
            ui_state.selection = Some(crate::ui::editor_ui::Selection::Entity(entity_id));
        }
        ui_state.enter_placement_mode(entity_def_name.clone());
        let grab_offset = world_pos - entity.position.as_vec2();
        ui_state.begin_entity_move_drag(EntityMoveDragState {
            scene_name: active_scene_name,
            entity,
            dragged_entities,
            grab_offset,
        });
        viewport.suppress_entity_rendering_many(
            ui_state
                .entity_move_drag
                .as_ref()
                .into_iter()
                .flat_map(|drag| drag.dragged_entities.iter().map(|entity| entity.id)),
        );
    }

    pub fn handle_marquee_drag_start(ui_state: &mut EditorUI, drag_start_pos: egui::Pos2) {
        ui_state.start_marquee_selection(drag_start_pos);
    }

    pub fn handle_marquee_drag_update(ui_state: &mut EditorUI, drag_pos: egui::Pos2) {
        ui_state.update_marquee_selection(drag_pos);
    }

    pub fn handle_marquee_drag_release(
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        rect: egui::Rect,
        ctrl_pressed: bool,
    ) {
        let Some(marquee) = ui_state.finish_marquee_selection() else {
            return;
        };

        let world_start = viewport.screen_to_world_pos(marquee.start_screen, rect);
        let world_end = viewport.screen_to_world_pos(marquee.current_screen, rect);
        let selected_entity_ids =
            Self::collect_scene_entities_in_world_rect(ui_state, world_start, world_end);
        Self::apply_marquee_selection(ui_state, selected_entity_ids, ctrl_pressed);
    }

    /// Handle drag release: try to drop entity at release position.
    /// On invalid drop, entity remains at original position (snap back behavior).
    pub fn handle_drag_release(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        drop_pos: Option<egui::Pos2>,
        rect: egui::Rect,
        config: Option<&EditorConfig>,
    ) {
        let Some(drag_state) = ui_state.entity_move_drag.clone() else {
            return;
        };

        let Some(drop_pos) = drop_pos else {
            tracing::warn!(
                "Entity drag ended without pointer position - cancelling move for entity {}",
                drag_state.entity.id
            );
            ui_state.exit_placement_mode();
            viewport.clear_suppressed_entity_rendering();
            viewport.mark_dirty();
            return;
        };

        let drop_world_pos = GridInteraction::drag_target_world_position(
            viewport.screen_to_world_pos_raw(drop_pos, rect),
            drag_state.grab_offset,
            viewport.scene_manager().tilemap(),
            config,
        );
        let drop_world_pos_i32 = Self::drop_world_position_to_entity_position(drop_world_pos);

        let drop_delta = drop_world_pos_i32 - drag_state.entity.position;
        let can_drop =
            Self::can_drop_dragged_entities(viewport, &drag_state.dragged_entities, drop_delta);
        if can_drop {
            let before_positions = drag_state
                .dragged_entities
                .iter()
                .map(|entity| EntityPosition::new(entity.id, entity.position))
                .collect::<Vec<_>>();
            let after_positions = drag_state
                .dragged_entities
                .iter()
                .map(|entity| EntityPosition::new(entity.id, entity.position + drop_delta))
                .collect::<Vec<_>>();
            let moved_count = after_positions.len();

            if ui_state.execute_command(EditorCommand::move_entities(
                drag_state.scene_name.clone(),
                before_positions,
                after_positions,
            )) {
                ui_state.scene_content_changed = true;
                if drag_state.dragged_entities.len() == 1 {
                    ui_state.set_single_entity_selection(drag_state.entity.id);
                } else {
                    ui_state.selection = Some(crate::ui::editor_ui::Selection::Entity(
                        drag_state.entity.id,
                    ));
                }
                tracing::info!(
                    "Dropped {} dragged entities with anchor {} at ({}, {})",
                    moved_count,
                    drag_state.entity.id,
                    drop_world_pos_i32.x,
                    drop_world_pos_i32.y
                );
            } else {
                tracing::warn!(
                    "Entity move drag drop failed - entity {} no longer present in scene '{}'",
                    drag_state.entity.id,
                    drag_state.scene_name
                );
            }
        } else {
            tracing::warn!(
                "Invalid drop for entity {} at ({}, {}) - snapping back to original position ({}, {})",
                drag_state.entity.id,
                drop_world_pos_i32.x,
                drop_world_pos_i32.y,
                drag_state.entity.position.x,
                drag_state.entity.position.y
            );
        }

        ui_state.exit_placement_mode();
        viewport.clear_suppressed_entity_rendering();
        viewport.mark_dirty();
    }

    fn can_place_entity_at(
        viewport: &SceneViewport,
        entity: &Entity,
        world_pos_i32: glam::IVec2,
    ) -> bool {
        if let Some(tilemap) = viewport.scene_manager().tilemap() {
            let terrain_atlas = viewport.scene_manager().resources().get_terrain_atlas();
            toki_core::collision::can_entity_move_to_position(
                entity,
                world_pos_i32,
                tilemap,
                terrain_atlas,
            )
        } else {
            true
        }
    }

    fn can_drop_dragged_entities(
        viewport: &SceneViewport,
        dragged_entities: &[Entity],
        drop_delta: glam::IVec2,
    ) -> bool {
        dragged_entities.iter().all(|entity| {
            let target_position = entity.position + drop_delta;
            Self::can_place_entity_at(viewport, entity, target_position)
        })
    }

    fn drop_world_position_to_entity_position(drop_world_pos: glam::Vec2) -> glam::IVec2 {
        glam::IVec2::new(
            drop_world_pos.x.floor() as i32,
            drop_world_pos.y.floor() as i32,
        )
    }

    fn collect_scene_entities_in_world_rect(
        ui_state: &EditorUI,
        world_start: glam::Vec2,
        world_end: glam::Vec2,
    ) -> Vec<toki_core::entity::EntityId> {
        let Some(active_scene_name) = ui_state.active_scene.as_ref() else {
            return Vec::new();
        };
        let Some(scene) = ui_state
            .scenes
            .iter()
            .find(|s| &s.name == active_scene_name)
        else {
            return Vec::new();
        };

        let min_x = world_start.x.min(world_end.x);
        let min_y = world_start.y.min(world_end.y);
        let max_x = world_start.x.max(world_end.x);
        let max_y = world_start.y.max(world_end.y);

        scene
            .entities
            .iter()
            .filter(|entity| {
                let entity_min_x = entity.position.x as f32;
                let entity_min_y = entity.position.y as f32;
                let entity_max_x = entity_min_x + entity.size.x as f32;
                let entity_max_y = entity_min_y + entity.size.y as f32;

                entity_min_x < max_x
                    && entity_max_x > min_x
                    && entity_min_y < max_y
                    && entity_max_y > min_y
            })
            .map(|entity| entity.id)
            .collect()
    }

    fn apply_marquee_selection(
        ui_state: &mut EditorUI,
        selected_entity_ids: Vec<toki_core::entity::EntityId>,
        ctrl_pressed: bool,
    ) {
        if selected_entity_ids.is_empty() {
            if !ctrl_pressed {
                ui_state.clear_entity_selection();
            }
            return;
        }

        if !ctrl_pressed {
            ui_state.clear_entity_selection();
        }

        for entity_id in selected_entity_ids {
            ui_state.add_entity_to_selection(entity_id);
        }
    }

    fn find_scene_entity(
        ui_state: &EditorUI,
        scene_name: &str,
        entity_id: toki_core::entity::EntityId,
    ) -> Option<Entity> {
        let scene = ui_state.scenes.iter().find(|s| s.name == scene_name)?;
        scene.entities.iter().find(|e| e.id == entity_id).cloned()
    }

    fn drag_entities_for_start(
        ui_state: &EditorUI,
        scene_name: &str,
        clicked_entity: &Entity,
        clicked_entity_id: toki_core::entity::EntityId,
    ) -> Vec<Entity> {
        if ui_state.selected_entity_ids().len() <= 1
            || !ui_state.selected_entity_ids().contains(&clicked_entity_id)
        {
            return vec![clicked_entity.clone()];
        }

        let Some(scene) = ui_state.scenes.iter().find(|s| s.name == scene_name) else {
            return vec![clicked_entity.clone()];
        };

        let selected_set = ui_state
            .selected_entity_ids()
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>();
        let mut dragged = scene
            .entities
            .iter()
            .filter(|entity| selected_set.contains(&entity.id))
            .cloned()
            .collect::<Vec<_>>();

        if dragged.is_empty() {
            dragged.push(clicked_entity.clone());
        }
        dragged
    }

    /// Resolve the best entity definition name for placement preview during drag-move.
    fn resolve_entity_definition_name(
        entity: &Entity,
        project_path: Option<&Path>,
    ) -> Option<String> {
        if let Some(name) = &entity.definition_name {
            return Some(name.clone());
        }

        if let Some(project_path) = project_path {
            if let Some(name) = Self::find_best_matching_definition_name(project_path, entity) {
                return Some(name);
            }
        }

        // Last-resort fallback for legacy scene entities that predate definition_name.
        Some(Self::entity_kind_name(&entity.entity_kind).to_string())
    }

    fn find_best_matching_definition_name(project_path: &Path, entity: &Entity) -> Option<String> {
        let entities_dir = project_path.join("entities");
        let entries = std::fs::read_dir(&entities_dir).ok()?;

        let mut definition_files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        definition_files.sort();

        let mut best_match: Option<(i32, String)> = None;

        for path in definition_files {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(definition) = serde_json::from_str::<EntityDefinition>(&content) else {
                continue;
            };
            let Some(score) = Self::definition_match_score(entity, &definition) else {
                continue;
            };

            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let candidate = stem.to_string();

            if best_match
                .as_ref()
                .map(|(best_score, _)| score > *best_score)
                .unwrap_or(true)
            {
                best_match = Some((score, candidate));
            }
        }

        best_match.map(|(_, name)| name)
    }

    fn definition_match_score(entity: &Entity, definition: &EntityDefinition) -> Option<i32> {
        if !definition
            .category
            .eq_ignore_ascii_case(Self::entity_kind_name(&entity.entity_kind))
        {
            return None;
        }

        let mut score = 10;

        if definition.rendering.size == [entity.size.x, entity.size.y] {
            score += 4;
        }
        if definition.attributes.speed == entity.attributes.speed {
            score += 2;
        }
        if definition.attributes.solid == entity.attributes.solid {
            score += 2;
        }
        if definition.attributes.can_move == entity.attributes.can_move {
            score += 1;
        }
        if definition.attributes.active == entity.attributes.active {
            score += 1;
        }
        if definition.rendering.render_layer == entity.attributes.render_layer {
            score += 1;
        }
        if definition.attributes.health == entity.attributes.health {
            score += 1;
        }
        if definition.collision.enabled == entity.collision_box.is_some() {
            score += 2;
        }

        if let Some(collision_box) = &entity.collision_box {
            if definition.collision.offset == [collision_box.offset.x, collision_box.offset.y] {
                score += 1;
            }
            if definition.collision.size == [collision_box.size.x, collision_box.size.y] {
                score += 2;
            }
            if definition.collision.trigger == collision_box.trigger {
                score += 1;
            }
        }

        Some(score)
    }

    fn entity_kind_name(entity_kind: &EntityKind) -> &'static str {
        match entity_kind {
            EntityKind::Player => "human",
            EntityKind::Npc => "creature",
            EntityKind::Item => "item",
            EntityKind::Decoration => "decoration",
            EntityKind::Trigger => "trigger",
            EntityKind::Projectile => "projectile",
        }
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
