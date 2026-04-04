use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use crate::graphics::image::DecodedImage;
use crate::CoreError;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// PaletteSize
// ---------------------------------------------------------------------------

/// Supported indexed-palette sizes.
///
/// Each variant maps to a fixed color count used by canonical shade generation
/// and palette validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum PaletteSize {
    #[default]
    Pal4,
    Pal8,
    Pal16,
    Pal32,
    Pal64,
    Pal256,
}

impl PaletteSize {
    pub const fn color_count(self) -> usize {
        match self {
            Self::Pal4 => 4,
            Self::Pal8 => 8,
            Self::Pal16 => 16,
            Self::Pal32 => 32,
            Self::Pal64 => 64,
            Self::Pal256 => 256,
        }
    }

    /// Compute the canonical grayscale shades for this palette size.
    ///
    /// Shade `i` = `i * 255 / (N - 1)`, producing evenly spaced gray values
    /// from black to white. For `Pal4` this yields `[0x00, 0x55, 0xAA, 0xFF]`.
    pub fn canonical_shades(self) -> Vec<[u8; 3]> {
        let n = self.color_count();
        (0..n)
            .map(|i| {
                let v = (i * 255 / (n - 1)) as u8;
                [v, v, v]
            })
            .collect()
    }
}

impl TryFrom<u16> for PaletteSize {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            4 => Ok(Self::Pal4),
            8 => Ok(Self::Pal8),
            16 => Ok(Self::Pal16),
            32 => Ok(Self::Pal32),
            64 => Ok(Self::Pal64),
            256 => Ok(Self::Pal256),
            _ => Err(format!(
                "invalid palette size {value}: expected 4, 8, 16, 32, 64, or 256"
            )),
        }
    }
}

impl From<PaletteSize> for u16 {
    fn from(size: PaletteSize) -> Self {
        size.color_count() as u16
    }
}

impl Serialize for PaletteSize {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        u16::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PaletteSize {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u16::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for PaletteSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.color_count())
    }
}

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

/// An indexed color palette with a configurable number of colors.
///
/// Replaces the former fixed-size `Palette4`. Supports 4, 8, 16, 32, 64, or
/// 256 colors. The palette carries its [`PaletteSize`] so consumers can query
/// how many indices are valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    size: PaletteSize,
    colors: Vec<[u8; 4]>,
}

impl Palette {
    pub fn new(size: PaletteSize, colors: Vec<[u8; 4]>) -> Result<Self, String> {
        if colors.len() != size.color_count() {
            return Err(format!(
                "expected {} colors for {:?}, got {}",
                size.color_count(),
                size,
                colors.len()
            ));
        }
        Ok(Self { size, colors })
    }

    /// Create a palette with canonical grayscale shades for the given size.
    pub fn grayscale(size: PaletteSize) -> Self {
        let colors = size
            .canonical_shades()
            .into_iter()
            .map(|[r, g, b]| [r, g, b, 0xFF])
            .collect();
        Self { size, colors }
    }

    pub fn size(&self) -> PaletteSize {
        self.size
    }

    pub fn colors(&self) -> &[[u8; 4]] {
        &self.colors
    }

    pub fn colors_mut(&mut self) -> &mut [[u8; 4]] {
        &mut self.colors
    }

    pub fn color(&self, index: usize) -> [u8; 4] {
        self.colors[index]
    }
}

// -- Serde (backward-compatible) -------------------------------------------

/// Wire format for [`Palette`]. Legacy files omit `size` and always have 4
/// colors; we default to `Pal4` when the field is absent.
#[derive(Serialize, Deserialize)]
struct PaletteWire {
    #[serde(default = "default_palette_size")]
    size: PaletteSize,
    colors: Vec<[u8; 4]>,
}

fn default_palette_size() -> PaletteSize {
    PaletteSize::Pal4
}

