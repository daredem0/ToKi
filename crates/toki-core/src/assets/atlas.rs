use crate::CoreError;
use glam::UVec2;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TileProperties {
    pub solid: bool,
    #[serde(default)]
    pub trigger: bool,
}

impl Default for TileProperties {
    fn default() -> Self {
        Self {
            solid: false,
            trigger: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TileInfo {
    pub position: UVec2,
    #[serde(default)]
    pub properties: TileProperties,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct AtlasMeta {
    #[serde_as(as = "DisplayFromStr")]
    pub image: PathBuf,

    pub tile_size: UVec2,
    pub tiles: HashMap<String, TileInfo>,
}

impl AtlasMeta {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = fs::read_to_string(path)?;
        let meta = serde_json::from_str::<AtlasMeta>(&content)?;
        Ok(meta)
    }

    pub fn image_size(&self) -> Option<UVec2> {
        let max_tile = self.tiles.values()
            .map(|tile_info| tile_info.position)
            .max_by_key(|v| (v.y, v.x))?;
        Some((max_tile + UVec2::ONE) * self.tile_size)
    }

    pub fn get_tile_rect(&self, name: &str) -> Option<[u32; 4]> {
        let tile_info = self.tiles.get(name)?;
        let size = self.tile_size;

        Some([
            tile_info.position.x * size.x,
            tile_info.position.y * size.y,
            size.x,
            size.y,
        ])
    }

    pub fn is_tile_solid(&self, name: &str) -> bool {
        self.tiles.get(name)
            .map(|tile_info| tile_info.properties.solid)
            .unwrap_or(false)
    }

    pub fn is_tile_trigger(&self, name: &str) -> bool {
        self.tiles.get(name)
            .map(|tile_info| tile_info.properties.trigger)
            .unwrap_or(false)
    }

    pub fn get_tile_properties(&self, name: &str) -> Option<&TileProperties> {
        self.tiles.get(name).map(|tile_info| &tile_info.properties)
    }
}
