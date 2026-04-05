use std::collections::{BTreeMap, HashMap};
use toki_core::assets::{
    atlas::AtlasMeta,
    object_sheet::ObjectSheetMeta,
    tilemap::TileMap,
    tileset::{TileSetAtlasSource, TileSetMeta, TileSetResolver},
};
use toki_core::indexed_presentation::{
    materialize_tilemap_batches, resolve_indexed_palette, IndexedPresentationSettings,
    PresentedTextureSource,
};
use toki_core::palette::{Palette, PaletteMismatchStrategy};
pub use toki_core::project_assets::{
    classify_sprite_metadata_file, find_first_json_file, first_existing_path,
    load_project_palettes, normalize_asset_name, resolve_atlas_texture_path,
    resolve_object_sheet_texture_path, resolve_project_resource_paths, resolve_tilemap_tileset_path,
    resolve_tileset_atlas_paths,
    ResolvedProjectResourcePaths, SpriteMetadataFileKind,
};
use toki_core::project_runtime::ProjectRuntimeMetadata;
use toki_core::sprite_render::{
    resolve_atlas_tile_frame, resolve_object_sheet_frame, ResolvedSpriteVisual,
    SpriteAssetResolver, SpriteRenderMaterial, SpriteResolveError,
};
use toki_render::RenderError;
use toki_render::SceneTilemapBatch;

use crate::systems::DecodedProjectCache;

type SpriteAtlasRegistry = HashMap<String, AtlasMeta>;
type SpriteTextureRegistry = HashMap<String, Option<std::path::PathBuf>>;
type ObjectSheetRegistry = HashMap<String, ObjectSheetMeta>;
type ObjectTextureRegistry = HashMap<String, Option<std::path::PathBuf>>;
type TileSetAtlasRegistry = HashMap<String, TileSetAtlasSource>;

fn to_render_error(error: toki_core::project_assets::ProjectAssetError) -> RenderError {
    RenderError::Other(error.to_string())
}

/// Resource management system that handles loading and providing access to game assets.
///
/// Centralizes asset loading and provides clean APIs for accessing resources.
/// Future-ready for additional asset types like fonts, sounds, and shaders.
#[derive(Debug)]
pub struct ResourceManager {
    tileset: TileSetMeta,
    tileset_atlases: TileSetAtlasRegistry,
    sprite_atlases: SpriteAtlasRegistry,
    sprite_texture_paths: SpriteTextureRegistry,
    object_sheets: ObjectSheetRegistry,
    object_texture_paths: ObjectTextureRegistry,
    tilemap: TileMap,
    project_palettes: BTreeMap<String, Palette>,
    indexed_palette_override: Option<String>,
    palette_mismatch_strategy: PaletteMismatchStrategy,
}

pub struct PreloadedResources {
    pub tileset: TileSetMeta,
    pub tileset_atlases: TileSetAtlasRegistry,
    pub sprite_atlases: SpriteAtlasRegistry,
    pub sprite_texture_paths: SpriteTextureRegistry,
    pub object_sheets: ObjectSheetRegistry,
    pub object_texture_paths: ObjectTextureRegistry,
    pub tilemap: TileMap,
    pub project_palettes: BTreeMap<String, Palette>,
    pub indexed_palette_override: Option<String>,
    pub palette_mismatch_strategy: PaletteMismatchStrategy,
}

impl ResourceManager {
    /// Load all game resources from their respective files
    pub fn load_all() -> Result<Self, RenderError> {
        let tileset = TileSetMeta::load_from_file("assets/tilesets/new_town_map_64x64_crossings.json")?;
        let mut tileset_atlases = HashMap::new();
        let terrain_path = std::path::PathBuf::from("assets/sprites/terrain.json");
        let terrain_atlas = AtlasMeta::load_from_file(&terrain_path)?;
        register_tileset_atlas(
            &mut tileset_atlases,
            &terrain_path,
            terrain_atlas,
        );
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
            tileset,
            tileset_atlases,
            sprite_atlases,
            sprite_texture_paths,
            object_sheets,
            object_texture_paths,
            tilemap,
            project_palettes: BTreeMap::new(),
            indexed_palette_override: None,
            palette_mismatch_strategy: PaletteMismatchStrategy::default(),
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
        let tileset = decoded_project_cache.load_tileset_from_path(&resolved_paths.tileset_path)?;
        let tileset_atlases = load_tileset_atlas_registry_with_cache(
            &resolved_paths.tileset_atlas_paths,
            decoded_project_cache,
        )?;
        let (sprite_atlases, sprite_texture_paths) = load_sprite_atlas_registry_with_cache(
            &resolved_paths.sprite_atlas_paths,
            decoded_project_cache,
        )?;
        let (object_sheets, object_texture_paths) = load_object_sheet_registry_with_cache(
            &resolved_paths.object_sheet_paths,
            decoded_project_cache,
        )?;
        let palette_settings = load_palette_settings(project_path).unwrap_or_else(|error| {
            tracing::warn!(
                "Failed to load palette settings from '{}': {}",
                project_path.display(),
                error
            );
            LoadedPaletteSettings {
                palettes: BTreeMap::new(),
                override_id: None,
                mismatch_strategy: PaletteMismatchStrategy::default(),
            }
        });

        Ok((
            Self {
                tileset,
                tileset_atlases,
                sprite_atlases,
                sprite_texture_paths,
                object_sheets,
                object_texture_paths,
                tilemap,
                project_palettes: palette_settings.palettes,
                indexed_palette_override: palette_settings.override_id,
                palette_mismatch_strategy: palette_settings.mismatch_strategy,
            },
            resolved_paths,
        ))
    }

