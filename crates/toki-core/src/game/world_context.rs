use crate::assets::tilemap::TileMap;
use crate::assets::tileset::TileSetResolver;

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorldContext<'a> {
    pub(crate) bounds: glam::UVec2,
    pub(crate) tilemap: &'a TileMap,
    pub(crate) tileset: &'a TileSetResolver<'a>,
}

impl<'a> From<super::UpdateContext<'a>> for WorldContext<'a> {
    fn from(value: super::UpdateContext<'a>) -> Self {
        Self {
            bounds: value.world_bounds,
            tilemap: value.tilemap,
            tileset: value.tileset,
        }
    }
}
