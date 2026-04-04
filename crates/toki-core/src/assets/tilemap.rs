use crate::assets::atlas::AtlasMeta;
use crate::assets::autotile;
use crate::assets::tile_animation::TileAnimationClock;
use crate::graphics::vertex::QuadVertex;
use crate::io::text::{
    read_text_file_with_limit, too_large_io_error, DEFAULT_TEXT_FILE_SIZE_LIMIT,
};
use crate::CoreError;
use glam::UVec2;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;

pub const CHUNK_SIZE: u32 = 16; //16x16 tiles per chunk

/// Tilemap vertices split by render order relative to entities.
#[derive(Debug, Default)]
pub struct SplitTilemapVertices {
    pub below: Vec<QuadVertex>,
    pub above: Vec<QuadVertex>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileLayer {
    pub name: String,
    pub tiles: Vec<String>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub collision_enabled: bool,
    #[serde(default)]
    pub above_entities: bool,
}

fn default_true() -> bool {
    true
}

impl TileLayer {
    pub fn new(name: impl Into<String>, tiles: Vec<String>) -> Self {
        Self {
            name: name.into(),
            tiles,
            visible: true,
            collision_enabled: true,
            above_entities: false,
        }
    }

    pub fn new_empty(name: impl Into<String>, tile_count: usize) -> Self {
        Self {
            name: name.into(),
            tiles: vec![String::new(); tile_count],
            visible: true,
            collision_enabled: false,
            above_entities: false,
        }
    }
}

/// Wire format that accepts both old (flat `tiles`) and new (`layers`) JSON.
#[derive(Deserialize)]
struct TileMapWire {
    size: UVec2,
    tile_size: UVec2,
    atlas: PathBuf,
    #[serde(default)]
    tiles: Option<Vec<String>>,
    #[serde(default)]
    layers: Option<Vec<TileLayer>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileMap {
    pub size: UVec2,
    pub tile_size: UVec2,
    pub atlas: PathBuf,
    pub layers: Vec<TileLayer>,
}

impl Serialize for TileMap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TileMap", 4)?;
        state.serialize_field("size", &self.size)?;
        state.serialize_field("tile_size", &self.tile_size)?;
        state.serialize_field("atlas", &self.atlas)?;
        state.serialize_field("layers", &self.layers)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TileMap {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = TileMapWire::deserialize(deserializer)?;
        let layers = match (wire.layers, wire.tiles) {
            (Some(layers), _) => layers,
            (None, Some(tiles)) => vec![TileLayer::new("ground", tiles)],
            (None, None) => {
                return Err(serde::de::Error::custom(
                    "tilemap must have either 'layers' or 'tiles'",
                ));
            }
        };
        Ok(TileMap {
            size: wire.size,
            tile_size: wire.tile_size,
            atlas: wire.atlas,
            layers,
        })
    }
}

fn is_same_group(tiles: &[String], size: UVec2, x: i32, y: i32, group_name: &str) -> bool {
    if x < 0 || y < 0 || x >= size.x as i32 || y >= size.y as i32 {
        return false;
    }
    let index = (y as u32 * size.x + x as u32) as usize;
    tiles.get(index).is_some_and(|t| t == group_name)
}

fn resolve_auto_tile<'a>(
    tile_name: &'a str,
    x: u32,
    y: u32,
    tiles: &[String],
    size: UVec2,
    atlas: &'a AtlasMeta,
) -> &'a str {
    let Some(group) = atlas.get_auto_tile_group(tile_name) else {
        return tile_name;
    };
    let ix = x as i32;
    let iy = y as i32;
    let n = is_same_group(tiles, size, ix, iy - 1, tile_name);
    let e = is_same_group(tiles, size, ix + 1, iy, tile_name);
    let s = is_same_group(tiles, size, ix, iy + 1, tile_name);
    let w = is_same_group(tiles, size, ix - 1, iy, tile_name);
    let mask = match group.mode {
        autotile::AutoTileMode::FourBit => autotile::compute_4bit_mask(n, e, s, w),
        autotile::AutoTileMode::EightBit => {
            let ne = is_same_group(tiles, size, ix + 1, iy - 1, tile_name);
            let se = is_same_group(tiles, size, ix + 1, iy + 1, tile_name);
            let sw = is_same_group(tiles, size, ix - 1, iy + 1, tile_name);
            let nw = is_same_group(tiles, size, ix - 1, iy - 1, tile_name);
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
        .unwrap_or(tile_name)
}

fn resolve_tile_name<'a>(
    tile_name: &'a str,
    x: u32,
    y: u32,
    tiles: &[String],
    size: UVec2,
    atlas: &'a AtlasMeta,
    tile_anim: Option<&TileAnimationClock>,
) -> &'a str {
    let resolved = resolve_auto_tile(tile_name, x, y, tiles, size, atlas);
    if let Some(clock) = tile_anim {
        if let Some(frame_tile) = clock.current_frame_tile(resolved, atlas) {
            return frame_tile;
        }
    }
    resolved
}

