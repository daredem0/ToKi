use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorldContext<'a> {
    pub(crate) bounds: glam::UVec2,
    pub(crate) tilemap: &'a TileMap,
    pub(crate) atlas: &'a AtlasMeta,
}

impl<'a> From<super::UpdateContext<'a>> for WorldContext<'a> {
    fn from(value: super::UpdateContext<'a>) -> Self {
        Self {
            bounds: value.world_bounds,
            tilemap: value.tilemap,
            atlas: value.atlas,
        }
    }
}
