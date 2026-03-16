use crate::CoreError;
use glam::UVec2;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectSheetType {
    Objects,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ObjectSpriteInfo {
    pub position: UVec2,
    pub size_tiles: UVec2,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ObjectSheetMeta {
    pub sheet_type: ObjectSheetType,
    #[serde_as(as = "DisplayFromStr")]
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
}

#[cfg(test)]
#[path = "object_sheet_tests.rs"]
mod tests;
