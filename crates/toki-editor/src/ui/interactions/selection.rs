use super::GridInteraction;
use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use crate::ui::editor_ui::{EntityMoveDragState, Selection};
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::entity::{Entity, EntityDefinition, EntityType};

/// Handles entity selection and drag operations
pub struct SelectionInteraction;

impl SelectionInteraction {
    /// Handle selection click (single click): select clicked entity and update inspector state.
    pub fn handle_click(
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        click_pos: egui::Pos2,
        rect: egui::Rect,
    ) {
        // Ignore plain click-selection while an explicit move drag operation is active.
        if ui_state.is_entity_move_drag_active() {
            return;
        }

        let world_pos = viewport.screen_to_world_pos(click_pos, rect);
        if let Some(entity_id) = viewport.get_entity_at_world_pos(world_pos) {
            tracing::info!("Selected entity {} via viewport click", entity_id);
            ui_state.set_selection(Selection::Entity(entity_id));
            ui_state.selected_entity_id = Some(entity_id);
        } else {
            tracing::info!(
                "Clearing selection - no entity at world position ({:.1}, {:.1})",
                world_pos.x,
                world_pos.y
            );
            ui_state.clear_selection();
            ui_state.selected_entity_id = None;
        }
    }

    /// Handle drag start (click+hold+drag): begin move operation if drag started over an entity.
    pub fn handle_drag_start(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        drag_start_pos: egui::Pos2,
        rect: egui::Rect,
        config: Option<&EditorConfig>,
    ) {
        if ui_state.is_in_placement_mode() || ui_state.is_entity_move_drag_active() {
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
            .unwrap_or_else(|| Self::entity_type_name(&entity.entity_type).to_string());

        tracing::info!(
            "Starting move drag for entity {} using definition '{}'",
            entity_id,
            entity_def_name
        );

        ui_state.set_selection(Selection::Entity(entity_id));
        ui_state.selected_entity_id = Some(entity_id);
        ui_state.enter_placement_mode(entity_def_name.clone());
        let grab_offset = world_pos - entity.position.as_vec2();
        ui_state.begin_entity_move_drag(EntityMoveDragState {
            scene_name: active_scene_name,
            entity,
            grab_offset,
        });
        viewport.suppress_entity_rendering(entity_id);
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

        let can_drop = Self::can_place_entity_at(viewport, &drag_state.entity, drop_world_pos_i32);
        if can_drop {
            let moved = Self::update_scene_entity_position(
                ui_state,
                &drag_state.scene_name,
                drag_state.entity.id,
                drop_world_pos_i32,
            );
            if moved {
                ui_state.scene_content_changed = true;
                ui_state.set_selection(Selection::Entity(drag_state.entity.id));
                ui_state.selected_entity_id = Some(drag_state.entity.id);
                tracing::info!(
                    "Dropped entity {} at ({}, {})",
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

    fn drop_world_position_to_entity_position(drop_world_pos: glam::Vec2) -> glam::IVec2 {
        glam::IVec2::new(
            drop_world_pos.x.floor() as i32,
            drop_world_pos.y.floor() as i32,
        )
    }

    fn find_scene_entity(
        ui_state: &EditorUI,
        scene_name: &str,
        entity_id: toki_core::entity::EntityId,
    ) -> Option<Entity> {
        let scene = ui_state.scenes.iter().find(|s| s.name == scene_name)?;
        scene.entities.iter().find(|e| e.id == entity_id).cloned()
    }

    fn update_scene_entity_position(
        ui_state: &mut EditorUI,
        scene_name: &str,
        entity_id: toki_core::entity::EntityId,
        new_position: glam::IVec2,
    ) -> bool {
        let Some(scene) = ui_state.scenes.iter_mut().find(|s| s.name == scene_name) else {
            return false;
        };

        let Some(entity) = scene.entities.iter_mut().find(|e| e.id == entity_id) else {
            return false;
        };

        entity.position = new_position;
        true
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
        Some(Self::entity_type_name(&entity.entity_type).to_string())
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
            .entity_type
            .eq_ignore_ascii_case(Self::entity_type_name(&entity.entity_type))
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

    fn entity_type_name(entity_type: &EntityType) -> &'static str {
        match entity_type {
            EntityType::Player => "player",
            EntityType::Npc => "npc",
            EntityType::Item => "item",
            EntityType::Decoration => "decoration",
            EntityType::Trigger => "trigger",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SelectionInteraction;
    use crate::ui::EditorUI;
    use glam::{IVec2, UVec2};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use toki_core::entity::{
        AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityAttributes,
        EntityDefinition, EntityManager, EntityType, RenderingDef,
    };

    fn unique_temp_project_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before UNIX_EPOCH")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "toki-selection-tests-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(dir.join("entities")).expect("failed to create temp entities directory");
        dir
    }

    fn sample_entity_definition(name: &str, entity_type: &str, size: [u32; 2]) -> EntityDefinition {
        EntityDefinition {
            name: name.to_string(),
            display_name: format!("Display {name}"),
            description: format!("Definition for {name}"),
            entity_type: entity_type.to_string(),
            rendering: RenderingDef {
                size,
                render_layer: 0,
                visible: true,
            },
            attributes: AttributesDef {
                health: Some(50),
                speed: 1,
                solid: true,
                active: true,
                can_move: false,
                has_inventory: false,
            },
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size,
                trigger: false,
            },
            audio: AudioDef {
                footstep_trigger_distance: 16.0,
                movement_sound: "sfx_step".to_string(),
                collision_sound: Some("sfx_hit2".to_string()),
            },
            animations: AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: vec![AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["slime/idle_0".to_string()],
                    frame_duration_ms: 150.0,
                    loop_mode: "loop".to_string(),
                }],
                default_state: "idle".to_string(),
            },
            category: "test".to_string(),
            tags: vec!["selection".to_string()],
        }
    }

    fn write_entity_definition_file(project_dir: &Path, definition: &EntityDefinition) {
        let file_path = project_dir
            .join("entities")
            .join(format!("{}.json", definition.name));
        let json = serde_json::to_string_pretty(definition)
            .expect("failed to serialize entity definition");
        fs::write(&file_path, json).expect("failed to write entity definition file");
    }

    #[test]
    fn resolve_entity_definition_name_prefers_entity_metadata() {
        let mut manager = EntityManager::new();
        let entity_id = manager.spawn_entity(
            EntityType::Npc,
            IVec2::new(10, 20),
            UVec2::new(16, 16),
            EntityAttributes::default(),
        );
        let mut entity = manager
            .get_entity(entity_id)
            .expect("missing spawned entity")
            .clone();
        entity.definition_name = Some("custom_npc".to_string());

        let resolved = SelectionInteraction::resolve_entity_definition_name(&entity, None);
        assert_eq!(resolved, Some("custom_npc".to_string()));
    }

    #[test]
    fn resolve_entity_definition_name_uses_project_lookup_for_legacy_entities() {
        let project_dir = unique_temp_project_dir();

        let small = sample_entity_definition("small_npc", "npc", [16, 16]);
        let large = sample_entity_definition("large_npc", "npc", [32, 32]);
        write_entity_definition_file(&project_dir, &small);
        write_entity_definition_file(&project_dir, &large);

        let mut legacy_entity = large
            .create_entity(IVec2::new(0, 0), 1)
            .expect("failed to create runtime entity");
        legacy_entity.definition_name = None;

        let resolved = SelectionInteraction::resolve_entity_definition_name(
            &legacy_entity,
            Some(project_dir.as_path()),
        );
        assert_eq!(resolved, Some("large_npc".to_string()));
    }

    #[test]
    fn resolve_entity_definition_name_falls_back_to_entity_type_name() {
        let mut manager = EntityManager::new();
        let entity_id = manager.spawn_entity(
            EntityType::Player,
            IVec2::new(0, 0),
            UVec2::new(16, 16),
            EntityAttributes::default(),
        );
        let entity = manager
            .get_entity(entity_id)
            .expect("missing spawned entity")
            .clone();

        let resolved = SelectionInteraction::resolve_entity_definition_name(&entity, None);
        assert_eq!(resolved, Some("player".to_string()));
    }

    #[test]
    fn update_scene_entity_position_moves_target_entity() {
        let mut ui_state = EditorUI::new();
        let mut manager = EntityManager::new();
        let entity_id = manager.spawn_entity(
            EntityType::Npc,
            IVec2::new(10, 20),
            UVec2::new(16, 16),
            EntityAttributes::default(),
        );
        let entity = manager
            .get_entity(entity_id)
            .expect("missing spawned entity")
            .clone();

        let scene = ui_state
            .scenes
            .iter_mut()
            .find(|s| s.name == "Main Scene")
            .expect("missing default scene");
        scene.entities.push(entity);

        let moved = SelectionInteraction::update_scene_entity_position(
            &mut ui_state,
            "Main Scene",
            entity_id,
            IVec2::new(42, 84),
        );
        assert!(moved);

        let scene = ui_state
            .scenes
            .iter()
            .find(|s| s.name == "Main Scene")
            .expect("missing default scene");
        let moved_entity = scene
            .entities
            .iter()
            .find(|e| e.id == entity_id)
            .expect("missing moved entity");
        assert_eq!(moved_entity.position, IVec2::new(42, 84));
    }

    #[test]
    fn update_scene_entity_position_returns_false_when_entity_missing() {
        let mut ui_state = EditorUI::new();
        let moved = SelectionInteraction::update_scene_entity_position(
            &mut ui_state,
            "Main Scene",
            999,
            IVec2::new(42, 84),
        );
        assert!(!moved);
    }

    #[test]
    fn update_scene_entity_position_returns_false_when_scene_missing() {
        let mut ui_state = EditorUI::new();
        let moved = SelectionInteraction::update_scene_entity_position(
            &mut ui_state,
            "Missing Scene",
            1,
            IVec2::new(42, 84),
        );
        assert!(!moved);
    }

    #[test]
    fn drop_world_position_to_entity_position_keeps_snapped_top_left() {
        // Regression: drag-drop used an extra center->top-left conversion, causing one-tile offset.
        let drop_world = glam::Vec2::new(32.0, 48.0);
        let dropped = SelectionInteraction::drop_world_position_to_entity_position(drop_world);
        assert_eq!(dropped, IVec2::new(32, 48));
    }
}
