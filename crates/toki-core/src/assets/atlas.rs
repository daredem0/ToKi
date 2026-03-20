use crate::CoreError;
use glam::UVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

mod pathbuf_as_string {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::path::{Path, PathBuf};

    pub fn serialize<S: Serializer>(path: &Path, s: S) -> Result<S::Ok, S::Error> {
        path.to_string_lossy().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PathBuf, D::Error> {
        let s = String::deserialize(d)?;
        Ok(PathBuf::from(s))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct TileProperties {
    pub solid: bool,
    #[serde(default)]
    pub trigger: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TileInfo {
    pub position: UVec2,
    #[serde(default)]
    pub properties: TileProperties,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AtlasMeta {
    #[serde(with = "pathbuf_as_string")]
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
        let max_tile = self
            .tiles
            .values()
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

    /// Get tile UV coordinates for rendering (0.0 to 1.0 range)
    pub fn get_tile_uvs(&self, name: &str, texture_size: UVec2) -> Option<[f32; 4]> {
        let rect = self.get_tile_rect(name)?;
        let u0 = rect[0] as f32 / texture_size.x as f32;
        let v0 = rect[1] as f32 / texture_size.y as f32;
        let u1 = (rect[0] + rect[2]) as f32 / texture_size.x as f32;
        let v1 = (rect[1] + rect[3]) as f32 / texture_size.y as f32;

        Some([u0, v0, u1, v1])
    }

    pub fn is_tile_solid(&self, name: &str) -> bool {
        self.tiles
            .get(name)
            .map(|tile_info| tile_info.properties.solid)
            .unwrap_or(false)
    }

    pub fn is_tile_trigger(&self, name: &str) -> bool {
        self.tiles
            .get(name)
            .map(|tile_info| tile_info.properties.trigger)
            .unwrap_or(false)
    }

    pub fn get_tile_properties(&self, name: &str) -> Option<&TileProperties> {
        self.tiles.get(name).map(|tile_info| &tile_info.properties)
    }

    /// Create a new atlas with a single tile covering the entire image.
    pub fn new_single_tile(image_filename: impl Into<PathBuf>, tile_size: UVec2) -> Self {
        let mut tiles = HashMap::new();
        tiles.insert(
            "default".to_string(),
            TileInfo {
                position: UVec2::ZERO,
                properties: TileProperties::default(),
            },
        );
        Self {
            image: image_filename.into(),
            tile_size,
            tiles,
        }
    }

    /// Create a new atlas from a grid of tiles.
    pub fn new_grid(
        image_filename: impl Into<PathBuf>,
        tile_size: UVec2,
        cols: u32,
        rows: u32,
    ) -> Self {
        let mut tiles = HashMap::new();
        for row in 0..rows {
            for col in 0..cols {
                let index = row * cols + col;
                tiles.insert(
                    format!("tile_{index}"),
                    TileInfo {
                        position: UVec2::new(col, row),
                        properties: TileProperties::default(),
                    },
                );
            }
        }
        Self {
            image: image_filename.into(),
            tile_size,
            tiles,
        }
    }

    /// Save the atlas metadata to a JSON file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), CoreError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
