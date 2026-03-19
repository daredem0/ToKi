use std::fs;
use std::path::{Path, PathBuf};

use toki_core::assets::{atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::TileMap};
use toki_core::entity::EntityDefinition;
use toki_core::AssetCache;
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

/// Cache for decoded project assets to avoid re-reading files.
///
/// Uses the generic `AssetCache` from toki-core to eliminate duplicate
/// get-or-load patterns for different asset types.
#[derive(Debug, Default)]
pub struct DecodedProjectCache {
    scenes: AssetCache<PathBuf, Scene>,
    tilemaps: AssetCache<PathBuf, TileMap>,
    atlases: AssetCache<PathBuf, AtlasMeta>,
    object_sheets: AssetCache<PathBuf, ObjectSheetMeta>,
    entity_definitions: AssetCache<PathBuf, EntityDefinition>,
}

impl DecodedProjectCache {
    pub fn load_scene_from_path(&mut self, scene_path: &Path) -> Result<Scene, String> {
        self.scenes.get_or_load(scene_path.to_path_buf(), |path| {
            let json = fs::read_to_string(path).map_err(|error| {
                format!("Could not read scene file '{}': {}", path.display(), error)
            })?;
            serde_json::from_str::<Scene>(&json).map_err(|error| {
                format!("Could not parse scene file '{}': {}", path.display(), error)
            })
        })
    }

    pub fn load_tilemap_from_path(
        &mut self,
        tilemap_path: &Path,
    ) -> Result<TileMap, toki_core::CoreError> {
        self.tilemaps
            .get_or_load(tilemap_path.to_path_buf(), |path| {
                TileMap::load_from_file(path)
            })
    }

    pub fn load_atlas_from_path(
        &mut self,
        atlas_path: &Path,
    ) -> Result<AtlasMeta, toki_core::CoreError> {
        self.atlases.get_or_load(atlas_path.to_path_buf(), |path| {
            AtlasMeta::load_from_file(path)
        })
    }

    pub fn load_object_sheet_from_path(
        &mut self,
        object_sheet_path: &Path,
    ) -> Result<ObjectSheetMeta, toki_core::CoreError> {
        self.object_sheets
            .get_or_load(object_sheet_path.to_path_buf(), |path| {
                ObjectSheetMeta::load_from_file(path)
            })
    }

    pub fn load_entity_definition_from_path(
        &mut self,
        entity_definition_path: &Path,
    ) -> Result<EntityDefinition, toki_core::CoreError> {
        self.entity_definitions
            .get_or_load(entity_definition_path.to_path_buf(), |path| {
                let json = fs::read_to_string(path)?;
                Ok(serde_json::from_str::<EntityDefinition>(&json)?)
            })
    }
}

#[cfg(test)]
#[path = "asset_loading_tests.rs"]
mod tests;