impl Serialize for Palette {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let wire = PaletteWire {
            size: self.size,
            colors: self.colors.clone(),
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Palette {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = PaletteWire::deserialize(deserializer)?;
        let expected_size = infer_palette_size(&wire);
        Self::new(expected_size, wire.colors).map_err(serde::de::Error::custom)
    }
}

/// When `size` is the default Pal4 but `colors` has a different valid count,
/// infer from the actual color count (supports legacy files that have no size
/// field but might have been hand-edited to a different count).
fn infer_palette_size(wire: &PaletteWire) -> PaletteSize {
    if wire.size == PaletteSize::Pal4 && wire.colors.len() != 4 {
        if let Ok(inferred) = PaletteSize::try_from(wire.colors.len() as u16) {
            return inferred;
        }
    }
    wire.size
}

// -- Convenience constructors for built-in 4-color palettes ----------------

fn pal4(colors: [[u8; 4]; 4]) -> Palette {
    Palette {
        size: PaletteSize::Pal4,
        colors: colors.to_vec(),
    }
}

pub type PaletteAssetFile = Palette;

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

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

/// Validate that all opaque pixels in RGBA8 `data` use only the canonical
/// grayscale shades for the given `palette_size`.
pub fn validate_indexed_rgba8(data: &[u8], palette_size: PaletteSize) -> IndexedImageValidation {
    let valid_shades = palette_size.canonical_shades();
    let mut unique_colors = BTreeMap::<[u8; 4], ()>::new();
    let mut invalid_colors = BTreeMap::<[u8; 4], ()>::new();

    for rgba in data.chunks_exact(4) {
        let color = [rgba[0], rgba[1], rgba[2], rgba[3]];
        unique_colors.insert(color, ());
        if rgba[3] == 0 {
            continue;
        }
        let rgb = [rgba[0], rgba[1], rgba[2]];
        if !valid_shades.contains(&rgb) {
            invalid_colors.insert(color, ());
        }
    }

    IndexedImageValidation {
        unique_color_count: unique_colors.len(),
        invalid_colors: invalid_colors.into_keys().collect(),
    }
}

// ---------------------------------------------------------------------------
// Recoloring
// ---------------------------------------------------------------------------

/// Build a lookup table mapping each canonical gray value to its palette index.
///
/// Returns an array indexed by the gray byte value (0–255). Entries for
/// non-canonical values are `None`.
fn build_shade_lookup(palette_size: PaletteSize) -> [Option<usize>; 256] {
    let mut table = [None; 256];
    for (i, shade) in palette_size.canonical_shades().iter().enumerate() {
        table[shade[0] as usize] = Some(i);
    }
    table
}

/// Recolor an indexed grayscale image using the given palette.
///
/// Each opaque pixel's gray value is mapped to the corresponding palette
/// color. Transparent pixels are left unchanged.
pub fn recolor_indexed_image(
    image: &DecodedImage,
    palette: &Palette,
) -> Result<DecodedImage, IndexedImageValidationError> {
    let validation = validate_indexed_rgba8(&image.data, palette.size());
    if !validation.is_valid() {
        return Err(validation);
    }

    let lookup = build_shade_lookup(palette.size());
    let data = recolor_pixels(&image.data, palette, &lookup);
    Ok(DecodedImage {
        width: image.width,
        height: image.height,
        data,
    })
}

fn recolor_pixels(data: &[u8], palette: &Palette, lookup: &[Option<usize>; 256]) -> Vec<u8> {
    let mut recolored = data.to_vec();
    for rgba in recolored.chunks_exact_mut(4) {
        if rgba[3] == 0 {
            continue;
        }
        // After validation, all opaque pixels have a canonical gray value
        // where R == G == B, so any channel works as lookup key.
        let index = lookup[rgba[0] as usize]
            .expect("validated indexed image must only contain canonical shades");
        let target = palette.color(index);
        rgba[0] = target[0];
        rgba[1] = target[1];
        rgba[2] = target[2];
        rgba[3] = ((rgba[3] as u16 * target[3] as u16) / 255) as u8;
    }
    recolored
}

// ---------------------------------------------------------------------------
// Built-in palettes
// ---------------------------------------------------------------------------

static BUILTIN_PALETTES: LazyLock<BTreeMap<String, Palette>> = LazyLock::new(|| {
    [
        (
            "gb_default",
            pal4([
                [0x0F, 0x38, 0x0F, 0xFF],
                [0x30, 0x62, 0x30, 0xFF],
                [0x8B, 0xAC, 0x0F, 0xFF],
                [0x9B, 0xBC, 0x0F, 0xFF],
            ]),
        ),
        (
            "gray",
            pal4([
                [0x11, 0x11, 0x11, 0xFF],
                [0x55, 0x55, 0x55, 0xFF],
                [0xAA, 0xAA, 0xAA, 0xFF],
                [0xF0, 0xF0, 0xF0, 0xFF],
            ]),
        ),
        (
            "night",
            pal4([
                [0x10, 0x18, 0x2B, 0xFF],
                [0x2D, 0x4F, 0x6C, 0xFF],
                [0x65, 0x87, 0xA3, 0xFF],
                [0xB8, 0xD0, 0xE0, 0xFF],
            ]),
        ),
        (
            "poison",
            pal4([
                [0x1B, 0x0F, 0x1B, 0xFF],
                [0x4B, 0x1F, 0x6B, 0xFF],
                [0x7B, 0x4F, 0xA5, 0xFF],
                [0xC8, 0x9F, 0xE8, 0xFF],
            ]),
        ),
        (
            "sepia",
            pal4([
                [0x2C, 0x1B, 0x12, 0xFF],
                [0x6B, 0x44, 0x2A, 0xFF],
                [0xB0, 0x7A, 0x45, 0xFF],
                [0xE7, 0xC9, 0x8A, 0xFF],
            ]),
        ),
    ]
    .into_iter()
    .map(|(name, palette)| (name.to_string(), palette))
    .collect()
});

pub fn builtin_palettes() -> BTreeMap<String, Palette> {
    BUILTIN_PALETTES.clone()
}

pub fn resolve_palette(
    palette_id: &str,
    project_palettes: &BTreeMap<String, Palette>,
) -> Option<Palette> {
    project_palettes
        .get(palette_id)
        .cloned()
        .or_else(|| BUILTIN_PALETTES.get(palette_id).cloned())
}

// ---------------------------------------------------------------------------
// File I/O
// ---------------------------------------------------------------------------

pub fn load_palette_asset_from_path(path: &Path) -> Result<Palette, CoreError> {
    let content = crate::io::text::read_text_file_with_limit(
        path,
        crate::io::text::DEFAULT_TEXT_FILE_SIZE_LIMIT,
        |path, size_bytes, max_bytes| {
            crate::io::text::too_large_io_error(path, size_bytes, max_bytes, "palette file")
        },
    )?;
    serde_json::from_str::<Palette>(&content).map_err(Into::into)
}

pub fn save_palette_asset_to_path(path: &Path, palette: &Palette) -> Result<(), CoreError> {
    let content = serde_json::to_string_pretty(palette)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Palette expansion (mismatch strategy support)
// ---------------------------------------------------------------------------

/// Strategy for handling palette-size mismatches between an atlas and a
/// palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteMismatchStrategy {
    /// Use the palette as-is; unmapped shades keep their canonical gray.
    #[default]
    Lenient,
    /// Linearly interpolate the palette's color ramp to fill the target size.
    Interpolate,
}

/// Expand (or shrink) `source` to `target_size` by linearly interpolating
/// along the palette's color ramp.
///
/// For each target index `i` in `0..M`:
///   `t = i * (N - 1) / (M - 1)` maps into the source palette's range,
///   then we linearly interpolate between the two bracketing source colors.
pub fn expand_palette(source: &Palette, target_size: PaletteSize) -> Palette {
    let n = source.size().color_count();
    let m = target_size.color_count();
    if n == m {
        return source.clone();
    }

    let colors = (0..m)
        .map(|i| lerp_palette_color(source, n, m, i))
        .collect();
    Palette {
        size: target_size,
        colors,
    }
}

fn lerp_palette_color(source: &Palette, n: usize, m: usize, i: usize) -> [u8; 4] {
    let t = i as f64 * (n - 1) as f64 / (m - 1) as f64;
    let lower = t.floor() as usize;
    let upper = (lower + 1).min(n - 1);
    let frac = t - lower as f64;

    let a = source.color(lower);
    let b = source.color(upper);
    [
        lerp_u8(a[0], b[0], frac),
        lerp_u8(a[1], b[1], frac),
        lerp_u8(a[2], b[2], frac),
        lerp_u8(a[3], b[3], frac),
    ]
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn pal4_test(colors: [[u8; 4]; 4]) -> Palette {
        Palette::new(PaletteSize::Pal4, colors.to_vec()).unwrap()
    }

    // -- PaletteSize -------------------------------------------------------

    #[test]
    fn canonical_shades_pal4_matches_legacy_values() {
        assert_eq!(
            PaletteSize::Pal4.canonical_shades(),
            vec![
                [0x00, 0x00, 0x00],
                [0x55, 0x55, 0x55],
                [0xAA, 0xAA, 0xAA],
                [0xFF, 0xFF, 0xFF]
            ]
        );
    }

    #[test]
    fn canonical_shades_pal8_has_eight_entries() {
        let shades = PaletteSize::Pal8.canonical_shades();
        assert_eq!(shades.len(), 8);
        assert_eq!(shades[0], [0, 0, 0]);
        assert_eq!(shades[7], [255, 255, 255]);
    }

    #[test]
    fn canonical_shades_pal256_covers_full_range() {
        let shades = PaletteSize::Pal256.canonical_shades();
        assert_eq!(shades.len(), 256);
        assert_eq!(shades[0], [0, 0, 0]);
        assert_eq!(shades[128], [128, 128, 128]);
        assert_eq!(shades[255], [255, 255, 255]);
    }

    #[test]
    fn palette_size_try_from_valid() {
        assert_eq!(PaletteSize::try_from(4u16), Ok(PaletteSize::Pal4));
        assert_eq!(PaletteSize::try_from(16u16), Ok(PaletteSize::Pal16));
        assert_eq!(PaletteSize::try_from(256u16), Ok(PaletteSize::Pal256));
    }

    #[test]
    fn palette_size_try_from_rejects_invalid() {
        assert!(PaletteSize::try_from(0u16).is_err());
        assert!(PaletteSize::try_from(5u16).is_err());
        assert!(PaletteSize::try_from(512u16).is_err());
    }

    #[test]
    fn palette_size_roundtrips_u16() {
        for size in [
            PaletteSize::Pal4,
            PaletteSize::Pal8,
            PaletteSize::Pal16,
            PaletteSize::Pal32,
            PaletteSize::Pal64,
            PaletteSize::Pal256,
        ] {
            let n: u16 = size.into();
            assert_eq!(PaletteSize::try_from(n), Ok(size));
        }
    }

    // -- Palette -----------------------------------------------------------

    #[test]
    fn palette_new_rejects_mismatched_color_count() {
        let result = Palette::new(PaletteSize::Pal4, vec![[0, 0, 0, 255]; 8]);
        assert!(result.is_err());
    }

    #[test]
    fn palette_grayscale_produces_correct_defaults() {
        let p = Palette::grayscale(PaletteSize::Pal4);
        assert_eq!(p.colors().len(), 4);
        assert_eq!(p.color(0), [0x00, 0x00, 0x00, 0xFF]);
        assert_eq!(p.color(3), [0xFF, 0xFF, 0xFF, 0xFF]);
    }

    // -- Serde backward compat ---------------------------------------------

    #[test]
    fn legacy_json_without_size_deserializes_as_pal4() {
        let json = r#"{"colors":[[1,2,3,255],[4,5,6,255],[7,8,9,255],[10,11,12,255]]}"#;
        let palette: Palette = serde_json::from_str(json).unwrap();
        assert_eq!(palette.size(), PaletteSize::Pal4);
        assert_eq!(palette.colors().len(), 4);
    }

    #[test]
    fn new_json_with_size_roundtrips() {
        let palette = Palette::new(
            PaletteSize::Pal8,
            (0..8).map(|i| [i * 30, i * 30, i * 30, 255]).collect(),
        )
        .unwrap();
        let json = serde_json::to_string(&palette).unwrap();
        let loaded: Palette = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, palette);
    }

