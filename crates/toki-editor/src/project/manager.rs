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
