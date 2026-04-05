use glam::UVec2;
use std::collections::{BTreeMap, HashMap};
use toki_core::assets::atlas::{AtlasMeta, ColorMode, ImportedAutoTile, TileInfo, TileProperties};
use toki_core::graphics::image::DecodedImage;
use toki_core::palette::{recolor_indexed_image, resolve_palette, Palette};

fn indexed_palette_id(atlas: &AtlasMeta) -> &str {
    atlas.palette.as_deref().unwrap_or("gb_default")
}

fn prepare_source_image_for_import(
    base_atlas: &AtlasMeta,
    source_atlas: &AtlasMeta,
    source_image: &DecodedImage,
    available_palettes: &BTreeMap<String, Palette>,
) -> Result<DecodedImage, String> {
    match (base_atlas.color_mode, source_atlas.color_mode) {
        (ColorMode::TrueColor, ColorMode::TrueColor) => Ok(source_image.clone()),
        (ColorMode::TrueColor, ColorMode::PaletteIndexed) => {
            let palette_id = indexed_palette_id(source_atlas);
            let palette = resolve_palette(palette_id, available_palettes).ok_or_else(|| {
                format!(
                    "Missing palette '{}' required by indexed auto-tile import",
                    palette_id
                )
            })?;
            recolor_indexed_image(source_image, &palette).map_err(|error| {
                format!(
                    "Failed to bake indexed auto-tile '{}' into truecolor atlas: {}",
                    palette_id, error
                )
            })
        }
        (ColorMode::PaletteIndexed, ColorMode::PaletteIndexed) => {
            let base_palette_id = indexed_palette_id(base_atlas);
            let source_palette_id = indexed_palette_id(source_atlas);
            if base_atlas.effective_palette_size() != source_atlas.effective_palette_size() {
                return Err(format!(
                    "Palette size mismatch: base={}, source={}",
                    base_atlas.effective_palette_size(),
                    source_atlas.effective_palette_size()
                ));
            }
            if base_palette_id != source_palette_id {
                return Err(format!(
                    "Palette mismatch: base='{}', source='{}'. Import into indexed atlases requires the same palette.",
                    base_palette_id, source_palette_id
                ));
            }
            Ok(source_image.clone())
        }
        (ColorMode::PaletteIndexed, ColorMode::TrueColor) => {
            Err("Cannot import a truecolor auto-tile into a palette-indexed atlas.".to_string())
        }
    }
}

/// Merge auto-tile tiles from a source atlas into a base atlas image and metadata.
pub fn import_auto_tile_into_atlas(
    base_atlas: &mut AtlasMeta,
    base_image: &mut DecodedImage,
    source_atlas: &AtlasMeta,
    source_image: &DecodedImage,
    source_path: &std::path::Path,
    available_palettes: &BTreeMap<String, Palette>,
) -> Result<(), String> {
    let group = source_atlas
        .auto_tile_groups
        .iter()
        .next()
        .ok_or("Source atlas has no auto-tile groups")?;
    let (group_name, group_def) = (group.0.clone(), group.1.clone());

    if base_atlas.auto_tile_groups.contains_key(&group_name) {
        return Err(format!(
            "Auto-tile group '{}' already exists in target atlas",
            group_name
        ));
    }

    if base_atlas.tile_size != source_atlas.tile_size {
        return Err(format!(
            "Tile size mismatch: base={}x{}, source={}x{}",
            base_atlas.tile_size.x,
            base_atlas.tile_size.y,
            source_atlas.tile_size.x,
            source_atlas.tile_size.y
        ));
    }

    let prepared_source_image = prepare_source_image_for_import(
        base_atlas,
        source_atlas,
        source_image,
        available_palettes,
    )?;
    let base_cols = base_image.width / base_atlas.tile_size.x;
    let positions =
        append_tiles_to_image(base_image, &prepared_source_image, source_atlas, base_cols);

    let mut tile_names = Vec::new();
    let mut remapped_variants = HashMap::new();
    for (mask, source_tile_name) in &group_def.variants {
        let Some(source_tile) = source_atlas.tiles.get(source_tile_name) else {
            continue;
        };
        let src_idx = (source_tile.position.y * source_cols(&prepared_source_image, source_atlas)
            + source_tile.position.x) as usize;
        let Some(&new_pos) = positions.get(src_idx) else {
            continue;
        };
        let merged_name = format!("{}_{}", group_name, mask);
        base_atlas.tiles.insert(
            merged_name.clone(),
            TileInfo {
                position: new_pos,
                properties: source_tile.properties.clone(),
            },
        );
        remapped_variants.insert(*mask, merged_name.clone());
        tile_names.push(merged_name);
    }

    let mut merged_group = group_def;
    merged_group.variants = remapped_variants;
    if let Some(ref preview) = merged_group.preview_tile {
        if let Some(src_tile) = source_atlas.tiles.get(preview) {
            let src_idx = (src_tile.position.y * source_cols(&prepared_source_image, source_atlas)
                + src_tile.position.x) as usize;
            if let Some(&new_pos) = positions.get(src_idx) {
                let preview_name = format!("{}_preview", group_name);
                if !base_atlas.tiles.contains_key(&preview_name) {
                    base_atlas.tiles.insert(
                        preview_name.clone(),
                        TileInfo {
                            position: new_pos,
                            properties: TileProperties::default(),
                        },
                    );
                }
                merged_group.preview_tile = Some(preview_name);
            }
        }
    }

    base_atlas
        .auto_tile_groups
        .insert(group_name.clone(), merged_group);
    base_atlas.imported_auto_tiles.push(ImportedAutoTile {
        source_path: source_path.to_path_buf(),
        group_name,
        tile_names,
    });

    Ok(())
}