    // -- Validation --------------------------------------------------------

    #[test]
    fn validate_indexed_rgba8_accepts_pal4_canonical_shades() {
        let data = [0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let v = validate_indexed_rgba8(&data, PaletteSize::Pal4);
        assert!(v.is_valid());
    }

    #[test]
    fn validate_indexed_rgba8_reports_invalid_colors() {
        let v =
            validate_indexed_rgba8(&[0, 0, 0, 255, 1, 2, 3, 255, 0, 0, 0, 0], PaletteSize::Pal4);
        assert_eq!(v.unique_color_count, 3);
        assert_eq!(v.invalid_colors, vec![[1, 2, 3, 255]]);
    }

    #[test]
    fn validate_indexed_rgba8_works_with_pal16() {
        // shade[1] for Pal16 = 1 * 255 / 15 = 17
        let data = [17, 17, 17, 255];
        let v = validate_indexed_rgba8(&data, PaletteSize::Pal16);
        assert!(v.is_valid());

        // 18 is not a canonical Pal16 shade
        let data_invalid = [18, 18, 18, 255];
        let v2 = validate_indexed_rgba8(&data_invalid, PaletteSize::Pal16);
        assert!(!v2.is_valid());
    }

    // -- Recoloring --------------------------------------------------------

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
        let palette = pal4_test([
            [1, 2, 3, 255],
            [4, 5, 6, 255],
            [7, 8, 9, 255],
            [10, 11, 12, 255],
        ]);

        let recolored = recolor_indexed_image(&image, &palette).expect("indexed image is valid");

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
    fn recolor_indexed_image_works_with_pal8() {
        let shades = PaletteSize::Pal8.canonical_shades();
        // Use shade[0] and shade[7]
        let image = DecodedImage {
            width: 1,
            height: 2,
            data: vec![
                shades[0][0],
                shades[0][1],
                shades[0][2],
                0xFF, //
                shades[7][0],
                shades[7][1],
                shades[7][2],
                0xFF,
            ],
        };
        let colors: Vec<[u8; 4]> = (0..8).map(|i| [i * 30, 0, 0, 255]).collect();
        let palette = Palette::new(PaletteSize::Pal8, colors).unwrap();

        let recolored = recolor_indexed_image(&image, &palette).unwrap();

        assert_eq!(recolored.data[0], 0); // color[0].r
        assert_eq!(recolored.data[4], 210); // color[7].r = 7*30
    }

    #[test]
    fn recolor_indexed_image_returns_validation_details_for_invalid_input() {
        let image = DecodedImage {
            width: 1,
            height: 1,
            data: vec![1, 2, 3, 255],
        };
        let palette = Palette::grayscale(PaletteSize::Pal4);

        let error = recolor_indexed_image(&image, &palette).unwrap_err();

        assert_eq!(error.unique_color_count, 1);
        assert_eq!(error.invalid_colors, vec![[1, 2, 3, 255]]);
    }

    // -- Resolve & builtins ------------------------------------------------

    #[test]
    fn resolve_palette_prefers_project_over_builtin() {
        let mut project_palettes = BTreeMap::new();
        project_palettes.insert("gb_default".to_string(), pal4_test([[1, 2, 3, 255]; 4]));

        let resolved =
            resolve_palette("gb_default", &project_palettes).expect("project palette should win");

        assert_eq!(resolved.colors(), &[[1, 2, 3, 255]; 4]);
    }

    // -- File I/O ----------------------------------------------------------

    #[test]
    fn palette_asset_roundtrips_via_json_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let path = temp_dir.path().join("forest.json");
        let palette = pal4_test([
            [1, 2, 3, 255],
            [4, 5, 6, 255],
            [7, 8, 9, 255],
            [10, 11, 12, 255],
        ]);

        save_palette_asset_to_path(&path, &palette).expect("save palette");
        let loaded = load_palette_asset_from_path(&path).expect("load palette");

        assert_eq!(loaded, palette);
    }

