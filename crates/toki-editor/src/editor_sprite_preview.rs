use std::collections::BTreeMap;
use std::path::Path;

use toki_core::assets::atlas::ColorMode;
use toki_core::graphics::image::{load_image_rgba8, DecodedImage};
use toki_core::palette::{recolor_indexed_image, resolve_palette, Palette};

pub fn resolve_indexed_preview_palette(
    color_mode: ColorMode,
    available_palettes: &BTreeMap<String, Palette>,
    global_override: Option<&str>,
    local_override: Option<&str>,
    atlas_palette: Option<&str>,
) -> Result<Option<(String, Palette)>, String> {
    if color_mode != ColorMode::PaletteIndexed {
        return Ok(None);
    }

    for palette_id in [
        global_override,
        local_override,
        atlas_palette,
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

pub fn texture_preview_cache_key(
    texture_path: &Path,
    color_mode: ColorMode,
    palette_id: Option<&str>,
) -> String {
    format!(
        "{}#{:?}#{}",
        texture_path.display(),
        color_mode,
        palette_id.unwrap_or("")
    )
}

pub fn load_texture_preview_image(
    texture_path: &Path,
    color_mode: ColorMode,
    available_palettes: &BTreeMap<String, Palette>,
    global_override: Option<&str>,
    local_override: Option<&str>,
    atlas_palette: Option<&str>,
) -> Result<(DecodedImage, Option<String>), String> {
    let decoded = load_image_rgba8(texture_path).map_err(|error| error.to_string())?;
    let Some((palette_id, palette)) = resolve_indexed_preview_palette(
        color_mode,
        available_palettes,
        global_override,
        local_override,
        atlas_palette,
    )?
    else {
        return Ok((decoded, None));
    };

    let recolored = recolor_indexed_image(&decoded, &palette).map_err(|error| error.to_string())?;
    Ok((recolored, Some(palette_id)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use toki_core::graphics::image::save_image_rgba8;
    use toki_core::palette::PaletteSize;

    #[test]
    fn resolve_indexed_preview_palette_prefers_global_override() {
        let mut palettes = toki_core::palette::builtin_palettes();
        palettes.insert(
            "custom".to_string(),
            Palette::new(PaletteSize::Pal4, vec![[1, 2, 3, 255]; 4]).unwrap(),
        );

        let resolved = resolve_indexed_preview_palette(
            ColorMode::PaletteIndexed,
            &palettes,
            Some("custom"),
            Some("poison"),
            Some("gb_default"),
        )
        .expect("palette should resolve")
        .expect("indexed preview should need a palette");

        assert_eq!(resolved.0, "custom");
        assert_eq!(resolved.1.colors(), &[[1, 2, 3, 255]; 4]);
    }

    #[test]
    fn load_texture_preview_image_recolors_indexed_source() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let png_path = temp_dir.path().join("atlas.png");
        save_image_rgba8(&png_path, 1, 1, &[0x00, 0x00, 0x00, 0xFF]).expect("png should save");

        let palettes = toki_core::palette::builtin_palettes();
        let (image, palette_id) = load_texture_preview_image(
            &png_path,
            ColorMode::PaletteIndexed,
            &palettes,
            None,
            Some("poison"),
            None,
        )
        .expect("preview image should load");

        assert_eq!(palette_id.as_deref(), Some("poison"));
        let poison = resolve_palette("poison", &palettes).expect("poison palette should exist");
        assert_eq!(image.data[..4].to_vec(), poison.color(0).to_vec());
    }
}
