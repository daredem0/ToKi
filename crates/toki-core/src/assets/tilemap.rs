use crate::CoreError;
use glam::UVec2;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct TileMap {
    pub size: UVec2,        // map dimensions in tiles (width x height)
    pub tile_size: UVec2,   // tile dimensions in pixels (width x height)
    pub atlas: PathBuf,     // path to atlas file
    pub tiles: Vec<String>, // row-major list of tile names
}

impl TileMap {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = fs::read_to_string(path)?;
        let map = serde_json::from_str::<TileMap>(&content)?;
        Ok(map)
    }

    pub fn get_tile_name(&self, x: u32, y: u32) -> Option<&str> {
        if x >= self.size.x || y >= self.size.y {
            return None;
        }
        let index = (y * self.size.x + x) as usize;
        self.tiles.get(index).map(String::as_str)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        let expected_len = (self.size.x * self.size.y) as usize;
        let actual_len = self.tiles.len();
        if expected_len != actual_len {
            return Err(CoreError::InvalidMapSize {
                expected: expected_len,
                actual: actual_len,
            });
        }
        Ok(())
    }

    pub fn tile_to_world(&self, tile_pos: UVec2) -> Option<UVec2> {
        if tile_pos.x >= self.size.x || tile_pos.y >= self.size.y {
            return None;
        }
        Some(tile_pos * self.tile_size)
    }
}
