use crate::CoreError;
use glam::UVec2;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct AtlasMeta {
    #[serde_as(as = "DisplayFromStr")]
    pub image: PathBuf,

    pub tile_size: UVec2,
    pub tiles: HashMap<String, UVec2>,
}

impl AtlasMeta {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = fs::read_to_string(path)?;
        let meta = serde_json::from_str::<AtlasMeta>(&content)?;
        Ok(meta)
    }

    pub fn image_size(&self) -> Option<UVec2> {
        let max_tile = self.tiles.values().copied().max_by_key(|v| (v.y, v.x))?;
        Some((max_tile + UVec2::ONE) * self.tile_size)
    }

    pub fn get_tile_rect(&self, name: &str) -> Option<[u32; 4]> {
        let tile_pos = self.tiles.get(name)?;
        let size = self.tile_size;

        Some([tile_pos.x * size.x, tile_pos.y * size.y, size.x, size.y])
    }
}
