use std::collections::HashMap;
use toki_core::assets::{atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::TileMap};
use toki_render::RenderError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProjectResourcePaths {
    pub tilemap_path: std::path::PathBuf,
    pub terrain_atlas_path: std::path::PathBuf,
    pub tilemap_texture_path: Option<std::path::PathBuf>,
    pub sprite_texture_path: Option<std::path::PathBuf>,
    pub sprite_atlas_paths: Vec<std::path::PathBuf>,
    pub object_sheet_paths: Vec<std::path::PathBuf>,
}

type SpriteAtlasRegistry = HashMap<String, AtlasMeta>;
type SpriteTextureRegistry = HashMap<String, Option<std::path::PathBuf>>;
type ObjectSheetRegistry = HashMap<String, ObjectSheetMeta>;
type ObjectTextureRegistry = HashMap<String, Option<std::path::PathBuf>>;

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
            resolve_atlas_texture_path(&creatures_path)?,
        );
        // let tilemap = TileMap::load_from_file("assets/maps/tilemap_64x64_chunk.json")?;
        let tilemap = TileMap::load_from_file("assets/maps/new_town_map_64x64_crossings.json")?;
        // let tilemap = TileMap::load_from_file("assets/maps/my_new_map.json")?;

        // Validate the tilemap
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
        let resolved_paths = resolve_project_resource_paths(project_path, map_name)?;
        let tilemap = TileMap::load_from_file(&resolved_paths.tilemap_path)?;
        tilemap.validate()?;
        let terrain_atlas = AtlasMeta::load_from_file(resolved_paths.terrain_atlas_path)?;
        let (sprite_atlases, sprite_texture_paths) =
            load_sprite_atlas_registry(&resolved_paths.sprite_atlas_paths)?;
        let (object_sheets, object_texture_paths) =
            load_object_sheet_registry(&resolved_paths.object_sheet_paths)?;

        Ok(Self {
            terrain_atlas,
            sprite_atlases,
            sprite_texture_paths,
            object_sheets,
            object_texture_paths,
            tilemap,
        })
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

    /// Get reference to the terrain atlas
    pub fn get_terrain_atlas(&self) -> &AtlasMeta {
        &self.terrain_atlas
    }

    /// Get reference to a sprite atlas by logical name or filename.
    pub fn get_sprite_atlas(&self, atlas_name: &str) -> Option<&AtlasMeta> {
        self.sprite_atlases.get(atlas_name).or_else(|| {
            atlas_name
                .strip_suffix(".json")
                .and_then(|trimmed| self.sprite_atlases.get(trimmed))
        })
    }

    pub fn get_sprite_texture_path(&self, atlas_name: &str) -> Option<&std::path::PathBuf> {
        self.sprite_texture_paths
            .get(atlas_name)
            .or_else(|| {
                atlas_name
                    .strip_suffix(".json")
                    .and_then(|trimmed| self.sprite_texture_paths.get(trimmed))
            })
            .and_then(|path| path.as_ref())
    }

    pub fn get_object_sheet(&self, sheet_name: &str) -> Option<&ObjectSheetMeta> {
        self.object_sheets.get(sheet_name).or_else(|| {
            sheet_name
                .strip_suffix(".json")
                .and_then(|trimmed| self.object_sheets.get(trimmed))
        })
    }

    pub fn get_object_texture_path(&self, sheet_name: &str) -> Option<&std::path::PathBuf> {
        self.object_texture_paths
            .get(sheet_name)
            .or_else(|| {
                sheet_name
                    .strip_suffix(".json")
                    .and_then(|trimmed| self.object_texture_paths.get(trimmed))
            })
            .and_then(|path| path.as_ref())
    }

    /// Get reference to the default creature atlas for legacy code paths.
    pub fn get_creature_atlas(&self) -> &AtlasMeta {
        self.get_sprite_atlas("creatures.json")
            .or_else(|| self.sprite_atlases.values().next())
            .expect("at least one sprite atlas should be loaded")
    }

    /// Get reference to the tilemap
    pub fn get_tilemap(&self) -> &TileMap {
        &self.tilemap
    }

    /// Get terrain atlas tile size for convenience
    pub fn terrain_tile_size(&self) -> glam::UVec2 {
        self.terrain_atlas.tile_size
    }

    /// Get creature atlas tile size for convenience
    pub fn creature_tile_size(&self) -> glam::UVec2 {
        self.get_creature_atlas().tile_size
    }

    /// Get terrain atlas image size for convenience
    pub fn terrain_image_size(&self) -> Option<glam::UVec2> {
        self.terrain_atlas.image_size()
    }

    /// Get creature atlas image size for convenience
    pub fn creature_image_size(&self) -> Option<glam::UVec2> {
        self.get_creature_atlas().image_size()
    }

    /// Get tilemap size for convenience
    pub fn tilemap_size(&self) -> glam::UVec2 {
        self.tilemap.size
    }

    /// Get tilemap tile size for convenience
    pub fn tilemap_tile_size(&self) -> glam::UVec2 {
        self.tilemap.tile_size
    }
}

