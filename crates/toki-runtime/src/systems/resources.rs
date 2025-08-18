use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_render::RenderError;

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
    /// Load all game resources from their respective files
    pub fn load_all() -> Result<Self, RenderError> {
        let terrain_atlas = AtlasMeta::load_from_file("assets/terrain.json")?;
        let creature_atlas = AtlasMeta::load_from_file("assets/creatures.json")?;
        let tilemap = TileMap::load_from_file("assets/maps/tilemap_64x64_chunk.json")?;

        // Validate the tilemap
        tilemap.validate()?;

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