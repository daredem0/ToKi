use crate::assets::{atlas::AtlasMeta, tilemap::TileMap};
use crate::CoreError;
use std::path::Path;

/// Errors that can occur during resource loading
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Failed to load atlas '{path}': {source}")]
    AtlasLoad {
        path: String,
        #[source]
        source: CoreError,
    },
    #[error("Failed to load tilemap '{path}': {source}")]
    TilemapLoad {
        path: String,
        #[source]
        source: CoreError,
    },
    #[error("Asset validation error: {0}")]
    Validation(String),
}

/// Resource management system that handles loading and providing access to game assets.
///
/// Centralizes asset loading and provides clean APIs for accessing resources.
/// Future-ready for additional asset types like fonts, sounds, and shaders.
#[derive(Debug)]
pub struct ResourceManager {
    terrain_atlas: AtlasMeta,
    creature_atlas: AtlasMeta,
    tilemap: TileMap,
}

impl ResourceManager {
    /// Load all game resources from their respective files using hardcoded paths (runtime compatibility)
    pub fn load_all() -> Result<Self, ResourceError> {
        Self::load_from_project_dir(Path::new("."))
    }

    /// Load all game resources from a project directory
    pub fn load_from_project_dir(project_dir: &Path) -> Result<Self, ResourceError> {
        let assets_dir = project_dir.join("assets");
        let terrain_path = assets_dir.join("terrain.json");
        let creature_path = assets_dir.join("creatures.json");
        let tilemap_path = assets_dir
            .join("maps")
            .join("new_town_map_64x64_crossings.json");

        let terrain_atlas = AtlasMeta::load_from_file(&terrain_path).map_err(|source| {
            ResourceError::AtlasLoad {
                path: terrain_path.display().to_string(),
                source,
            }
        })?;

        let creature_atlas = AtlasMeta::load_from_file(&creature_path).map_err(|source| {
            ResourceError::AtlasLoad {
                path: creature_path.display().to_string(),
                source,
            }
        })?;

        // Default to the main map used by runtime
        let tilemap = TileMap::load_from_file(&tilemap_path).map_err(|source| {
            ResourceError::TilemapLoad {
                path: tilemap_path.display().to_string(),
                source,
            }
        })?;

        // Validate the tilemap
        tilemap
            .validate()
            .map_err(|e| ResourceError::Validation(format!("Tilemap validation failed: {}", e)))?;

        Ok(Self {
            terrain_atlas,
            creature_atlas,
            tilemap,
        })
    }

    /// Load resources with custom asset paths
    pub fn load_with_paths(
        terrain_atlas_path: &Path,
        creature_atlas_path: &Path,
        tilemap_path: &Path,
    ) -> Result<Self, ResourceError> {
        let terrain_atlas = AtlasMeta::load_from_file(terrain_atlas_path).map_err(|source| {
            ResourceError::AtlasLoad {
                path: terrain_atlas_path.display().to_string(),
                source,
            }
        })?;

        let creature_atlas = AtlasMeta::load_from_file(creature_atlas_path).map_err(|source| {
            ResourceError::AtlasLoad {
                path: creature_atlas_path.display().to_string(),
                source,
            }
        })?;

        let tilemap =
            TileMap::load_from_file(tilemap_path).map_err(|source| ResourceError::TilemapLoad {
                path: tilemap_path.display().to_string(),
                source,
            })?;

        // Validate the tilemap
        tilemap
            .validate()
            .map_err(|e| ResourceError::Validation(format!("Tilemap validation failed: {}", e)))?;

        Ok(Self {
            terrain_atlas,
            creature_atlas,
            tilemap,
        })
    }

    /// Get reference to the terrain atlas
    pub fn get_terrain_atlas(&self) -> &AtlasMeta {
        &self.terrain_atlas
    }

    /// Get reference to the creature atlas
    pub fn get_creature_atlas(&self) -> &AtlasMeta {
        &self.creature_atlas
    }

    /// Get reference to the tilemap
    pub fn get_tilemap(&self) -> &TileMap {
        &self.tilemap
    }

    /// Get terrain atlas tile size for convenience
    pub fn terrain_tile_size(&self) -> glam::UVec2 {
        self.terrain_atlas.tile_size
    }

    /// Get creature atlas tile size for convenience
    pub fn creature_tile_size(&self) -> glam::UVec2 {
        self.creature_atlas.tile_size
    }

    /// Get terrain atlas image size for convenience
    pub fn terrain_image_size(&self) -> Option<glam::UVec2> {
        self.terrain_atlas.image_size()
    }

    /// Get creature atlas image size for convenience
    pub fn creature_image_size(&self) -> Option<glam::UVec2> {
        self.creature_atlas.image_size()
    }

    /// Get tilemap size for convenience
    pub fn tilemap_size(&self) -> glam::UVec2 {
        self.tilemap.size
    }

    /// Get tilemap tile size for convenience
    pub fn tilemap_tile_size(&self) -> glam::UVec2 {
        self.tilemap.tile_size
    }
}