/// Remove an imported auto-tile group from the atlas.
pub fn remove_auto_tile_from_atlas(atlas: &mut AtlasMeta, group_name: &str) {
    let Some(idx) = atlas
        .imported_auto_tiles
        .iter()
        .position(|i| i.group_name == group_name)
    else {
        return;
    };
    let import = atlas.imported_auto_tiles.remove(idx);
    for tile_name in &import.tile_names {
        atlas.tiles.remove(tile_name);
    }
    atlas.auto_tile_groups.remove(group_name);
}

fn source_cols(source_image: &DecodedImage, source_atlas: &AtlasMeta) -> u32 {
    if source_atlas.tile_size.x == 0 {
        return 1;
    }
    source_image.width / source_atlas.tile_size.x
}

/// Appends all tiles from the source image into the base image.
/// Returns the new grid positions for each source tile (indexed row-major).
fn append_tiles_to_image(
    base_image: &mut DecodedImage,
    source_image: &DecodedImage,
    source_atlas: &AtlasMeta,
    base_cols: u32,
) -> Vec<UVec2> {
    let tw = source_atlas.tile_size.x;
    let th = source_atlas.tile_size.y;
    let src_cols = source_cols(source_image, source_atlas);
    let src_rows = if th == 0 { 0 } else { source_image.height / th };
    let tile_count = (src_cols * src_rows) as usize;

    // Find first free row in base image
    let base_rows = if th == 0 { 0 } else { base_image.height / th };
    let insert_row = base_rows;

    // Calculate how many new rows we need
    let new_rows_needed = if base_cols == 0 {
        0
    } else {
        (tile_count as u32).div_ceil(base_cols)
    };
    let new_height = base_image.height + new_rows_needed * th;
    let new_width = base_image.width;

    // Expand the base image
    let mut new_data = vec![0u8; (new_width * new_height * 4) as usize];
    for y in 0..base_image.height {
        let src_offset = (y * base_image.width * 4) as usize;
        let dst_offset = (y * new_width * 4) as usize;
        let row_bytes = (base_image.width * 4) as usize;
        new_data[dst_offset..dst_offset + row_bytes]
            .copy_from_slice(&base_image.data[src_offset..src_offset + row_bytes]);
    }

    // Copy source tiles into the new rows
    let mut positions = Vec::with_capacity(tile_count);
    let row_bytes = (tw * 4) as usize;
    for src_idx in 0..tile_count {
        let src_x = ((src_idx as u32) % src_cols) * tw;
        let src_y = ((src_idx as u32) / src_cols) * th;
        let dst_col = (src_idx as u32) % base_cols;
        let dst_row = insert_row + (src_idx as u32) / base_cols;
        let dst_x = dst_col * tw;
        let dst_y = dst_row * th;
        positions.push(UVec2::new(dst_col, dst_row));

        for py in 0..th {
            let so = ((src_y + py) * source_image.width + src_x) as usize * 4;
            let do_ = ((dst_y + py) * new_width + dst_x) as usize * 4;
            if so + row_bytes <= source_image.data.len() && do_ + row_bytes <= new_data.len() {
                new_data[do_..do_ + row_bytes]
                    .copy_from_slice(&source_image.data[so..so + row_bytes]);
            }
        }
    }

    base_image.width = new_width;
    base_image.height = new_height;
    base_image.data = new_data;
    positions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use toki_core::assets::autotile::{AutoTileGroup, AutoTileMode};
    use toki_core::palette::{builtin_palettes, resolve_palette};

    fn make_test_image(width: u32, height: u32) -> DecodedImage {
        DecodedImage {
            width,
            height,
            data: vec![128u8; (width * height * 4) as usize],
        }
    }

    fn make_source_atlas(group_name: &str) -> (AtlasMeta, DecodedImage) {
        let tile_size = UVec2::new(8, 8);
        let mut tiles = HashMap::new();
        let mut variants = HashMap::new();
        for i in 0u8..4 {
            let name = format!("{group_name}_{i}");
            tiles.insert(
                name.clone(),
                TileInfo {
                    position: UVec2::new(u32::from(i) % 2, u32::from(i) / 2),
                    properties: TileProperties::default(),
                },
            );
            variants.insert(i, name);
        }
        let mut auto_tile_groups = HashMap::new();
        auto_tile_groups.insert(
            group_name.to_string(),
            AutoTileGroup {
                mode: AutoTileMode::FourBit,
                preview_tile: Some(format!("{group_name}_3")),
                variants,
            },
        );
        let atlas = AtlasMeta {
            image: PathBuf::from("source.png"),
            tile_size,
            color_mode: toki_core::assets::atlas::ColorMode::TrueColor,
            palette: None,
            palette_size: None,
            tiles,
            auto_tile_groups,
            animated_tiles: HashMap::new(),
            imported_auto_tiles: Vec::new(),
        };
        let image = make_test_image(16, 16); // 2x2 grid of 8x8 tiles
        (atlas, image)
    }

    #[test]
    fn import_adds_tiles_and_group_to_base_atlas() {
        let mut base = AtlasMeta::new_single_tile("base.png", UVec2::new(8, 8));
        let mut base_image = make_test_image(8, 8); // 1x1 tile
        let (source, source_image) = make_source_atlas("grass");

        let result = import_auto_tile_into_atlas(
            &mut base,
            &mut base_image,
            &source,
            &source_image,
            std::path::Path::new("grass_autotile.json"),
            &builtin_palettes(),
        );
        assert!(result.is_ok(), "import failed: {:?}", result);
        assert!(base.auto_tile_groups.contains_key("grass"));
        assert_eq!(base.imported_auto_tiles.len(), 1);
        assert_eq!(base.imported_auto_tiles[0].group_name, "grass");
        // Base image should have expanded
        assert!(base_image.height > 8);
    }

    #[test]
    fn import_rejects_duplicate_group() {
        let mut base = AtlasMeta::new_single_tile("base.png", UVec2::new(8, 8));
        let mut base_image = make_test_image(8, 8);
        let (source, source_image) = make_source_atlas("grass");

        import_auto_tile_into_atlas(
            &mut base,
            &mut base_image,
            &source,
            &source_image,
            std::path::Path::new("grass.json"),
            &builtin_palettes(),
        )
        .unwrap();
        let result = import_auto_tile_into_atlas(
            &mut base,
            &mut base_image,
            &source,
            &source_image,
            std::path::Path::new("grass.json"),
            &builtin_palettes(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn remove_strips_tiles_and_group() {
        let mut base = AtlasMeta::new_single_tile("base.png", UVec2::new(8, 8));
        let mut base_image = make_test_image(8, 8);
        let (source, source_image) = make_source_atlas("grass");

        import_auto_tile_into_atlas(
            &mut base,
            &mut base_image,
            &source,
            &source_image,
            std::path::Path::new("grass.json"),
            &builtin_palettes(),
        )
        .unwrap();

        let tile_count_before = base.tiles.len();
        remove_auto_tile_from_atlas(&mut base, "grass");
        assert!(!base.auto_tile_groups.contains_key("grass"));
        assert!(base.imported_auto_tiles.is_empty());
        assert!(base.tiles.len() < tile_count_before);
    }

    #[test]
    fn import_bakes_indexed_auto_tile_when_target_atlas_is_truecolor() {
        let mut base = AtlasMeta::new_single_tile("base.png", UVec2::new(8, 8));
        let mut base_image = make_test_image(8, 8);
        let (mut source, mut source_image) = make_source_atlas("grass");
        source.color_mode = ColorMode::PaletteIndexed;
        source.palette = Some("poison".to_string());

        source_image.data.fill(0);
        for rgba in source_image.data.chunks_exact_mut(4) {
            rgba[3] = 0xFF;
        }

        import_auto_tile_into_atlas(
            &mut base,
            &mut base_image,
            &source,
            &source_image,
            std::path::Path::new("grass.json"),
            &builtin_palettes(),
        )
        .expect("indexed source should bake into truecolor target");

        let poison = resolve_palette("poison", &builtin_palettes()).expect("poison palette");
        let appended_offset = (base.tile_size.y * base_image.width * 4) as usize;
        assert_eq!(
            &base_image.data[appended_offset..appended_offset + 4],
            &poison.color(0)
        );
        assert_eq!(base.color_mode, ColorMode::TrueColor);
    }

    #[test]
    fn import_rejects_palette_indexed_auto_tile_with_different_palette() {
        let mut base = AtlasMeta::new_single_tile("base.png", UVec2::new(8, 8));
        base.color_mode = ColorMode::PaletteIndexed;
        base.palette = Some("gb_default".to_string());
        let mut base_image = make_test_image(8, 8);

        let (mut source, source_image) = make_source_atlas("grass");
        source.color_mode = ColorMode::PaletteIndexed;
        source.palette = Some("poison".to_string());

        let error = import_auto_tile_into_atlas(
            &mut base,
            &mut base_image,
            &source,
            &source_image,
            std::path::Path::new("grass.json"),
            &builtin_palettes(),
        )
        .expect_err("mismatched indexed palettes should be rejected");

        assert!(error.contains("Palette mismatch"));
    }
}
