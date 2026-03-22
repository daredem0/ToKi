use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PlacementPreviewVisual {
    pub frame: toki_core::sprite::SpriteFrame,
    pub texture_path: Option<PathBuf>,
    pub size: glam::UVec2,
}
