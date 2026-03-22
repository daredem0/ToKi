use crate::config::EditorConfig;
use glam::{UVec2, Vec2};
use toki_core::assets::tilemap::TileMap;
use toki_core::math::coordinates::snap_to_grid;

pub struct GridInteraction;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacementPose {
    pub world_origin: Vec2,
    pub marker_world: Vec2,
    pub snapped_cell_size: Option<UVec2>,
}

impl GridInteraction {
    pub fn effective_grid_size(
        tilemap: Option<&TileMap>,
        config: Option<&EditorConfig>,
    ) -> Option<UVec2> {
        tilemap.map(|map| map.tile_size).or_else(|| {
            config.map(|cfg| {
                UVec2::new(
                    cfg.editor_settings.grid.grid_size[0],
                    cfg.editor_settings.grid.grid_size[1],
                )
            })
        })
    }

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
        Self::placement_pose(anchored_world, tilemap, config).world_origin
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

        let grid_size = Self::effective_grid_size(tilemap, Some(config))
            .unwrap_or(UVec2::ONE)
            .max(UVec2::ONE);

        snap_to_grid(world_pos, grid_size)
    }

    pub fn placement_pose(
        world_pos: Vec2,
        tilemap: Option<&TileMap>,
        config: Option<&EditorConfig>,
    ) -> PlacementPose {
        let Some(config) = config else {
            return PlacementPose {
                world_origin: world_pos,
                marker_world: world_pos,
                snapped_cell_size: None,
            };
        };
        if !config.editor_settings.grid.snap_to_grid {
            return PlacementPose {
                world_origin: world_pos,
                marker_world: world_pos,
                snapped_cell_size: None,
            };
        }

        let Some(grid_size) = Self::effective_grid_size(tilemap, Some(config)) else {
            return PlacementPose {
                world_origin: world_pos,
                marker_world: world_pos,
                snapped_cell_size: None,
            };
        };
        let world_origin = Self::maybe_snap_world_position(world_pos, tilemap, Some(config));
        PlacementPose {
            world_origin,
            marker_world: world_origin + grid_size.as_vec2() * 0.5,
            snapped_cell_size: Some(grid_size),
        }
    }
}
