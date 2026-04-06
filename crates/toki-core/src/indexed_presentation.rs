use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::assets::atlas::ColorMode;
use crate::assets::tileset::{TileRenderMaterial, TilemapRenderBatch};
use crate::graphics::image::{load_image_rgba8, DecodedImage};
use crate::graphics::vertex::QuadVertex;
use crate::palette::{
    builtin_palettes, recolor_indexed_image, resolve_palette, Palette, PaletteSize,
};
use crate::project_runtime::{
    PostProcessMode, QuantizeStrategy, ResolvedPostProcessSettings, RuntimePostProcessSettings,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IndexedPresentationSettings {
    pub indexed_palette_override: Option<String>,
    pub post_process: RuntimePostProcessSettings,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct IndexedImageMaterialization<'a> {
    pub local_override: Option<&'a str>,
    pub asset_palette: Option<&'a str>,
    pub apply_post_process: bool,
}

impl IndexedPresentationSettings {
    pub fn resolve_post_process(
        &self,
        available_palettes: &BTreeMap<String, Palette>,
    ) -> ResolvedPostProcessSettings {
        self.post_process.resolve(available_palettes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIndexedPresentation {
    pub palette_id: Option<String>,
    pub cache_key: String,
    pub image: DecodedImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentedTextureSource {
    File(PathBuf),
    Rgba8 {
        image: DecodedImage,
        cache_key: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresentedTilemapBatch {
    pub vertices: Vec<QuadVertex>,
    pub texture: PresentedTextureSource,
    pub above_entities: bool,
}

pub fn resolve_indexed_palette(
    color_mode: ColorMode,
    available_palettes: &BTreeMap<String, Palette>,
    settings: &IndexedPresentationSettings,
    local_override: Option<&str>,
    asset_palette: Option<&str>,
) -> Result<Option<(String, Palette)>, String> {
    if color_mode != ColorMode::PaletteIndexed {
        return Ok(None);
    }

    for palette_id in [
        settings.indexed_palette_override.as_deref(),
        local_override,
        asset_palette,
        Some("gb_default"),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(palette) = resolve_palette(palette_id, available_palettes) {
            return Ok(Some((palette_id.to_string(), palette)));
        }
    }

    Err("palette id could not be resolved".to_string())
}

pub fn texture_material_cache_key(
    texture_key: &str,
    color_mode: ColorMode,
    palette_id: Option<&str>,
) -> String {
    format!(
        "{}#{:?}#{}",
        texture_key,
        color_mode,
        palette_id.unwrap_or("")
    )
}

pub fn texture_preview_cache_key(
    texture_key: &str,
    color_mode: ColorMode,
    palette_id: Option<&str>,
    post_process: &ResolvedPostProcessSettings,
) -> String {
    format!(
        "{}#pp={}",
        texture_material_cache_key(texture_key, color_mode, palette_id),
        post_process_cache_fragment(post_process)
    )
}

pub fn materialize_indexed_image(
    decoded: &DecodedImage,
    cache_source: &str,
    color_mode: ColorMode,
    available_palettes: &BTreeMap<String, Palette>,
    settings: &IndexedPresentationSettings,
    options: IndexedImageMaterialization<'_>,
) -> Result<ResolvedIndexedPresentation, String> {
    let resolved_post_process = settings.resolve_post_process(available_palettes);
    let resolved_palette = resolve_indexed_palette(
        color_mode,
        available_palettes,
        settings,
        options.local_override,
        options.asset_palette,
    )?;
    let palette_id = resolved_palette
        .as_ref()
        .map(|(palette_id, _)| palette_id.clone());

    let mut image = if let Some((_, palette)) = resolved_palette {
        recolor_indexed_image(decoded, &palette).map_err(|error| error.to_string())?
    } else {
        decoded.clone()
    };

    if options.apply_post_process {
        apply_post_process_to_image(&mut image, &resolved_post_process);
    }

    let cache_key = if options.apply_post_process {
        texture_preview_cache_key(
            cache_source,
            color_mode,
            palette_id.as_deref(),
            &resolved_post_process,
        )
    } else {
        texture_material_cache_key(cache_source, color_mode, palette_id.as_deref())
    };

    Ok(ResolvedIndexedPresentation {
        palette_id,
        cache_key,
        image,
    })
}

pub fn load_materialized_indexed_image(
    texture_path: &Path,
    color_mode: ColorMode,
    available_palettes: &BTreeMap<String, Palette>,
    settings: &IndexedPresentationSettings,
    local_override: Option<&str>,
    asset_palette: Option<&str>,
    apply_post_process: bool,
) -> Result<ResolvedIndexedPresentation, String> {
    let decoded = load_image_rgba8(texture_path).map_err(|error| error.to_string())?;
    materialize_indexed_image(
        &decoded,
        &texture_path.display().to_string(),
        color_mode,
        available_palettes,
        settings,
        IndexedImageMaterialization {
            local_override,
            asset_palette,
            apply_post_process,
        },
    )
}

pub fn materialize_tilemap_batches(
    batches: Vec<TilemapRenderBatch>,
    available_palettes: &BTreeMap<String, Palette>,
    settings: &IndexedPresentationSettings,
) -> Result<Vec<PresentedTilemapBatch>, String> {
    let mut presented = Vec::with_capacity(batches.len());
    for batch in batches {
        let texture = match batch.key.material {
            TileRenderMaterial::TrueColor => PresentedTextureSource::File(batch.texture_path),
            TileRenderMaterial::PaletteIndexed { ref palette_id } => {
                let decoded =
                    load_image_rgba8(&batch.texture_path).map_err(|error| error.to_string())?;
                let presentation = materialize_indexed_image(
                    &decoded,
                    &batch.texture_path.display().to_string(),
                    ColorMode::PaletteIndexed,
                    available_palettes,
                    settings,
                    IndexedImageMaterialization {
                        local_override: Some(palette_id.as_str()),
                        asset_palette: None,
                        apply_post_process: false,
                    },
                )?;
                PresentedTextureSource::Rgba8 {
                    image: presentation.image,
                    cache_key: presentation.cache_key,
                }
            }
        };

        presented.push(PresentedTilemapBatch {
            vertices: batch.vertices,
            texture,
            above_entities: batch.key.above_entities,
        });
    }

    Ok(presented)
}

fn post_process_cache_fragment(settings: &ResolvedPostProcessSettings) -> String {
    let palette = settings
        .quantize_palette
        .colors()
        .iter()
        .take(4)
        .map(|color| {
            format!(
                "{:02x}{:02x}{:02x}{:02x}",
                color[0], color[1], color[2], color[3]
            )
        })
        .collect::<Vec<_>>()
        .join("-");
    format!(
        "{:?}:{}:{}:{:02x}{:02x}{:02x}{:02x}:{}:{}:{}:{}",
        settings.mode,
        settings.quantize_strategy as u8,
        palette,
        settings.tint_color[0],
        settings.tint_color[1],
        settings.tint_color[2],
        settings.tint_color[3],
        settings.tint_strength_percent,
        settings.brightness_percent,
        settings.saturation_percent,
        settings.gb_contrast_percent,
    )
}

fn apply_post_process_to_image(image: &mut DecodedImage, settings: &ResolvedPostProcessSettings) {
    if settings.mode == PostProcessMode::None {
        return;
    }

    let width = image.width.max(1);
    let height = image.height.max(1);
    for (pixel_index, rgba) in image.data.chunks_exact_mut(4).enumerate() {
        let x = (pixel_index as u32) % width;
        let y = (pixel_index as u32) / width;
        let uv = [
            (x as f32 + 0.5) / width as f32,
            (y as f32 + 0.5) / height as f32,
        ];
        let processed =
            apply_post_process_pixel_at(settings, [rgba[0], rgba[1], rgba[2], rgba[3]], [x, y], uv);
        rgba.copy_from_slice(&processed);
    }
}

fn apply_contrast(value: f32, contrast: f32) -> f32 {
    ((value - 0.5) * (1.0 + contrast) + 0.5).clamp(0.0, 1.0)
}

fn luminance(rgb: [f32; 3]) -> f32 {
    rgb[0] * 0.299 + rgb[1] * 0.587 + rgb[2] * 0.114
}

fn quantize_index(luminance: f32) -> usize {
    if luminance < 0.25 {
        0
    } else if luminance < 0.5 {
        1
    } else if luminance < 0.75 {
        2
    } else {
        3
    }
}

fn rgb_distance_sq(lhs: [f32; 3], rhs: [f32; 3]) -> f32 {
    let dr = lhs[0] - rhs[0];
    let dg = lhs[1] - rhs[1];
    let db = lhs[2] - rhs[2];
    dr * dr + dg * dg + db * db
}

fn nearest_palette_color(rgb: [f32; 3], palette: &Palette) -> [u8; 4] {
    let mut best = palette.color(0);
    let mut best_distance = f32::MAX;
    for &color in palette.colors() {
        let candidate_rgb = [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
        ];
        let distance = rgb_distance_sq(rgb, candidate_rgb);
        if distance < best_distance {
            best_distance = distance;
            best = color;
        }
    }
    best
}

fn apply_contrast_rgb(rgb: [f32; 3], contrast: f32) -> [f32; 3] {
    [
        apply_contrast(rgb[0], contrast),
        apply_contrast(rgb[1], contrast),
        apply_contrast(rgb[2], contrast),
    ]
}

fn apply_brightness_saturation(rgb: [f32; 3], brightness: f32, saturation: f32) -> [f32; 3] {
    let lum = luminance(rgb);
    let gray = [lum, lum, lum];
    [
        (gray[0] * (1.0 - saturation) + rgb[0] * saturation + brightness).clamp(0.0, 1.0),
        (gray[1] * (1.0 - saturation) + rgb[1] * saturation + brightness).clamp(0.0, 1.0),
        (gray[2] * (1.0 - saturation) + rgb[2] * saturation + brightness).clamp(0.0, 1.0),
    ]
}

fn bayer4x4_threshold(pixel: [u32; 2]) -> f32 {
    const BAYER4X4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let x = (pixel[0] % 4) as usize;
    let y = (pixel[1] % 4) as usize;
    (BAYER4X4[y][x] as f32 + 0.5) / 16.0
}

fn ordered_dither_quantize_index(lum: f32, pixel: [u32; 2]) -> usize {
    let threshold_bias = (bayer4x4_threshold(pixel) - 0.5) / 4.0;
    quantize_index((lum + threshold_bias).clamp(0.0, 1.0))
}

fn apply_vignette(rgb: [f32; 3], uv: [f32; 2], strength: f32) -> [f32; 3] {
    let dx = uv[0] * 2.0 - 1.0;
    let dy = uv[1] * 2.0 - 1.0;
    let dist = (dx * dx + dy * dy).sqrt();
    let edge = ((dist - 0.35) / (1.0 - 0.35)).clamp(0.0, 1.0);
    let smooth = edge * edge * (3.0 - 2.0 * edge);
    let vignette = 1.0 - smooth * strength;
    [rgb[0] * vignette, rgb[1] * vignette, rgb[2] * vignette]
}

fn apply_post_process_pixel_at(
    settings: &ResolvedPostProcessSettings,
    color: [u8; 4],
    pixel: [u32; 2],
    uv: [f32; 2],
) -> [u8; 4] {
    if color[3] == 0 {
        return color;
    }

    let alpha = color[3];
    let rgb = [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
    ];

    match settings.mode {
        PostProcessMode::None => color,
        PostProcessMode::Tint => {
            let tint = [
                settings.tint_color[0] as f32 / 255.0,
                settings.tint_color[1] as f32 / 255.0,
                settings.tint_color[2] as f32 / 255.0,
            ];
            let strength = settings.tint_strength_percent.min(100) as f32 / 100.0;
            let out = [
                rgb[0] * (1.0 - strength) + tint[0] * strength,
                rgb[1] * (1.0 - strength) + tint[1] * strength,
                rgb[2] * (1.0 - strength) + tint[2] * strength,
            ];
            [
                (out[0] * 255.0).round() as u8,
                (out[1] * 255.0).round() as u8,
                (out[2] * 255.0).round() as u8,
                alpha,
            ]
        }
        PostProcessMode::BrightnessSaturation => {
            let out = apply_brightness_saturation(
                rgb,
                settings.brightness_percent.clamp(-100, 100) as f32 / 100.0,
                settings.saturation_percent.min(200) as f32 / 100.0,
            );
            [
                (out[0] * 255.0).round() as u8,
                (out[1] * 255.0).round() as u8,
                (out[2] * 255.0).round() as u8,
                alpha,
            ]
        }
        PostProcessMode::Quantize4 => {
            let target = match settings.quantize_strategy {
                QuantizeStrategy::Luminance => {
                    let index = quantize_index(luminance(rgb));
                    settings.quantize_palette.color(index)
                }
                QuantizeStrategy::RgbDistance => {
                    nearest_palette_color(rgb, &settings.quantize_palette)
                }
            };
            [target[0], target[1], target[2], alpha]
        }
        PostProcessMode::OrderedDitherQuantize => {
            let index = ordered_dither_quantize_index(luminance(rgb), pixel);
            let target = settings.quantize_palette.color(index);
            [target[0], target[1], target[2], alpha]
        }
        PostProcessMode::GbPalette => {
            let gb_palette = builtin_palettes()
                .remove("gb_default")
                .or_else(|| {
                    Palette::new(
                        PaletteSize::Pal4,
                        vec![
                            [0x0F, 0x38, 0x0F, 0xFF],
                            [0x30, 0x62, 0x30, 0xFF],
                            [0x8B, 0xAC, 0x0F, 0xFF],
                            [0x9B, 0xBC, 0x0F, 0xFF],
                        ],
                    )
                    .ok()
                })
                .expect("GB palette should exist");
            let contrast = settings.gb_contrast_percent.clamp(-100, 100) as f32 / 100.0;
            let target = match settings.quantize_strategy {
                QuantizeStrategy::Luminance => {
                    let lum = apply_contrast(luminance(rgb), contrast);
                    let index = quantize_index(lum);
                    gb_palette.color(index)
                }
                QuantizeStrategy::RgbDistance => {
                    let adjusted = apply_contrast_rgb(rgb, contrast);
                    nearest_palette_color(adjusted, &gb_palette)
                }
            };
            [target[0], target[1], target[2], alpha]
        }
        PostProcessMode::Vignette => {
            let out = apply_vignette(
                rgb,
                uv,
                settings.vignette_strength_percent.min(100) as f32 / 100.0,
            );
            [
                (out[0] * 255.0).round() as u8,
                (out[1] * 255.0).round() as u8,
                (out[2] * 255.0).round() as u8,
                alpha,
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::atlas::TileInfo;
    use crate::assets::tilemap::{TileLayer, TileMap};
    use crate::assets::tileset::{TileSetAtlasSource, TileSetMeta, TileSetResolver};
    use crate::graphics::image::save_image_rgba8;
    use glam::UVec2;
    use std::collections::HashMap;

    #[test]
    fn resolve_indexed_palette_prefers_project_override() {
        let mut palettes = crate::palette::builtin_palettes();
        palettes.insert(
            "custom".to_string(),
            Palette::new(PaletteSize::Pal4, vec![[1, 2, 3, 255]; 4]).unwrap(),
        );
        let settings = IndexedPresentationSettings {
            indexed_palette_override: Some("custom".to_string()),
            post_process: RuntimePostProcessSettings::default(),
        };

        let resolved = resolve_indexed_palette(
            ColorMode::PaletteIndexed,
            &palettes,
            &settings,
            Some("poison"),
            Some("gb_default"),
        )
        .expect("palette should resolve")
        .expect("indexed palette should resolve");

        assert_eq!(resolved.0, "custom");
    }

    #[test]
    fn materialize_indexed_image_recolors_with_asset_palette() {
        let palettes = crate::palette::builtin_palettes();
        let image = DecodedImage {
            width: 1,
            height: 1,
            data: vec![0, 0, 0, 255],
        };
        let resolved = materialize_indexed_image(
            &image,
            "memory://atlas",
            ColorMode::PaletteIndexed,
            &palettes,
            &IndexedPresentationSettings::default(),
            IndexedImageMaterialization {
                local_override: None,
                asset_palette: Some("poison"),
                apply_post_process: false,
            },
        )
        .expect("presentation should resolve");

        assert_eq!(resolved.palette_id.as_deref(), Some("poison"));
        let poison = resolve_palette("poison", &palettes).expect("poison palette should exist");
        assert_eq!(resolved.image.data[..4].to_vec(), poison.color(0).to_vec());
    }

    #[test]
    fn materialize_tilemap_batches_recolors_indexed_tiles() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let texture_path = temp_dir.path().join("atlas.png");
        save_image_rgba8(&texture_path, 1, 1, &[0, 0, 0, 255]).expect("png should save");

        let atlas = crate::assets::atlas::AtlasMeta {
            image: "atlas.png".into(),
            tile_size: UVec2::new(1, 1),
            color_mode: ColorMode::PaletteIndexed,
            palette: Some("poison".to_string()),
            palette_size: None,
            tiles: HashMap::from([(
                "tile".to_string(),
                TileInfo {
                    position: UVec2::ZERO,
                    properties: Default::default(),
                },
            )]),
            auto_tile_groups: HashMap::new(),
            animated_tiles: HashMap::new(),
            imported_auto_tiles: Vec::new(),
        };
        let tileset = TileSetMeta::from_atlas("atlas", &atlas);
        let atlases = HashMap::from([(
            "atlas".to_string(),
            TileSetAtlasSource {
                name: "atlas".to_string(),
                path: temp_dir.path().join("atlas.json"),
                meta: atlas,
            },
        )]);
        let resolver = TileSetResolver::new(&tileset, &atlases);
        let tilemap = TileMap {
            size: UVec2::new(1, 1),
            tile_size: UVec2::new(1, 1),
            tileset: "atlas.json".into(),
            layers: vec![TileLayer::new(
                "ground",
                vec!["atlas/tile/tile".to_string()],
            )],
        };

        let batches = tilemap
            .generate_render_batches(&resolver, None, None)
            .expect("batches should generate");
        let presented = materialize_tilemap_batches(
            batches,
            &crate::palette::builtin_palettes(),
            &IndexedPresentationSettings::default(),
        )
        .expect("batches should materialize");

        match &presented[0].texture {
            PresentedTextureSource::Rgba8 { image, .. } => {
                let poison =
                    resolve_palette("poison", &crate::palette::builtin_palettes()).unwrap();
                assert_eq!(image.data[..4].to_vec(), poison.color(0).to_vec());
            }
            PresentedTextureSource::File(_) => panic!("expected rgba8 presentation"),
        }
    }
}
