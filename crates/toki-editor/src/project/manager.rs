use crate::project::Project;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use toki_core::{GameState, Scene};

/// Manages project operations (create, load, save)
#[derive(Debug)]
pub struct ProjectManager {
    /// Currently active project
    pub current_project: Option<Project>,
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

        // Create default scene
        let default_scene = Scene::new("main".to_string());
        self.ensure_scene_file_path_exists("main")?;
        self.write_scene_to_file(&default_scene)?;
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

        // Load the last opened scene or default scene
        let scene_name = project
            .metadata
            .editor
            .last_scene
            .clone()
            .unwrap_or_else(|| "main".to_string());

        let _scene = self.load_scene(&project, &scene_name)?;
        // TODO: Convert scene to game state for now, until we fully refactor the editor
        let game_state = GameState::new_empty();

        // Add to recent projects
        self.add_to_recent(project_path);

        // Set as current project
        self.current_project = Some(project);

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

        // Save all scenes and update metadata
        for scene in scenes {
            self.ensure_scene_file_path_exists(&scene.name)?;
            self.write_scene_to_file(scene)?;
        }

        // Save project metadata and mark as clean
        let project = self.current_project.as_mut().unwrap();
        project.save_metadata()?;
        project.mark_clean();

        tracing::info!("Successfully saved project with {} scenes", scenes.len());
        Ok(())
    }
    
    /// Legacy method for backward compatibility
    pub fn save_current_project_legacy(&mut self, game_state: &GameState) -> Result<()> {
        // For now, create a scene from the game state
        let scene = Scene::new("main".to_string()); // TODO: Extract proper scene data
        self.save_current_project(&[scene])
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

    /// Ensure a scene has a file path mapping, create one if needed
    fn ensure_scene_file_path_exists(&mut self, scene_name: &str) -> Result<()> {
        let project = self.current_project.as_mut().unwrap();
        
        if project.scene_file_path(scene_name).is_none() {
            // Create a new scene file path mapping
            let sanitized_name = scene_name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect::<String>()
                .to_lowercase();
            let relative_path = format!("scenes/{}.json", sanitized_name);
            
            // Add to project metadata
            project.metadata.scenes.insert(scene_name.to_string(), relative_path.clone());
            project.is_dirty = true; // Mark project as needing metadata save
            
            tracing::info!("Created new scene file mapping: '{}' -> {}", scene_name, relative_path);
        }
        Ok(())
    }

    /// Write a scene to its file
    fn write_scene_to_file(&self, scene: &Scene) -> Result<()> {
        let project = self.current_project.as_ref().unwrap();
        let scene_path = project
            .scene_file_path(&scene.name)
            .ok_or_else(|| anyhow::anyhow!("Scene '{}' not found in project", scene.name))?;

        // Ensure scenes directory exists
        if let Some(parent) = scene_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Serialize and save scene data
        let json_data = serde_json::to_string_pretty(scene)
            .map_err(|e| anyhow::anyhow!("Failed to serialize scene: {}", e))?;

        fs::write(&scene_path, json_data)
            .map_err(|e| anyhow::anyhow!("Failed to write scene file {:?}: {}", scene_path, e))?;

        tracing::debug!("Saved scene '{}' to {:?}", scene.name, scene_path);
        Ok(())
    }

    /// Load a scene from file
    fn load_scene(&self, project: &Project, scene_name: &str) -> Result<Scene> {
        let scene_path = project
            .scene_file_path(scene_name)
            .ok_or_else(|| anyhow::anyhow!("Scene '{}' not found in project", scene_name))?;

        if !scene_path.exists() {
            tracing::warn!(
                "Scene file {:?} does not exist, creating empty scene",
                scene_path
            );
            return Ok(Scene::new(scene_name.to_string()));
        }

        let json_data = fs::read_to_string(&scene_path)
            .map_err(|e| anyhow::anyhow!("Failed to read scene file {:?}: {}", scene_path, e))?;

        // Try to load as Scene first, fall back to GameState for legacy files
        let scene: Scene = match serde_json::from_str(&json_data) {
            Ok(scene) => scene,
            Err(_) => {
                tracing::warn!("Scene file {:?} appears to be legacy GameState format, converting to Scene", scene_path);
                // If it's a legacy GameState file, create a basic Scene
                Scene::new(scene_name.to_string())
            }
        };

        tracing::debug!("Loaded scene '{}' from {:?}", scene_name, scene_path);
        Ok(scene)
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
