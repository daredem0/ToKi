
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

    let object_sheet = ObjectSheetMeta::load_from_file(&path).expect("object sheet should load");

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
