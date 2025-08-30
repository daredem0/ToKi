use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use toki_core::Scene;

/// Asset discovery and management for project
#[derive(Debug, Clone)]
pub struct ProjectAssets {
    /// Project root path
    pub project_path: PathBuf,
    /// Discovered scenes
    pub scenes: HashMap<String, SceneAsset>,
    /// Discovered tilemaps
    pub tilemaps: HashMap<String, TilemapAsset>,
    /// Discovered sprite atlases
    pub sprite_atlases: HashMap<String, SpriteAtlasAsset>,
    /// Discovered music files
    pub music: HashMap<String, AudioAsset>,
    /// Discovered sound effects
    pub sfx: HashMap<String, AudioAsset>,
}

/// Scene asset information
#[derive(Debug, Clone)]
pub struct SceneAsset {
    /// Scene name (from filename)
    pub name: String,
    /// Full path to scene file
    pub path: PathBuf,
    /// Scene data (loaded lazily)
    pub scene: Option<Scene>,
}

/// Tilemap asset information
#[derive(Debug, Clone)]
pub struct TilemapAsset {
    /// Tilemap name (from filename)
    pub name: String,
    /// Full path to tilemap file
    pub path: PathBuf,
}

/// Sprite atlas asset information
#[derive(Debug, Clone)]
pub struct SpriteAtlasAsset {
    /// Atlas name (from filename)
    pub name: String,
    /// Full path to atlas JSON file
    pub path: PathBuf,
}

/// Audio asset information
#[derive(Debug, Clone)]
pub struct AudioAsset {
    /// Audio name (from filename)
    pub name: String,
    /// Full path to audio file
    pub path: PathBuf,
    /// Audio file format
    pub format: AudioFormat,
}

/// Supported audio formats
#[derive(Debug, Clone, PartialEq)]
pub enum AudioFormat {
    Ogg,
    Wav,
    Mp3,
    Unknown,
}

