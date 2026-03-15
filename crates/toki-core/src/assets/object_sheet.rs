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
mod tests {
    use super::{ObjectSheetMeta, ObjectSheetType};
    use glam::UVec2;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn object_sheet_meta_loads_named_objects_and_rects() {
        let temp_dir = tempdir().expect("tempdir should be created");
        let path = temp_dir.path().join("fauna.json");
        fs::write(
            &path,
            r#"{
                "sheet_type": "objects",
                "image": "fauna.png",
                "tile_size": [16, 16],
                "objects": {
                    "fauna_a": {
                        "position": [0, 0],
                        "size_tiles": [1, 1]
                    },
                    "fauna_banner": {
                        "position": [0, 0],
                        "size_tiles": [2, 1]
                    }
                }
            }"#,
        )
        .expect("object sheet should be written");

        let object_sheet =
            ObjectSheetMeta::load_from_file(&path).expect("object sheet should load");

        assert_eq!(object_sheet.sheet_type, ObjectSheetType::Objects);
        assert_eq!(object_sheet.image, std::path::PathBuf::from("fauna.png"));
        assert_eq!(object_sheet.tile_size, UVec2::new(16, 16));
        assert_eq!(
            object_sheet.get_object_rect("fauna_banner"),
            Some([0, 0, 32, 16])
        );
        assert_eq!(object_sheet.image_size(), Some(UVec2::new(32, 16)));
    }

    #[test]
    fn object_sheet_meta_reports_uvs_from_tile_bounds() {
        let object_sheet: ObjectSheetMeta = serde_json::from_str(
            r#"{
                "sheet_type": "objects",
                "image": "fauna.png",
                "tile_size": [16, 16],
                "objects": {
                    "fauna_b": {
                        "position": [1, 0],
                        "size_tiles": [1, 1]
                    }
                }
            }"#,
        )
        .expect("inline object sheet should parse");

        assert_eq!(
            object_sheet.get_object_uvs("fauna_b", UVec2::new(32, 16)),
            Some([0.5, 0.0, 1.0, 1.0])
        );
    }
}