    pub fn from_preloaded(preloaded: PreloadedResources) -> Self {
        Self {
            tileset: preloaded.tileset,
            tileset_atlases: preloaded.tileset_atlases,
            sprite_atlases: preloaded.sprite_atlases,
            sprite_texture_paths: preloaded.sprite_texture_paths,
            object_sheets: preloaded.object_sheets,
            object_texture_paths: preloaded.object_texture_paths,
            tilemap: preloaded.tilemap,
            project_palettes: preloaded.project_palettes,
            indexed_palette_override: preloaded.indexed_palette_override,
            palette_mismatch_strategy: preloaded.palette_mismatch_strategy,
        }
    }

    pub fn get_tileset(&self) -> &TileSetMeta {
        &self.tileset
    }

    pub fn tileset_resolver(&self) -> TileSetResolver<'_> {
        TileSetResolver::new(&self.tileset, &self.tileset_atlases)
    }

    pub fn get_tileset_atlas(&self, atlas_name: &str) -> Option<&TileSetAtlasSource> {
        let normalized = normalize_asset_name(atlas_name);
        self.tileset_atlases
            .get(atlas_name)
            .or_else(|| self.tileset_atlases.get(normalized))
    }

    pub fn tileset_atlas_metas(&self) -> impl Iterator<Item = &AtlasMeta> {
        self.tileset_atlases.values().map(|source| &source.meta)
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
        self.tileset.tile_size
    }

    pub fn creature_tile_size(&self) -> glam::UVec2 {
        self.get_creature_atlas().tile_size
    }

    pub fn terrain_image_size(&self) -> Option<glam::UVec2> {
        self.tileset_atlases
            .values()
            .next()
            .and_then(|atlas| atlas.meta.image_size())
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

    pub fn project_palettes(&self) -> &BTreeMap<String, Palette> {
        &self.project_palettes
    }

    pub fn indexed_palette_override(&self) -> Option<&str> {
        self.indexed_palette_override.as_deref()
    }

    pub fn set_indexed_palette_override(&mut self, palette_id: Option<String>) {
        self.indexed_palette_override = palette_id;
    }

    pub fn build_runtime_tilemap_batches(
        &self,
        visible_chunks: &[(u32, u32)],
        tile_animation_clock: Option<&toki_core::assets::tile_animation::TileAnimationClock>,
    ) -> Result<Vec<SceneTilemapBatch>, RenderError> {
        let resolver = self.tileset_resolver();
        let batches = if visible_chunks.is_empty() {
            self.tilemap.generate_render_batches(
                &resolver,
                tile_animation_clock,
                self.indexed_palette_override(),
            )
        } else {
            self.tilemap.generate_render_batches_for_chunks(
                &resolver,
                visible_chunks,
                tile_animation_clock,
                self.indexed_palette_override(),
            )
        }
        .map_err(|error| RenderError::Other(error.to_string()))?;

        let presented = materialize_tilemap_batches(
            batches,
            &self.project_palettes,
            &self.indexed_presentation_settings(),
        )
        .map_err(RenderError::Other)?;

        Ok(presented
            .into_iter()
            .map(|batch| match batch.texture {
                PresentedTextureSource::File(texture_path) => SceneTilemapBatch {
                    vertices: batch.vertices,
                    texture_path: Some(texture_path),
                    texture_image: None,
                    texture_cache_key: None,
                    above_entities: batch.above_entities,
                },
                PresentedTextureSource::Rgba8 { image, cache_key } => SceneTilemapBatch {
                    vertices: batch.vertices,
                    texture_path: None,
                    texture_image: Some(image),
                    texture_cache_key: Some(cache_key),
                    above_entities: batch.above_entities,
                },
            })
            .collect())
    }

    pub fn palette_mismatch_strategy(&self) -> PaletteMismatchStrategy {
        self.palette_mismatch_strategy
    }

    pub fn resolve_indexed_palette(
        &self,
        atlas: &AtlasMeta,
        palette_override: Option<&str>,
    ) -> Result<(String, Palette), SpriteResolveError> {
        self.resolve_indexed_palette_by_id(atlas.palette.as_deref(), palette_override)
    }

    fn resolve_indexed_palette_by_id(
        &self,
        asset_palette: Option<&str>,
        palette_override: Option<&str>,
    ) -> Result<(String, Palette), SpriteResolveError> {
        resolve_indexed_palette(
            toki_core::assets::atlas::ColorMode::PaletteIndexed,
            &self.project_palettes,
            &self.indexed_presentation_settings(),
            palette_override,
            asset_palette,
        )
        .map_err(|message| SpriteResolveError::AssetLoadFailed {
            asset_kind: "palette",
            asset_name: self
                .indexed_palette_override
                .as_deref()
                .or(palette_override)
                .or(asset_palette)
                .unwrap_or("gb_default")
                .to_string(),
            message,
        })?
        .ok_or_else(|| SpriteResolveError::AssetLoadFailed {
            asset_kind: "palette",
            asset_name: self
                .indexed_palette_override
                .as_deref()
                .or(palette_override)
                .or(asset_palette)
                .unwrap_or("gb_default")
                .to_string(),
            message: "palette id could not be resolved".to_string(),
        })
    }
}