impl ProjectAssets {
    /// Create new project assets manager
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            scenes: HashMap::new(),
            tilemaps: HashMap::new(),
            sprite_atlases: HashMap::new(),
            music: HashMap::new(),
            sfx: HashMap::new(),
        }
    }

    /// Scan project directories for assets
    pub fn scan_assets(&mut self) -> Result<()> {
        self.scan_scenes()?;
        self.scan_tilemaps()?;
        self.scan_sprite_atlases()?;
        self.scan_audio()?;
        
        tracing::info!(
            "Scanned project assets: {} scenes, {} tilemaps, {} atlases, {} music, {} sfx",
            self.scenes.len(),
            self.tilemaps.len(),
            self.sprite_atlases.len(),
            self.music.len(),
            self.sfx.len()
        );
        
        Ok(())
    }

    /// Scan for scene files
    fn scan_scenes(&mut self) -> Result<()> {
        let scenes_dir = self.project_path.join("scenes");
        if !scenes_dir.exists() {
            tracing::debug!("Scenes directory does not exist: {:?}", scenes_dir);
            return Ok(());
        }

        tracing::info!("🔍 Scanning for scenes in {:?}", scenes_dir);

        for entry in fs::read_dir(&scenes_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let scene_asset = SceneAsset {
                        name: stem.to_string(),
                        path: path.clone(),
                        scene: None, // Loaded lazily
                    };
                    
                    self.scenes.insert(stem.to_string(), scene_asset);
                    tracing::info!("📄 Found scene file: '{}' at {:?}", stem, path);
                }
            }
        }

        Ok(())
    }

    /// Scan for tilemap files
    fn scan_tilemaps(&mut self) -> Result<()> {
        let tilemaps_dir = self.project_path.join("assets/tilemaps");
        if !tilemaps_dir.exists() {
            tracing::debug!("Tilemaps directory does not exist: {:?}", tilemaps_dir);
            return Ok(());
        }

        tracing::info!("🔍 Scanning for tilemaps in {:?}", tilemaps_dir);

        for entry in fs::read_dir(&tilemaps_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let tilemap_asset = TilemapAsset {
                        name: stem.to_string(),
                        path: path.clone(),
                    };
                    
                    self.tilemaps.insert(stem.to_string(), tilemap_asset);
                    tracing::info!("🗺️ Found tilemap file: '{}' at {:?}", stem, path);
                }
            }
        }

        Ok(())
    }

    /// Scan for sprite atlas files
    fn scan_sprite_atlases(&mut self) -> Result<()> {
        let sprites_dir = self.project_path.join("assets/sprites");
        if !sprites_dir.exists() {
            tracing::debug!("Sprites directory does not exist: {:?}", sprites_dir);
            return Ok(());
        }

        tracing::info!("🔍 Scanning for sprite atlases in {:?}", sprites_dir);

        for entry in fs::read_dir(&sprites_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let atlas_asset = SpriteAtlasAsset {
                        name: stem.to_string(),
                        path: path.clone(),
                    };
                    
                    self.sprite_atlases.insert(stem.to_string(), atlas_asset);
                    tracing::info!("🎨 Found sprite atlas file: '{}' at {:?}", stem, path);
                }
            }
        }

        Ok(())
    }

    /// Scan for audio files
    fn scan_audio(&mut self) -> Result<()> {
        // Scan music
        let music_dir = self.project_path.join("assets/audio/music");
        if music_dir.exists() {
            tracing::info!("🔍 Scanning for music in {:?}", music_dir);
            let mut music_assets = HashMap::new();
            self.scan_audio_directory(&music_dir, &mut music_assets)?;
            self.music = music_assets;
        }

        // Scan SFX
        let sfx_dir = self.project_path.join("assets/audio/sfx");
        if sfx_dir.exists() {
            tracing::info!("🔍 Scanning for SFX in {:?}", sfx_dir);
            let mut sfx_assets = HashMap::new();
            self.scan_audio_directory(&sfx_dir, &mut sfx_assets)?;
            self.sfx = sfx_assets;
        }

        Ok(())
    }

    /// Scan a specific audio directory
    fn scan_audio_directory(&self, dir: &Path, audio_map: &mut HashMap<String, AudioAsset>) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let (Some(stem), Some(ext)) = (
                    path.file_stem().and_then(|s| s.to_str()),
                    path.extension().and_then(|s| s.to_str())
                ) {
                    let format = match ext.to_lowercase().as_str() {
                        "ogg" => AudioFormat::Ogg,
                        "wav" => AudioFormat::Wav,
                        "mp3" => AudioFormat::Mp3,
                        _ => AudioFormat::Unknown,
                    };

                    if format != AudioFormat::Unknown {
                        let audio_asset = AudioAsset {
                            name: stem.to_string(),
                            path: path.clone(),
                            format,
                        };
                        
                        audio_map.insert(stem.to_string(), audio_asset);
                        tracing::info!("🎵 Found audio file: '{}' at {:?}", stem, path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a scene by name
    pub fn load_scene(&mut self, scene_name: &str) -> Result<Option<Scene>> {
        if let Some(scene_asset) = self.scenes.get_mut(scene_name) {
            if scene_asset.scene.is_none() {
                // Load scene from file
                let json_data = fs::read_to_string(&scene_asset.path)?;
                let scene: Scene = serde_json::from_str(&json_data)
                    .map_err(|e| anyhow::anyhow!("Failed to parse scene '{}': {}", scene_name, e))?;
                
                scene_asset.scene = Some(scene.clone());
                tracing::info!("📖 Loaded scene '{}' from {:?}", scene_name, scene_asset.path);
                Ok(Some(scene))
            } else {
                Ok(scene_asset.scene.clone())
            }
        } else {
            Ok(None)
        }
    }

    /// Save a scene
    pub fn save_scene(&mut self, scene: &Scene) -> Result<()> {
        // Ensure scene asset exists
        if !self.scenes.contains_key(&scene.name) {
            let scene_path = self.project_path.join("scenes").join(format!("{}.json", scene.name));
            tracing::info!("📝 Creating new scene asset for '{}' at {:?}", scene.name, scene_path);
            let scene_asset = SceneAsset {
                name: scene.name.clone(),
                path: scene_path,
                scene: Some(scene.clone()),
            };
            self.scenes.insert(scene.name.clone(), scene_asset);
        }

        // Get scene asset and save
        if let Some(scene_asset) = self.scenes.get_mut(&scene.name) {
            // Ensure directory exists
            if let Some(parent) = scene_asset.path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Serialize and save
            let json_data = serde_json::to_string_pretty(scene)?;
            fs::write(&scene_asset.path, json_data)?;
            
            // Update cached scene
            scene_asset.scene = Some(scene.clone());
            
            tracing::info!("💾 Saved scene '{}' to {:?}", scene.name, scene_asset.path);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to create scene asset for '{}'", scene.name))
        }
    }

    /// Get all scene names
    pub fn get_scene_names(&self) -> Vec<String> {
        self.scenes.keys().cloned().collect()
    }

    /// Get all tilemap names
    pub fn get_tilemap_names(&self) -> Vec<String> {
        self.tilemaps.keys().cloned().collect()
    }

    /// Get all sprite atlas names
    pub fn get_sprite_atlas_names(&self) -> Vec<String> {
        self.sprite_atlases.keys().cloned().collect()
    }

    /// Get all music names
    pub fn get_music_names(&self) -> Vec<String> {
        self.music.keys().cloned().collect()
    }

    /// Get all SFX names
    pub fn get_sfx_names(&self) -> Vec<String> {
        self.sfx.keys().cloned().collect()
    }
}