//! File I/O operations for SpriteEditorState.

use super::{
    DiscoveredSpriteAsset, PixelColor, SpriteAssetKind, SpriteCanvas, SpriteCanvasViewport,
    SpriteEditorState,
};

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
        cs.original_cell_names = None;
        cs.show_cell_grid = false;
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

        let (cell_size, is_sheet, original_names) = match asset.kind {
            SpriteAssetKind::TileAtlas => {
                let meta = AtlasMeta::load_from_file(&asset.json_path)
                    .map_err(|e| format!("Failed to load atlas metadata: {e}"))?;
                let is_sheet = meta.tiles.len() > 1;
                let mut names: Vec<_> = meta.tiles.keys().cloned().collect();
                names.sort();
                (meta.tile_size, is_sheet, names)
            }
            SpriteAssetKind::ObjectSheet => {
                let meta = ObjectSheetMeta::load_from_file(&asset.json_path)
                    .map_err(|e| format!("Failed to load object sheet metadata: {e}"))?;
                let is_sheet = meta.objects.len() > 1;
                let mut names: Vec<_> = meta.objects.keys().cloned().collect();
                names.sort();
                (meta.tile_size, is_sheet, names)
            }
        };

        self.reset_canvas_state(canvas, false);
        let cs = self.active_mut();
        cs.active_sprite = Some(asset.json_path.to_string_lossy().to_string());
        cs.asset_kind = Some(asset.kind);
        cs.save_asset_name = asset.name.clone();
        cs.save_asset_kind = asset.kind;
        cs.original_cell_names = Some(original_names);
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

            copy_image_to_canvas(&mut canvas, data, *img_w, *img_h, start_x + offset_x, start_y + offset_y);
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
        let json_filename = format!("{name}.json");
        let png_path = sprites_dir.join(&png_filename);
        let json_path = sprites_dir.join(&json_filename);
        let save_asset_kind = cs.save_asset_kind;
        let canvas_width = canvas.width;
        let canvas_height = canvas.height;
        let pixels = canvas.pixels().to_vec();

        toki_core::graphics::image::save_image_rgba8(
            &png_path,
            canvas_width,
            canvas_height,
            &pixels,
        )
        .map_err(|e| format!("Failed to save PNG: {e}"))?;

        self.save_metadata(&json_path, &png_filename, &name, save_asset_kind, canvas_width, canvas_height)?;

        let cs = self.active_mut();
        cs.active_sprite = Some(json_path.to_string_lossy().to_string());
        cs.dirty = false;
        self.show_save_dialog = false;
        self.needs_asset_rescan = true;

        Ok(())
    }

    fn save_metadata(
        &self,
        json_path: &std::path::Path,
        png_filename: &str,
        name: &str,
        kind: SpriteAssetKind,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Result<(), String> {
        use glam::UVec2;
        use toki_core::assets::atlas::AtlasMeta;
        use toki_core::assets::object_sheet::ObjectSheetMeta;

        match kind {
            SpriteAssetKind::TileAtlas => {
                let meta = if self.is_sheet() {
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    self.create_atlas_with_names(png_filename, cols, rows)
                } else {
                    AtlasMeta::new_single_tile(png_filename, UVec2::new(canvas_width, canvas_height))
                };
                meta.save_to_file(json_path)
                    .map_err(|e| format!("Failed to save metadata: {e}"))?;
            }
            SpriteAssetKind::ObjectSheet => {
                let meta = if self.is_sheet() {
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    self.create_object_sheet_with_names(png_filename, cols, rows)
                } else {
                    ObjectSheetMeta::new_single_object(
                        png_filename,
                        name,
                        UVec2::new(canvas_width, canvas_height),
                    )
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
    ) -> toki_core::assets::atlas::AtlasMeta {
        use std::collections::HashMap;
        use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};

        let cs = self.active();
        let total_cells = (cols * rows) as usize;
        let mut tiles = HashMap::new();

        for row in 0..rows {
            for col in 0..cols {
                let index = (row * cols + col) as usize;
                let name = self.get_cell_name(index, total_cells, "tile");
                tiles.insert(
                    name,
                    TileInfo {
                        position: glam::UVec2::new(col, row),
                        properties: TileProperties::default(),
                    },
                );
            }
        }

        AtlasMeta {
            image: png_filename.into(),
            tile_size: cs.cell_size,
            tiles,
        }
    }

    fn create_object_sheet_with_names(
        &self,
        png_filename: &str,
        cols: u32,
        rows: u32,
    ) -> toki_core::assets::object_sheet::ObjectSheetMeta {
        use std::collections::HashMap;
        use toki_core::assets::object_sheet::{ObjectSheetMeta, ObjectSheetType, ObjectSpriteInfo};

        let cs = self.active();
        let total_cells = (cols * rows) as usize;
        let mut objects = HashMap::new();

        for row in 0..rows {
            for col in 0..cols {
                let index = (row * cols + col) as usize;
                let name = self.get_cell_name(index, total_cells, "object");
                objects.insert(
                    name,
                    ObjectSpriteInfo {
                        position: glam::UVec2::new(col, row),
                        size_tiles: glam::UVec2::ONE,
                    },
                );
            }
        }

        ObjectSheetMeta {
            sheet_type: ObjectSheetType::Objects,
            image: png_filename.into(),
            tile_size: cs.cell_size,
            objects,
        }
    }

    fn get_cell_name(&self, index: usize, total_cells: usize, prefix: &str) -> String {
        let cs = self.active();
        if let Some(ref names) = cs.original_cell_names {
            if names.len() == total_cells {
                if let Some(name) = names.get(index) {
                    return name.clone();
                }
            }
        }
        format!("{}_{}", prefix, index)
    }

    /// Import an external image file into the active canvas
    pub fn import_external_image(&mut self, path: &std::path::Path) -> Result<(), String> {
        use toki_core::graphics::image::load_image_rgba8;

        let decoded = load_image_rgba8(path).map_err(|e| format!("Failed to load image: {e}"))?;

        let canvas = SpriteCanvas::from_rgba(decoded.width, decoded.height, decoded.data)
            .ok_or_else(|| "Failed to create canvas from image data".to_string())?;

        let save_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported").to_string();
        let cell_size = glam::UVec2::new(decoded.width, decoded.height);
        self.reset_canvas_state(canvas, true);
        let cs = self.active_mut();
        cs.save_asset_name = save_name;
        cs.cell_size = cell_size;
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