    // -- Expand palette (interpolation) ------------------------------------

    #[test]
    fn expand_palette_same_size_returns_clone() {
        let p = pal4_test([
            [0, 0, 0, 255],
            [85, 85, 85, 255],
            [170, 170, 170, 255],
            [255, 255, 255, 255],
        ]);
        let expanded = expand_palette(&p, PaletteSize::Pal4);
        assert_eq!(expanded, p);
    }

    #[test]
    fn expand_palette_preserves_endpoints() {
        let p = pal4_test([
            [10, 20, 30, 255],
            [50, 60, 70, 255],
            [90, 100, 110, 255],
            [130, 140, 150, 255],
        ]);
        let expanded = expand_palette(&p, PaletteSize::Pal16);
        assert_eq!(expanded.color(0), [10, 20, 30, 255]);
        assert_eq!(expanded.color(15), [130, 140, 150, 255]);
        assert_eq!(expanded.size(), PaletteSize::Pal16);
    }

    #[test]
    fn expand_palette_interpolates_midpoints() {
        let p = Palette::new(
            PaletteSize::Pal4,
            vec![
                [0, 0, 0, 255],
                [100, 100, 100, 255],
                [200, 200, 200, 255],
                [255, 255, 255, 255],
            ],
        )
        .unwrap();
        let expanded = expand_palette(&p, PaletteSize::Pal8);
        // With 8 target colors from 4 source, index 1 maps to t = 1*3/7 ≈ 0.4286
        // That's between source[0]=[0,0,0] and source[1]=[100,100,100]
        let c1 = expanded.color(1);
        assert!(c1[0] > 0 && c1[0] < 100, "midpoint should be interpolated");
    }

    // -- PaletteMismatchStrategy serde -------------------------------------

    #[test]
    fn palette_mismatch_strategy_serializes_as_lowercase() {
        let json = serde_json::to_string(&PaletteMismatchStrategy::Interpolate).unwrap();
        assert_eq!(json, r#""interpolate""#);
        let json2 = serde_json::to_string(&PaletteMismatchStrategy::Lenient).unwrap();
        assert_eq!(json2, r#""lenient""#);
    }
}