impl TileMap {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = read_text_file_with_limit(
            path.as_ref(),
            DEFAULT_TEXT_FILE_SIZE_LIMIT,
            |path, size_bytes, max_bytes| {
                too_large_io_error(path, size_bytes, max_bytes, "tilemap file")
            },
        )?;
        let map = serde_json::from_str::<TileMap>(&content)?;
        Ok(map)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.layers.is_empty() {
            return Err(CoreError::InvalidMapSize {
                expected: 1,
                actual: 0,
            });
        }
        let expected_len = (self.size.x * self.size.y) as usize;
        for layer in &self.layers {
            if layer.tiles.len() != expected_len {
                return Err(CoreError::InvalidMapSize {
                    expected: expected_len,
                    actual: layer.tiles.len(),
                });
            }
        }
        Ok(())
    }

    /// Returns the tile name at (x, y) on the first layer. Convenience for
    /// backward-compatible call sites (collision, transitions).
    pub fn get_tile_name(&self, x: u32, y: u32) -> Result<&str, CoreError> {
        self.get_tile_name_on_layer(0, x, y)
    }

    pub fn get_tile_name_on_layer(&self, layer: usize, x: u32, y: u32) -> Result<&str, CoreError> {
        if x >= self.size.x || y >= self.size.y {
            return Err(CoreError::TileOutOfBounds {
                x,
                y,
                map_width: self.size.x,
                map_height: self.size.y,
            });
        }
        let tiles = self.layer_tiles(layer)?;
        let index = (y * self.size.x + x) as usize;
        tiles
            .get(index)
            .map(String::as_str)
            .ok_or(CoreError::InvalidMapSize {
                expected: (self.size.x * self.size.y) as usize,
                actual: tiles.len(),
            })
    }

    /// Returns the tile name at (x, y) within the given tile slice.
    pub fn tile_name_in<'a>(
        &self,
        tiles: &'a [String],
        x: u32,
        y: u32,
    ) -> Result<&'a str, CoreError> {
        if x >= self.size.x || y >= self.size.y {
            return Err(CoreError::TileOutOfBounds {
                x,
                y,
                map_width: self.size.x,
                map_height: self.size.y,
            });
        }
        let index = (y * self.size.x + x) as usize;
        tiles
            .get(index)
            .map(String::as_str)
            .ok_or(CoreError::InvalidMapSize {
                expected: (self.size.x * self.size.y) as usize,
                actual: tiles.len(),
            })
    }

    fn layer_tiles(&self, layer: usize) -> Result<&[String], CoreError> {
        self.layers
            .get(layer)
            .map(|l| l.tiles.as_slice())
            .ok_or(CoreError::InvalidMapSize {
                expected: self.layers.len(),
                actual: layer,
            })
    }

    /// Convenience: returns first layer tiles for call sites that need direct access.
    pub fn tiles(&self) -> &[String] {
        self.layers
            .first()
            .map(|l| l.tiles.as_slice())
            .unwrap_or(&[])
    }

    /// Mutable access to first layer tiles.
    pub fn tiles_mut(&mut self) -> &mut Vec<String> {
        &mut self.layers[0].tiles
    }

    /// Mutable access to tiles on a specific layer.
    pub fn layer_tiles_mut(&mut self, layer: usize) -> Option<&mut Vec<String>> {
        self.layers.get_mut(layer).map(|l| &mut l.tiles)
    }

    pub fn is_tile_solid_at(
        &self,
        atlas: &AtlasMeta,
        tile_x: u32,
        tile_y: u32,
    ) -> Result<bool, CoreError> {
        if tile_x >= self.size.x || tile_y >= self.size.y {
            return Err(CoreError::TileOutOfBounds {
                x: tile_x,
                y: tile_y,
                map_width: self.size.x,
                map_height: self.size.y,
            });
        }
        let index = (tile_y * self.size.x + tile_x) as usize;
        for layer in &self.layers {
            if !layer.collision_enabled {
                continue;
            }
            let Some(tile_name) = layer.tiles.get(index) else {
                continue;
            };
            if atlas.is_tile_solid(tile_name) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn is_world_position_solid(
        &self,
        atlas: &AtlasMeta,
        world_pos: glam::UVec2,
    ) -> Result<bool, CoreError> {
        let tile_x = world_pos.x / self.tile_size.x;
        let tile_y = world_pos.y / self.tile_size.y;
        self.is_tile_solid_at(atlas, tile_x, tile_y)
    }

    pub fn tile_to_world(&self, tile_pos: UVec2) -> Option<UVec2> {
        if tile_pos.x >= self.size.x || tile_pos.y >= self.size.y {
            return None;
        }
        Some(tile_pos * self.tile_size)
    }

    pub fn generate_vertices(
        &self,
        atlas: &AtlasMeta,
        texture_size: UVec2,
        tile_anim: Option<&TileAnimationClock>,
    ) -> Vec<QuadVertex> {
        let mut vertices = Vec::new();
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            self.append_layer_vertices(&layer.tiles, atlas, texture_size, tile_anim, &mut vertices);
        }
        vertices
    }

    pub fn generate_split_vertices(
        &self,
        atlas: &AtlasMeta,
        texture_size: UVec2,
        tile_anim: Option<&TileAnimationClock>,
    ) -> SplitTilemapVertices {
        let mut result = SplitTilemapVertices::default();
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            let target = if layer.above_entities {
                &mut result.above
            } else {
                &mut result.below
            };
            self.append_layer_vertices(&layer.tiles, atlas, texture_size, tile_anim, target);
        }
        result
    }

    fn append_layer_vertices(
        &self,
        tiles: &[String],
        atlas: &AtlasMeta,
        texture_size: UVec2,
        tile_anim: Option<&TileAnimationClock>,
        vertices: &mut Vec<QuadVertex>,
    ) {
        for y in 0..self.size.y {
            for x in 0..self.size.x {
                let tile_name = match self.tile_name_in(tiles, x, y) {
                    Ok(name) => name,
                    Err(_) => continue,
                };
                let resolved =
                    resolve_tile_name(tile_name, x, y, tiles, self.size, atlas, tile_anim);
                self.push_tile_quad(resolved, x, y, atlas, texture_size, vertices);
            }
        }
    }

    fn push_tile_quad(
        &self,
        tile_name: &str,
        tile_x: u32,
        tile_y: u32,
        atlas: &AtlasMeta,
        texture_size: UVec2,
        vertices: &mut Vec<QuadVertex>,
    ) {
        let Some(rect) = atlas.get_tile_rect(tile_name) else {
            return;
        };
        let Some(pos) = self.tile_to_world(UVec2::new(tile_x, tile_y)) else {
            return;
        };

        let tile_w = rect[2] as f32;
        let tile_h = rect[3] as f32;
        let u0 = rect[0] as f32 / texture_size.x as f32;
        let v0 = rect[1] as f32 / texture_size.y as f32;
        let u1 = (rect[0] + rect[2]) as f32 / texture_size.x as f32;
        let v1 = (rect[1] + rect[3]) as f32 / texture_size.y as f32;
        let x = pos.x as f32;
        let y = pos.y as f32;

        vertices.push(QuadVertex {
            position: [x, y],
            tex_coords: [u0, v0],
            tint_alpha: 0.0,
        });
        vertices.push(QuadVertex {
            position: [x + tile_w, y],
            tex_coords: [u1, v0],
            tint_alpha: 0.0,
        });
        vertices.push(QuadVertex {
            position: [x, y + tile_h],
            tex_coords: [u0, v1],
            tint_alpha: 0.0,
        });
        vertices.push(QuadVertex {
            position: [x + tile_w, y],
            tex_coords: [u1, v0],
            tint_alpha: 0.0,
        });
        vertices.push(QuadVertex {
            position: [x + tile_w, y + tile_h],
            tex_coords: [u1, v1],
            tint_alpha: 0.0,
        });
        vertices.push(QuadVertex {
            position: [x, y + tile_h],
            tex_coords: [u0, v1],
            tint_alpha: 0.0,
        });
    }

    pub fn chunk_count(&self) -> UVec2 {
        UVec2::new(
            self.size.x.div_ceil(CHUNK_SIZE),
            self.size.y.div_ceil(CHUNK_SIZE),
        )
    }

    pub fn chunk_bounds(&self, chunk_x: u32, chunk_y: u32) -> Option<(UVec2, UVec2)> {
        let chunks = self.chunk_count();
        if chunk_x >= chunks.x || chunk_y >= chunks.y {
            return None;
        }
        let start_tile = UVec2::new(chunk_x * CHUNK_SIZE, chunk_y * CHUNK_SIZE);
        let end_tile = UVec2::new(
            ((chunk_x + 1) * CHUNK_SIZE).min(self.size.x),
            ((chunk_y + 1) * CHUNK_SIZE).min(self.size.y),
        );
        let start_world = self.tile_to_world(start_tile)?;
        let end_world = self.tile_to_world(UVec2::new(end_tile.x - 1, end_tile.y - 1))?;
        Some((start_world, end_world + self.tile_size))
    }

    pub fn visible_chunks(&self, camera_world_pos: UVec2, viewport_size: UVec2) -> Vec<(u32, u32)> {
        let mut visible = Vec::new();
        let camera_end = camera_world_pos + viewport_size + viewport_size / 2;
        let chunks = self.chunk_count();
        for chunk_y in 0..chunks.y {
            for chunk_x in 0..chunks.x {
                if let Some((start, end)) = self.chunk_bounds(chunk_x, chunk_y) {
                    let overlaps = !(end.x < camera_world_pos.x
                        || start.x > camera_end.x
                        || end.y < camera_world_pos.y
                        || start.y > camera_end.y);
                    if overlaps {
                        visible.push((chunk_x, chunk_y));
                    }
                }
            }
        }
        visible
    }

    pub fn generate_vertices_for_chunks(
        &self,
        atlas: &AtlasMeta,
        texture_size: UVec2,
        visible_chunks: &[(u32, u32)],
        tile_anim: Option<&TileAnimationClock>,
    ) -> Vec<QuadVertex> {
        let mut vertices = Vec::new();
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            self.append_chunk_vertices(
                &layer.tiles,
                atlas,
                texture_size,
                visible_chunks,
                tile_anim,
                &mut vertices,
            );
        }
        vertices
    }

    pub fn generate_split_vertices_for_chunks(
        &self,
        atlas: &AtlasMeta,
        texture_size: UVec2,
        visible_chunks: &[(u32, u32)],
        tile_anim: Option<&TileAnimationClock>,
    ) -> SplitTilemapVertices {
        let mut result = SplitTilemapVertices::default();
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            let target = if layer.above_entities {
                &mut result.above
            } else {
                &mut result.below
            };
            self.append_chunk_vertices(
                &layer.tiles,
                atlas,
                texture_size,
                visible_chunks,
                tile_anim,
                target,
            );
        }
        result
    }

    fn append_chunk_vertices(
        &self,
        tiles: &[String],
        atlas: &AtlasMeta,
        texture_size: UVec2,
        visible_chunks: &[(u32, u32)],
        tile_anim: Option<&TileAnimationClock>,
        vertices: &mut Vec<QuadVertex>,
    ) {
        for &(chunk_x, chunk_y) in visible_chunks {
            let start_x = chunk_x * CHUNK_SIZE;
            let start_y = chunk_y * CHUNK_SIZE;
            let end_x = ((chunk_x + 1) * CHUNK_SIZE).min(self.size.x);
            let end_y = ((chunk_y + 1) * CHUNK_SIZE).min(self.size.y);

            for tile_y in start_y..end_y {
                for tile_x in start_x..end_x {
                    let tile_name = match self.tile_name_in(tiles, tile_x, tile_y) {
                        Ok(name) => name,
                        Err(_) => continue,
                    };
                    let resolved = resolve_tile_name(
                        tile_name, tile_x, tile_y, tiles, self.size, atlas, tile_anim,
                    );
                    self.push_tile_quad(resolved, tile_x, tile_y, atlas, texture_size, vertices);
                }
            }
        }
    }
}
