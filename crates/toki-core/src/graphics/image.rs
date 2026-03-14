use crate::errors::CoreError;

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

pub fn load_image_rgba8<P: AsRef<std::path::Path> + std::fmt::Debug>(
    path: P,
) -> Result<DecodedImage, CoreError> {
    tracing::debug!("Loading image: {path:?}");
    let img = image::open(&path)
        .map_err(|e| CoreError::ImageLoad(e.to_string()))?
        .into_rgba8();
    let (width, height) = img.dimensions();
    Ok(DecodedImage {
        width,
        height,
        data: img.into_raw(),
    })
}

pub fn load_image_rgba8_from_bytes(bytes: &[u8]) -> Result<DecodedImage, CoreError> {
    tracing::debug!("Loading image from embedded bytes ({} bytes)", bytes.len());
    let img = image::load_from_memory(bytes)
        .map_err(|e| CoreError::ImageLoad(e.to_string()))?
        .into_rgba8();
    let (width, height) = img.dimensions();
    Ok(DecodedImage {
        width,
        height,
        data: img.into_raw(),
    })
}
