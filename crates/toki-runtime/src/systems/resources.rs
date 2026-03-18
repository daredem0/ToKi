use std::collections::HashMap;
use toki_core::assets::{atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::TileMap};
pub use toki_core::project_assets::{
    classify_sprite_metadata_file, find_first_json_file, first_existing_path, normalize_asset_name,
    resolve_atlas_texture_path, resolve_object_sheet_texture_path, resolve_project_resource_paths,
    resolve_tilemap_atlas_path, ResolvedProjectResourcePaths, SpriteMetadataFileKind,
};
use toki_core::sprite_render::{
    resolve_atlas_tile_frame, resolve_object_sheet_frame, ResolvedSpriteVisual,
    SpriteAssetResolver, SpriteResolveError,
};
use toki_render::RenderError;

use crate::systems::DecodedProjectCache;

type SpriteAtlasRegistry = HashMap<String, AtlasMeta>;
type SpriteTextureRegistry = HashMap<String, Option<std::path::PathBuf>>;
type ObjectSheetRegistry = HashMap<String, ObjectSheetMeta>;
type ObjectTextureRegistry = HashMap<String, Option<std::path::PathBuf>>;

fn to_render_error(error: toki_core::project_assets::ProjectAssetError) -> RenderError {
    RenderError::Other(error.to_string())
}

/// Resource management system that handles loading and providing access to game assets.
///
/// Centralizes asset loading and provides clean APIs for accessing resources.
/// Future-ready for additional asset types like fonts, sounds, and shaders.
#[derive(Debug)]
pub struct ResourceManager {
    terrain_atlas: AtlasMeta,
    sprite_atlases: SpriteAtlasRegistry,
    sprite_texture_paths: SpriteTextureRegistry,
    object_sheets: ObjectSheetRegistry,
    object_texture_paths: ObjectTextureRegistry,
    tilemap: TileMap,
}

impl ResourceManager {
    /// Load all game resources from their respective files
    pub fn load_all() -> Result<Self, RenderError> {
        let terrain_atlas = AtlasMeta::load_from_file("assets/terrain.json")?;
        let mut sprite_atlases = HashMap::new();
        let mut sprite_texture_paths = HashMap::new();
        let object_sheets = HashMap::new();
        let object_texture_paths = HashMap::new();
        let creatures_path = std::path::PathBuf::from("assets/creatures.json");
        let creature_atlas = AtlasMeta::load_from_file(&creatures_path)?;
        register_sprite_atlas(
            &mut sprite_atlases,
            &mut sprite_texture_paths,
            &creatures_path,
            creature_atlas,
            resolve_atlas_texture_path(&creatures_path).map_err(to_render_error)?,
        );
        let tilemap = TileMap::load_from_file("assets/maps/new_town_map_64x64_crossings.json")?;
        tilemap.validate()?;

        Ok(Self {
            terrain_atlas,
            sprite_atlases,
            sprite_texture_paths,
            object_sheets,
            object_texture_paths,
            tilemap,
        })
    }

    /// Load project resources from a ToKi project root.
    ///
    /// `map_name` should be the map filename stem (without `.json`) as stored in scenes.
    pub fn load_for_project(
        project_path: &std::path::Path,
        map_name: Option<&str>,
    ) -> Result<Self, RenderError> {
        let mut cache = DecodedProjectCache::default();
        let (resources, _) = Self::load_for_project_with_cache(project_path, map_name, &mut cache)?;
        Ok(resources)
    }

