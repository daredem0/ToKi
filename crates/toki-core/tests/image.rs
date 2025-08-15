use std::path::PathBuf;
use toki_core::graphics::image::{load_image_rgba8, DecodedImage};

#[test]
fn loading_invalid_image_returns_error() {
    let result = load_image_rgba8("nonexistent.png");
    assert!(result.is_err());

    if let Err(e) = result {
        let msg = format!("{e}");
        assert!(msg.contains("Image load failed"));
    }
}

// Note: this test is skipped unless a known-good image is present.
#[test]
fn loading_valid_image_returns_data() {
    let path = PathBuf::from("../../assets/slime_sprite_bounce_64_16.png");
    let result = load_image_rgba8(&path);
    assert!(result.is_ok());
    let DecodedImage {
        width,
        height,
        data,
    } = result.unwrap();
    assert!(width > 0);
    assert!(height > 0);
    assert_eq!(data.len() as u32, width * height * 4);
}
