use crate::assets::atlas::{AtlasMeta, ColorMode};
use crate::assets::autotile;
use crate::assets::tile_animation::TileAnimationClock;
use crate::graphics::vertex::QuadVertex;
use crate::io::text::{
    read_text_file_with_limit, too_large_io_error, DEFAULT_TEXT_FILE_SIZE_LIMIT,
};
use crate::project_assets::normalize_asset_name;
use crate::CoreError;
use glam::UVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileSetEntryKind {
    #[serde(rename = "tile")]
    Tile,
    #[serde(rename = "autotile")]
    AutoTileGroup,
    #[serde(rename = "animated")]
    AnimatedTile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileSetEntry {
    pub atlas_name: String,
    pub kind: TileSetEntryKind,
    pub source_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileSetMeta {
    pub tile_size: UVec2,
    pub entries: HashMap<String, TileSetEntry>,
}

#[derive(Debug, Clone)]
pub struct TileSetAtlasSource {
    pub name: String,
    pub path: PathBuf,
    pub meta: AtlasMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TileRenderMaterial {
    TrueColor,
    PaletteIndexed { palette_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TilemapBatchKey {
    pub atlas_name: String,
    #[allow(dead_code)]
    pub above_entities: bool,
    pub material: TileRenderMaterial,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TilemapRenderBatch {
    pub key: TilemapBatchKey,
    pub atlas_path: PathBuf,
    pub texture_path: PathBuf,
    pub vertices: Vec<QuadVertex>,
}

#[derive(Debug, Clone)]
pub struct ResolvedTileCell<'a> {
    pub entry_id: String,
    pub entry: &'a TileSetEntry,
    pub atlas_source: &'a TileSetAtlasSource,
    pub concrete_tile_name: String,
}

#[derive(Debug)]
pub struct TileSetResolver<'a> {
    tileset: &'a TileSetMeta,
    atlases: &'a HashMap<String, TileSetAtlasSource>,
}

impl TileSetEntryKind {
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::Tile => "tile",
            Self::AutoTileGroup => "autotile",
            Self::AnimatedTile => "animated",
        }
    }
}

impl TileSetMeta {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = read_text_file_with_limit(
            path.as_ref(),
            DEFAULT_TEXT_FILE_SIZE_LIMIT,
            |path, size_bytes, max_bytes| {
                too_large_io_error(path, size_bytes, max_bytes, "tileset file")
            },
        )?;
        let meta = serde_json::from_str::<TileSetMeta>(&content)?;
        Ok(meta)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), CoreError> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn get_entry(&self, entry_id: &str) -> Option<&TileSetEntry> {
        self.entries.get(entry_id)
    }

    pub fn build_entry_id(
        atlas_name: &str,
        kind: TileSetEntryKind,
        source_name: &str,
    ) -> String {
        format!(
            "{}/{}/{}",
            normalize_asset_name(atlas_name),
            kind.path_segment(),
            source_name
        )
    }

    pub fn from_atlas(atlas_name: &str, atlas: &AtlasMeta) -> Self {
        let mut entries = HashMap::new();

        let mut tile_names = atlas.tiles.keys().cloned().collect::<Vec<_>>();
        tile_names.sort();
        for tile_name in tile_names {
            let entry_id = Self::build_entry_id(atlas_name, TileSetEntryKind::Tile, &tile_name);
            entries.insert(
                entry_id,
                TileSetEntry {
                    atlas_name: atlas_name.to_string(),
                    kind: TileSetEntryKind::Tile,
                    source_name: tile_name,
                    display_name: None,
                },
            );
        }

        let mut auto_tile_names = atlas.auto_tile_groups.keys().cloned().collect::<Vec<_>>();
        auto_tile_names.sort();
        for group_name in auto_tile_names {
            let entry_id =
                Self::build_entry_id(atlas_name, TileSetEntryKind::AutoTileGroup, &group_name);
            entries.insert(
                entry_id,
                TileSetEntry {
                    atlas_name: atlas_name.to_string(),
                    kind: TileSetEntryKind::AutoTileGroup,
                    source_name: group_name,
                    display_name: None,
                },
            );
        }

        let mut animated_names = atlas.animated_tiles.keys().cloned().collect::<Vec<_>>();
        animated_names.sort();
        for animated_name in animated_names {
            let entry_id =
                Self::build_entry_id(atlas_name, TileSetEntryKind::AnimatedTile, &animated_name);
            entries.insert(
                entry_id,
                TileSetEntry {
                    atlas_name: atlas_name.to_string(),
                    kind: TileSetEntryKind::AnimatedTile,
                    source_name: animated_name,
                    display_name: None,
                },
            );
        }

        Self {
            tile_size: atlas.tile_size,
            entries,
        }
    }

    pub fn from_legacy_atlas(atlas_name: &str, atlas: &AtlasMeta) -> Self {
        let mut entries = HashMap::new();

        let mut tile_names = atlas.tiles.keys().cloned().collect::<Vec<_>>();
        tile_names.sort();
        for tile_name in tile_names {
            entries.insert(
                tile_name.clone(),
                TileSetEntry {
                    atlas_name: atlas_name.to_string(),
                    kind: TileSetEntryKind::Tile,
                    source_name: tile_name,
                    display_name: None,
                },
            );
        }

        let mut auto_tile_names = atlas.auto_tile_groups.keys().cloned().collect::<Vec<_>>();
        auto_tile_names.sort();
        for group_name in auto_tile_names {
            entries.insert(
                group_name.clone(),
                TileSetEntry {
                    atlas_name: atlas_name.to_string(),
                    kind: TileSetEntryKind::AutoTileGroup,
                    source_name: group_name,
                    display_name: None,
                },
            );
        }

        let mut animated_names = atlas.animated_tiles.keys().cloned().collect::<Vec<_>>();
        animated_names.sort();
        for animated_name in animated_names {
            entries.insert(
                animated_name.clone(),
                TileSetEntry {
                    atlas_name: atlas_name.to_string(),
                    kind: TileSetEntryKind::AnimatedTile,
                    source_name: animated_name,
                    display_name: None,
                },
            );
        }

        Self {
            tile_size: atlas.tile_size,
            entries,
        }
    }

    pub fn first_tile_entry_id(&self) -> Option<String> {
        let mut entry_ids = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.kind == TileSetEntryKind::Tile)
            .map(|(entry_id, _)| entry_id.clone())
            .collect::<Vec<_>>();
        entry_ids.sort();
        entry_ids.into_iter().next()
    }
}

