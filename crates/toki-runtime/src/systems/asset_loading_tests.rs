
use super::{common_preloaded_sfx_names, DecodedProjectCache, RuntimeAssetLoadPlan};
use std::fs;

fn make_unique_temp_dir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("toki_runtime_asset_plan_tests_{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

fn write_minimal_atlas(path: &std::path::Path, image_name: &str) {
    let content = format!(
        r#"{{
  "image": "{image_name}",
  "tile_size": [16, 16],
  "tiles": {{
    "floor": {{
      "position": [0, 0],
      "properties": {{
        "solid": false
      }}
    }}
  }}
}}"#
    );
    fs::write(path, content).expect("atlas write");
}

fn write_minimal_map(path: &std::path::Path, atlas_ref: &str) {
    let content = format!(
        r#"{{
  "size": [1, 1],
  "tile_size": [16, 16],
  "atlas": "{atlas_ref}",
  "tiles": ["floor"]
}}"#
    );
    fs::write(path, content).expect("map write");
}

#[test]
fn load_plan_prefers_hot_textures_and_preloads_common_sfx() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    let tilemaps_dir = project_dir.join("assets").join("tilemaps");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

    write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
    write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
    write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
    fs::write(sprites_dir.join("creatures.png"), "png").expect("creatures image");
    fs::write(tilemaps_dir.join("terrain.png"), "png").expect("terrain image");

    let plan =
        RuntimeAssetLoadPlan::for_project(&project_dir, Some("Main Scene"), Some("demo_map"))
            .expect("plan should resolve");

    assert_eq!(plan.scene_name.as_deref(), Some("Main Scene"));
    assert_eq!(plan.map_name.as_deref(), Some("demo_map"));
    assert_eq!(
        plan.tilemap_texture_path,
        Some(tilemaps_dir.join("terrain.png"))
    );
    assert_eq!(
        plan.sprite_texture_path,
        Some(sprites_dir.join("creatures.png"))
    );
    assert!(plan.stream_music);
    assert!(plan.preloaded_sfx_names.contains(&"sfx_jump".to_string()));
    assert!(plan.preloaded_sfx_names.contains(&"sfx_select".to_string()));
}

#[test]
fn decoded_project_cache_reuses_cached_scene_after_file_is_removed() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let scene_path = temp_dir.path().join("Main Scene.json");
    fs::write(
        &scene_path,
        r#"{
  "name": "Main Scene",
  "description": null,
  "maps": [],
  "entities": [],
  "rules": {
    "chains": []
  },
  "camera_position": null,
  "camera_scale": null
}"#,
    )
    .expect("scene write");

    let mut cache = DecodedProjectCache::default();
    let first = cache
        .load_scene_from_path(&scene_path)
        .expect("first scene load");
    fs::remove_file(&scene_path).expect("remove original scene file");
    let second = cache
        .load_scene_from_path(&scene_path)
        .expect("cached scene load should still work");

    assert_eq!(first.name, second.name);
    assert_eq!(cache.scenes.len(), 1);
}

#[test]
fn common_preloaded_sfx_names_match_hot_sfx_policy() {
    let names = common_preloaded_sfx_names();
    assert!(names
        .iter()
        .all(|name| RuntimeAssetLoadPlan::should_preload_sfx(name)));
    assert!(!RuntimeAssetLoadPlan::should_preload_sfx("lavandia"));
}
