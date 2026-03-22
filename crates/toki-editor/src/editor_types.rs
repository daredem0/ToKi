use std::path::PathBuf;
use toki_core::graphics::image::DecodedImage;

#[derive(Debug, Clone)]
pub struct PlacementPreviewVisual {
    pub frame: toki_core::sprite::SpriteFrame,
    pub texture_path: Option<PathBuf>,
    pub texture_image: Option<DecodedImage>,
    pub texture_cache_key: Option<String>,
    pub size: glam::UVec2,
}
