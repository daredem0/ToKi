use crate::assets::{
    atlas::AtlasMeta,
    tile_animation::TileAnimationClock,
    tileset::{TileSetAtlasSource, TileSetMeta, TileSetResolver, TilemapRenderBatch},
};
use crate::graphics::vertex::QuadVertex;
use crate::io::text::{
    read_text_file_with_limit, too_large_io_error, DEFAULT_TEXT_FILE_SIZE_LIMIT,
};
use crate::CoreError;
use glam::UVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const CHUNK_SIZE: u32 = 16;

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
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub collision_overrides: HashMap<u32, bool>,
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
            collision_overrides: HashMap::new(),
        }
    }

    pub fn new_empty(name: impl Into<String>, tile_count: usize) -> Self {
        Self {
            name: name.into(),
            tiles: vec![String::new(); tile_count],
            visible: true,
            collision_enabled: false,
            above_entities: false,
            collision_overrides: HashMap::new(),
        }
    }
}

#[derive(Deserialize)]
struct TileMapWire {
    size: UVec2,
    tile_size: UVec2,
    #[serde(default)]
    tileset: Option<PathBuf>,
    #[serde(default)]
    atlas: Option<PathBuf>,
    #[serde(default)]
    tiles: Option<Vec<String>>,
    #[serde(default)]
    layers: Option<Vec<TileLayer>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileMap {
    pub size: UVec2,
    pub tile_size: UVec2,
    pub tileset: PathBuf,
    pub layers: Vec<TileLayer>,
}

impl Serialize for TileMap {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("TileMap", 4)?;
        state.serialize_field("size", &self.size)?;
        state.serialize_field("tile_size", &self.tile_size)?;
        state.serialize_field("tileset", &self.tileset)?;
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
        let tileset = wire.tileset.or(wire.atlas).ok_or_else(|| {
            serde::de::Error::custom("tilemap must contain 'tileset'")
        })?;
        Ok(TileMap {
            size: wire.size,
            tile_size: wire.tile_size,
            tileset,
            layers,
        })
    }
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
            .map(|layer| layer.tiles.as_slice())
            .ok_or(CoreError::InvalidMapSize {
                expected: self.layers.len(),
                actual: layer,
            })
    }

    pub fn tiles(&self) -> &[String] {
        self.layers
            .first()
            .map(|layer| layer.tiles.as_slice())
            .unwrap_or(&[])
    }

    pub fn tiles_mut(&mut self) -> &mut Vec<String> {
        &mut self.layers[0].tiles
    }

    pub fn layer_tiles_mut(&mut self, layer: usize) -> Option<&mut Vec<String>> {
        self.layers.get_mut(layer).map(|layer| &mut layer.tiles)
    }

    pub fn is_tile_solid_at(
        &self,
        resolver: &TileSetResolver<'_>,
        tile_x: u32,
        tile_y: u32,
    ) -> Result<bool, CoreError> {
        resolver.is_tile_solid_at(self, tile_x, tile_y)
    }

    pub fn is_world_position_solid(
        &self,
        resolver: &TileSetResolver<'_>,
        world_pos: glam::UVec2,
    ) -> Result<bool, CoreError> {
        let tile_x = world_pos.x / self.tile_size.x;
        let tile_y = world_pos.y / self.tile_size.y;
        resolver.is_tile_solid_at(self, tile_x, tile_y)
    }

    pub fn tile_to_world(&self, tile_pos: UVec2) -> Option<UVec2> {
        if tile_pos.x >= self.size.x || tile_pos.y >= self.size.y {
            return None;
        }
        Some(tile_pos * self.tile_size)
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

    pub fn generate_render_batches(
        &self,
        resolver: &TileSetResolver<'_>,
        tile_anim: Option<&TileAnimationClock>,
        indexed_palette_override: Option<&str>,
    ) -> Result<Vec<TilemapRenderBatch>, CoreError> {
        resolver.generate_render_batches(self, None, tile_anim, indexed_palette_override)
    }

    pub fn generate_render_batches_for_chunks(
        &self,
        resolver: &TileSetResolver<'_>,
        visible_chunks: &[(u32, u32)],
        tile_anim: Option<&TileAnimationClock>,
        indexed_palette_override: Option<&str>,
    ) -> Result<Vec<TilemapRenderBatch>, CoreError> {
        resolver.generate_render_batches(
            self,
            Some(visible_chunks),
            tile_anim,
            indexed_palette_override,
        )
    }

    fn legacy_resolver_storage(
        atlas: &AtlasMeta,
    ) -> (
        TileSetMeta,
        std::collections::HashMap<String, TileSetAtlasSource>,
    ) {
        let atlas_name = "legacy_atlas.json".to_string();
        let tileset = TileSetMeta::from_legacy_atlas(&atlas_name, atlas);
        let atlases = std::collections::HashMap::from([(
            atlas_name.clone(),
            TileSetAtlasSource {
                name: "legacy_atlas".to_string(),
                path: PathBuf::from(&atlas_name),
                meta: atlas.clone(),
            },
        )]);
        (tileset, atlases)
    }

    pub fn generate_vertices(
        &self,
        atlas: &AtlasMeta,
        _texture_size: UVec2,
        tile_anim: Option<&TileAnimationClock>,
    ) -> Vec<QuadVertex> {
        let (tileset, atlases) = Self::legacy_resolver_storage(atlas);
        let resolver = TileSetResolver::new(&tileset, &atlases);
        self.generate_render_batches(&resolver, tile_anim, None)
            .map(|batches| batches.into_iter().flat_map(|batch| batch.vertices).collect())
            .unwrap_or_default()
    }

    pub fn generate_split_vertices(
        &self,
        atlas: &AtlasMeta,
        _texture_size: UVec2,
        tile_anim: Option<&TileAnimationClock>,
    ) -> SplitTilemapVertices {
        let (tileset, atlases) = Self::legacy_resolver_storage(atlas);
        let resolver = TileSetResolver::new(&tileset, &atlases);
        let mut split = SplitTilemapVertices::default();
        if let Ok(batches) = self.generate_render_batches(&resolver, tile_anim, None) {
            for batch in batches {
                if batch.key.above_entities {
                    split.above.extend(batch.vertices);
                } else {
                    split.below.extend(batch.vertices);
                }
            }
        }
        split
    }

    pub fn generate_split_vertices_for_chunks(
        &self,
        atlas: &AtlasMeta,
        _texture_size: UVec2,
        visible_chunks: &[(u32, u32)],
        tile_anim: Option<&TileAnimationClock>,
    ) -> SplitTilemapVertices {
        let (tileset, atlases) = Self::legacy_resolver_storage(atlas);
        let resolver = TileSetResolver::new(&tileset, &atlases);
        let mut split = SplitTilemapVertices::default();
        if let Ok(batches) =
            self.generate_render_batches_for_chunks(&resolver, visible_chunks, tile_anim, None)
        {
            for batch in batches {
                if batch.key.above_entities {
                    split.above.extend(batch.vertices);
                } else {
                    split.below.extend(batch.vertices);
                }
            }
        }
        split
    }
}
