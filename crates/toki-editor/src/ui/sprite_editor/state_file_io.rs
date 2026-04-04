//! File I/O operations for SpriteEditorState.

use super::{
    canonical_indexed_color_for_size, DiscoveredSpriteAsset, PixelColor, SpriteAssetKind,
    SpriteCanvas, SpriteCanvasViewport, SpriteEditorState,
};

struct SaveAssetRequest<'a> {
    json_path: &'a std::path::Path,
    png_path: &'a std::path::Path,
    png_filename: &'a str,
    name: &'a str,
    kind: SpriteAssetKind,
    canvas_width: u32,
    canvas_height: u32,
    pixels: &'a [u8],
    source_metadata_path: Option<&'a std::path::Path>,
}

impl SpriteEditorState {
    /// Reset canvas state with a new canvas. Sets common defaults.
    fn reset_canvas_state(&mut self, canvas: SpriteCanvas, dirty: bool) {
        let cs = self.active_mut();
        cs.canvas = Some(canvas);
        cs.active_sprite = None;
        cs.asset_kind = None;
        cs.dirty = dirty;
        cs.history.clear();
        cs.selection = None;
        cs.canvas_texture = None;
        cs.viewport = SpriteCanvasViewport::default();
        cs.selected_cell = None;
        cs.original_cell_aliases = None;
        cs.show_cell_grid = false;
        self.color_mode = toki_core::assets::atlas::ColorMode::TrueColor;
        self.selected_palette_id = None;
    }

    /// Open the save dialog
    pub fn begin_save_dialog(&mut self) {
        self.show_save_dialog = true;
        let cs = self.active_mut();
        if cs.save_asset_name.is_empty() {
            cs.save_asset_name = "new_sprite".to_string();
        }
    }

    /// Open the load dialog and scan for assets
    pub fn begin_load_dialog(&mut self, sprites_dir: &std::path::Path) {
        self.discovered_assets = Self::scan_sprite_assets(sprites_dir);
        self.selected_asset_index = None;
        self.show_load_dialog = true;
    }

    /// Open the new-canvas dialog with an image already selected as the source.
    pub fn begin_new_canvas_from_image_dialog(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), String> {
        use toki_core::graphics::image::load_image_rgba8;

        let decoded = load_image_rgba8(path).map_err(|e| format!("Failed to load image: {e}"))?;
        self.new_canvas_source_image = Some(path.to_path_buf());
        self.new_canvas_source_image_size = Some(glam::UVec2::new(decoded.width, decoded.height));
        self.new_canvas_is_sheet = true;
        self.new_canvas_error = None;
        self.show_new_canvas_dialog = true;
        Ok(())
    }

    /// Open the merge dialog and scan for assets
    pub fn begin_merge_dialog(&mut self, sprites_dir: &std::path::Path) {
        self.discovered_assets = Self::scan_sprite_assets(sprites_dir);
        self.merge_selected_indices.clear();
        self.merge_target_cols = 4;
        self.show_merge_dialog = true;
    }

    /// Toggle selection of an asset for merging
    pub fn toggle_merge_selection(&mut self, index: usize) {
        if let Some(pos) = self.merge_selected_indices.iter().position(|&i| i == index) {
            self.merge_selected_indices.remove(pos);
        } else {
            self.merge_selected_indices.push(index);
        }
    }

    /// Rename a sprite asset (both PNG and JSON files)
    pub fn rename_asset(
        sprites_dir: &std::path::Path,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), String> {
        if new_name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        if new_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
            return Err("Name contains invalid characters".to_string());
        }
        if old_name == new_name {
            return Ok(());
        }

        let old_png = sprites_dir.join(format!("{old_name}.png"));
        let old_json = sprites_dir.join(format!("{old_name}.json"));
        let new_png = sprites_dir.join(format!("{new_name}.png"));
        let new_json = sprites_dir.join(format!("{new_name}.json"));

        if !old_png.exists() {
            return Err(format!("Source PNG not found: {}", old_png.display()));
        }
        if new_png.exists() {
            return Err(format!("Target already exists: {}", new_png.display()));
        }

