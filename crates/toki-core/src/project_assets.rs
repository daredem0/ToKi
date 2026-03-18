use crate::assets::{atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::TileMap};
use crate::CoreError;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectAssetError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("{0}")]
    Validation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteMetadataFileKind {
    Atlas,
    ObjectSheet,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAudioFormat {
    Ogg,
    Wav,
    Mp3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredAudioAsset {
    pub name: String,
    pub path: PathBuf,
    pub format: ProjectAudioFormat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSpriteMetadata {
    pub sprite_atlas_paths: Vec<PathBuf>,
    pub object_sheet_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProjectResourcePaths {
    pub tilemap_path: PathBuf,
    pub terrain_atlas_path: PathBuf,
    pub tilemap_texture_path: Option<PathBuf>,
    pub sprite_texture_path: Option<PathBuf>,
    pub sprite_atlas_paths: Vec<PathBuf>,
    pub object_sheet_paths: Vec<PathBuf>,
}

pub fn first_existing_path(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}

/// Normalizes an asset name by stripping the `.json` suffix if present.
///
/// This utility consolidates the common pattern of removing `.json` extensions
/// when looking up assets by name.
///
/// # Examples
/// ```
/// use toki_core::project_assets::normalize_asset_name;
/// assert_eq!(normalize_asset_name("terrain.json"), "terrain");
/// assert_eq!(normalize_asset_name("terrain"), "terrain");
/// ```
pub fn normalize_asset_name(name: &str) -> &str {
    name.strip_suffix(".json").unwrap_or(name)
}

/// Returns the canonical path for a scene file in a project.
///
/// Scene files are stored as `{project_path}/scenes/{scene_name}.json`.
pub fn scene_file_path(project_path: &Path, scene_name: &str) -> PathBuf {
    project_path
        .join("scenes")
        .join(format!("{scene_name}.json"))
}

/// Returns the canonical path for a tilemap file in a project.
///
/// Tilemap files are stored as `{project_path}/assets/tilemaps/{map_name}.json`.
pub fn tilemap_file_path(project_path: &Path, map_name: &str) -> PathBuf {
    project_path
        .join("assets")
        .join("tilemaps")
        .join(format!("{map_name}.json"))
}

pub fn find_json_files(dir: &Path) -> Result<Vec<PathBuf>, ProjectAssetError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut json_files = fs::read_dir(dir)?
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
    Ok(json_files)
}

pub fn find_first_json_file(dir: &Path) -> Result<Option<PathBuf>, ProjectAssetError> {
    Ok(find_json_files(dir)?.into_iter().next())
}

pub fn classify_sprite_metadata_file(
    path: &Path,
) -> Result<SpriteMetadataFileKind, ProjectAssetError> {
    let json_data = fs::read_to_string(path)?;

    if let Ok(object_sheet) = serde_json::from_str::<ObjectSheetMeta>(&json_data) {
        if matches!(
            object_sheet.sheet_type,
            crate::assets::object_sheet::ObjectSheetType::Objects
        ) {
            return Ok(SpriteMetadataFileKind::ObjectSheet);
        }
    }

    if serde_json::from_str::<AtlasMeta>(&json_data).is_ok() {
        return Ok(SpriteMetadataFileKind::Atlas);
    }

    Ok(SpriteMetadataFileKind::Unknown)
}

pub fn discover_sprite_metadata(dir: &Path) -> Result<DiscoveredSpriteMetadata, ProjectAssetError> {
    let mut sprite_atlas_paths = Vec::new();
    let mut object_sheet_paths = Vec::new();

    for path in find_json_files(dir)? {
        match classify_sprite_metadata_file(&path)? {
            SpriteMetadataFileKind::Atlas => sprite_atlas_paths.push(path),
            SpriteMetadataFileKind::ObjectSheet => object_sheet_paths.push(path),
            SpriteMetadataFileKind::Unknown => {}
        }
    }

    Ok(DiscoveredSpriteMetadata {
        sprite_atlas_paths,
        object_sheet_paths,
    })
}

pub fn discover_audio_files(dir: &Path) -> Result<Vec<DiscoveredAudioAsset>, ProjectAssetError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut assets = fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?.to_string();
            let format = match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
                "ogg" => ProjectAudioFormat::Ogg,
                "wav" => ProjectAudioFormat::Wav,
                "mp3" => ProjectAudioFormat::Mp3,
                _ => return None,
            };

            Some(DiscoveredAudioAsset {
                name: stem,
                path,
                format,
            })
        })
        .collect::<Vec<_>>();
    assets.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    Ok(assets)
}