impl TileSetAtlasSource {
    pub fn texture_path(&self) -> PathBuf {
        self.path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&self.meta.image)
    }
}

impl<'a> TileSetResolver<'a> {
    pub fn new(tileset: &'a TileSetMeta, atlases: &'a HashMap<String, TileSetAtlasSource>) -> Self {
        Self { tileset, atlases }
    }

    pub fn tileset(&self) -> &TileSetMeta {
        self.tileset
    }

    pub fn atlas_source(&self, atlas_name: &str) -> Option<&'a TileSetAtlasSource> {
        self.atlases
            .get(atlas_name)
            .or_else(|| self.atlases.get(normalize_asset_name(atlas_name)))
    }

    pub fn resolved_palette_id(
        atlas: &AtlasMeta,
        indexed_palette_override: Option<&str>,
    ) -> Option<String> {
        if atlas.color_mode != ColorMode::PaletteIndexed {
            return None;
        }

        Some(
            indexed_palette_override
                .or(atlas.palette.as_deref())
                .unwrap_or("gb_default")
                .to_string(),
        )
    }

    fn resolve_entry(
        &self,
        entry_id: &str,
    ) -> Result<(&'a TileSetEntry, &'a TileSetAtlasSource), CoreError> {
        let entry: &'a TileSetEntry = self
            .tileset
            .entries
            .get(entry_id)
            .ok_or_else(|| CoreError::MissingTileInTileSet {
                entry_id: entry_id.to_string(),
            })?;
        let atlas_source: &'a TileSetAtlasSource = self
            .atlas_source(&entry.atlas_name)
            .ok_or_else(|| CoreError::MissingAtlasInTileSet {
                entry_id: entry_id.to_string(),
                atlas_name: entry.atlas_name.clone(),
            })?;
        Ok((entry, atlas_source))
    }

    fn resolve_auto_tile_name(
        entry_id: &str,
        source_name: &str,
        x: u32,
        y: u32,
        tiles: &[String],
        size: UVec2,
        atlas: &AtlasMeta,
    ) -> Result<String, CoreError> {
        let Some(group) = atlas.get_auto_tile_group(source_name) else {
            return Err(CoreError::MissingTileSourceInTileSet {
                entry_id: entry_id.to_string(),
                atlas_name: String::new(),
                source_name: source_name.to_string(),
            });
        };

        let ix = x as i32;
        let iy = y as i32;
        let is_same_entry = |tx: i32, ty: i32| -> bool {
            if tx < 0 || ty < 0 || tx >= size.x as i32 || ty >= size.y as i32 {
                return false;
            }
            let index = (ty as u32 * size.x + tx as u32) as usize;
            tiles.get(index).is_some_and(|tile| tile == entry_id)
        };

        let n = is_same_entry(ix, iy - 1);
        let e = is_same_entry(ix + 1, iy);
        let s = is_same_entry(ix, iy + 1);
        let w = is_same_entry(ix - 1, iy);
        let mask = match group.mode {
            autotile::AutoTileMode::FourBit => autotile::compute_4bit_mask(n, e, s, w),
            autotile::AutoTileMode::EightBit => {
                let ne = is_same_entry(ix + 1, iy - 1);
                let se = is_same_entry(ix + 1, iy + 1);
                let sw = is_same_entry(ix - 1, iy + 1);
                let nw = is_same_entry(ix - 1, iy - 1);
                autotile::compute_8bit_mask(&autotile::FullNeighbors {
                    n,
                    ne,
                    e,
                    se,
                    s,
                    sw,
                    w,
                    nw,
                })
            }
        };

        group
            .resolve_variant(mask)
            .or(group.preview_tile.as_deref())
            .map(ToOwned::to_owned)
            .ok_or_else(|| CoreError::MissingTileSourceInTileSet {
                entry_id: entry_id.to_string(),
                atlas_name: String::new(),
                source_name: source_name.to_string(),
            })
    }

    pub fn resolve_cell_on_layer(
        &self,
        tilemap: &crate::assets::tilemap::TileMap,
        tiles: &[String],
        x: u32,
        y: u32,
        tile_anim: Option<&TileAnimationClock>,
    ) -> Result<ResolvedTileCell<'a>, CoreError> {
        let entry_id = tilemap.tile_name_in(tiles, x, y)?;
        if entry_id.is_empty() {
            return Err(CoreError::MissingTileInTileSet {
                entry_id: entry_id.to_string(),
            });
        }

        let (entry, atlas_source) = self.resolve_entry(entry_id)?;
        let atlas = &atlas_source.meta;
        let concrete_tile_name = match entry.kind {
            TileSetEntryKind::Tile => entry.source_name.clone(),
            TileSetEntryKind::AutoTileGroup => Self::resolve_auto_tile_name(
                entry_id,
                &entry.source_name,
                x,
                y,
                tiles,
                tilemap.size,
                atlas,
            )
            .map_err(|_| CoreError::MissingTileSourceInTileSet {
                entry_id: entry_id.to_string(),
                atlas_name: entry.atlas_name.clone(),
                source_name: entry.source_name.clone(),
            })?,
            TileSetEntryKind::AnimatedTile => entry.source_name.clone(),
        };

        let concrete_tile_name = if let Some(clock) = tile_anim {
            clock.current_frame_tile(&concrete_tile_name, atlas)
        } else {
            None
        }
        .unwrap_or(concrete_tile_name.as_str())
        .to_string();

        if !atlas.tiles.contains_key(&concrete_tile_name) {
            return Err(CoreError::MissingTileSourceInTileSet {
                entry_id: entry_id.to_string(),
                atlas_name: entry.atlas_name.clone(),
                source_name: concrete_tile_name,
            });
        }

        Ok(ResolvedTileCell {
            entry_id: entry_id.to_string(),
            entry,
            atlas_source,
            concrete_tile_name,
        })
    }

    pub fn is_tile_solid_at(
        &self,
        tilemap: &crate::assets::tilemap::TileMap,
        tile_x: u32,
        tile_y: u32,
    ) -> Result<bool, CoreError> {
        if tile_x >= tilemap.size.x || tile_y >= tilemap.size.y {
            return Err(CoreError::TileOutOfBounds {
                x: tile_x,
                y: tile_y,
                map_width: tilemap.size.x,
                map_height: tilemap.size.y,
            });
        }
        let index = (tile_y * tilemap.size.x + tile_x) as usize;
        let index_u32 = index as u32;
        for layer in &tilemap.layers {
            if !layer.collision_enabled {
                continue;
            }
            if let Some(&override_solid) = layer.collision_overrides.get(&index_u32) {
                if override_solid {
                    return Ok(true);
                }
                continue;
            }
            let Some(entry_id) = layer.tiles.get(index) else {
                continue;
            };
            if entry_id.is_empty() {
                continue;
            }
            let resolved = self.resolve_cell_on_layer(tilemap, &layer.tiles, tile_x, tile_y, None)?;
            if resolved
                .atlas_source
                .meta
                .is_tile_solid(&resolved.concrete_tile_name)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn is_tile_trigger_at(
        &self,
        tilemap: &crate::assets::tilemap::TileMap,
        tile_x: u32,
        tile_y: u32,
    ) -> Result<bool, CoreError> {
        if tile_x >= tilemap.size.x || tile_y >= tilemap.size.y {
            return Err(CoreError::TileOutOfBounds {
                x: tile_x,
                y: tile_y,
                map_width: tilemap.size.x,
                map_height: tilemap.size.y,
            });
        }

        let index = (tile_y * tilemap.size.x + tile_x) as usize;
        for layer in &tilemap.layers {
            let Some(entry_id) = layer.tiles.get(index) else {
                continue;
            };
            if entry_id.is_empty() {
                continue;
            }
            let resolved = self.resolve_cell_on_layer(tilemap, &layer.tiles, tile_x, tile_y, None)?;
            if resolved
                .atlas_source
                .meta
                .is_tile_trigger(&resolved.concrete_tile_name)
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn generate_render_batches_inner(
        &self,
        tilemap: &crate::assets::tilemap::TileMap,
        visible_chunks: Option<&[(u32, u32)]>,
        tile_anim: Option<&TileAnimationClock>,
        indexed_palette_override: Option<&str>,
        skip_invalid_cells: bool,
    ) -> Result<Vec<TilemapRenderBatch>, CoreError> {
        let mut batches = Vec::<TilemapRenderBatch>::new();
        let chunk_set = visible_chunks.map(|chunks| chunks.to_vec());

        for layer in &tilemap.layers {
            if !layer.visible {
                continue;
            }
            for y in 0..tilemap.size.y {
                for x in 0..tilemap.size.x {
                    if let Some(chunks) = &chunk_set {
                        let chunk = (
                            x / crate::assets::tilemap::CHUNK_SIZE,
                            y / crate::assets::tilemap::CHUNK_SIZE,
                        );
                        if !chunks.contains(&chunk) {
                            continue;
                        }
                    }

                    let entry_id = match tilemap.tile_name_in(&layer.tiles, x, y) {
                        Ok(entry_id) => entry_id,
                        Err(err) => {
                            if skip_invalid_cells {
                                continue;
                            }
                            return Err(err);
                        }
                    };
                    if entry_id.is_empty() {
                        continue;
                    }

                    let resolved =
                        match self.resolve_cell_on_layer(tilemap, &layer.tiles, x, y, tile_anim) {
                            Ok(resolved) => resolved,
                            Err(err) => {
                                if skip_invalid_cells {
                                    continue;
                                }
                                return Err(err);
                            }
                        };
                    let atlas = &resolved.atlas_source.meta;
                    let texture_size = atlas.image_size().unwrap_or(UVec2::new(64, 16));
                    let uvs = match atlas.get_tile_uvs(&resolved.concrete_tile_name, texture_size) {
                        Some(uvs) => uvs,
                        None => {
                            if skip_invalid_cells {
                                continue;
                            }
                            return Err(CoreError::MissingTileSourceInTileSet {
                                entry_id: entry_id.to_string(),
                                atlas_name: resolved.entry.atlas_name.clone(),
                                source_name: resolved.concrete_tile_name.clone(),
                            });
                        }
                    };

                    let world_x = x as f32 * tilemap.tile_size.x as f32;
                    let world_y = y as f32 * tilemap.tile_size.y as f32;
                    let color_material = match Self::resolved_palette_id(atlas, indexed_palette_override) {
                        Some(palette_id) => TileRenderMaterial::PaletteIndexed { palette_id },
                        None => TileRenderMaterial::TrueColor,
                    };
                    let key = TilemapBatchKey {
                        atlas_name: resolved.entry.atlas_name.clone(),
                        above_entities: layer.above_entities,
                        material: color_material,
                    };

                    let batch_index = batches.iter().position(|batch| batch.key == key);
                    let batch = if let Some(index) = batch_index {
                        &mut batches[index]
                    } else {
                        batches.push(TilemapRenderBatch {
                            key,
                            atlas_path: resolved.atlas_source.path.clone(),
                            texture_path: resolved.atlas_source.texture_path(),
                            vertices: Vec::new(),
                        });
                        batches.last_mut().expect("batch inserted")
                    };

                    batch.vertices.extend_from_slice(&QuadVertex::quad(
                        [world_x, world_y],
                        [tilemap.tile_size.x as f32, tilemap.tile_size.y as f32],
                        uvs,
                    ));
                }
            }
        }

        Ok(batches)
    }

    pub fn generate_render_batches(
        &self,
        tilemap: &crate::assets::tilemap::TileMap,
        visible_chunks: Option<&[(u32, u32)]>,
        tile_anim: Option<&TileAnimationClock>,
        indexed_palette_override: Option<&str>,
    ) -> Result<Vec<TilemapRenderBatch>, CoreError> {
        self.generate_render_batches_inner(
            tilemap,
            visible_chunks,
            tile_anim,
            indexed_palette_override,
            false,
        )
    }

    pub(crate) fn generate_render_batches_best_effort(
        &self,
        tilemap: &crate::assets::tilemap::TileMap,
        visible_chunks: Option<&[(u32, u32)]>,
        tile_anim: Option<&TileAnimationClock>,
        indexed_palette_override: Option<&str>,
    ) -> Vec<TilemapRenderBatch> {
        self.generate_render_batches_inner(
            tilemap,
            visible_chunks,
            tile_anim,
            indexed_palette_override,
            true,
        )
        .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::atlas::{TileInfo, TileProperties};
    use crate::assets::autotile::{AutoTileGroup, AutoTileMode};
    use crate::assets::tilemap::{TileLayer, TileMap};

    fn source(name: &str, atlas: AtlasMeta) -> TileSetAtlasSource {
        TileSetAtlasSource {
            name: name.to_string(),
            path: PathBuf::from(format!("assets/sprites/{name}.json")),
            meta: atlas,
        }
    }

    fn atlas() -> AtlasMeta {
        let mut tiles = HashMap::new();
        tiles.insert(
            "grass".to_string(),
            TileInfo {
                position: UVec2::ZERO,
                properties: TileProperties::default(),
            },
        );
        tiles.insert(
            "terrain_0".to_string(),
            TileInfo {
                position: UVec2::new(1, 0),
                properties: TileProperties::default(),
            },
        );
        tiles.insert(
            "terrain_15".to_string(),
            TileInfo {
                position: UVec2::new(2, 0),
                properties: TileProperties::default(),
            },
        );
        let mut auto_tile_groups = HashMap::new();
        auto_tile_groups.insert(
            "terrain".to_string(),
            AutoTileGroup {
                mode: AutoTileMode::FourBit,
                variants: HashMap::from([(0, "terrain_0".to_string()), (15, "terrain_15".to_string())]),
                preview_tile: Some("terrain_15".to_string()),
            },
        );
        AtlasMeta {
            image: PathBuf::from("terrain.png"),
            tile_size: UVec2::new(16, 16),
            color_mode: ColorMode::TrueColor,
            palette: None,
            palette_size: None,
            tiles,
            auto_tile_groups,
            animated_tiles: HashMap::new(),
            imported_auto_tiles: Vec::new(),
        }
    }

    #[test]
    fn from_atlas_creates_stable_entry_ids() {
        let tileset = TileSetMeta::from_atlas("terrain", &atlas());
        assert!(tileset.entries.contains_key("terrain/tile/grass"));
        assert!(tileset.entries.contains_key("terrain/autotile/terrain"));
    }

    #[test]
    fn resolver_uses_logical_entry_ids_for_autotiling() {
        let tileset = TileSetMeta {
            tile_size: UVec2::new(16, 16),
            entries: HashMap::from([(
                "terrain/autotile/terrain".to_string(),
                TileSetEntry {
                    atlas_name: "terrain".to_string(),
                    kind: TileSetEntryKind::AutoTileGroup,
                    source_name: "terrain".to_string(),
                    display_name: None,
                },
            )]),
        };
        let atlas_name = "terrain".to_string();
        let atlases = HashMap::from([(atlas_name.clone(), source(&atlas_name, atlas()))]);
        let resolver = TileSetResolver::new(&tileset, &atlases);
        let tilemap = TileMap {
            size: UVec2::new(2, 1),
            tile_size: UVec2::new(16, 16),
            tileset: PathBuf::from("terrain.json"),
            layers: vec![TileLayer::new(
                "ground",
                vec![
                    "terrain/autotile/terrain".to_string(),
                    "terrain/autotile/terrain".to_string(),
                ],
            )],
        };

        let left = resolver
            .resolve_cell_on_layer(&tilemap, &tilemap.layers[0].tiles, 0, 0, None)
            .expect("resolve left");
        assert_eq!(left.concrete_tile_name, "terrain_15");
    }
}
