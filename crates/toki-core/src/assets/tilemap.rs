use crate::assets::atlas::AtlasMeta;
use crate::graphics::vertex::QuadVertex;
use crate::CoreError;
use glam::UVec2;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub const CHUNK_SIZE: u32 = 16; //16x16 tiles per chunk

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

    pub fn chunk_count(&self) -> UVec2 {
        UVec2::new(
            self.size.x.div_ceil(CHUNK_SIZE), // Ceiling division: 64/16=4, 50/16=4 chunks
            self.size.y.div_ceil(CHUNK_SIZE), // Ensures partial edge chunks are counted
        )
    }

    pub fn chunk_bounds(&self, chunk_x: u32, chunk_y: u32) -> Option<(UVec2, UVec2)> {
        let chunks = self.chunk_count();
        if chunk_x >= chunks.x || chunk_y >= chunks.y {
            return None; // Invalid chunk coordinates
        }
        // Convert chunk coords to tile coords (chunk 0,0 = tiles 0,0 to 15,15)
        let start_tile = UVec2::new(chunk_x * CHUNK_SIZE, chunk_y * CHUNK_SIZE);
        let end_tile = UVec2::new(
            ((chunk_x + 1) * CHUNK_SIZE).min(self.size.x), // Handle edge chunks
            ((chunk_y + 1) * CHUNK_SIZE).min(self.size.y), // May be smaller than 16x16
        );
        // Convert tile positions to world pixel positions
        let start_world = self.tile_to_world(start_tile)?;
        let end_world = self.tile_to_world(UVec2::new(end_tile.x - 1, end_tile.y - 1))?;
        Some((start_world, end_world + self.tile_size)) // Add tile_size for full bounding box
    }

    pub fn visible_chunks(&self, camera_world_pos: UVec2, viewport_size: UVec2) -> Vec<(u32, u32)> {
        let mut visible = Vec::new();

        // Calculate the world bounds of what the camera can see
        // We are actually rendering more than the viewport here ( + viewport_size/2)
        // This is to have a slight margin for slower systems. There might be edge cases
        // Where the loading of the chunks is not fast enough. If we preload a bit that helps us
        // to smoothen that out and the cost is not too high
        let camera_end = camera_world_pos + viewport_size + viewport_size / 2;

        // Check each chunk to see if it overlaps with the camera viewport
        let chunks = self.chunk_count();
        for chunk_y in 0..chunks.y {
            for chunk_x in 0..chunks.x {
                if let Some((chunk_start, chunk_end)) = self.chunk_bounds(chunk_x, chunk_y) {
                    let overlaps = !(chunk_end.x < camera_world_pos.x ||     // Chunk is to the left
                        chunk_start.x > camera_end.x ||                        // Chunk is to the right
                        chunk_end.y < camera_world_pos.y ||                    // Chunk is above
                        chunk_start.y > camera_end.y); // Chunk is below
                    if overlaps {
                        visible.push((chunk_x, chunk_y));
                    }
                }
            }
        }
        visible
    }

    fn generate_vertices_for_tile(
        &self,
        tile_x: u32,
        tile_y: u32,
        atlas: &AtlasMeta,
        texture_size: UVec2,
    ) -> Option<Vec<QuadVertex>> {
        // Get the tile name
        let tile_name = self.get_tile_name(tile_x, tile_y)?;

        // Get atlas rectangle
        let rect = atlas.get_tile_rect(tile_name)?;

        // Calculate world position
        let pos = self.tile_to_world(UVec2::new(tile_x, tile_y))?;

        // Calculate UV coordinates and tile dimensions
        let tile_w = rect[2] as f32;
        let tile_h = rect[3] as f32;
        let u0 = rect[0] as f32 / texture_size.x as f32;
        let v0 = rect[1] as f32 / texture_size.y as f32;
        let u1 = (rect[0] + rect[2]) as f32 / texture_size.x as f32;
        let v1 = (rect[1] + rect[3]) as f32 / texture_size.y as f32;

        let x = pos.x as f32;
        let y = pos.y as f32;

        let tile_vertices = vec![
            // Triangle 1: top-left, top-right, bottom-left
            QuadVertex {
                position: [x, y],
                tex_coords: [u0, v0],
            },
            QuadVertex {
                position: [x + tile_w, y],
                tex_coords: [u1, v0],
            },
            QuadVertex {
                position: [x, y + tile_h],
                tex_coords: [u0, v1],
            },
            // Triangle 2: top-right, bottom-right, bottom-left
            QuadVertex {
                position: [x + tile_w, y],
                tex_coords: [u1, v0],
            },
            QuadVertex {
                position: [x + tile_w, y + tile_h],
                tex_coords: [u1, v1],
            },
            QuadVertex {
                position: [x, y + tile_h],
                tex_coords: [u0, v1],
            },
        ];

        Some(tile_vertices)
    }

    pub fn generate_vertices_for_chunks(
        &self,
        atlas: &AtlasMeta,
        texture_size: UVec2,
        visible_chunks: &[(u32, u32)],
    ) -> Vec<QuadVertex> {
        let mut vertices = Vec::new();

        for &(chunk_x, chunk_y) in visible_chunks {
            // Calculate which tiles are in this chunk
            let start_tile_x = chunk_x * CHUNK_SIZE;
            let start_tile_y = chunk_y * CHUNK_SIZE;
            let end_tile_x = ((chunk_x + 1) * CHUNK_SIZE).min(self.size.x);
            let end_tile_y = ((chunk_y + 1) * CHUNK_SIZE).min(self.size.y);

            // Iterate through the tiles in this chunk
            for tile_y in start_tile_y..end_tile_y {
                for tile_x in start_tile_x..end_tile_x {
                    if let Some(tile_vertices) =
                        self.generate_vertices_for_tile(tile_x, tile_y, atlas, texture_size)
                    {
                        vertices.extend(tile_vertices);
                    }
                }
            }
        }
        vertices
    }
}