pub fn resolve_tilemap_atlas_path(
    project_path: &Path,
    tilemap_path: &Path,
    tilemap: &TileMap,
) -> Option<PathBuf> {
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

pub fn resolve_atlas_texture_path(atlas_path: &Path) -> Result<Option<PathBuf>, ProjectAssetError> {
    let atlas = AtlasMeta::load_from_file(atlas_path)?;
    let atlas_dir = atlas_path.parent().ok_or_else(|| {
        ProjectAssetError::Validation(format!(
            "Atlas path '{}' has no parent directory",
            atlas_path.display()
        ))
    })?;
    Ok(first_existing_path(&[atlas_dir.join(&atlas.image)]))
}

pub fn resolve_object_sheet_texture_path(
    object_sheet_path: &Path,
) -> Result<Option<PathBuf>, ProjectAssetError> {
    let object_sheet = ObjectSheetMeta::load_from_file(object_sheet_path)?;
    let object_sheet_dir = object_sheet_path.parent().ok_or_else(|| {
        ProjectAssetError::Validation(format!(
            "Object sheet path '{}' has no parent directory",
            object_sheet_path.display()
        ))
    })?;
    Ok(first_existing_path(&[
        object_sheet_dir.join(&object_sheet.image)
    ]))
}

pub fn resolve_project_resource_paths(
    project_path: &Path,
    map_name: Option<&str>,
) -> Result<ResolvedProjectResourcePaths, ProjectAssetError> {
    let sprite_metadata = discover_sprite_metadata(&project_path.join("assets").join("sprites"))?;
    if sprite_metadata.sprite_atlas_paths.is_empty() {
        return Err(ProjectAssetError::Validation(format!(
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
            ProjectAssetError::Validation(format!(
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
        .or_else(|| {
            find_first_json_file(&project_path.join("assets").join("tilemaps"))
                .ok()
                .flatten()
        })
        .or_else(|| {
            find_first_json_file(&project_path.join("assets").join("maps"))
                .ok()
                .flatten()
        })
        .ok_or_else(|| {
            ProjectAssetError::Validation(format!(
                "Could not find any tilemap in project '{}'",
                project_path.display()
            ))
        })?
    };

    let tilemap = TileMap::load_from_file(&tilemap_path)?;
    tilemap.validate().map_err(ProjectAssetError::Core)?;

    let terrain_atlas_path = resolve_tilemap_atlas_path(project_path, &tilemap_path, &tilemap)
        .ok_or_else(|| {
            ProjectAssetError::Validation(format!(
                "Could not resolve tilemap atlas '{}' for map '{}'",
                tilemap.atlas.display(),
                tilemap_path.display()
            ))
        })?;

    let tilemap_texture_path = resolve_atlas_texture_path(&terrain_atlas_path)?;
    let sprite_texture_path = resolve_atlas_texture_path(&sprite_metadata.sprite_atlas_paths[0])?;

    Ok(ResolvedProjectResourcePaths {
        tilemap_path,
        terrain_atlas_path,
        tilemap_texture_path,
        sprite_texture_path,
        sprite_atlas_paths: sprite_metadata.sprite_atlas_paths,
        object_sheet_paths: sprite_metadata.object_sheet_paths,
    })
}

#[cfg(test)]
#[path = "project_assets_tests.rs"]
mod tests;
