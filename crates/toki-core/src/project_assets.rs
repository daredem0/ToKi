use crate::assets::{atlas::AtlasMeta, object_sheet::ObjectSheetMeta, tilemap::TileMap};
use crate::dialog::DialogTree;
use crate::entity::{build_decoration_entity, DecorationSpec, EntityDefinition, EntityGrounding};
use crate::io::text::{
    read_text_file_with_limit, too_large_io_error, DEFAULT_TEXT_FILE_SIZE_LIMIT,
};
use crate::palette::{load_palette_asset_from_path, Palette4};
use crate::scene::Scene;
use crate::ui_layout::UiLayoutAsset;
use crate::CoreError;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, serde::Deserialize, Default)]
struct LegacyTileMapObjectsWire {
    #[serde(default)]
    objects: Vec<LegacyMapObjectInstance>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LegacyMapObjectInstance {
    sheet: PathBuf,
    object_name: String,
    position: glam::UVec2,
    #[serde(default = "default_legacy_map_object_size_px")]
    size_px: glam::UVec2,
    #[serde(default, skip_serializing_if = "EntityGrounding::is_empty")]
    grounding: EntityGrounding,
    #[serde(default = "default_legacy_map_object_visible")]
    visible: bool,
    #[serde(default = "default_legacy_map_object_solid")]
    solid: bool,
}

fn default_legacy_map_object_size_px() -> glam::UVec2 {
    glam::UVec2::new(16, 16)
}

fn default_legacy_map_object_visible() -> bool {
    true
}

fn default_legacy_map_object_solid() -> bool {
    true
}

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
pub struct DiscoveredPaletteAsset {
    pub name: String,
    pub path: PathBuf,
    pub palette: Palette4,
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

pub fn dialog_file_path(project_path: &Path, dialog_name: &str) -> PathBuf {
    project_path
        .join("dialogs")
        .join(format!("{dialog_name}.json"))
}

pub fn ui_layout_file_path(project_path: &Path, layout_name: &str) -> PathBuf {
    project_path.join("ui").join(format!("{layout_name}.json"))
}

#[derive(Debug, serde::Deserialize, Default)]
struct ProjectSceneManifest {
    #[serde(default)]
    scenes: HashMap<String, String>,
}

fn load_project_scene_manifest(
    project_path: &Path,
) -> Result<ProjectSceneManifest, ProjectAssetError> {
    let project_file = project_path.join("project.toml");
    if !project_file.exists() {
        return Ok(ProjectSceneManifest::default());
    }

    let content = read_text_file_with_limit(
        &project_file,
        DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| {
            too_large_io_error(path, size_bytes, max_bytes, "project scene manifest")
        },
    )?;
    Ok(toml::from_str(&content).unwrap_or_default())
}

pub fn resolve_project_scene_path(project_path: &Path, scene_name: &str) -> Option<PathBuf> {
    let manifest = load_project_scene_manifest(project_path).ok()?;
    if let Some(relative_path) = manifest.scenes.get(scene_name) {
        let mapped = project_path.join(relative_path);
        if mapped.exists() {
            return Some(mapped);
        }
    }

    let canonical = scene_file_path(project_path, scene_name);
    canonical.exists().then_some(canonical)
}

pub fn discover_project_scene_paths(
    project_path: &Path,
) -> Result<Vec<(String, PathBuf)>, ProjectAssetError> {
    let manifest = load_project_scene_manifest(project_path)?;
    let mut scene_paths = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    let mut seen_paths = std::collections::HashSet::new();

    for (scene_name, relative_path) in manifest.scenes {
        let mapped = project_path.join(relative_path);
        let resolved = if mapped.exists() {
            Some(mapped)
        } else {
            let canonical = scene_file_path(project_path, &scene_name);
            canonical.exists().then_some(canonical)
        };

        if let Some(path) = resolved {
            seen_names.insert(scene_name.clone());
            seen_paths.insert(path.clone());
            scene_paths.push((scene_name, path));
        }
    }

    let scenes_dir = project_path.join("scenes");
    let mut discovered = find_json_files(&scenes_dir)?
        .into_iter()
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_string();
            if seen_names.contains(&name) || seen_paths.contains(&path) {
                None
            } else {
                Some((name, path))
            }
        })
        .collect::<Vec<_>>();
    scene_paths.append(&mut discovered);
    scene_paths.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(scene_paths)
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

pub fn discover_project_entity_definition_paths(
    project_path: &Path,
) -> Result<Vec<PathBuf>, ProjectAssetError> {
    find_json_files(&project_path.join("entities"))
}

pub fn discover_project_dialog_paths(
    project_path: &Path,
) -> Result<Vec<PathBuf>, ProjectAssetError> {
    find_json_files(&project_path.join("dialogs"))
}

pub fn discover_project_ui_paths(project_path: &Path) -> Result<Vec<PathBuf>, ProjectAssetError> {
    find_json_files(&project_path.join("ui"))
}

pub fn load_scene_from_path(path: &Path) -> Result<Scene, ProjectAssetError> {
    let json = read_text_file_with_limit(
        path,
        DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| too_large_io_error(path, size_bytes, max_bytes, "scene file"),
    )?;
    let mut scene = serde_json::from_str::<Scene>(&json).map_err(CoreError::from)?;
    migrate_legacy_tilemap_objects_into_scene(path, &mut scene)?;
    Ok(scene)
}

pub fn load_entity_definition_from_path(
    path: &Path,
) -> Result<EntityDefinition, ProjectAssetError> {
    let json = read_text_file_with_limit(
        path,
        DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| {
            too_large_io_error(path, size_bytes, max_bytes, "entity definition file")
        },
    )?;
    Ok(serde_json::from_str::<EntityDefinition>(&json).map_err(CoreError::from)?)
}

pub fn load_dialog_from_path(path: &Path) -> Result<DialogTree, ProjectAssetError> {
    let json = read_text_file_with_limit(
        path,
        DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| {
            too_large_io_error(path, size_bytes, max_bytes, "dialog file")
        },
    )?;
    Ok(serde_json::from_str::<DialogTree>(&json).map_err(CoreError::from)?)
}

pub fn load_ui_layout_from_path(path: &Path) -> Result<UiLayoutAsset, ProjectAssetError> {
    let json = read_text_file_with_limit(
        path,
        DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| {
            too_large_io_error(path, size_bytes, max_bytes, "ui layout file")
        },
    )?;
    Ok(serde_json::from_str::<UiLayoutAsset>(&json).map_err(CoreError::from)?)
}

pub fn save_dialog_to_path(path: &Path, dialog: &DialogTree) -> Result<(), ProjectAssetError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(dialog).map_err(CoreError::from)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn save_ui_layout_to_path(
    path: &Path,
    layout: &UiLayoutAsset,
) -> Result<(), ProjectAssetError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(layout).map_err(CoreError::from)?;
    fs::write(path, json)?;
    Ok(())
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

fn migrate_legacy_tilemap_objects_into_scene(
    scene_path: &Path,
    scene: &mut Scene,
) -> Result<(), ProjectAssetError> {
    let Some(project_root) = scene_path.parent().and_then(Path::parent) else {
        return Ok(());
    };

    let mut next_id = scene.next_entity_id();
    for map_name in scene.maps.clone() {
        let tilemap_path = tilemap_file_path(project_root, &map_name);
        let legacy_objects = load_legacy_tilemap_objects(&tilemap_path)?;
        for legacy in legacy_objects {
            let sheet = normalize_asset_name(&legacy.sheet.to_string_lossy()).to_string();
            if scene.entities().iter().any(|entity| {
                entity.entity_kind == crate::entity::EntityKind::Decoration
                    && entity.position == legacy.position.as_ivec2()
                    && entity.size == legacy.size_px
                    && entity.rendering.visible == legacy.visible
                    && entity.solid == legacy.solid
                    && entity.rendering.grounding == legacy.grounding
                    && entity
                        .rendering
                        .static_object_render
                        .as_ref()
                        .is_some_and(|render| {
                            render.sheet == sheet && render.object_name == legacy.object_name
                        })
            }) {
                continue;
            }
            let entity = build_decoration_entity(
                next_id,
                DecorationSpec {
                    position: legacy.position.as_ivec2(),
                    size: legacy.size_px,
                    sheet,
                    object_name: legacy.object_name,
                    grounding: legacy.grounding,
                    visible: legacy.visible,
                    solid: legacy.solid,
                },
            );
            scene.add_entity(entity);
            next_id += 1;
        }
    }

    Ok(())
}

fn load_legacy_tilemap_objects(
    path: &Path,
) -> Result<Vec<LegacyMapObjectInstance>, ProjectAssetError> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let json = read_text_file_with_limit(
        path,
        DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| {
            too_large_io_error(path, size_bytes, max_bytes, "tilemap file")
        },
    )?;
    Ok(serde_json::from_str::<LegacyTileMapObjectsWire>(&json)
        .map_err(CoreError::from)?
        .objects)
}

