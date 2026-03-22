use crate::rule_graph::RuleGraph;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use toki_core::project_runtime::RuntimeSettings;

/// Main project data structure
#[derive(Debug, Clone)]
pub struct Project {
    /// Project name
    pub name: String,
    /// Path to project folder
    pub path: PathBuf,
    /// Project metadata (saved to project.toml)
    pub metadata: ProjectMetadata,
    /// Whether project has unsaved changes
    pub is_dirty: bool,
}

/// Project metadata stored in project.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project configuration
    pub project: ProjectConfig,
    /// Scene configuration
    pub scenes: HashMap<String, String>,
    /// Asset configuration
    pub assets: AssetConfig,
    /// Runtime-specific settings
    #[serde(default)]
    pub runtime: RuntimeSettings,
    /// Editor-specific settings
    #[serde(default)]
    pub editor: EditorSettings,
}

/// Core project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,
    /// Project version
    pub version: String,
    /// Creation timestamp
    pub created: DateTime<Utc>,
    /// Last modified timestamp
    pub modified: DateTime<Utc>,
    /// Toki editor version used to create this project
    pub toki_editor_version: String,
    /// Project description
    #[serde(default)]
    pub description: String,
}

/// Asset paths configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    /// Sprites directory relative to project root
    pub sprites: String,
    /// Tilemaps directory relative to project root
    pub tilemaps: String,
    /// Audio directory relative to project root
    pub audio: String,
}

/// Editor-specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EditorSettings {
    /// Last opened scene
    pub last_scene: Option<String>,
    /// Recent files
    #[serde(default)]
    pub recent_files: Vec<String>,
    /// Editor camera settings for each scene
    #[serde(default)]
    pub camera_settings: HashMap<String, CameraSettings>,
    /// Scene graph layout settings for each scene
    #[serde(default)]
    pub graph_layouts: HashMap<String, SceneGraphLayout>,
    /// Persisted scene rule graph drafts for each scene
    #[serde(default)]
    pub rule_graph_drafts: HashMap<String, RuleGraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraphLayout {
    /// Persisted graph node positions keyed by stable node key
    #[serde(default)]
    pub node_positions: HashMap<String, [f32; 2]>,
    /// Graph zoom level for the scene
    #[serde(default = "default_graph_zoom")]
    pub zoom: f32,
    /// Graph pan offset for the scene
    #[serde(default = "default_graph_pan")]
    pub pan: [f32; 2],
}

impl Default for SceneGraphLayout {
    fn default() -> Self {
        Self {
            node_positions: HashMap::new(),
            zoom: default_graph_zoom(),
            pan: default_graph_pan(),
        }
    }
}

fn default_graph_zoom() -> f32 {
    1.0
}

fn default_graph_pan() -> [f32; 2] {
    [16.0, 16.0]
}

/// Camera settings for a specific scene
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraSettings {
    /// Camera position
    pub position: [i32; 2],
    /// Camera scale/zoom
    pub scale: u32,
    /// Viewport size
    pub viewport_size: [u32; 2],
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            sprites: "assets/sprites/".to_string(),
            tilemaps: "assets/tilemaps/".to_string(),
            audio: "assets/audio/".to_string(),
        }
    }
}

impl Project {
    /// Create a new project with default settings
    pub fn new(name: String, path: PathBuf) -> Self {
        let now = Utc::now();

        let metadata = ProjectMetadata {
            project: ProjectConfig {
                name: name.clone(),
                version: "1.0.0".to_string(),
                created: now,
                modified: now,
                toki_editor_version: env!("TOKI_VERSION").to_string(),
                description: String::new(),
            },
            scenes: {
                let mut scenes = HashMap::new();
                scenes.insert("main".to_string(), "scenes/main.json".to_string());
                scenes
            },
            assets: AssetConfig::default(),
            runtime: RuntimeSettings::default(),
            editor: EditorSettings::default(),
        };

        Self {
            name,
            path,
            metadata,
            is_dirty: false,
        }
    }

    /// Get the project.toml file path
    pub fn project_file_path(&self) -> PathBuf {
        self.path.join("project.toml")
    }

    /// Mark the project as saved
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    /// Get a reference to the runtime audio configuration.
    ///
    /// This accessor reduces message chain depth from
    /// `project.metadata.runtime.audio` to `project.audio_config()`.
    pub fn audio_config(&self) -> &toki_core::project_runtime::RuntimeAudioMixSettings {
        &self.metadata.runtime.audio
    }

    /// Get a mutable reference to the runtime audio configuration.
    ///
    /// This accessor reduces message chain depth from
    /// `project.metadata.runtime.audio` to `project.audio_config_mut()`.
    pub fn audio_config_mut(&mut self) -> &mut toki_core::project_runtime::RuntimeAudioMixSettings {
        &mut self.metadata.runtime.audio
    }

    /// Load project metadata from project.toml
    pub fn load_metadata(&mut self) -> Result<()> {
        let project_file = self.project_file_path();
        let toml_content = std::fs::read_to_string(&project_file).map_err(|e| {
            anyhow::anyhow!("Failed to read project file {:?}: {}", project_file, e)
        })?;

        self.metadata = toml::from_str(&toml_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse project file: {}", e))?;

        Ok(())
    }

    /// Save project metadata to project.toml
    pub fn save_metadata(&self) -> Result<()> {
        let project_file = self.project_file_path();
        let toml_content = toml::to_string_pretty(&self.metadata)
            .map_err(|e| anyhow::anyhow!("Failed to serialize project metadata: {}", e))?;

        std::fs::write(&project_file, toml_content).map_err(|e| {
            anyhow::anyhow!("Failed to write project file {:?}: {}", project_file, e)
        })?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "project_data_tests.rs"]
mod tests;
