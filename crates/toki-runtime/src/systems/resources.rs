use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_render::RenderError;

/// Resource management system that handles loading and providing access to game assets.
///
/// Centralizes asset loading and provides clean APIs for accessing resources.
/// Future-ready for additional asset types like fonts, sounds, and shaders.
#[derive(Debug)]
pub struct ResourceManager {
    terrain_atlas: AtlasMeta,
    creature_atlas: AtlasMeta,
    tilemap: TileMap,
}

impl ResourceManager {
    /// Load all game resources from their respective files
    pub fn load_all() -> Result<Self, RenderError> {
        let terrain_atlas = AtlasMeta::load_from_file("assets/terrain.json")?;
        let creature_atlas = AtlasMeta::load_from_file("assets/creatures.json")?;
        // let tilemap = TileMap::load_from_file("assets/maps/tilemap_64x64_chunk.json")?;
        let tilemap = TileMap::load_from_file("assets/maps/new_town_map_64x64_crossings.json")?;
        // let tilemap = TileMap::load_from_file("assets/maps/my_new_map.json")?;

        // Validate the tilemap
        tilemap.validate()?;

        Ok(Self {
            terrain_atlas,
            creature_atlas,
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
        let creatures_atlas_path = first_existing_path(&[
            project_path
                .join("assets")
                .join("sprites")
                .join("creatures.json"),
            project_path.join("assets").join("creatures.json"),
        ])
        .ok_or_else(|| {
            RenderError::Other(format!(
                "Could not find creatures atlas in project '{}'",
                project_path.display()
            ))
        })?;

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

        let terrain_atlas = AtlasMeta::load_from_file(terrain_atlas_path)?;
        let creature_atlas = AtlasMeta::load_from_file(creatures_atlas_path)?;

        Ok(Self {
            terrain_atlas,
            creature_atlas,
            tilemap,
        })
    }

    /// Get reference to the terrain atlas
    pub fn get_terrain_atlas(&self) -> &AtlasMeta {
        &self.terrain_atlas
    }

    /// Get reference to the creature atlas
    pub fn get_creature_atlas(&self) -> &AtlasMeta {
        &self.creature_atlas
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
        self.creature_atlas.tile_size
    }

    /// Get terrain atlas image size for convenience
    pub fn terrain_image_size(&self) -> Option<glam::UVec2> {
        self.terrain_atlas.image_size()
    }

    /// Get creature atlas image size for convenience
    pub fn creature_image_size(&self) -> Option<glam::UVec2> {
        self.creature_atlas.image_size()
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
