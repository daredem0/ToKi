use crate::assets::atlas::AtlasMeta;
use crate::graphics::vertex::QuadVertex;
use crate::CoreError;
use glam::UVec2;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct TileMap {
    pub size: UVec2,        // map dimensions in tiles (width x height)
    pub tile_size: UVec2,   // tile dimensions in pixels (width x height)
    pub atlas: PathBuf,     // path to atlas file
    pub tiles: Vec<String>, // row-major list of tile names
}

impl TileMap {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let content = fs::read_to_string(path)?;
        let map = serde_json::from_str::<TileMap>(&content)?;
        Ok(map)
    }

    pub fn get_tile_name(&self, x: u32, y: u32) -> Option<&str> {
        if x >= self.size.x || y >= self.size.y {
            return None;
        }
        let index = (y * self.size.x + x) as usize;
        self.tiles.get(index).map(String::as_str)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        let expected_len = (self.size.x * self.size.y) as usize;
        let actual_len = self.tiles.len();
        if expected_len != actual_len {
            return Err(CoreError::InvalidMapSize {
                expected: expected_len,
                actual: actual_len,
            });
        }
        Ok(())
    }

    pub fn tile_to_world(&self, tile_pos: UVec2) -> Option<UVec2> {
        if tile_pos.x >= self.size.x || tile_pos.y >= self.size.y {
            return None;
        }
        Some(tile_pos * self.tile_size)
    }

    pub fn generate_vertices(&self, atlas: &AtlasMeta, texture_size: UVec2) -> Vec<QuadVertex> {
        let mut vertices = Vec::new();

        for y in 0..self.size.y {
            for x in 0..self.size.x {
                let tile_name = match self.get_tile_name(x, y) {
                    Some(name) => name,
                    None => continue,
                };

                let rect = match atlas.get_tile_rect(tile_name) {
                    Some(r) => r,
                    None => continue,
                };

                let pos = match self.tile_to_world(UVec2::new(x, y)) {
                    Some(p) => p,
                    None => continue,
                };

                let tile_w = rect[2] as f32;
                let tile_h = rect[3] as f32;
                let u0 = rect[0] as f32 / texture_size.x as f32;
                let v0 = rect[1] as f32 / texture_size.y as f32;
                let u1 = (rect[0] + rect[2]) as f32 / texture_size.x as f32;
                let v1 = (rect[1] + rect[3]) as f32 / texture_size.y as f32;

                let x = pos.x as f32;
                let y = pos.y as f32;

                // Triangle 1
                vertices.push(QuadVertex {
                    position: [x, y],
                    tex_coords: [u0, v0],
                });
                vertices.push(QuadVertex {
                    position: [x + tile_w, y],
                    tex_coords: [u1, v0],
                });
                vertices.push(QuadVertex {
                    position: [x, y + tile_h],
                    tex_coords: [u0, v1],
                });

                // Triangle 2
                vertices.push(QuadVertex {
                    position: [x + tile_w, y],
                    tex_coords: [u1, v0],
                });
                vertices.push(QuadVertex {
                    position: [x + tile_w, y + tile_h],
                    tex_coords: [u1, v1],
                });
                vertices.push(QuadVertex {
                    position: [x, y + tile_h],
                    tex_coords: [u0, v1],
                });
            }
        }

        vertices
    }
}
