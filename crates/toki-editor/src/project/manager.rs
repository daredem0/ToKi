use crate::project::{Project, ProjectAssets};
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
        // Create project folder
        let project_path = parent_path.join(&name);
        if project_path.exists() {
            return Err(anyhow::anyhow!(
                "Project folder already exists: {:?}",
                project_path
            ));
        }

        tracing::info!("Creating new project '{}' at {:?}", name, project_path);

        // Create project structure
        self.create_project_structure(&project_path)?;

        // Create project data
        let project = Project::new(name, project_path);

        // Save project metadata
        project.save_metadata()?;

        // Set as current project first
        self.current_project = Some(project);

        // Initialize asset manager
        let project_path = self.current_project.as_ref().unwrap().path.clone();
        let mut project_assets = ProjectAssets::new(project_path);

        // Create default scene and save it through asset manager
        let default_scene = Scene::new("main".to_string());
        project_assets.save_scene(&default_scene)?;

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
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
    use toki_core::assets::tilemap::TileMap;
    use toki_core::game::{AudioChannel, AudioEvent};
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger,
    };
    use toki_core::{GameState, Scene};

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
