use crate::errors::CoreError;
use image::{ImageBuffer, Rgba};

pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// Save RGBA8 image data to a PNG file.
pub fn save_image_rgba8<P: AsRef<std::path::Path>>(
    path: P,
    width: u32,
    height: u32,
    data: &[u8],
) -> Result<(), CoreError> {
    let expected_len = (width as usize) * (height as usize) * 4;
    if data.len() != expected_len {
        return Err(CoreError::ImageLoad(format!(
            "Invalid image data length: expected {expected_len}, got {}",
            data.len()
        )));
    }

    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width, height, data.to_vec()).ok_or_else(|| {
            CoreError::ImageLoad("Failed to create image buffer from raw data".to_string())
        })?;

    img.save(&path).map_err(|e| {
        CoreError::ImageLoad(format!("Failed to save image to {:?}: {e}", path.as_ref()))
    })?;

    tracing::debug!("Saved image: {:?}", path.as_ref());
    Ok(())
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