    pub fn load_for_project_with_cache(
        project_path: &std::path::Path,
        map_name: Option<&str>,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> Result<(Self, ResolvedProjectResourcePaths), RenderError> {
        let resolved_paths =
            resolve_project_resource_paths(project_path, map_name).map_err(to_render_error)?;
        let tilemap = decoded_project_cache.load_tilemap_from_path(&resolved_paths.tilemap_path)?;
        tilemap.validate()?;
        let terrain_atlas =
            decoded_project_cache.load_atlas_from_path(&resolved_paths.terrain_atlas_path)?;
        let (sprite_atlases, sprite_texture_paths) = load_sprite_atlas_registry_with_cache(
            &resolved_paths.sprite_atlas_paths,
            decoded_project_cache,
        )?;
        let (object_sheets, object_texture_paths) = load_object_sheet_registry_with_cache(
            &resolved_paths.object_sheet_paths,
            decoded_project_cache,
        )?;

        Ok((
            Self {
                terrain_atlas,
                sprite_atlases,
                sprite_texture_paths,
                object_sheets,
                object_texture_paths,
                tilemap,
            },
            resolved_paths,
        ))
    }

    pub fn from_preloaded(
        terrain_atlas: AtlasMeta,
        sprite_atlases: SpriteAtlasRegistry,
        sprite_texture_paths: SpriteTextureRegistry,
        object_sheets: ObjectSheetRegistry,
        object_texture_paths: ObjectTextureRegistry,
        tilemap: TileMap,
    ) -> Self {
        Self {
            terrain_atlas,
            sprite_atlases,
            sprite_texture_paths,
            object_sheets,
            object_texture_paths,
            tilemap,
        }
    }

    pub fn get_terrain_atlas(&self) -> &AtlasMeta {
        &self.terrain_atlas
    }

    pub fn get_sprite_atlas(&self, atlas_name: &str) -> Option<&AtlasMeta> {
        let normalized = normalize_asset_name(atlas_name);
        self.sprite_atlases
            .get(atlas_name)
            .or_else(|| self.sprite_atlases.get(normalized))
    }

    pub fn get_sprite_texture_path(&self, atlas_name: &str) -> Option<&std::path::PathBuf> {
        let normalized = normalize_asset_name(atlas_name);
        self.sprite_texture_paths
            .get(atlas_name)
            .or_else(|| self.sprite_texture_paths.get(normalized))
            .and_then(|path| path.as_ref())
    }

    pub fn get_object_sheet(&self, sheet_name: &str) -> Option<&ObjectSheetMeta> {
        let normalized = normalize_asset_name(sheet_name);
        self.object_sheets
            .get(sheet_name)
            .or_else(|| self.object_sheets.get(normalized))
    }

    pub fn get_object_texture_path(&self, sheet_name: &str) -> Option<&std::path::PathBuf> {
        let normalized = normalize_asset_name(sheet_name);
        self.object_texture_paths
            .get(sheet_name)
            .or_else(|| self.object_texture_paths.get(normalized))
            .and_then(|path| path.as_ref())
    }

    pub fn get_creature_atlas(&self) -> &AtlasMeta {
        self.get_sprite_atlas("creatures.json")
            .or_else(|| self.sprite_atlases.values().next())
            .expect("at least one sprite atlas should be loaded")
    }

    pub fn get_tilemap(&self) -> &TileMap {
        &self.tilemap
    }

    pub fn terrain_tile_size(&self) -> glam::UVec2 {
        self.terrain_atlas.tile_size
    }

    pub fn creature_tile_size(&self) -> glam::UVec2 {
        self.get_creature_atlas().tile_size
    }

    pub fn terrain_image_size(&self) -> Option<glam::UVec2> {
        self.terrain_atlas.image_size()
    }

    pub fn creature_image_size(&self) -> Option<glam::UVec2> {
        self.get_creature_atlas().image_size()
    }

    pub fn tilemap_size(&self) -> glam::UVec2 {
        self.tilemap.size
    }

    pub fn tilemap_tile_size(&self) -> glam::UVec2 {
        self.tilemap.tile_size
    }
}

impl SpriteAssetResolver for ResourceManager {
    fn resolve_atlas_tile(
        &mut self,
        atlas_name: &str,
        tile_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        let atlas =
            self.get_sprite_atlas(atlas_name)
                .ok_or_else(|| SpriteResolveError::MissingAtlas {
                    atlas_name: atlas_name.to_string(),
                })?;
        let (frame, intrinsic_size) = resolve_atlas_tile_frame(atlas, atlas_name, tile_name)?;

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: self.get_sprite_texture_path(atlas_name).cloned(),
        })
    }

    fn resolve_object_sheet_object(
        &mut self,
        sheet_name: &str,
        object_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        let object_sheet = self.get_object_sheet(sheet_name).ok_or_else(|| {
            SpriteResolveError::MissingObjectSheet {
                sheet_name: sheet_name.to_string(),
            }
        })?;
        let (frame, intrinsic_size) =
            resolve_object_sheet_frame(object_sheet, sheet_name, object_name)?;

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: self.get_object_texture_path(sheet_name).cloned(),
        })
    }
}