pub fn find_first_json_file(dir: &Path) -> Result<Option<PathBuf>, ProjectAssetError> {
    Ok(find_json_files(dir)?.into_iter().next())
}

pub fn classify_sprite_metadata_file(
    path: &Path,
) -> Result<SpriteMetadataFileKind, ProjectAssetError> {
    let json_data = read_text_file_with_limit(
        path,
        DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| {
            too_large_io_error(path, size_bytes, max_bytes, "sprite metadata file")
        },
    )?;

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

pub fn discover_palette_assets(
    dir: &Path,
) -> Result<Vec<DiscoveredPaletteAsset>, ProjectAssetError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut assets = Vec::new();
    for path in find_json_files(dir)? {
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let palette = load_palette_asset_from_path(&path)?;
        assets.push(DiscoveredPaletteAsset {
            name: name.into(),
            path,
            palette,
        });
    }
    assets.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));
    Ok(assets)
}

pub fn load_project_palettes(
    project_path: &Path,
) -> Result<BTreeMap<String, Palette4>, ProjectAssetError> {
    let palette_dir = project_path.join("palettes");
    let discovered = discover_palette_assets(&palette_dir)?;
    Ok(discovered
        .into_iter()
        .map(|asset| (asset.name, asset.palette))
        .collect())
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