        std::fs::rename(&old_png, &new_png).map_err(|e| format!("Failed to rename PNG: {e}"))?;

        if old_json.exists() {
            std::fs::rename(&old_json, &new_json)
                .map_err(|e| format!("Failed to rename JSON: {e}"))?;
        }

        Ok(())
    }

    /// Delete a sprite asset (both PNG and JSON files)
    pub fn delete_asset(sprites_dir: &std::path::Path, name: &str) -> Result<(), String> {
        let png_path = sprites_dir.join(format!("{name}.png"));
        let json_path = sprites_dir.join(format!("{name}.json"));

        if png_path.exists() {
            std::fs::remove_file(&png_path).map_err(|e| format!("Failed to delete PNG: {e}"))?;
        }
        if json_path.exists() {
            std::fs::remove_file(&json_path).map_err(|e| format!("Failed to delete JSON: {e}"))?;
        }

        Ok(())
    }

    /// Scan a sprites directory for available sprite assets
    pub fn scan_sprite_assets(sprites_dir: &std::path::Path) -> Vec<DiscoveredSpriteAsset> {
        use toki_core::project_assets::{classify_sprite_metadata_file, SpriteMetadataFileKind};

        let mut assets = Vec::new();
        let Ok(entries) = std::fs::read_dir(sprites_dir) else {
            return assets;
        };

        for entry in entries.flatten() {
            if let Some((json_path, name, png_path)) = classify_json_entry(&entry) {
                if let Ok(kind) = classify_sprite_metadata_file(&json_path) {
                    let sprite_kind = match kind {
                        SpriteMetadataFileKind::Atlas => SpriteAssetKind::TileAtlas,
                        SpriteMetadataFileKind::ObjectSheet => SpriteAssetKind::ObjectSheet,
                        SpriteMetadataFileKind::Unknown => continue,
                    };
                    assets.push(DiscoveredSpriteAsset {
                        name,
                        json_path,
                        png_path,
                        kind: sprite_kind,
                    });
                }
            }
        }

        assets.sort_by(|a, b| a.name.cmp(&b.name));
        assets
    }

    /// Load an existing sprite asset into the canvas
    pub fn load_sprite_asset(&mut self, asset: &DiscoveredSpriteAsset) -> Result<(), String> {
        use toki_core::assets::atlas::AtlasMeta;
        use toki_core::assets::object_sheet::ObjectSheetMeta;
        use toki_core::graphics::image::load_image_rgba8;

        let decoded =
            load_image_rgba8(&asset.png_path).map_err(|e| format!("Failed to load image: {e}"))?;

        let canvas = SpriteCanvas::from_rgba(decoded.width, decoded.height, decoded.data)
            .ok_or_else(|| "Failed to create canvas from image data".to_string())?;

        let (cell_size, is_sheet, original_aliases, color_mode, selected_palette_id) =
            match asset.kind {
                SpriteAssetKind::TileAtlas => {
                    let meta = AtlasMeta::load_from_file(&asset.json_path)
                        .map_err(|e| format!("Failed to load atlas metadata: {e}"))?;
                    let is_sheet = meta.tiles.len() > 1;
                    let aliases = ordered_atlas_aliases(
                        &meta,
                        decoded.width / meta.tile_size.x.max(1),
                        decoded.height / meta.tile_size.y.max(1),
                    );
                    (
                        meta.tile_size,
                        is_sheet,
                        aliases,
                        meta.color_mode,
                        meta.palette.clone(),
                    )
                }
                SpriteAssetKind::ObjectSheet => {
                    let meta = ObjectSheetMeta::load_from_file(&asset.json_path)
                        .map_err(|e| format!("Failed to load object sheet metadata: {e}"))?;
                    let is_sheet = meta.objects.len() > 1;
                    let aliases = ordered_object_aliases(
                        &meta,
                        decoded.width / meta.tile_size.x.max(1),
                        decoded.height / meta.tile_size.y.max(1),
                    );
                    (
                        meta.tile_size,
                        is_sheet,
                        aliases,
                        meta.color_mode,
                        meta.palette.clone(),
                    )
                }
            };

        self.reset_canvas_state(canvas, false);
        self.color_mode = color_mode;
        self.selected_palette_id = selected_palette_id;
        if self.color_mode == toki_core::assets::atlas::ColorMode::PaletteIndexed {
            // Default to the last canonical shade (white) for the palette size.
            // The palette size isn't available here, so we use Pal4 as a safe default.
            self.foreground_color =
                canonical_indexed_color_for_size(3, toki_core::palette::PaletteSize::Pal4);
        }
        let cs = self.active_mut();
        cs.active_sprite = Some(asset.json_path.to_string_lossy().to_string());
        cs.asset_kind = Some(asset.kind);
        cs.save_asset_name = asset.name.clone();
        cs.save_asset_kind = asset.kind;
        cs.original_cell_aliases = Some(original_aliases);
        cs.cell_size = cell_size;
        cs.show_cell_grid = is_sheet;
        self.show_load_dialog = false;
        Ok(())
    }

    /// Merge selected sprites into a new sheet canvas
    pub fn merge_sprites_into_sheet(&mut self) -> Result<(), String> {
        if self.merge_selected_indices.is_empty() {
            return Err("No sprites selected for merge".to_string());
        }

        let (images, max_width, max_height) = self.load_merge_images()?;
        let canvas = self.create_merged_canvas(&images, max_width, max_height);
        self.reset_canvas_state(canvas, true);
        let cs = self.active_mut();
        cs.cell_size = glam::UVec2::new(max_width, max_height);
        cs.show_cell_grid = true;
        self.show_merge_dialog = false;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn load_merge_images(&self) -> Result<(Vec<(u32, u32, Vec<u8>)>, u32, u32), String> {
        use toki_core::graphics::image::load_image_rgba8;

        let mut images = Vec::new();
        let mut max_width = 0u32;
        let mut max_height = 0u32;

        for &idx in &self.merge_selected_indices {
            let asset = self
                .discovered_assets
                .get(idx)
                .ok_or_else(|| "Invalid asset index".to_string())?;

            let decoded = load_image_rgba8(&asset.png_path)
                .map_err(|e| format!("Failed to load {}: {e}", asset.name))?;

            max_width = max_width.max(decoded.width);
            max_height = max_height.max(decoded.height);
            images.push((decoded.width, decoded.height, decoded.data));
        }

        Ok((images, max_width, max_height))
    }

    fn create_merged_canvas(
        &self,
        images: &[(u32, u32, Vec<u8>)],
        cell_w: u32,
        cell_h: u32,
    ) -> SpriteCanvas {
        let cols = self.merge_target_cols.max(1);
        let rows = (images.len() as u32).div_ceil(cols);
        let sheet_w = cols * cell_w;
        let sheet_h = rows * cell_h;

        let mut canvas = SpriteCanvas::new(sheet_w, sheet_h);

        for (i, (img_w, img_h, data)) in images.iter().enumerate() {
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            let start_x = col * cell_w;
            let start_y = row * cell_h;
            let offset_x = (cell_w - img_w) / 2;
            let offset_y = (cell_h - img_h) / 2;

            copy_image_to_canvas(
                &mut canvas,
                data,
                *img_w,
                *img_h,
                start_x + offset_x,
                start_y + offset_y,
            );
        }

        canvas
    }

    /// Save the current canvas as a sprite asset.
    pub fn save_as_asset(&mut self, sprites_dir: &std::path::Path) -> Result<(), String> {
        let cs = self.active();
        let canvas = cs.canvas.as_ref().ok_or("No canvas to save")?;
        let name = cs.save_asset_name.trim();
        if name.is_empty() {
            return Err("Asset name cannot be empty".to_string());
        }

        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(
                "Asset name can only contain letters, numbers, and underscores".to_string(),
            );
        }

        std::fs::create_dir_all(sprites_dir)
            .map_err(|e| format!("Failed to create sprites directory: {e}"))?;

        let name = name.to_string();
        let png_filename = format!("{name}.png");
        let png_path = sprites_dir.join(&png_filename);
        let json_path = sprites_dir.join(format!("{name}.json"));
        let save_asset_kind = cs.save_asset_kind;
        let canvas_width = canvas.width;
        let canvas_height = canvas.height;
        let pixels = canvas.pixels().to_vec();
        let source_metadata_path = cs.active_sprite.clone();

        self.save_asset_to_paths(SaveAssetRequest {
            json_path: &json_path,
            png_path: &png_path,
            png_filename: &png_filename,
            name: &name,
            kind: save_asset_kind,
            canvas_width,
            canvas_height,
            pixels: &pixels,
            source_metadata_path: source_metadata_path.as_deref().map(std::path::Path::new),
        })
    }

    /// Save the current canvas back to its existing sprite asset paths.
    pub fn save_current_asset(&mut self) -> Result<(), String> {
        let (canvas_width, canvas_height, pixels, active_sprite, save_asset_kind) = {
            let cs = self.active();
            let canvas = cs.canvas.as_ref().ok_or("No canvas to save")?;
            (
                canvas.width,
                canvas.height,
                canvas.pixels().to_vec(),
                cs.active_sprite
                    .clone()
                    .ok_or_else(|| "No existing sprite asset to save".to_string())?,
                cs.asset_kind
                    .ok_or_else(|| "Missing existing sprite asset kind".to_string())?,
            )
        };
        let json_path = std::path::PathBuf::from(active_sprite);
        let name = json_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("sprite")
            .to_string();
        let png_filename = self.resolve_existing_png_filename(&json_path, save_asset_kind);
        let png_path = json_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(&png_filename);

        self.save_asset_to_paths(SaveAssetRequest {
            json_path: &json_path,
            png_path: &png_path,
            png_filename: &png_filename,
            name: &name,
            kind: save_asset_kind,
            canvas_width,
            canvas_height,
            pixels: &pixels,
            source_metadata_path: Some(&json_path),
        })
    }

    fn save_asset_to_paths(&mut self, request: SaveAssetRequest<'_>) -> Result<(), String> {
        toki_core::graphics::image::save_image_rgba8(
            request.png_path,
            request.canvas_width,
            request.canvas_height,
            request.pixels,
        )
        .map_err(|e| format!("Failed to save PNG: {e}"))?;

        self.save_metadata(
            request.json_path,
            request.png_filename,
            request.kind,
            glam::UVec2::new(request.canvas_width, request.canvas_height),
            request.source_metadata_path,
        )?;
        let saved_aliases = self.read_saved_cell_aliases(
            request.json_path,
            request.kind,
            request.canvas_width,
            request.canvas_height,
        );

        let cs = self.active_mut();
        cs.active_sprite = Some(request.json_path.to_string_lossy().to_string());
        cs.asset_kind = Some(request.kind);
        cs.save_asset_name = request.name.to_string();
        cs.save_asset_kind = request.kind;
        cs.original_cell_aliases = saved_aliases;
        cs.dirty = false;
        self.show_save_dialog = false;
        self.needs_asset_rescan = true;

        Ok(())
    }

    fn save_metadata(
        &self,
        json_path: &std::path::Path,
        png_filename: &str,
        kind: SpriteAssetKind,
        canvas_size: glam::UVec2,
        source_metadata_path: Option<&std::path::Path>,
    ) -> Result<(), String> {
        match kind {
            SpriteAssetKind::TileAtlas => {
                let mut meta = if self.is_sheet() {
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    self.create_atlas_with_names(png_filename, cols, rows, source_metadata_path)
                } else {
                    self.create_atlas_with_names(png_filename, 1, 1, source_metadata_path)
                };
                meta.tile_size = glam::UVec2::new(
                    if self.is_sheet() {
                        self.active().cell_size.x
                    } else {
                        canvas_size.x
                    },
                    if self.is_sheet() {
                        self.active().cell_size.y
                    } else {
                        canvas_size.y
                    },
                );
                meta.color_mode = self.color_mode;
                meta.palette =
                    if self.color_mode == toki_core::assets::atlas::ColorMode::PaletteIndexed {
                        self.selected_palette_id.clone()
                    } else {
                        None
                    };
                meta.save_to_file(json_path)
                    .map_err(|e| format!("Failed to save metadata: {e}"))?;
            }
            SpriteAssetKind::ObjectSheet => {
                let mut meta = if self.is_sheet() {
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    self.create_object_sheet_with_names(
                        png_filename,
                        cols,
                        rows,
                        source_metadata_path,
                    )
                } else {
                    self.create_object_sheet_with_names(png_filename, 1, 1, source_metadata_path)
                };
                meta.color_mode = self.color_mode;
                meta.palette =
                    if self.color_mode == toki_core::assets::atlas::ColorMode::PaletteIndexed {
                        self.selected_palette_id.clone()
                    } else {
                        None
                    };
                meta.save_to_file(json_path)
                    .map_err(|e| format!("Failed to save metadata: {e}"))?;
            }
        }
        Ok(())
    }

    fn create_atlas_with_names(
        &self,
        png_filename: &str,
        cols: u32,
        rows: u32,
        source_metadata_path: Option<&std::path::Path>,
    ) -> toki_core::assets::atlas::AtlasMeta {
        use std::collections::HashMap;
        use toki_core::assets::atlas::{AtlasMeta, TileInfo};

        let cs = self.active();
        let existing_meta = source_metadata_path
            .and_then(|path| AtlasMeta::load_from_file(path).ok())
            .filter(|_| cs.asset_kind == Some(SpriteAssetKind::TileAtlas));
        let mut used_names = std::collections::HashSet::new();
        let mut tiles = HashMap::new();

        for row in 0..rows {
            for col in 0..cols {
                let index = (row * cols + col) as usize;
                let aliases = self.get_cell_aliases(index, "tile", &mut used_names);
                for alias in aliases {
                    let properties = existing_meta
                        .as_ref()
                        .and_then(|meta| meta.tiles.get(&alias))
                        .map(|tile| tile.properties.clone())
                        .unwrap_or_default();
                    tiles.insert(
                        alias,
                        TileInfo {
                            position: glam::UVec2::new(col, row),
                            properties,
                        },
                    );
                }
            }
        }

        AtlasMeta {
            image: png_filename.into(),
            tile_size: cs.cell_size,
            color_mode: self.color_mode,
            palette: if self.color_mode == toki_core::assets::atlas::ColorMode::PaletteIndexed {
                self.selected_palette_id.clone()
            } else {
                None
            },
            palette_size: None,
            tiles,
        }
    }

    fn create_object_sheet_with_names(
        &self,
        png_filename: &str,
        cols: u32,
        rows: u32,
        source_metadata_path: Option<&std::path::Path>,
    ) -> toki_core::assets::object_sheet::ObjectSheetMeta {
        use std::collections::HashMap;
        use toki_core::assets::object_sheet::{ObjectSheetMeta, ObjectSheetType, ObjectSpriteInfo};

        let cs = self.active();
        let existing_meta = source_metadata_path
            .and_then(|path| ObjectSheetMeta::load_from_file(path).ok())
            .filter(|_| cs.asset_kind == Some(SpriteAssetKind::ObjectSheet));
        let mut used_names = std::collections::HashSet::new();
        let mut objects = HashMap::new();

        for row in 0..rows {
            for col in 0..cols {
                let index = (row * cols + col) as usize;
                let aliases = self.get_cell_aliases(index, "object", &mut used_names);
                let name = aliases.into_iter().next().unwrap_or_else(|| {
                    unreachable!("get_cell_aliases always returns at least one alias")
                });
                let object_info = existing_meta
                    .as_ref()
                    .and_then(|meta| meta.objects.get(&name))
                    .cloned()
                    .filter(|object| {
                        object.position.x < cols
                            && object.position.y < rows
                            && object.position.x + object.size_tiles.x <= cols
                            && object.position.y + object.size_tiles.y <= rows
                    })
                    .unwrap_or(ObjectSpriteInfo {
                        position: glam::UVec2::new(col, row),
                        size_tiles: glam::UVec2::ONE,
                    });
                objects.insert(
                    name,
                    ObjectSpriteInfo {
                        position: glam::UVec2::new(col, row),
                        ..object_info
                    },
                );
            }
        }

        ObjectSheetMeta {
            sheet_type: ObjectSheetType::Objects,
            image: png_filename.into(),
            tile_size: cs.cell_size,
            color_mode: self.color_mode,
            palette: if self.color_mode == toki_core::assets::atlas::ColorMode::PaletteIndexed {
                self.selected_palette_id.clone()
            } else {
                None
            },
            palette_size: None,
            objects,
        }
    }

    fn get_cell_aliases(
        &self,
        index: usize,
        prefix: &str,
        used_names: &mut std::collections::HashSet<String>,
    ) -> Vec<String> {
        let cs = self.active();
        if let Some(ref aliases) = cs.original_cell_aliases {
            if let Some(cell_aliases) = aliases.get(index) {
                let filtered = cell_aliases
                    .iter()
                    .filter_map(|alias| {
                        if used_names.insert(alias.clone()) {
                            Some(alias.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                if !filtered.is_empty() {
                    return filtered;
                }
            }
        }
        vec![self.generate_unique_cell_name(index, prefix, used_names)]
    }

    /// Import an external image file into the active canvas
    pub fn import_external_image(&mut self, path: &std::path::Path) -> Result<(), String> {
        use toki_core::graphics::image::load_image_rgba8;

        let decoded = load_image_rgba8(path).map_err(|e| format!("Failed to load image: {e}"))?;

        let canvas = SpriteCanvas::from_rgba(decoded.width, decoded.height, decoded.data)
            .ok_or_else(|| "Failed to create canvas from image data".to_string())?;

        let save_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported")
            .to_string();
        let cell_size = glam::UVec2::new(decoded.width, decoded.height);
        self.reset_canvas_state(canvas, true);
        let cs = self.active_mut();
        cs.save_asset_name = save_name;
        cs.cell_size = cell_size;
        Ok(())
    }

    /// Import an external image file as a sheet canvas using the configured tile size.
    pub fn import_external_image_as_sheet(
        &mut self,
        path: &std::path::Path,
        cell_width: u32,
        cell_height: u32,
    ) -> Result<(), String> {
        use toki_core::graphics::image::load_image_rgba8;

        let decoded = load_image_rgba8(path).map_err(|e| format!("Failed to load image: {e}"))?;
        let cell_width = cell_width.max(1);
        let cell_height = cell_height.max(1);

        if decoded.width % cell_width != 0 || decoded.height % cell_height != 0 {
            return Err(format!(
                "Image size {}x{} does not divide evenly by configured tile size {}x{}",
                decoded.width, decoded.height, cell_width, cell_height
            ));
        }

        let canvas = SpriteCanvas::from_rgba(decoded.width, decoded.height, decoded.data)
            .ok_or_else(|| "Failed to create canvas from image data".to_string())?;
        let save_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported")
            .to_string();

        self.reset_canvas_state(canvas, true);
        let cs = self.active_mut();
        cs.save_asset_name = save_name;
        cs.cell_size = glam::UVec2::new(cell_width, cell_height);
        cs.show_cell_grid = true;
        cs.save_asset_kind = SpriteAssetKind::TileAtlas;
        Ok(())
    }

    /// Export the current canvas as PNG
    pub fn export_as_png(&self, path: &std::path::Path) -> Result<(), String> {
        let cs = self.active();
        let canvas = cs.canvas.as_ref().ok_or("No canvas to export")?;

        toki_core::graphics::image::save_image_rgba8(
            path,
            canvas.width,
            canvas.height,
            canvas.pixels(),
        )
        .map_err(|e| format!("Failed to save image: {e}"))?;

        Ok(())
    }
}

impl SpriteEditorState {
    fn resolve_existing_png_filename(
        &self,
        json_path: &std::path::Path,
        kind: SpriteAssetKind,
    ) -> String {
        let fallback = format!(
            "{}.png",
            json_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("sprite")
        );
        match kind {
            SpriteAssetKind::TileAtlas => {
                toki_core::assets::atlas::AtlasMeta::load_from_file(json_path)
                    .ok()
                    .and_then(|meta| {
                        meta.image
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(str::to_string)
                    })
                    .unwrap_or(fallback)
            }
            SpriteAssetKind::ObjectSheet => {
                toki_core::assets::object_sheet::ObjectSheetMeta::load_from_file(json_path)
                    .ok()
                    .and_then(|meta| {
                        meta.image
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(str::to_string)
                    })
                    .unwrap_or(fallback)
            }
        }
    }

    fn read_saved_cell_aliases(
        &self,
        json_path: &std::path::Path,
        kind: SpriteAssetKind,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Option<Vec<Vec<String>>> {
        match kind {
            SpriteAssetKind::TileAtlas => {
                toki_core::assets::atlas::AtlasMeta::load_from_file(json_path)
                    .ok()
                    .map(|meta| {
                        let cols = (canvas_width / meta.tile_size.x.max(1)).max(1);
                        let rows = (canvas_height / meta.tile_size.y.max(1)).max(1);
                        ordered_atlas_aliases(&meta, cols, rows)
                    })
            }
            SpriteAssetKind::ObjectSheet => {
                toki_core::assets::object_sheet::ObjectSheetMeta::load_from_file(json_path)
                    .ok()
                    .map(|meta| {
                        let cols = (canvas_width / meta.tile_size.x.max(1)).max(1);
                        let rows = (canvas_height / meta.tile_size.y.max(1)).max(1);
                        ordered_object_aliases(&meta, cols, rows)
                    })
            }
        }
    }

    fn generate_unique_cell_name(
        &self,
        index: usize,
        prefix: &str,
        used_names: &mut std::collections::HashSet<String>,
    ) -> String {
        let mut candidate_index = index;
        loop {
            let candidate = format!("{}_{}", prefix, candidate_index);
            if used_names.insert(candidate.clone()) {
                return candidate;
            }
            candidate_index += 1;
        }
    }
}

/// Classify a directory entry as a JSON sprite asset
fn classify_json_entry(
    entry: &std::fs::DirEntry,
) -> Option<(std::path::PathBuf, String, std::path::PathBuf)> {
    let path = entry.path();
    if !path.is_file() {
        return None;
    }
    let ext = path.extension()?;
    if ext != "json" {
        return None;
    }
    let stem = path.file_stem()?.to_str()?.to_string();
    let sprites_dir = path.parent()?;
    let png_path = sprites_dir.join(format!("{stem}.png"));
    if !png_path.exists() {
        return None;
    }
    Some((path, stem, png_path))
}

fn ordered_atlas_aliases(
    meta: &toki_core::assets::atlas::AtlasMeta,
    cols: u32,
    rows: u32,
) -> Vec<Vec<String>> {
    let total_cells = (cols * rows) as usize;
    let mut ordered = vec![Vec::new(); total_cells];
    for (name, tile) in &meta.tiles {
        let index = (tile.position.y * cols + tile.position.x) as usize;
        if index < ordered.len() {
            ordered[index].push(name.clone());
        }
    }
    for aliases in &mut ordered {
        aliases.sort();
    }
    while ordered.last().is_some_and(|aliases| aliases.is_empty()) {
        ordered.pop();
    }
    ordered
}

fn ordered_object_aliases(
    meta: &toki_core::assets::object_sheet::ObjectSheetMeta,
    cols: u32,
    rows: u32,
) -> Vec<Vec<String>> {
    let total_cells = (cols * rows) as usize;
    let mut ordered = vec![Vec::new(); total_cells];
    for (name, object) in &meta.objects {
        let index = (object.position.y * cols + object.position.x) as usize;
        if index < ordered.len() {
            ordered[index].push(name.clone());
        }
    }
    for aliases in &mut ordered {
        aliases.sort();
    }
    while ordered.last().is_some_and(|aliases| aliases.is_empty()) {
        ordered.pop();
    }
    ordered
}

/// Copy image data to canvas at specified position
fn copy_image_to_canvas(
    canvas: &mut SpriteCanvas,
    data: &[u8],
    width: u32,
    height: u32,
    start_x: u32,
    start_y: u32,
) {
    for py in 0..height {
        for px in 0..width {
            let src_idx = ((py * width + px) * 4) as usize;
            let color = PixelColor::from_rgba_array([
                data[src_idx],
                data[src_idx + 1],
                data[src_idx + 2],
                data[src_idx + 3],
            ]);
            canvas.set_pixel(start_x + px, start_y + py, color);
        }
    }
}
