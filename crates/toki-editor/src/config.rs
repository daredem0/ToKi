use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "toki_editor_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Path to the current project folder
    pub project_path: Option<PathBuf>,
    
    /// Editor UI settings
    pub editor_settings: EditorSettings,
    
    /// Recently opened projects
    pub recent_projects: Vec<PathBuf>,
    
    /// Rendering and viewport settings
    pub rendering: RenderingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    /// Default window size [width, height]
    pub window_size: [u32; 2],
    
    /// Panel visibility settings
    pub panels: PanelSettings,
    
    /// Grid settings for the viewport
    pub grid: GridSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelSettings {
    pub hierarchy_visible: bool,
    pub inspector_visible: bool,
    pub console_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSettings {
    pub show_grid: bool,
    pub grid_size: u32,
    pub snap_to_grid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingSettings {
    pub vsync: bool,
    pub target_fps: u32,
    pub show_collision_boxes: bool,
    pub show_debug_info: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            project_path: None,
            editor_settings: EditorSettings::default(),
            recent_projects: Vec::new(),
            rendering: RenderingSettings::default(),
        }
    }
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            window_size: [1200, 800],
            panels: PanelSettings::default(),
            grid: GridSettings::default(),
        }
    }
}

impl Default for PanelSettings {
    fn default() -> Self {
        Self {
            hierarchy_visible: true,
            inspector_visible: true,
            console_visible: false,
        }
    }
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            show_grid: true,
            grid_size: 16,
            snap_to_grid: true,
        }
    }
}

impl Default for RenderingSettings {
    fn default() -> Self {
        Self {
            vsync: true,
            target_fps: 60,
            show_collision_boxes: true,
            show_debug_info: false,
        }
    }
}

impl EditorConfig {
    /// Load config from the current working directory
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        if !config_path.exists() {
            tracing::info!("Config file not found at {:?}, creating default config", config_path);
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        
        let json_data = fs::read_to_string(&config_path)?;
        let config: EditorConfig = serde_json::from_str(&json_data)?;
        
        tracing::info!("Loaded editor config from {:?}", config_path);
        Ok(config)
    }
    
    /// Save config to the current working directory
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_file_path()?;
        
        let json_data = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, json_data)?;
        
        tracing::info!("Saved editor config to {:?}", config_path);
        Ok(())
    }
    
    /// Initialize config with default values and save
    pub fn init_default_config() -> Result<Self> {
        let config = Self::default();
        let config_path = Self::config_file_path()?;
        config.save()?;
        tracing::info!("Initialized default editor config at {:?}", config_path);
        Ok(config)
    }
    
    /// Get the config file path in current working directory
    fn config_file_path() -> Result<PathBuf> {
        let current_dir = std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("Cannot determine current directory: {}", e))?;
        
        Ok(current_dir.join(CONFIG_FILE_NAME))
    }
    
    /// Set the current project path
    pub fn set_project_path(&mut self, path: PathBuf) {
        // Add to recent projects if it's different from current
        if Some(&path) != self.project_path.as_ref() {
            self.add_recent_project(path.clone());
        }
        self.project_path = Some(path);
    }
    
    /// Add a project to recent projects list
    pub fn add_recent_project(&mut self, path: PathBuf) {
        // Remove if already exists
        self.recent_projects.retain(|p| p != &path);
        
        // Add to front
        self.recent_projects.insert(0, path);
        
        // Keep only last 10
        self.recent_projects.truncate(10);
    }
    
    /// Get current project path
    pub fn current_project_path(&self) -> Option<&PathBuf> {
        self.project_path.as_ref()
    }
    
    /// Check if a project path is set
    pub fn has_project_path(&self) -> bool {
        self.project_path.is_some()
    }
}