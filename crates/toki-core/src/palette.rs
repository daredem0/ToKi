use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::graphics::image::DecodedImage;
use crate::CoreError;
use serde::{Deserialize, Serialize};

pub const CANONICAL_INDEXED_SHADES: [[u8; 3]; 4] = [
    [0x00, 0x00, 0x00],
    [0x55, 0x55, 0x55],
    [0xAA, 0xAA, 0xAA],
    [0xFF, 0xFF, 0xFF],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette4 {
    pub colors: [[u8; 4]; 4],
}

impl Palette4 {
    pub const fn new(colors: [[u8; 4]; 4]) -> Self {
        Self { colors }
    }

    pub fn color(self, index: usize) -> [u8; 4] {
        self.colors[index]
    }
}

pub type PaletteAssetFile = Palette4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedImageValidation {
    pub unique_color_count: usize,
    pub invalid_colors: Vec<[u8; 4]>,
}

impl IndexedImageValidation {
    pub fn is_valid(&self) -> bool {
        self.invalid_colors.is_empty()
    }
}

impl std::fmt::Display for IndexedImageValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "indexed image uses {} invalid colors ({} unique colors total)",
            self.invalid_colors.len(),
            self.unique_color_count
        )
    }
}

impl std::error::Error for IndexedImageValidation {}

pub type IndexedImageValidationError = IndexedImageValidation;

pub fn builtin_palettes() -> BTreeMap<String, Palette4> {
    [
        (
            "gb_default".to_string(),
            Palette4::new([
                [0x0F, 0x38, 0x0F, 0xFF],
                [0x30, 0x62, 0x30, 0xFF],
                [0x8B, 0xAC, 0x0F, 0xFF],
                [0x9B, 0xBC, 0x0F, 0xFF],
            ]),
        ),
        (
            "gray".to_string(),
            Palette4::new([
                [0x11, 0x11, 0x11, 0xFF],
                [0x55, 0x55, 0x55, 0xFF],
                [0xAA, 0xAA, 0xAA, 0xFF],
                [0xF0, 0xF0, 0xF0, 0xFF],
            ]),
        ),
        (
            "night".to_string(),
            Palette4::new([
                [0x10, 0x18, 0x2B, 0xFF],
                [0x2D, 0x4F, 0x6C, 0xFF],
                [0x65, 0x87, 0xA3, 0xFF],
                [0xB8, 0xD0, 0xE0, 0xFF],
            ]),
        ),
        (
            "poison".to_string(),
            Palette4::new([
                [0x1B, 0x0F, 0x1B, 0xFF],
                [0x4B, 0x1F, 0x6B, 0xFF],
                [0x7B, 0x4F, 0xA5, 0xFF],
                [0xC8, 0x9F, 0xE8, 0xFF],
            ]),
        ),
        (
            "sepia".to_string(),
            Palette4::new([
                [0x2C, 0x1B, 0x12, 0xFF],
                [0x6B, 0x44, 0x2A, 0xFF],
                [0xB0, 0x7A, 0x45, 0xFF],
                [0xE7, 0xC9, 0x8A, 0xFF],
            ]),
        ),
    ]
    .into_iter()
    .collect()
}

pub fn resolve_palette(
    palette_id: &str,
    project_palettes: &BTreeMap<String, Palette4>,
) -> Option<Palette4> {
    project_palettes
        .get(palette_id)
        .copied()
        .or_else(|| builtin_palettes().get(palette_id).copied())
}

pub fn load_palette_asset_from_path(path: &Path) -> Result<Palette4, CoreError> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str::<PaletteAssetFile>(&content).map_err(Into::into)
}