fn register_sprite_atlas(
    atlas_map: &mut SpriteAtlasRegistry,
    texture_map: &mut SpriteTextureRegistry,
    atlas_path: &std::path::Path,
    atlas: AtlasMeta,
    texture_path: Option<std::path::PathBuf>,
) {
    if let Some(file_name) = atlas_path.file_name().and_then(|name| name.to_str()) {
        atlas_map.insert(file_name.to_string(), atlas.clone());
        texture_map.insert(file_name.to_string(), texture_path.clone());
    }
    if let Some(stem) = atlas_path.file_stem().and_then(|name| name.to_str()) {
        atlas_map.insert(stem.to_string(), atlas);
        texture_map.insert(stem.to_string(), texture_path);
    }
}

fn load_sprite_atlas_registry_with_cache(
    atlas_paths: &[std::path::PathBuf],
    decoded_project_cache: &mut DecodedProjectCache,
) -> Result<(SpriteAtlasRegistry, SpriteTextureRegistry), RenderError> {
    let mut atlas_map = HashMap::new();
    let mut texture_map = HashMap::new();

    for atlas_path in atlas_paths {
        let atlas = decoded_project_cache.load_atlas_from_path(atlas_path)?;
        let texture_path = resolve_atlas_texture_path(atlas_path).map_err(to_render_error)?;
        register_sprite_atlas(
            &mut atlas_map,
            &mut texture_map,
            atlas_path,
            atlas,
            texture_path,
        );
    }

    Ok((atlas_map, texture_map))
}

fn register_object_sheet(
    sheet_map: &mut ObjectSheetRegistry,
    texture_map: &mut ObjectTextureRegistry,
    object_sheet_path: &std::path::Path,
    object_sheet: ObjectSheetMeta,
    texture_path: Option<std::path::PathBuf>,
) {
    if let Some(file_name) = object_sheet_path.file_name().and_then(|name| name.to_str()) {
        sheet_map.insert(file_name.to_string(), object_sheet.clone());
        texture_map.insert(file_name.to_string(), texture_path.clone());
    }
    if let Some(stem) = object_sheet_path.file_stem().and_then(|name| name.to_str()) {
        sheet_map.insert(stem.to_string(), object_sheet);
        texture_map.insert(stem.to_string(), texture_path);
    }
}

fn load_object_sheet_registry_with_cache(
    object_sheet_paths: &[std::path::PathBuf],
    decoded_project_cache: &mut DecodedProjectCache,
) -> Result<(ObjectSheetRegistry, ObjectTextureRegistry), RenderError> {
    let mut sheet_map = HashMap::new();
    let mut texture_map = HashMap::new();

    for object_sheet_path in object_sheet_paths {
        let object_sheet = decoded_project_cache.load_object_sheet_from_path(object_sheet_path)?;
        let texture_path =
            resolve_object_sheet_texture_path(object_sheet_path).map_err(to_render_error)?;
        register_object_sheet(
            &mut sheet_map,
            &mut texture_map,
            object_sheet_path,
            object_sheet,
            texture_path,
        );
    }

    Ok((sheet_map, texture_map))
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