impl ResourceManager {
    fn indexed_presentation_settings(&self) -> IndexedPresentationSettings {
        IndexedPresentationSettings {
            indexed_palette_override: self.indexed_palette_override.clone(),
            post_process: toki_core::project_runtime::RuntimePostProcessSettings::default(),
        }
    }
}

impl SpriteAssetResolver for ResourceManager {
    fn resolve_atlas_tile(
        &mut self,
        atlas_name: &str,
        tile_name: &str,
        palette_override: Option<&str>,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        let atlas =
            self.get_sprite_atlas(atlas_name)
                .ok_or_else(|| SpriteResolveError::MissingAtlas {
                    atlas_name: atlas_name.to_string(),
                })?;
        let (frame, intrinsic_size) = resolve_atlas_tile_frame(atlas, atlas_name, tile_name)?;
        let material = if atlas.is_palette_indexed() {
            let (palette_id, palette) = self.resolve_indexed_palette(atlas, palette_override)?;
            SpriteRenderMaterial::PaletteIndexed {
                palette_id,
                palette,
            }
        } else {
            SpriteRenderMaterial::TrueColor
        };

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: self.get_sprite_texture_path(atlas_name).cloned(),
            material,
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
        let material = if object_sheet.is_palette_indexed() {
            let atlas_palette = object_sheet.palette.as_deref();
            let (palette_id, palette) = self.resolve_indexed_palette_by_id(atlas_palette, None)?;
            SpriteRenderMaterial::PaletteIndexed {
                palette_id,
                palette,
            }
        } else {
            SpriteRenderMaterial::TrueColor
        };

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: self.get_object_texture_path(sheet_name).cloned(),
            material,
        })
    }
}

struct LoadedPaletteSettings {
    palettes: BTreeMap<String, Palette>,
    override_id: Option<String>,
    mismatch_strategy: PaletteMismatchStrategy,
}

fn load_palette_settings(project_path: &std::path::Path) -> Result<LoadedPaletteSettings, String> {
    let project_palettes =
        load_project_palettes(project_path).map_err(|error| error.to_string())?;
    let project_file = project_path.join("project.toml");
    if !project_file.exists() {
        return Ok(LoadedPaletteSettings {
            palettes: project_palettes,
            override_id: None,
            mismatch_strategy: PaletteMismatchStrategy::default(),
        });
    }

    let metadata = std::fs::metadata(&project_file).map_err(|error| error.to_string())?;
    if metadata.len() > 1024 * 1024 {
        return Err(format!(
            "project runtime settings are too large to load safely: {} ({} bytes, max {})",
            project_file.display(),
            metadata.len(),
            1024 * 1024
        ));
    }
    let content = std::fs::read_to_string(&project_file).map_err(|error| error.to_string())?;
    let metadata =
        toml::from_str::<ProjectRuntimeMetadata>(&content).map_err(|error| error.to_string())?;
    Ok(LoadedPaletteSettings {
        palettes: project_palettes,
        override_id: metadata.runtime.display.indexed_palette_override,
        mismatch_strategy: metadata.runtime.display.palette_mismatch_strategy,
    })
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

fn register_tileset_atlas(
    atlas_map: &mut TileSetAtlasRegistry,
    atlas_path: &std::path::Path,
    atlas: AtlasMeta,
) {
    let source = TileSetAtlasSource {
        name: atlas_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        path: atlas_path.to_path_buf(),
        meta: atlas,
    };
    if let Some(file_name) = atlas_path.file_name().and_then(|name| name.to_str()) {
        atlas_map.insert(file_name.to_string(), source.clone());
    }
    if let Some(stem) = atlas_path.file_stem().and_then(|name| name.to_str()) {
        atlas_map.insert(stem.to_string(), source);
    }
}

fn load_tileset_atlas_registry_with_cache(
    atlas_paths: &[std::path::PathBuf],
    decoded_project_cache: &mut DecodedProjectCache,
) -> Result<TileSetAtlasRegistry, RenderError> {
    let mut atlas_map = HashMap::new();
    for atlas_path in atlas_paths {
        let atlas = decoded_project_cache.load_atlas_from_path(atlas_path)?;
        register_tileset_atlas(&mut atlas_map, atlas_path, atlas);
    }
    Ok(atlas_map)
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
