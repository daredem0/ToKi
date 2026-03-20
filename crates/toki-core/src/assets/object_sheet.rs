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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectSheetType {
    Objects,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ObjectSpriteInfo {
    pub position: UVec2,
    pub size_tiles: UVec2,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ObjectSheetMeta {
    pub sheet_type: ObjectSheetType,
    #[serde(with = "pathbuf_as_string")]
    pub image: PathBuf,
    pub tile_size: UVec2,
    pub objects: HashMap<String, ObjectSpriteInfo>,
}

impl ObjectSheetMeta {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = fs::read_to_string(path)?;
        let meta = serde_json::from_str::<ObjectSheetMeta>(&content)?;
        Ok(meta)
    }

    pub fn image_size(&self) -> Option<UVec2> {
        let max_object = self
            .objects
            .values()
            .map(|object_info| object_info.position + object_info.size_tiles)
            .max_by_key(|v| (v.y, v.x))?;
        Some(max_object * self.tile_size)
    }

    pub fn get_object_rect(&self, name: &str) -> Option<[u32; 4]> {
        let object_info = self.objects.get(name)?;
        Some([
            object_info.position.x * self.tile_size.x,
            object_info.position.y * self.tile_size.y,
            object_info.size_tiles.x * self.tile_size.x,
            object_info.size_tiles.y * self.tile_size.y,
        ])
    }

    pub fn get_object_uvs(&self, name: &str, texture_size: UVec2) -> Option<[f32; 4]> {
        let rect = self.get_object_rect(name)?;
        let u0 = rect[0] as f32 / texture_size.x as f32;
        let v0 = rect[1] as f32 / texture_size.y as f32;
        let u1 = (rect[0] + rect[2]) as f32 / texture_size.x as f32;
        let v1 = (rect[1] + rect[3]) as f32 / texture_size.y as f32;

        Some([u0, v0, u1, v1])
    }

    /// Create a new object sheet with a single object covering the entire image.
    pub fn new_single_object(
        image_filename: impl Into<PathBuf>,
        object_name: &str,
        size: UVec2,
    ) -> Self {
        let mut objects = HashMap::new();
        objects.insert(
            object_name.to_string(),
            ObjectSpriteInfo {
                position: UVec2::ZERO,
                size_tiles: UVec2::ONE,
            },
        );
        Self {
            sheet_type: ObjectSheetType::Objects,
            image: image_filename.into(),
            tile_size: size,
            objects,
        }
    }

    /// Create a new object sheet from a grid of equal-sized objects.
    pub fn new_grid(
        image_filename: impl Into<PathBuf>,
        cell_size: UVec2,
        cols: u32,
        rows: u32,
    ) -> Self {
        let mut objects = HashMap::new();
        for row in 0..rows {
            for col in 0..cols {
                let index = row * cols + col;
                objects.insert(
                    format!("object_{index}"),
                    ObjectSpriteInfo {
                        position: UVec2::new(col, row),
                        size_tiles: UVec2::ONE,
                    },
                );
            }
        }
        Self {
            sheet_type: ObjectSheetType::Objects,
            image: image_filename.into(),
            tile_size: cell_size,
            objects,
        }
    }

    /// Save the object sheet metadata to a JSON file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), CoreError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "object_sheet_tests.rs"]
mod tests;
