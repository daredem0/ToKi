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
mod tests {
    use super::PlacementInteraction;
    use crate::ui::EditorUI;
    use glam::{IVec2, UVec2, Vec2};
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
    use toki_core::assets::tilemap::TileMap;
    use toki_core::entity::{
        AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
        EntityType, RenderingDef,
    };

    fn sample_entity_definition(name: &str) -> EntityDefinition {
        EntityDefinition {
            name: name.to_string(),
            display_name: "Sample Entity".to_string(),
            description: "Entity used for placement tests".to_string(),
            entity_type: "npc".to_string(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
            },
            attributes: AttributesDef {
                health: Some(10),
                speed: 1,
                solid: true,
                active: true,
                can_move: false,
                ai_behavior: toki_core::entity::AiBehavior::Wander,
                has_inventory: false,
            },
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
            },
            audio: AudioDef {
                footstep_trigger_distance: 32.0,
                movement_sound: "sfx_step".to_string(),
                collision_sound: Some("sfx_hit2".to_string()),
            },
            animations: AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: vec![AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["slime/idle_0".to_string()],
                    frame_duration_ms: 120.0,
                    loop_mode: "loop".to_string(),
                }],
                default_state: "idle".to_string(),
            },
            category: "test".to_string(),
            tags: vec!["placement".to_string()],
        }
    }

    fn unique_temp_project_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before UNIX_EPOCH")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "toki-placement-tests-{}-{}",
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(dir.join("entities")).expect("failed to create temp entities directory");
        dir
    }

    fn write_entity_definition_file(project_dir: &Path, entity_def: &EntityDefinition) {
        let file_path = project_dir
            .join("entities")
            .join(format!("{}.json", entity_def.name));
        let json = serde_json::to_string_pretty(entity_def)
            .expect("failed to serialize entity definition");
        fs::write(&file_path, json).expect("failed to write entity definition file");
    }

    fn placement_collision_assets() -> (TileMap, AtlasMeta) {
        let mut tiles = HashMap::new();
        tiles.insert(
            "solid".to_string(),
            TileInfo {
                position: UVec2::new(0, 0),
                properties: TileProperties {
                    solid: true,
                    trigger: false,
                },
            },
        );
        tiles.insert(
            "floor".to_string(),
            TileInfo {
                position: UVec2::new(1, 0),
                properties: TileProperties {
                    solid: false,
                    trigger: false,
                },
            },
        );

        let atlas = AtlasMeta {
            image: PathBuf::from("test.png"),
            tile_size: UVec2::new(16, 16),
            tiles,
        };

        let tilemap = TileMap {
            size: UVec2::new(2, 2),
            tile_size: UVec2::new(16, 16),
            atlas: PathBuf::from("test_atlas.json"),
            // top-left is solid, others are floor
            tiles: vec![
                "solid".to_string(),
                "floor".to_string(),
                "floor".to_string(),
                "floor".to_string(),
            ],
        };

        (tilemap, atlas)
    }

    #[test]
    fn placement_world_position_to_entity_position_uses_top_left_floored_coordinates() {
        let placed = PlacementInteraction::placement_world_position_to_entity_position(Vec2::new(
            64.9, 48.1,
        ));
        assert_eq!(placed, IVec2::new(64, 48));
    }

    #[test]
    fn placement_world_position_to_entity_position_handles_negative_values_with_floor() {
        let placed = PlacementInteraction::placement_world_position_to_entity_position(Vec2::new(
            -0.1, -16.1,
        ));
        assert_eq!(placed, IVec2::new(-1, -17));
    }

    #[test]
    fn next_entity_id_returns_one_for_empty_scene() {
        let next = PlacementInteraction::next_entity_id(&[]);
        assert_eq!(next, 1);
    }

    #[test]
    fn next_entity_id_uses_max_id_plus_one() {
        let entity_def = sample_entity_definition("entity_a");
        let a = entity_def
            .create_entity(IVec2::new(0, 0), 7)
            .expect("failed to create entity a");
        let b = entity_def
            .create_entity(IVec2::new(0, 0), 42)
            .expect("failed to create entity b");
        let next = PlacementInteraction::next_entity_id(&[a, b]);
        assert_eq!(next, 43);
    }

    #[test]
    fn load_entity_definition_succeeds_for_valid_file() {
        let project_dir = unique_temp_project_dir();
        let entity_def = sample_entity_definition("valid_entity");
        write_entity_definition_file(&project_dir, &entity_def);

        let loaded = PlacementInteraction::load_entity_definition(&project_dir, "valid_entity")
            .expect("expected valid entity definition to load");
        assert_eq!(loaded.name, "valid_entity");
        assert_eq!(loaded.entity_type, "npc");
    }

    #[test]
    fn load_entity_definition_fails_for_missing_file() {
        let project_dir = unique_temp_project_dir();
        let err = PlacementInteraction::load_entity_definition(&project_dir, "does_not_exist")
            .expect_err("expected missing definition to fail");
        assert!(err.contains("not found"));
    }

    #[test]
    fn load_entity_definition_fails_for_invalid_json() {
        let project_dir = unique_temp_project_dir();
        let file_path = project_dir.join("entities").join("broken.json");
        fs::write(&file_path, "{ this is not valid json").expect("failed to write broken json");

        let err = PlacementInteraction::load_entity_definition(&project_dir, "broken")
            .expect_err("expected invalid json to fail");
        assert!(err.contains("Failed to parse entity definition"));
    }

    #[test]
    fn create_entity_in_scene_adds_entity_and_marks_scene_changed() {
        let mut ui_state = EditorUI::new();
        ui_state.enter_placement_mode("sample".to_string());
        let entity_def = sample_entity_definition("sample");

        let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
            &mut ui_state,
            entity_def,
            "sample",
            IVec2::new(32, 48),
            None,
            None,
        );
        assert!(placed);

        let scene = ui_state
            .scenes
            .iter()
            .find(|s| s.name == "Main Scene")
            .expect("missing default scene");
        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].position, IVec2::new(32, 48));
        assert_eq!(scene.entities[0].entity_type, EntityType::Npc);
        assert_eq!(scene.entities[0].definition_name.as_deref(), Some("sample"));
        assert!(ui_state.scene_content_changed);
        assert!(ui_state.can_undo());
        // Placement mode exits at a higher level after successful click.
        assert!(ui_state.is_in_placement_mode());

        assert!(ui_state.undo());
        let scene = ui_state
            .scenes
            .iter()
            .find(|s| s.name == "Main Scene")
            .expect("missing default scene");
        assert!(scene.entities.is_empty());

        assert!(ui_state.redo());
        let scene = ui_state
            .scenes
            .iter()
            .find(|s| s.name == "Main Scene")
            .expect("missing default scene");
        assert_eq!(scene.entities.len(), 1);
    }

    #[test]
    fn create_entity_in_scene_exits_placement_mode_when_no_active_scene() {
        let mut ui_state = EditorUI::new();
        ui_state.active_scene = None;
        ui_state.enter_placement_mode("sample".to_string());

        let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
            &mut ui_state,
            sample_entity_definition("sample"),
            "sample",
            IVec2::new(0, 0),
            None,
            None,
        );
        assert!(!placed);
        assert!(!ui_state.is_in_placement_mode());
    }

    #[test]
    fn create_entity_in_scene_exits_placement_mode_when_active_scene_missing() {
        let mut ui_state = EditorUI::new();
        ui_state.active_scene = Some("Missing Scene".to_string());
        ui_state.enter_placement_mode("sample".to_string());

        let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
            &mut ui_state,
            sample_entity_definition("sample"),
            "sample",
            IVec2::new(0, 0),
            None,
            None,
        );
        assert!(!placed);
        assert!(!ui_state.is_in_placement_mode());
    }

    #[test]
    fn create_entity_in_scene_blocks_on_solid_terrain_and_keeps_placement_mode() {
        let mut ui_state = EditorUI::new();
        ui_state.enter_placement_mode("sample".to_string());
        let (tilemap, atlas) = placement_collision_assets();

        let placed = PlacementInteraction::create_entity_in_scene_with_collision_context(
            &mut ui_state,
            sample_entity_definition("sample"),
            "sample",
            IVec2::new(0, 0), // top-left tile is solid in test map
            Some(&tilemap),
            Some(&atlas),
        );

        assert!(!placed);
        let scene = ui_state
            .scenes
            .iter()
            .find(|s| s.name == "Main Scene")
            .expect("missing default scene");
        assert_eq!(scene.entities.len(), 0);
        assert!(!ui_state.scene_content_changed);
        assert!(ui_state.is_in_placement_mode());
    }

    #[test]
    fn can_place_entity_returns_true_without_collision_context() {
        let entity = sample_entity_definition("sample")
            .create_entity(IVec2::new(0, 0), 1)
            .expect("failed to create entity");
        assert!(PlacementInteraction::can_place_entity(
            &entity,
            IVec2::new(0, 0),
            None,
            None
        ));
    }
}
