use anyhow::Result;
use std::path::PathBuf;
use std::fs;
use toki_core::GameState;
use crate::project::Project;

/// Manages project operations (create, load, save)
#[derive(Debug)]
pub struct ProjectManager {
    /// Currently active project
    current_project: Option<Project>,
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
            return Err(anyhow::anyhow!("Project folder already exists: {:?}", project_path));
        }
        
        tracing::info!("Creating new project '{}' at {:?}", name, project_path);
        
        // Create project structure
        self.create_project_structure(&project_path)?;
        
        // Create project data
        let project = Project::new(name, project_path);
        
        // Save project metadata
        project.save_metadata()?;
        
        // Create default scene
        let game_state = GameState::new_empty();
        self.save_scene(&project, "main", &game_state)?;
        
        // Add to recent projects
        self.add_to_recent(project.path.clone());
        
        // Set as current project
        self.current_project = Some(project);
        
        tracing::info!("Successfully created new project");
        Ok(game_state)
    }
    
    /// Open an existing project
    pub fn open_project(&mut self, project_path: PathBuf) -> Result<GameState> {
        tracing::info!("Opening project at {:?}", project_path);
        
        // Validate project structure
        if !project_path.is_dir() {
            return Err(anyhow::anyhow!("Project path is not a directory: {:?}", project_path));
        }
        
        let project_file = project_path.join("project.toml");
        if !project_file.exists() {
            return Err(anyhow::anyhow!("Not a valid project: project.toml not found in {:?}", project_path));
        }
        
        // Load project metadata
        let mut project = Project::new("temp".to_string(), project_path.clone());
        project.load_metadata()?;
        
        // Update project name from metadata
        project.name = project.metadata.project.name.clone();
        
        // Load the last opened scene or default scene
        let scene_name = project.metadata.editor.last_scene.clone()
            .unwrap_or_else(|| "main".to_string());
        
        let game_state = self.load_scene(&project, &scene_name)?;
        
        // Add to recent projects
        self.add_to_recent(project_path);
        
        // Set as current project
        self.current_project = Some(project);
        
        tracing::info!("Successfully opened project");
        Ok(game_state)
    }
    
    /// Save the current project
    pub fn save_current_project(&mut self, game_state: &GameState) -> Result<()> {
        let project_name = self.current_project.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No project currently loaded"))?
            .name.clone();
            
        let scene_name = self.current_project.as_ref()
            .and_then(|p| p.current_scene.clone());
        
        tracing::info!("Saving project '{}'", project_name);
        
        // Save current scene
        if let Some(scene_name) = scene_name {
            let project = self.current_project.as_ref().unwrap();
            self.save_scene(project, &scene_name, game_state)?;
        }
        
        // Save project metadata and mark as clean
        let project = self.current_project.as_mut().unwrap();
        project.save_metadata()?;
        project.mark_clean();
        
        tracing::info!("Successfully saved project");
        Ok(())
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
    
    /// Save a scene to file
    fn save_scene(&self, project: &Project, scene_name: &str, game_state: &GameState) -> Result<()> {
        let scene_path = project.scene_file_path(scene_name)
            .ok_or_else(|| anyhow::anyhow!("Scene '{}' not found in project", scene_name))?;
        
        // Ensure scenes directory exists
        if let Some(parent) = scene_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Serialize and save game state
        let json_data = serde_json::to_string_pretty(game_state)
            .map_err(|e| anyhow::anyhow!("Failed to serialize game state: {}", e))?;
        
        fs::write(&scene_path, json_data)
            .map_err(|e| anyhow::anyhow!("Failed to write scene file {:?}: {}", scene_path, e))?;
        
        tracing::debug!("Saved scene '{}' to {:?}", scene_name, scene_path);
        Ok(())
    }
    
    /// Load a scene from file
    fn load_scene(&self, project: &Project, scene_name: &str) -> Result<GameState> {
        let scene_path = project.scene_file_path(scene_name)
            .ok_or_else(|| anyhow::anyhow!("Scene '{}' not found in project", scene_name))?;
        
        if !scene_path.exists() {
            tracing::warn!("Scene file {:?} does not exist, creating empty scene", scene_path);
            return Ok(GameState::new_empty());
        }
        
        let json_data = fs::read_to_string(&scene_path)
            .map_err(|e| anyhow::anyhow!("Failed to read scene file {:?}: {}", scene_path, e))?;
        
        let game_state: GameState = serde_json::from_str(&json_data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize scene file {:?}: {}", scene_path, e))?;
        
        tracing::debug!("Loaded scene '{}' from {:?}", scene_name, scene_path);
        Ok(game_state)
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