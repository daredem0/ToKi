#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeMap;

use toki_core::assets::atlas::ColorMode;
use toki_core::indexed_presentation::{
    resolve_indexed_palette as core_resolve_indexed_palette, IndexedPresentationSettings,
};
use toki_core::palette::Palette;

#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use toki_core::indexed_presentation::{
    load_materialized_indexed_image, ResolvedIndexedPresentation,
};

pub fn resolve_indexed_preview_palette(
    color_mode: ColorMode,
    available_palettes: &BTreeMap<String, Palette>,
    settings: &IndexedPresentationSettings,
    local_override: Option<&str>,
    asset_palette: Option<&str>,
) -> Result<Option<(String, Palette)>, String> {
    core_resolve_indexed_palette(
        color_mode,
        available_palettes,
        settings,
        local_override,
        asset_palette,
    )
}

#[cfg(test)]
pub fn load_texture_preview_image(
    texture_path: &Path,
    color_mode: ColorMode,
    available_palettes: &BTreeMap<String, Palette>,
    settings: &IndexedPresentationSettings,
    local_override: Option<&str>,
    asset_palette: Option<&str>,
) -> Result<ResolvedIndexedPresentation, String> {
    load_materialized_indexed_image(
        texture_path,
        color_mode,
        available_palettes,
        settings,
        local_override,
        asset_palette,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use toki_core::graphics::image::save_image_rgba8;
    use toki_core::palette::{resolve_palette, PaletteSize};

    #[test]
    fn resolve_indexed_preview_palette_prefers_project_override() {
        let mut palettes = toki_core::palette::builtin_palettes();
        palettes.insert(
            "custom".to_string(),
            Palette::new(PaletteSize::Pal4, vec![[1, 2, 3, 255]; 4]).unwrap(),
        );

        let settings = IndexedPresentationSettings {
            indexed_palette_override: Some("custom".to_string()),
            ..Default::default()
        };
        let resolved = resolve_indexed_preview_palette(
            ColorMode::PaletteIndexed,
            &palettes,
            &settings,
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
        let presentation = load_texture_preview_image(
            &png_path,
            ColorMode::PaletteIndexed,
            &palettes,
            &IndexedPresentationSettings::default(),
            Some("poison"),
            None,
        )
        .expect("preview image should load");

        assert_eq!(presentation.palette_id.as_deref(), Some("poison"));
        let poison = resolve_palette("poison", &palettes).expect("poison palette should exist");
        assert_eq!(
            presentation.image.data[..4].to_vec(),
            poison.color(0).to_vec()
        );
    }
}
