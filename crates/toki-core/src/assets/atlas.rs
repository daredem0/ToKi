use glam::Uvec2;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct AtlasMeta {
    #[serde_as(as = "DisplayFromStr")]
    pub image: PathBuf,
    pub tile_size: [u32; 2],
    pub tiles: HashMap<String, [u32; 2]>,
}

impl AtlasMeta {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = fs::read_to_string(path)?;
        let meta = serde_json::from_str::<AtlasMeta>(&text)?;
        Ok(meta)
    }

    pub fn get_tile_rect(&self, name: &str) -> Option<[u32; 4]> {
        let tile_pos = self.tiles.get(name)?;
        let tile_w = self.tile_size[0];
        let tile_h = self.tile_size[1];

        Some([tile_pos[0] * tile_w, tile_pos[1] * tile_h, tile_w, tile_h])
    }
}
