use crate::project::templates::{populate_project_template, top_down_main_scene_bytes};
use crate::project::{Project, ProjectAssets, ProjectTemplateKind};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use toki_core::{GameState, Scene};

/// Manages project operations (create, load, save)
#[derive(Debug)]
pub struct ProjectManager {
    /// Currently active project
    pub current_project: Option<Project>,
    /// Asset manager for current project
    pub project_assets: Option<ProjectAssets>,
    /// List of recently opened projects
    recent_projects: Vec<PathBuf>,
    /// Maximum number of recent projects to track
    max_recent: usize,
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectManager {
    /// Create a new project manager
    pub fn new() -> Self {
        Self {
            current_project: None,
            project_assets: None,
            recent_projects: Vec::new(),
            max_recent: 10,
        }
    }

    /// Create a new project at the specified location
    pub fn create_new_project(&mut self, name: String, parent_path: PathBuf) -> Result<GameState> {
        self.create_new_project_with_template(name, parent_path, ProjectTemplateKind::Empty)
    }

    /// Create a new project at the specified location from a specific template
    pub fn create_new_project_with_template(
        &mut self,
        name: String,
        parent_path: PathBuf,
        template: ProjectTemplateKind,
    ) -> Result<GameState> {
        // Create project folder
        let project_path = parent_path.join(&name);
        if project_path.exists() {
            return Err(anyhow::anyhow!(
                "Project folder already exists: {:?}",
                project_path
            ));
        }

        tracing::info!(
            "Creating project '{}' from template '{}' at {:?}",
            name,
            template.label(),
            project_path
        );

        // Create project structure
        self.create_project_structure(&project_path)?;

        // Create project data
        let mut project = Project::new(name, project_path);
        if template == ProjectTemplateKind::TopDownStarter {
            project.metadata.project.description = template.description().to_string();
        }

        // Save project metadata
        project.save_metadata()?;

        // Set as current project first
        self.current_project = Some(project);

        // Initialize asset manager
        let project_path = self.current_project.as_ref().unwrap().path.clone();
        let mut project_assets = ProjectAssets::new(project_path);

        populate_project_template(&self.current_project.as_ref().unwrap().path, template)?;

        match template {
            ProjectTemplateKind::Empty => {
                let default_scene = Scene::new("main".to_string());
                project_assets.save_scene(&default_scene)?;
            }
            ProjectTemplateKind::TopDownStarter => {
                let starter_scene: Scene = serde_json::from_slice(top_down_main_scene_bytes())
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to parse built-in top-down scene: {}", e)
                    })?;
                project_assets.save_scene(&starter_scene)?;
            }
        }

        // Scan for any existing assets
        project_assets.scan_assets()?;
        self.project_assets = Some(project_assets);
        let game_state = GameState::new_empty();

        // Add to recent projects
        let project_path = self.current_project.as_ref().unwrap().path.clone();
        self.add_to_recent(project_path);