pub fn save_palette_asset_to_path(path: &Path, palette: Palette4) -> Result<(), CoreError> {
    let content = serde_json::to_string_pretty(&palette)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

pub fn validate_indexed_rgba8(data: &[u8]) -> IndexedImageValidation {
    let mut unique_colors = BTreeMap::<[u8; 4], ()>::new();
    let mut invalid_colors = BTreeMap::<[u8; 4], ()>::new();

    for rgba in data.chunks_exact(4) {
        let color = [rgba[0], rgba[1], rgba[2], rgba[3]];
        unique_colors.insert(color, ());
        if rgba[3] == 0 {
            continue;
        }

        let rgb = [rgba[0], rgba[1], rgba[2]];
        if !CANONICAL_INDEXED_SHADES.contains(&rgb) {
            invalid_colors.insert(color, ());
        }
    }

    IndexedImageValidation {
        unique_color_count: unique_colors.len(),
        invalid_colors: invalid_colors.into_keys().collect(),
    }
}

pub fn recolor_indexed_image(
    image: &DecodedImage,
    palette: Palette4,
) -> Result<DecodedImage, IndexedImageValidationError> {
    let validation = validate_indexed_rgba8(&image.data);
    if !validation.is_valid() {
        return Err(validation);
    }

    let mut recolored = image.data.clone();
    for rgba in recolored.chunks_exact_mut(4) {
        if rgba[3] == 0 {
            continue;
        }

        let index = match [rgba[0], rgba[1], rgba[2]] {
            [0x00, 0x00, 0x00] => 0,
            [0x55, 0x55, 0x55] => 1,
            [0xAA, 0xAA, 0xAA] => 2,
            [0xFF, 0xFF, 0xFF] => 3,
            _ => unreachable!("validated indexed image must only contain canonical shades"),
        };
        let target = palette.color(index);
        rgba[0] = target[0];
        rgba[1] = target[1];
        rgba[2] = target[2];
        rgba[3] = ((rgba[3] as u16 * target[3] as u16) / 255) as u8;
    }

    Ok(DecodedImage {
        width: image.width,
        height: image.height,
        data: recolored,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_palette_prefers_project_over_builtin() {
        let mut project_palettes = BTreeMap::new();
        project_palettes.insert("gb_default".to_string(), Palette4::new([[1, 2, 3, 255]; 4]));

        let resolved =
            resolve_palette("gb_default", &project_palettes).expect("project palette should win");

        assert_eq!(resolved.colors, [[1, 2, 3, 255]; 4]);
    }

    #[test]
    fn validate_indexed_rgba8_reports_invalid_colors() {
        let validation = validate_indexed_rgba8(&[
            0, 0, 0, 255, //
            1, 2, 3, 255, //
            0, 0, 0, 0,
        ]);

        assert_eq!(validation.unique_color_count, 3);
        assert_eq!(validation.invalid_colors, vec![[1, 2, 3, 255]]);
    }

    #[test]
    fn recolor_indexed_image_maps_canonical_shades_to_palette() {
        let image = DecodedImage {
            width: 2,
            height: 2,
            data: vec![
                0x00, 0x00, 0x00, 0xFF, //
                0x55, 0x55, 0x55, 0x80, //
                0xAA, 0xAA, 0xAA, 0xFF, //
                0xFF, 0xFF, 0xFF, 0x00,
            ],
        };
        let palette = Palette4::new([
            [1, 2, 3, 255],
            [4, 5, 6, 255],
            [7, 8, 9, 255],
            [10, 11, 12, 255],
        ]);

        let recolored = recolor_indexed_image(&image, palette).expect("indexed image is valid");

        assert_eq!(
            recolored.data,
            vec![
                1, 2, 3, 255, //
                4, 5, 6, 128, //
                7, 8, 9, 255, //
                255, 255, 255, 0,
            ]
        );
    }

    #[test]
    fn recolor_indexed_image_returns_validation_details_for_invalid_input() {
        let image = DecodedImage {
            width: 1,
            height: 1,
            data: vec![1, 2, 3, 255],
        };

        let error = recolor_indexed_image(&image, Palette4::new([[0, 0, 0, 255]; 4])).unwrap_err();

        assert_eq!(error.unique_color_count, 1);
        assert_eq!(error.invalid_colors, vec![[1, 2, 3, 255]]);
    }

    #[test]
    fn palette_asset_roundtrips_via_json_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let path = temp_dir.path().join("forest.json");
        let palette = Palette4::new([
            [1, 2, 3, 255],
            [4, 5, 6, 255],
            [7, 8, 9, 255],
            [10, 11, 12, 255],
        ]);

        save_palette_asset_to_path(&path, palette).expect("save palette");
        let loaded = load_palette_asset_from_path(&path).expect("load palette");

        assert_eq!(loaded, palette);
    }
}
