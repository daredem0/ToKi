use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use toki_core::assets::{atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::TileMap};
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
        let resolved = resolve_project_resource_paths(project_path, map_name)
            .map_err(|error| RenderError::Other(error.to_string()))?;
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
    object_sheets: HashMap<PathBuf, ObjectSheetMeta>,
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

    pub fn load_object_sheet_from_path(
        &mut self,
        object_sheet_path: &Path,
    ) -> Result<ObjectSheetMeta, toki_core::CoreError> {
        if let Some(object_sheet) = self.object_sheets.get(object_sheet_path) {
            return Ok(object_sheet.clone());
        }

        let object_sheet = ObjectSheetMeta::load_from_file(object_sheet_path)?;
        self.object_sheets
            .insert(object_sheet_path.to_path_buf(), object_sheet.clone());
        Ok(object_sheet)
    }
}

#[cfg(test)]
#[path = "asset_loading_tests.rs"]
mod tests;
