use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::Scene;
use toki_render::RenderError;

use crate::systems::resources::{resolve_project_resource_paths, ResolvedProjectResourcePaths};

const COMMON_SFX_NAMES: &[&str] = &[
    "Jump 1",
    "sfx_bump_soft",
    "sfx_coin",
    "sfx_hit",
    "sfx_hit2",
    "sfx_jump",
    "sfx_powerup",
    "sfx_select",
    "sfx_slime_bounce",
    "sfx_step",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAssetLoadPlan {
    pub scene_name: Option<String>,
    pub map_name: Option<String>,
    pub tilemap_texture_path: Option<PathBuf>,
    pub sprite_texture_path: Option<PathBuf>,
    pub preloaded_sfx_names: Vec<String>,
    pub stream_music: bool,
}

impl RuntimeAssetLoadPlan {
    pub fn for_project(
        project_path: &Path,
        scene_name: Option<&str>,
        map_name: Option<&str>,
    ) -> Result<Self, RenderError> {
        let resolved = resolve_project_resource_paths(project_path, map_name)?;
        Ok(Self::from_resolved_paths(
            scene_name.map(str::to_string),
            map_name.map(str::to_string),
            &resolved,
        ))
    }

    pub fn from_resolved_paths(
        scene_name: Option<String>,
        map_name: Option<String>,
        resolved: &ResolvedProjectResourcePaths,
    ) -> Self {
        Self {
            scene_name,
            map_name,
            tilemap_texture_path: resolved.tilemap_texture_path.clone(),
            sprite_texture_path: resolved.sprite_texture_path.clone(),
            preloaded_sfx_names: common_preloaded_sfx_names(),
            stream_music: true,
        }
    }

    pub fn should_preload_sfx(name: &str) -> bool {
        COMMON_SFX_NAMES.contains(&name)
    }
}

pub fn common_preloaded_sfx_names() -> Vec<String> {
    COMMON_SFX_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

#[derive(Debug, Default)]
pub struct DecodedProjectCache {
    scenes: HashMap<PathBuf, Scene>,
    tilemaps: HashMap<PathBuf, TileMap>,
    atlases: HashMap<PathBuf, AtlasMeta>,
}

impl DecodedProjectCache {
    pub fn load_scene_from_path(&mut self, scene_path: &Path) -> Result<Scene, String> {
        if let Some(scene) = self.scenes.get(scene_path) {
            return Ok(scene.clone());
        }

        let json = fs::read_to_string(scene_path).map_err(|error| {
            format!(
                "Could not read scene file '{}': {}",
                scene_path.display(),
                error
            )
        })?;
        let scene = serde_json::from_str::<Scene>(&json).map_err(|error| {
            format!(
                "Could not parse scene file '{}': {}",
                scene_path.display(),
                error
            )
        })?;
        self.scenes.insert(scene_path.to_path_buf(), scene.clone());
        Ok(scene)
    }

    pub fn load_tilemap_from_path(
        &mut self,
        tilemap_path: &Path,
    ) -> Result<TileMap, toki_core::CoreError> {
        if let Some(tilemap) = self.tilemaps.get(tilemap_path) {
            return Ok(tilemap.clone());
        }

        let tilemap = TileMap::load_from_file(tilemap_path)?;
        self.tilemaps
            .insert(tilemap_path.to_path_buf(), tilemap.clone());
        Ok(tilemap)
    }

    pub fn load_atlas_from_path(
        &mut self,
        atlas_path: &Path,
    ) -> Result<AtlasMeta, toki_core::CoreError> {
        if let Some(atlas) = self.atlases.get(atlas_path) {
            return Ok(atlas.clone());
        }

        let atlas = AtlasMeta::load_from_file(atlas_path)?;
        self.atlases.insert(atlas_path.to_path_buf(), atlas.clone());
        Ok(atlas)
    }
}

#[cfg(test)]
mod tests {
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
}
