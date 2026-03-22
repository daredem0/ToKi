use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toki_core::project_assets::{
    classify_sprite_metadata_file, discover_audio_files,
    load_entity_definition_from_path as load_entity_definition_from_project_path,
    load_scene_from_path as load_scene_from_project_path, SpriteMetadataFileKind,
};
use toki_core::{entity::EntityDefinition, Scene};

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
    /// Discovered object sheets
    pub object_sheets: HashMap<String, ObjectSheetAsset>,
    /// Discovered entity definitions
    pub entities: HashMap<String, EntityAsset>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAudioAssetKind {
    Music,
    Sfx,
}

/// Scene asset information.
#[derive(Debug, Clone)]
pub struct SceneAsset {
    /// Full path to scene file
    pub path: PathBuf,
    /// Scene data (loaded lazily)
    pub scene: Option<Scene>,
}

/// Tilemap asset information.
#[derive(Debug, Clone)]
pub struct TilemapAsset {
    /// Full path to tilemap file
    pub path: PathBuf,
}

/// Sprite atlas asset information.
#[derive(Debug, Clone)]
pub struct SpriteAtlasAsset {
    /// Full path to atlas JSON file
    pub path: PathBuf,
}

/// Object sheet asset information.
#[derive(Debug, Clone)]
pub struct ObjectSheetAsset {
    /// Full path to object sheet JSON file
    pub path: PathBuf,
}

/// Entity definition asset information.
#[derive(Debug, Clone)]
pub struct EntityAsset {
    /// Full path to entity definition file
    pub path: PathBuf,
    /// Entity definition data (loaded lazily)
    pub definition: Option<EntityDefinition>,
}

impl ProjectAssets {
    /// Create new project assets manager
    pub fn new(project_path: PathBuf) -> Self {
        Self {
            project_path,
            scenes: HashMap::new(),
            tilemaps: HashMap::new(),
            sprite_atlases: HashMap::new(),
            object_sheets: HashMap::new(),
            entities: HashMap::new(),
        }
    }

    /// Scan project directories for assets
    pub fn scan_assets(&mut self) -> Result<()> {
        self.scan_scenes()?;
        self.scan_tilemaps()?;
        self.scan_sprite_atlases()?;
        self.scan_entities()?;

        tracing::info!(
            "Scanned project assets: {} scenes, {} tilemaps, {} atlases, {} object sheets, {} entities",
            self.scenes.len(),
            self.tilemaps.len(),
            self.sprite_atlases.len(),
            self.object_sheets.len(),
            self.entities.len()
        );

        Ok(())
    }

    pub fn discover_project_audio_names(
        project_path: &Path,
        kind: ProjectAudioAssetKind,
    ) -> Vec<String> {
        let dir = match kind {
            ProjectAudioAssetKind::Music => project_path.join("assets/audio/music"),
            ProjectAudioAssetKind::Sfx => project_path.join("assets/audio/sfx"),
        };
        Self::discover_audio_names_in_dir(&dir)
    }