        tracing::info!("Successfully created new project");
        Ok(game_state)
    }

    /// Open an existing project
    pub fn open_project(&mut self, project_path: PathBuf) -> Result<GameState> {
        tracing::info!("Opening project at {:?}", project_path);

        // Validate project structure
        if !project_path.is_dir() {
            return Err(anyhow::anyhow!(
                "Project path is not a directory: {:?}",
                project_path
            ));
        }

        let project_file = project_path.join("project.toml");
        if !project_file.exists() {
            return Err(anyhow::anyhow!(
                "Not a valid project: project.toml not found in {:?}",
                project_path
            ));
        }

        // Load project metadata
        let mut project = Project::new("temp".to_string(), project_path.clone());
        project.load_metadata()?;

        // Update project name from metadata
        project.name = project.metadata.project.name.clone();

        // Set as current project
        self.current_project = Some(project);

        // Initialize asset manager and scan for assets
        let project_path = self.current_project.as_ref().unwrap().path.clone();
        let mut project_assets = ProjectAssets::new(project_path.clone());
        project_assets.scan_assets()?;
        self.project_assets = Some(project_assets);

        // Load the last opened scene or default scene (for now return empty game state)
        // TODO: Convert scene to game state for now, until we fully refactor the editor
        let game_state = GameState::new_empty();

        // Add to recent projects
        self.add_to_recent(project_path);

        tracing::info!(
            "Successfully opened project: {}",
            self.current_project.as_ref().unwrap().name
        );
        Ok(game_state)
    }

    /// Save the current project with scenes
    pub fn save_current_project(&mut self, scenes: &[Scene]) -> Result<()> {
        let project_name = self
            .current_project
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No project currently loaded"))?
            .name
            .clone();

        tracing::info!("Saving project '{}'", project_name);

        // Save all scenes through asset manager
        if let Some(project_assets) = &mut self.project_assets {
            for scene in scenes {
                project_assets.save_scene(scene)?;
            }
        } else {
            tracing::warn!("No project assets manager available for saving scenes");
        }

        // Save project metadata and mark as clean
        let project = self.current_project.as_mut().unwrap();
        project.save_metadata()?;
        project.mark_clean();

        tracing::info!("Successfully saved project with {} scenes", scenes.len());
        Ok(())
    }

    /// Legacy method for backward compatibility
    #[allow(dead_code)]
    pub fn save_current_project_legacy(&mut self, _game_state: &GameState) -> Result<()> {
        // For now, create a scene from the game state
        let scene = Scene::new("main".to_string()); // TODO: Extract proper scene data
        self.save_current_project(&[scene])
    }

    /// Get asset manager for current project
    pub fn get_project_assets(&self) -> Option<&ProjectAssets> {
        self.project_assets.as_ref()
    }

    /// Get mutable asset manager for current project
    #[allow(dead_code)]
    pub fn get_project_assets_mut(&mut self) -> Option<&mut ProjectAssets> {
        self.project_assets.as_mut()
    }

    /// Load scenes from the asset manager
    pub fn load_scenes(&mut self) -> Result<Vec<Scene>> {
        if let Some(project_assets) = &mut self.project_assets {
            let mut scenes = Vec::new();
            for scene_name in project_assets.get_scene_names() {
                if let Some(scene) = project_assets.load_scene(&scene_name)? {
                    scenes.push(scene);
                }
            }
            Ok(scenes)
        } else {
            Ok(Vec::new())
        }
    }

    /// Create the project folder structure
    fn create_project_structure(&self, project_path: &PathBuf) -> Result<()> {
        // Create main project directory
        fs::create_dir_all(project_path)?;

        // Create subdirectories
        fs::create_dir_all(project_path.join("scenes"))?;
        fs::create_dir_all(project_path.join("assets").join("sprites"))?;
        fs::create_dir_all(project_path.join("assets").join("tilemaps"))?;
        fs::create_dir_all(project_path.join("assets").join("audio"))?;
        fs::create_dir_all(project_path.join("entities"))?;
        fs::create_dir_all(project_path.join("settings"))?;

        tracing::debug!("Created project folder structure at {:?}", project_path);
        Ok(())
    }

    /// Add a project path to recent projects list
    fn add_to_recent(&mut self, project_path: PathBuf) {
        // Remove if already exists
        self.recent_projects.retain(|path| path != &project_path);

        // Add to front
        self.recent_projects.insert(0, project_path);

        // Trim to max size
        self.recent_projects.truncate(self.max_recent);
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectManager;
    use crate::project::ProjectTemplateKind;
    use crate::ui::rule_graph::RuleGraph;
    use jsonschema::JSONSchema;
    use serde_json::Value;
    use std::collections::{HashMap, HashSet};
    use std::fs;
    use std::path::PathBuf;
    use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
    use toki_core::assets::tilemap::TileMap;
    use toki_core::game::{AudioChannel, AudioEvent};
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger,
    };
    use toki_core::{GameState, InputKey, Scene};

    const FULL_SURFACE_FIXTURE: &str =
        include_str!("../../tests/fixtures/scene_rules_full_surface.json");
    const ON_PLAYER_MOVE_RUNTIME_FIXTURE: &str =
        include_str!("../../tests/fixtures/scene_rules_on_player_move_runtime.json");

    #[test]
    fn create_top_down_starter_project_populates_template_content() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let parent = temp_dir.path().to_path_buf();

        let mut manager = ProjectManager::new();
        manager
            .create_new_project_with_template(
                "starter_project".to_string(),
                parent.clone(),
                ProjectTemplateKind::TopDownStarter,
            )
            .expect("top-down starter project should be created");

        let project_path = parent.join("starter_project");
        assert!(project_path.join("entities/player.json").exists());
        assert!(project_path.join("entities/villager.json").exists());
        assert!(project_path.join("assets/sprites/terrain.png").exists());
        assert!(project_path.join("assets/sprites/creatures.png").exists());
        assert!(project_path
            .join("assets/tilemaps/starter_overworld.json")
            .exists());

        let loaded_scenes = manager.load_scenes().expect("starter scenes should load");
        assert_eq!(loaded_scenes.len(), 1);
        let scene = &loaded_scenes[0];
        assert_eq!(scene.name, "main");
        assert_eq!(scene.maps, vec!["starter_overworld".to_string()]);
        assert_eq!(scene.entities.len(), 2);

        let terrain_atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(
            project_path.join("assets/sprites/terrain.json"),
        )
        .expect("starter terrain atlas should load");
        let creature_atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(
            project_path.join("assets/sprites/creatures.json"),
        )
        .expect("starter creature atlas should load");
        let tilemap = toki_core::assets::tilemap::TileMap::load_from_file(
            project_path.join("assets/tilemaps/starter_overworld.json"),
        )
        .expect("starter tilemap should load");
        tilemap.validate().expect("starter tilemap should validate");
        assert_eq!(terrain_atlas.tile_size, glam::UVec2::new(8, 8));
        assert_eq!(creature_atlas.tile_size, glam::UVec2::new(16, 16));
        assert_eq!(tilemap.size, glam::UVec2::new(20, 18));
    }

    #[test]
    fn top_down_starter_player_is_loaded_as_runtime_player_and_can_move() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let parent = temp_dir.path().to_path_buf();

        let mut manager = ProjectManager::new();
        manager
            .create_new_project_with_template(
                "starter_project".to_string(),
                parent.clone(),
                ProjectTemplateKind::TopDownStarter,
            )
            .expect("top-down starter project should be created");

        let project_path = parent.join("starter_project");
        let mut loaded_scenes = manager.load_scenes().expect("starter scenes should load");
        let scene = loaded_scenes.pop().expect("starter scene should exist");

        let tilemap = toki_core::assets::tilemap::TileMap::load_from_file(
            project_path.join("assets/tilemaps/starter_overworld.json"),
        )
        .expect("starter tilemap should load");
        let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(
            project_path.join("assets/sprites/terrain.json"),
        )
        .expect("starter atlas should load");

        let mut game_state = GameState::new_empty();
        let scene_name = scene.name.clone();
        game_state.add_scene(scene);
        game_state
            .load_scene(&scene_name)
            .expect("starter scene should load into runtime game state");

        let player_id = game_state
            .player_id()
            .expect("starter scene should provide a player entity");
        let initial_position = game_state
            .entity_manager()
            .get_entity(player_id)
            .expect("player should exist")
            .position;

        game_state.handle_key_press(InputKey::Left);
        let _ = game_state.update(glam::UVec2::new(160, 144), &tilemap, &atlas);
        game_state.handle_key_release(InputKey::Left);

        let moved_position = game_state
            .entity_manager()
            .get_entity(player_id)
            .expect("player should still exist")
            .position;
        assert!(
            moved_position.x < initial_position.x,
            "starter player should be able to move left after scene load"
        );
    }

    #[test]
    fn scene_json_roundtrip_through_editor_persists_rules_and_executes_in_runtime() {
        let temp_dir = tempfile::tempdir().expect("temp dir should be created");
        let parent = temp_dir.path().to_path_buf();

        let mut creator = ProjectManager::new();
        creator
            .create_new_project("e2e_project".to_string(), parent.clone())
            .expect("project should be created");
        let project_path = parent.join("e2e_project");

        let authored_scene = Scene {
            name: "main".to_string(),
            description: Some("integration test scene".to_string()),
            maps: vec!["main_map".to_string()],
            entities: Vec::new(),
            rules: RuleSet {
                rules: vec![Rule {
                    id: "rule_1".to_string(),
                    enabled: true,
                    priority: 5,
                    once: false,
                    trigger: RuleTrigger::OnStart,
                    conditions: vec![RuleCondition::Always],
                    actions: vec![
                        RuleAction::PlayMusic {
                            track_id: "lavandia".to_string(),
                        },
                        RuleAction::PlaySound {
                            channel: RuleSoundChannel::Movement,
                            sound_id: "sfx_slime_bounce".to_string(),
                        },
                    ],
                }],
            },
            camera_position: None,
            camera_scale: None,
        };

        let scene_path = project_path.join("scenes").join("main.json");
        let authored_json = serde_json::to_string_pretty(&authored_scene)
            .expect("scene json serialization should succeed");
        fs::write(&scene_path, authored_json).expect("scene json should be written");

        let mut manager = ProjectManager::new();
        manager
            .open_project(project_path.clone())
            .expect("project should open");
        let mut loaded_scenes = manager.load_scenes().expect("scenes should load");
        assert_eq!(loaded_scenes.len(), 1);

        let scene = loaded_scenes
            .iter_mut()
            .find(|scene| scene.name == "main")
            .expect("main scene should be present");
        scene.description = Some("saved through editor project manager".to_string());

        manager
            .save_current_project(&loaded_scenes)
            .expect("scene save should succeed");

        let saved_scene_json =
            fs::read_to_string(&scene_path).expect("saved scene json should be readable");
        let saved_scene: Scene =
            serde_json::from_str(&saved_scene_json).expect("saved scene json should parse");
        assert_eq!(
            saved_scene.description.as_deref(),
            Some("saved through editor project manager")
        );
        assert_eq!(saved_scene.rules, authored_scene.rules);

        let mut game_state = GameState::new_empty();
        game_state.add_scene(saved_scene.clone());
        game_state
            .load_scene(&saved_scene.name)
            .expect("saved scene should load in runtime");

        let atlas = test_atlas();
        let tilemap = test_tilemap();

        let first_update = game_state.update(glam::UVec2::new(16, 16), &tilemap, &atlas);
        assert!(first_update.events.iter().any(|event| {
            matches!(
                event,
                AudioEvent::BackgroundMusic(track_id) if track_id == "lavandia"
            )
        }));
        assert!(first_update.events.iter().any(|event| {
            matches!(
                event,
                AudioEvent::PlaySound {
                    channel: AudioChannel::Movement,
                    sound_id
                } if sound_id == "sfx_slime_bounce"
            )
        }));

        let second_update = game_state.update(glam::UVec2::new(16, 16), &tilemap, &atlas);
        assert!(!second_update.events.iter().any(|event| {
            matches!(event, AudioEvent::BackgroundMusic(track_id) if track_id == "lavandia")
        }));
    }

    #[test]
    fn regression_fixtures_validate_schema_and_roundtrip_across_core_and_editor_graph() {
        let scene_schema = compile_scene_schema();

        for (fixture_name, fixture_source) in [
            ("scene_rules_full_surface.json", FULL_SURFACE_FIXTURE),
            (
                "scene_rules_on_player_move_runtime.json",
                ON_PLAYER_MOVE_RUNTIME_FIXTURE,
            ),
        ] {
            let fixture_value: Value =
                serde_json::from_str(fixture_source).unwrap_or_else(|error| {
                    panic!("Fixture '{}' should parse: {}", fixture_name, error)
                });
            assert_valid_scene_schema(&scene_schema, &fixture_value, fixture_name);

            let scene: Scene = serde_json::from_str(fixture_source).unwrap_or_else(|error| {
                panic!(
                    "Fixture '{}' should deserialize into toki-core Scene: {}",
                    fixture_name, error
                )
            });

            let graph = RuleGraph::from_rule_set(&scene.rules);
            let roundtrip_rules = graph.to_rule_set().unwrap_or_else(|error| {
                panic!(
                    "Fixture '{}' should roundtrip through editor RuleGraph: {:?}",
                    fixture_name, error
                )
            });
            assert_eq!(
                roundtrip_rules, scene.rules,
                "Fixture '{}' rules should survive RuleGraph conversion unchanged",
                fixture_name
            );

            let reserialized_scene =
                serde_json::to_value(&scene).expect("scene should serialize back to JSON");
            assert_valid_scene_schema(&scene_schema, &reserialized_scene, fixture_name);
        }
    }

    #[test]
    fn full_surface_fixture_covers_all_rule_trigger_and_action_variants() {
        let scene: Scene = serde_json::from_str(FULL_SURFACE_FIXTURE)
            .expect("full-surface fixture should deserialize into Scene");

        let mut seen_triggers = HashSet::new();
        let mut seen_actions = HashSet::new();

        for rule in &scene.rules.rules {
            seen_triggers.insert(trigger_kind_label(&rule.trigger));
            for action in &rule.actions {
                seen_actions.insert(action_kind_label(action));
            }
        }

        assert_eq!(
            seen_triggers,
            HashSet::from([
                "OnStart",
                "OnUpdate",
                "OnPlayerMove",
                "OnKey",
                "OnCollision",
                "OnTrigger",
            ]),
            "Fixture should cover every supported trigger variant"
        );
        assert_eq!(
            seen_actions,
            HashSet::from([
                "PlaySound",
                "PlayMusic",
                "PlayAnimation",
                "SetVelocity",
                "Spawn",
                "DestroySelf",
                "SwitchScene",
            ]),
            "Fixture should cover every supported action variant"
        );
    }

    #[test]
    fn on_player_move_fixture_executes_in_runtime_and_emits_expected_audio_event() {
        let scene: Scene = serde_json::from_str(ON_PLAYER_MOVE_RUNTIME_FIXTURE)
            .expect("on-player-move fixture should deserialize into Scene");

        let mut game_state = GameState::new_empty();
        game_state.spawn_player_at(glam::IVec2::new(0, 0));
        game_state.set_rules(scene.rules.clone());
        game_state.handle_key_press(InputKey::Right);

        let update = game_state.update(
            glam::UVec2::new(128, 128),
            &movement_test_tilemap(),
            &test_atlas(),
        );
        assert!(
            update.player_moved,
            "player should move to trigger OnPlayerMove"
        );
        assert!(update.events.iter().any(|event| {
            matches!(
                event,
                AudioEvent::PlaySound {
                    channel: AudioChannel::Movement,
                    sound_id
                } if sound_id == "sfx_move_tick"
            )
        }));
    }

    fn compile_scene_schema() -> JSONSchema {
        let scene_schema_json: Value =
            serde_json::from_str(toki_schemas::SCENE_SCHEMA).expect("schema should parse");
        JSONSchema::compile(&scene_schema_json).expect("scene schema should compile")
    }

    fn assert_valid_scene_schema(schema: &JSONSchema, scene_json: &Value, fixture_name: &str) {
        if let Err(errors) = schema.validate(scene_json) {
            let details = errors.map(|error| error.to_string()).collect::<Vec<_>>();
            panic!(
                "Fixture '{}' failed scene schema validation: {}",
                fixture_name,
                details.join(" | ")
            );
        }
    }

    fn trigger_kind_label(trigger: &RuleTrigger) -> &'static str {
        match trigger {
            RuleTrigger::OnStart => "OnStart",
            RuleTrigger::OnUpdate => "OnUpdate",
            RuleTrigger::OnPlayerMove => "OnPlayerMove",
            RuleTrigger::OnKey { .. } => "OnKey",
            RuleTrigger::OnCollision => "OnCollision",
            RuleTrigger::OnTrigger => "OnTrigger",
        }
    }

    fn action_kind_label(action: &RuleAction) -> &'static str {
        match action {
            RuleAction::PlaySound { .. } => "PlaySound",
            RuleAction::PlayMusic { .. } => "PlayMusic",
            RuleAction::PlayAnimation { .. } => "PlayAnimation",
            RuleAction::SetVelocity { .. } => "SetVelocity",
            RuleAction::Spawn { .. } => "Spawn",
            RuleAction::DestroySelf { .. } => "DestroySelf",
            RuleAction::SwitchScene { .. } => "SwitchScene",
        }
    }

    fn movement_test_tilemap() -> TileMap {
        TileMap {
            size: glam::UVec2::new(8, 8),
            tile_size: glam::UVec2::new(16, 16),
            atlas: PathBuf::from("atlas.json"),
            tiles: vec!["floor".to_string(); 64],
        }
    }

    fn test_tilemap() -> TileMap {
        TileMap {
            size: glam::UVec2::new(1, 1),
            tile_size: glam::UVec2::new(16, 16),
            atlas: PathBuf::from("atlas.json"),
            tiles: vec!["floor".to_string()],
        }
    }

    fn test_atlas() -> AtlasMeta {
        let mut tiles = HashMap::new();
        tiles.insert(
            "floor".to_string(),
            TileInfo {
                position: glam::UVec2::ZERO,
                properties: TileProperties {
                    solid: false,
                    trigger: false,
                },
            },
        );

        AtlasMeta {
            image: PathBuf::from("terrain.png"),
            tile_size: glam::UVec2::new(16, 16),
            tiles,
        }
    }
}