fn first_existing_path(candidates: &[std::path::PathBuf]) -> Option<std::path::PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}

fn find_first_json_file(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut json_files = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect::<Vec<_>>();
    json_files.sort();
    json_files.into_iter().next()
}

fn resolve_tilemap_atlas_path(
    project_path: &std::path::Path,
    tilemap_path: &std::path::Path,
    tilemap: &TileMap,
) -> Option<std::path::PathBuf> {
    let atlas_path = &tilemap.atlas;
    if atlas_path.is_absolute() && atlas_path.exists() {
        return Some(atlas_path.clone());
    }

    let map_dir = tilemap_path.parent()?;
    first_existing_path(&[
        map_dir.join(atlas_path),
        project_path.join("assets").join("sprites").join(atlas_path),
        project_path
            .join("assets")
            .join("tilemaps")
            .join(atlas_path),
        project_path.join("assets").join("maps").join(atlas_path),
        project_path.join("assets").join(atlas_path),
    ])
}

fn find_json_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut json_files = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .collect::<Vec<_>>();
    json_files.sort();
    json_files
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpriteMetadataFileKind {
    Atlas,
    ObjectSheet,
    Unknown,
}

fn classify_sprite_metadata_file(
    path: &std::path::Path,
) -> Result<SpriteMetadataFileKind, RenderError> {
    let json_data = std::fs::read_to_string(path).map_err(|error| {
        RenderError::Other(format!(
            "Failed to read sprite metadata file '{}': {}",
            path.display(),
            error
        ))
    })?;

    if let Ok(object_sheet) = serde_json::from_str::<ObjectSheetMeta>(&json_data) {
        if matches!(
            object_sheet.sheet_type,
            toki_core::assets::object_sheet::ObjectSheetType::Objects
        ) {
            return Ok(SpriteMetadataFileKind::ObjectSheet);
        }
    }

    if serde_json::from_str::<AtlasMeta>(&json_data).is_ok() {
        return Ok(SpriteMetadataFileKind::Atlas);
    }

    Ok(SpriteMetadataFileKind::Unknown)
}

