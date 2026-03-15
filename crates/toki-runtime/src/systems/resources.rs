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
mod tests {
    use super::{
        classify_sprite_metadata_file, find_first_json_file, first_existing_path,
        resolve_project_resource_paths, resolve_tilemap_atlas_path, ResourceManager,
        SpriteMetadataFileKind,
    };
    use std::fs;
    use std::path::PathBuf;
    use toki_core::assets::tilemap::TileMap;

    fn make_unique_temp_dir() -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("toki_runtime_resources_tests_{nanos}"));
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
    fn first_existing_path_picks_first_existing_candidate() {
        let dir = make_unique_temp_dir();
        let missing = dir.join("missing.json");
        let first = dir.join("a.json");
        let second = dir.join("b.json");
        fs::write(&first, "{}").expect("first write");
        fs::write(&second, "{}").expect("second write");

        let resolved = first_existing_path(&[missing, first.clone(), second]);
        assert_eq!(resolved, Some(first));
    }

    #[test]
    fn find_first_json_file_returns_sorted_first_json_entry() {
        let dir = make_unique_temp_dir();
        fs::create_dir_all(&dir).expect("dir");
        fs::write(dir.join("z_map.json"), "{}").expect("z map");
        fs::write(dir.join("a_map.json"), "{}").expect("a map");
        fs::write(dir.join("note.txt"), "ignore").expect("txt");

        let first = find_first_json_file(&dir).expect("json file should be found");
        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some("a_map.json")
        );
    }

    #[test]
    fn resolve_tilemap_atlas_path_prefers_map_directory_relative_atlas() {
        let project_dir = make_unique_temp_dir();
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");
        let tilemap_path = tilemaps_dir.join("main_map.json");
        let atlas_path = tilemaps_dir.join("terrain.json");
        fs::write(&tilemap_path, "{}").expect("tilemap file");
        fs::write(&atlas_path, "{}").expect("atlas file");

        let tilemap = TileMap {
            size: glam::UVec2::new(1, 1),
            tile_size: glam::UVec2::new(16, 16),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec!["floor".to_string()],
            objects: vec![],
        };

        let resolved = resolve_tilemap_atlas_path(&project_dir, &tilemap_path, &tilemap)
            .expect("atlas should resolve");
        assert_eq!(resolved, atlas_path);
    }

    #[test]
    fn resolve_tilemap_atlas_path_falls_back_to_project_sprites_dir() {
        let project_dir = make_unique_temp_dir();
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        let sprites_dir = project_dir.join("assets").join("sprites");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        let tilemap_path = tilemaps_dir.join("main_map.json");
        let sprites_atlas = sprites_dir.join("terrain.json");
        fs::write(&tilemap_path, "{}").expect("tilemap file");
        fs::write(&sprites_atlas, "{}").expect("sprites atlas");

        let tilemap = TileMap {
            size: glam::UVec2::new(1, 1),
            tile_size: glam::UVec2::new(16, 16),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec!["floor".to_string()],
            objects: vec![],
        };

        let resolved = resolve_tilemap_atlas_path(&project_dir, &tilemap_path, &tilemap)
            .expect("atlas should resolve from sprites dir");
        assert_eq!(resolved, sprites_atlas);
    }

    #[test]
    fn load_for_project_with_named_map_loads_resources() {
        let project_dir = make_unique_temp_dir();
        let sprites_dir = project_dir.join("assets").join("sprites");
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

        write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
        write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
        write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");

        let manager = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
            .expect("project resources should load");
        assert_eq!(manager.tilemap_size(), glam::UVec2::new(1, 1));
        assert_eq!(manager.tilemap_tile_size(), glam::UVec2::new(16, 16));
        assert_eq!(manager.terrain_tile_size(), glam::UVec2::new(16, 16));
        assert_eq!(manager.creature_tile_size(), glam::UVec2::new(16, 16));
    }

    #[test]
    fn load_for_project_without_map_name_discovers_first_tilemap() {
        let project_dir = make_unique_temp_dir();
        let sprites_dir = project_dir.join("assets").join("sprites");
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

        write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
        write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
        write_minimal_map(&tilemaps_dir.join("b_map.json"), "terrain.json");
        let a_map = r#"{
  "size": [2, 1],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": ["floor", "floor"]
}"#;
        fs::write(tilemaps_dir.join("a_map.json"), a_map).expect("a_map write");

        let manager =
            ResourceManager::load_for_project(&project_dir, None).expect("resources should load");
        assert_eq!(
            manager.tilemap_size(),
            glam::UVec2::new(2, 1),
            "alphabetically first discovered map should be selected"
        );
    }

    #[test]
    fn load_for_project_errors_when_no_sprite_atlas_exists() {
        let project_dir = make_unique_temp_dir();
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");
        write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
        write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");

        let error = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
            .expect_err("missing sprite atlas should fail");
        assert!(
            error
                .to_string()
                .contains("Could not find any sprite atlas"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn load_for_project_registers_sprite_atlas_by_filename_and_stem() {
        let project_dir = make_unique_temp_dir();
        let sprites_dir = project_dir.join("assets").join("sprites");
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

        write_minimal_atlas(&sprites_dir.join("players.json"), "player.png");
        write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
        write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
        fs::write(sprites_dir.join("player.png"), "png").expect("player image");

        let manager = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
            .expect("project resources should load");

        assert!(manager.get_sprite_atlas("players.json").is_some());
        assert!(manager.get_sprite_atlas("players").is_some());
        assert_eq!(
            manager.get_sprite_texture_path("players.json"),
            Some(&sprites_dir.join("player.png"))
        );
    }

    #[test]
    fn resolve_project_resource_paths_returns_expected_texture_paths() {
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

        let resolved = resolve_project_resource_paths(&project_dir, Some("demo_map"))
            .expect("project resource paths should resolve");
        assert_eq!(
            resolved.tilemap_texture_path,
            Some(tilemaps_dir.join("terrain.png"))
        );
        assert_eq!(
            resolved.sprite_texture_path,
            Some(sprites_dir.join("creatures.png"))
        );
    }

    #[test]
    fn classify_sprite_metadata_file_distinguishes_object_sheets_from_atlases() {
        let project_dir = make_unique_temp_dir();
        let sprites_dir = project_dir.join("assets").join("sprites");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");

        let atlas_path = sprites_dir.join("creatures.json");
        write_minimal_atlas(&atlas_path, "creatures.png");
        let object_sheet_path = sprites_dir.join("fauna.json");
        fs::write(
            &object_sheet_path,
            r#"{
  "sheet_type": "objects",
  "image": "fauna.png",
  "tile_size": [16, 16],
  "objects": {
    "fauna_a": {
      "position": [0, 0],
      "size_tiles": [1, 1]
    }
  }
}"#,
        )
        .expect("object sheet should be written");

        assert_eq!(
            classify_sprite_metadata_file(&atlas_path).expect("atlas should classify"),
            SpriteMetadataFileKind::Atlas
        );
        assert_eq!(
            classify_sprite_metadata_file(&object_sheet_path)
                .expect("object sheet should classify"),
            SpriteMetadataFileKind::ObjectSheet
        );
    }

    #[test]
    fn resolve_project_resource_paths_ignores_object_sheets_in_sprite_registry() {
        let project_dir = make_unique_temp_dir();
        let sprites_dir = project_dir.join("assets").join("sprites");
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

        write_minimal_atlas(&sprites_dir.join("creatures.json"), "creatures.png");
        fs::write(
            sprites_dir.join("fauna.json"),
            r#"{
  "sheet_type": "objects",
  "image": "fauna.png",
  "tile_size": [16, 16],
  "objects": {
    "fauna_a": {
      "position": [0, 0],
      "size_tiles": [1, 1]
    }
  }
}"#,
        )
        .expect("object sheet should be written");
        write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
        write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
        fs::write(sprites_dir.join("creatures.png"), "png").expect("creatures image");
        fs::write(tilemaps_dir.join("terrain.png"), "png").expect("terrain image");

        let resolved = resolve_project_resource_paths(&project_dir, Some("demo_map"))
            .expect("project resource paths should resolve");

        assert_eq!(resolved.sprite_atlas_paths.len(), 1);
        assert_eq!(
            resolved.sprite_atlas_paths[0]
                .file_name()
                .and_then(|name| name.to_str()),
            Some("creatures.json")
        );
    }

    #[test]
    fn load_for_project_registers_object_sheet_by_filename_and_stem() {
        let project_dir = make_unique_temp_dir();
        let sprites_dir = project_dir.join("assets").join("sprites");
        let tilemaps_dir = project_dir.join("assets").join("tilemaps");
        fs::create_dir_all(&sprites_dir).expect("sprites dir");
        fs::create_dir_all(&tilemaps_dir).expect("tilemaps dir");

        write_minimal_atlas(&sprites_dir.join("players.json"), "player.png");
        fs::write(
            sprites_dir.join("fauna.json"),
            r#"{
  "sheet_type": "objects",
  "image": "fauna.png",
  "tile_size": [16, 16],
  "objects": {
    "fauna_a": {
      "position": [0, 0],
      "size_tiles": [1, 1]
    }
  }
}"#,
        )
        .expect("object sheet should be written");
        write_minimal_atlas(&tilemaps_dir.join("terrain.json"), "terrain.png");
        write_minimal_map(&tilemaps_dir.join("demo_map.json"), "terrain.json");
        fs::write(sprites_dir.join("player.png"), "png").expect("player image");
        fs::write(sprites_dir.join("fauna.png"), "png").expect("fauna image");

        let manager = ResourceManager::load_for_project(&project_dir, Some("demo_map"))
            .expect("project resources should load");

        assert!(manager.get_object_sheet("fauna.json").is_some());
        assert!(manager.get_object_sheet("fauna").is_some());
        assert_eq!(
            manager.get_object_texture_path("fauna.json"),
            Some(&sprites_dir.join("fauna.png"))
        );
    }
}