    pub fn discover_project_entity_definition_names(project_path: &Path) -> Vec<String> {
        let dir = project_path.join("entities");
        if !dir.exists() {
            return Vec::new();
        }

        let mut names = match fs::read_dir(&dir) {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
                })
                .filter_map(|path| {
                    path.file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(str::to_string)
                })
                .collect::<Vec<_>>(),
            Err(error) => {
                tracing::warn!(
                    "Failed to read entity definitions from '{}': {}",
                    dir.display(),
                    error
                );
                Vec::new()
            }
        };

        names.sort();
        names.dedup();
        names
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

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let scene_asset = SceneAsset {
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

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let tilemap_asset = TilemapAsset {
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

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    match classify_sprite_metadata_file(&path)? {
                        SpriteMetadataFileKind::Atlas => {
                            let atlas_asset = SpriteAtlasAsset {
                                path: path.clone(),
                            };

                            self.sprite_atlases.insert(stem.to_string(), atlas_asset);
                            tracing::info!("🎨 Found sprite atlas file: '{}' at {:?}", stem, path);
                        }
                        SpriteMetadataFileKind::ObjectSheet => {
                            let object_sheet_asset = ObjectSheetAsset {
                                path: path.clone(),
                            };

                            self.object_sheets
                                .insert(stem.to_string(), object_sheet_asset);
                            tracing::info!("🌿 Found object sheet file: '{}' at {:?}", stem, path);
                        }
                        SpriteMetadataFileKind::Unknown => {
                            tracing::warn!(
                                "Skipping unrecognized sprite metadata file '{}' at {:?}",
                                stem,
                                path
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn discover_audio_names_in_dir(dir: &Path) -> Vec<String> {
        match discover_audio_files(dir) {
            Ok(assets) => {
                let mut names = assets.into_iter().map(|asset| asset.name).collect::<Vec<_>>();
                names.sort();
                names.dedup();
                names
            }
            Err(error) => {
                tracing::warn!(
                    "Failed to discover audio assets from '{}': {}",
                    dir.display(),
                    error
                );
                Vec::new()
            }
        }
    }

    /// Scan for entity definition files
    fn scan_entities(&mut self) -> Result<()> {
        let entities_dir = self.project_path.join("entities");
        if !entities_dir.exists() {
            tracing::debug!("Entities directory does not exist: {:?}", entities_dir);
            return Ok(());
        }

        tracing::info!("🔍 Scanning for entity definitions in {:?}", entities_dir);

        for entry in fs::read_dir(&entities_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let entity_asset = EntityAsset {
                        path: path.clone(),
                        definition: None, // Loaded lazily
                    };

                    self.entities.insert(stem.to_string(), entity_asset);
                    tracing::info!("🧙 Found entity definition: '{}' at {:?}", stem, path);
                }
            }
        }

        Ok(())
    }

    /// Load a scene by name
    pub fn load_scene(&mut self, scene_name: &str) -> Result<Option<Scene>> {
        if let Some(scene_asset) = self.scenes.get_mut(scene_name) {
            if scene_asset.scene.is_none() {
                let scene = load_scene_from_project_path(&scene_asset.path)
                    .map_err(|error| anyhow::anyhow!("Failed to load scene '{}': {}", scene_name, error))?;

                scene_asset.scene = Some(scene.clone());
                tracing::info!(
                    "📖 Loaded scene '{}' from {:?}",
                    scene_name,
                    scene_asset.path
                );
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
            let scene_path = self
                .project_path
                .join("scenes")
                .join(format!("{}.json", scene.name));
            tracing::info!(
                "📝 Creating new scene asset for '{}' at {:?}",
                scene.name,
                scene_path
            );
            let scene_asset = SceneAsset {
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
            Err(anyhow::anyhow!(
                "Failed to create scene asset for '{}'",
                scene.name
            ))
        }
    }

    /// Get all scene names
    pub fn get_scene_names(&self) -> Vec<String> {
        self.scenes.keys().cloned().collect()
    }

    /// Get all sprite atlas names
    #[cfg_attr(not(test), allow(dead_code))] // Used in tests
    pub fn get_sprite_atlas_names(&self) -> Vec<String> {
        self.sprite_atlases.keys().cloned().collect()
    }

    /// Get all object sheet names
    #[cfg_attr(not(test), allow(dead_code))] // Used in tests
    pub fn get_object_sheet_names(&self) -> Vec<String> {
        self.object_sheets.keys().cloned().collect()
    }

    /// Load an entity definition by name
    pub fn load_entity_definition(
        &mut self,
        entity_name: &str,
    ) -> Result<Option<EntityDefinition>> {
        if let Some(entity_asset) = self.entities.get_mut(entity_name) {
            if entity_asset.definition.is_none() {
                let definition = load_entity_definition_from_project_path(&entity_asset.path)
                    .map_err(|error| {
                        anyhow::anyhow!(
                            "Failed to load entity definition '{}': {}",
                            entity_name,
                            error
                        )
                    })?;

                entity_asset.definition = Some(definition.clone());
                tracing::info!(
                    "📖 Loaded entity definition '{}' from {:?}",
                    entity_name,
                    entity_asset.path
                );
                Ok(Some(definition))
            } else {
                Ok(entity_asset.definition.clone())
            }
        } else {
            Ok(None)
        }
    }

}

#[cfg(test)]
#[path = "assets_tests.rs"]
mod tests;