fn find_sprite_atlas_json_files(
    dir: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, RenderError> {
    let mut atlas_files = Vec::new();

    for path in find_json_files(dir) {
        match classify_sprite_metadata_file(&path)? {
            SpriteMetadataFileKind::Atlas => atlas_files.push(path),
            SpriteMetadataFileKind::ObjectSheet | SpriteMetadataFileKind::Unknown => {}
        }
    }

    Ok(atlas_files)
}

fn find_object_sheet_json_files(
    dir: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, RenderError> {
    let mut object_sheet_files = Vec::new();

    for path in find_json_files(dir) {
        match classify_sprite_metadata_file(&path)? {
            SpriteMetadataFileKind::ObjectSheet => object_sheet_files.push(path),
            SpriteMetadataFileKind::Atlas | SpriteMetadataFileKind::Unknown => {}
        }
    }

    Ok(object_sheet_files)
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

fn load_sprite_atlas_registry(
    atlas_paths: &[std::path::PathBuf],
) -> Result<(SpriteAtlasRegistry, SpriteTextureRegistry), RenderError> {
    let mut atlas_map = HashMap::new();
    let mut texture_map = HashMap::new();

    for atlas_path in atlas_paths {
        let atlas = AtlasMeta::load_from_file(atlas_path)?;
        let texture_path = resolve_atlas_texture_path(atlas_path)?;
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

pub fn resolve_object_sheet_texture_path(
    object_sheet_path: &std::path::Path,
) -> Result<Option<std::path::PathBuf>, RenderError> {
    let object_sheet = ObjectSheetMeta::load_from_file(object_sheet_path)?;
    let object_sheet_dir = object_sheet_path.parent().ok_or_else(|| {
        RenderError::Other(format!(
            "Object sheet path '{}' has no parent directory",
            object_sheet_path.display()
        ))
    })?;
    Ok(first_existing_path(&[
        object_sheet_dir.join(&object_sheet.image)
    ]))
}

fn load_object_sheet_registry(
    object_sheet_paths: &[std::path::PathBuf],
) -> Result<(ObjectSheetRegistry, ObjectTextureRegistry), RenderError> {
    let mut sheet_map = HashMap::new();
    let mut texture_map = HashMap::new();

    for object_sheet_path in object_sheet_paths {
        let object_sheet = ObjectSheetMeta::load_from_file(object_sheet_path)?;
        let texture_path = resolve_object_sheet_texture_path(object_sheet_path)?;
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

pub fn resolve_project_resource_paths(
    project_path: &std::path::Path,
    map_name: Option<&str>,
) -> Result<ResolvedProjectResourcePaths, RenderError> {
    let sprite_atlas_paths =
        find_sprite_atlas_json_files(&project_path.join("assets").join("sprites"))?;
    let object_sheet_paths =
        find_object_sheet_json_files(&project_path.join("assets").join("sprites"))?;
    if sprite_atlas_paths.is_empty() {
        return Err(RenderError::Other(format!(
            "Could not find any sprite atlas in project '{}'",
            project_path.display()
        )));
    }

    let tilemap_path = if let Some(map_name) = map_name {
        first_existing_path(&[
            project_path
                .join("assets")
                .join("tilemaps")
                .join(format!("{map_name}.json")),
            project_path
                .join("assets")
                .join("maps")
                .join(format!("{map_name}.json")),
        ])
        .ok_or_else(|| {
            RenderError::Other(format!(
                "Could not find tilemap '{}' in project '{}'",
                map_name,
                project_path.display()
            ))
        })?
    } else {
        first_existing_path(&[
            project_path
                .join("assets")
                .join("tilemaps")
                .join("new_town_map_64x64_crossings.json"),
            project_path
                .join("assets")
                .join("maps")
                .join("new_town_map_64x64_crossings.json"),
        ])
        .or_else(|| find_first_json_file(&project_path.join("assets").join("tilemaps")))
        .or_else(|| find_first_json_file(&project_path.join("assets").join("maps")))
        .ok_or_else(|| {
            RenderError::Other(format!(
                "Could not find any tilemap in project '{}'",
                project_path.display()
            ))
        })?
    };

    let tilemap = TileMap::load_from_file(&tilemap_path)?;
    tilemap.validate()?;

    let terrain_atlas_path = resolve_tilemap_atlas_path(project_path, &tilemap_path, &tilemap)
        .ok_or_else(|| {
            RenderError::Other(format!(
                "Could not resolve tilemap atlas '{}' for map '{}'",
                tilemap.atlas.display(),
                tilemap_path.display()
            ))
        })?;

    let tilemap_texture_path = resolve_atlas_texture_path(&terrain_atlas_path)?;
    let sprite_texture_path = resolve_atlas_texture_path(&sprite_atlas_paths[0])?;

    Ok(ResolvedProjectResourcePaths {
        tilemap_path,
        terrain_atlas_path,
        tilemap_texture_path,
        sprite_texture_path,
        sprite_atlas_paths,
        object_sheet_paths,
    })
}

pub fn resolve_atlas_texture_path(
    atlas_path: &std::path::Path,
) -> Result<Option<std::path::PathBuf>, RenderError> {
    let atlas = AtlasMeta::load_from_file(atlas_path)?;
    let atlas_dir = atlas_path.parent().ok_or_else(|| {
        RenderError::Other(format!(
            "Atlas path '{}' has no parent directory",
            atlas_path.display()
        ))
    })?;
    Ok(first_existing_path(&[atlas_dir.join(&atlas.image)]))
}

#[cfg(test)]
#[path = "resources_tests.rs"]
mod tests;
