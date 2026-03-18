use crate::config::EditorConfig;
use glam::{UVec2, Vec2};
use toki_core::assets::tilemap::TileMap;
use toki_core::math::coordinates::snap_to_grid;

pub struct GridInteraction;

impl GridInteraction {
    pub fn apply_drag_grab_offset(cursor_world: Vec2, grab_offset: Vec2) -> Vec2 {
        cursor_world - grab_offset
    }

    pub fn drag_target_world_position(
        cursor_world: Vec2,
        grab_offset: Vec2,
        tilemap: Option<&TileMap>,
        config: Option<&EditorConfig>,
    ) -> Vec2 {
        let anchored_world = Self::apply_drag_grab_offset(cursor_world, grab_offset);
        Self::maybe_snap_world_position(anchored_world, tilemap, config)
    }

    pub fn maybe_snap_world_position(
        world_pos: Vec2,
        tilemap: Option<&TileMap>,
        config: Option<&EditorConfig>,
    ) -> Vec2 {
        let Some(config) = config else {
            return world_pos;
        };

        if !config.editor_settings.grid.snap_to_grid {
            return world_pos;
        }

        let grid_size = tilemap
            .map(|map| map.tile_size)
            .unwrap_or_else(|| {
                UVec2::new(
                    config.editor_settings.grid.grid_size[0],
                    config.editor_settings.grid.grid_size[1],
                )
            })
            .max(UVec2::ONE);

        snap_to_grid(world_pos, grid_size)
    }
}

#[cfg(test)]
#[path = "grid_tests.rs"]
mod tests;
