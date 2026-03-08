use crate::config::EditorConfig;
use crate::scene::SceneViewport;
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::entity::{Entity, EntityDefinition, EntityType};

/// Handles entity selection and drag operations
pub struct SelectionInteraction;

impl SelectionInteraction {
    /// Handle entity selection click - starts drag operation
    pub fn handle_click(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        click_pos: egui::Pos2,
        rect: egui::Rect,
        config: Option<&EditorConfig>,
    ) {
        tracing::info!("Regular click detected at screen pos: {:?}", click_pos);
        let world_pos = viewport.screen_to_world_pos(click_pos, rect);
        let project_path = config.and_then(|cfg| cfg.current_project_path().map(|p| p.as_path()));

        if let Some(entity_id) = viewport.get_entity_at_world_pos(world_pos) {
            Self::start_entity_drag_operation(ui_state, viewport, entity_id, project_path);
        } else {
            tracing::info!(
                "No entity clicked at world position ({:.1}, {:.1})",
                world_pos.x,
                world_pos.y
            );
        }
    }

    /// Start drag operation for selected entity
    fn start_entity_drag_operation(
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        entity_id: toki_core::entity::EntityId,
        project_path: Option<&Path>,
    ) {
        tracing::info!("Entity {} clicked - starting drag operation", entity_id);

        let Some(entity) = viewport
            .scene_manager()
            .game_state()
            .entity_manager()
            .get_entity(entity_id)
        else {
            tracing::warn!("Could not find entity {} for drag operation", entity_id);
            return;
        };

        let entity_def_name = Self::resolve_entity_definition_name(entity, project_path);
        let Some(entity_def_name) = entity_def_name else {
            tracing::warn!(
                "Could not resolve definition name for entity {} - cannot start drag operation",
                entity_id
            );
            return;
        };
        tracing::info!(
            "Removing entity {} and entering placement mode with definition '{}'",
            entity_id,
            entity_def_name
        );

        Self::remove_entity_from_scene(ui_state, entity_id);
        ui_state.enter_placement_mode(entity_def_name);
    }

    /// Resolve the best entity definition name for re-entering placement mode.
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
        if entity.movement_sound.as_deref() == Some(definition.audio.movement_sound.as_str()) {
            score += 2;
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

    /// Remove entity from the active scene
    fn remove_entity_from_scene(ui_state: &mut EditorUI, entity_id: toki_core::entity::EntityId) {
        let Some(active_scene_name) = &ui_state.active_scene else {
            tracing::warn!("No active scene to remove entity from");
            return;
        };

        let Some(scene) = ui_state
            .scenes
            .iter_mut()
            .find(|s| s.name == *active_scene_name)
        else {
            tracing::warn!("Active scene '{}' not found", active_scene_name);
            return;
        };

        scene.entities.retain(|e| e.id != entity_id);
        ui_state.scene_content_changed = true;
        tracing::info!(
            "Removed entity {} from scene '{}'",
            entity_id,
            active_scene_name
        );
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
    fn remove_entity_from_scene_deletes_entity_and_marks_scene_dirty() {
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
        assert_eq!(scene.entities.len(), 1);

        SelectionInteraction::remove_entity_from_scene(&mut ui_state, entity_id);

        let scene = ui_state
            .scenes
            .iter()
            .find(|s| s.name == "Main Scene")
            .expect("missing default scene");
        assert_eq!(scene.entities.len(), 0);
        assert!(ui_state.scene_content_changed);
    }

    #[test]
    fn remove_entity_from_scene_is_noop_without_active_scene() {
        let mut ui_state = EditorUI::new();
        ui_state.active_scene = None;
        SelectionInteraction::remove_entity_from_scene(&mut ui_state, 999);
        assert!(!ui_state.scene_content_changed);
    }
}
