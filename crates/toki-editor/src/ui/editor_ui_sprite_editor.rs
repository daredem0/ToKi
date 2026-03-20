use super::EditorUI;

/// Tool for sprite/pixel editing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SpriteEditorTool {
    #[default]
    Drag,
    Brush,
    Eraser,
    Fill,
    Eyedropper,
    Select,
    Line,
}

/// Type of sprite asset being edited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SpriteAssetKind {
    /// Atlas-style tiles metadata (tiles with positions)
    TileAtlas,
    /// Object sheet metadata (objects with positions and sizes)
    ObjectSheet,
}

/// RGBA color for pixel editing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[allow(dead_code)]
impl PixelColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    pub const fn black() -> Self {
        Self::rgb(0, 0, 0)
    }

    pub const fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    pub fn to_rgba_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn from_rgba_array(rgba: [u8; 4]) -> Self {
        Self {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
            a: rgba[3],
        }
    }

    /// Convert to egui Color32
    pub fn to_color32(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }

    /// Convert from egui Color32
    pub fn from_color32(color: egui::Color32) -> Self {
        Self {
            r: color.r(),
            g: color.g(),
            b: color.b(),
            a: color.a(),
        }
    }
}

/// In-memory canvas for pixel editing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteCanvas {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel data in RGBA format (row-major, top-to-bottom)
    pixels: Vec<u8>,
}

#[allow(dead_code)]
impl SpriteCanvas {
    /// Create a new canvas filled with transparent pixels
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            pixels: vec![0; pixel_count * 4],
        }
    }

    /// Create a new canvas filled with a specific color
    pub fn filled(width: u32, height: u32, color: PixelColor) -> Self {
        let pixel_count = (width * height) as usize;
        let mut pixels = Vec::with_capacity(pixel_count * 4);
        let rgba = color.to_rgba_array();
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&rgba);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Create a canvas from RGBA pixel data
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected_len = (width * height * 4) as usize;
        if pixels.len() != expected_len {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    /// Get pixel color at position, returns None if out of bounds
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<PixelColor> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        Some(PixelColor::from_rgba_array([
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ]))
    }

    /// Set pixel color at position, returns false if out of bounds
    pub fn set_pixel(&mut self, x: u32, y: u32, color: PixelColor) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        let rgba = color.to_rgba_array();
        self.pixels[idx..idx + 4].copy_from_slice(&rgba);
        true
    }

    /// Get raw RGBA pixel data
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Get mutable raw RGBA pixel data
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Fill a rectangle with a color
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: PixelColor) {
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        for py in y..y_end {
            for px in x..x_end {
                self.set_pixel(px, py, color);
            }
        }
    }

    /// Clear entire canvas to transparent
    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Clear entire canvas to a specific color
    pub fn clear_to_color(&mut self, color: PixelColor) {
        let rgba = color.to_rgba_array();
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&rgba);
        }
    }
}

/// Viewport state for the sprite canvas
#[derive(Debug, Clone)]
pub struct SpriteCanvasViewport {
    /// Camera offset in canvas pixels (top-left corner of view)
    pub pan: glam::Vec2,
    /// Zoom level (1.0 = 1 canvas pixel = 1 screen pixel)
    pub zoom: f32,
    /// Minimum zoom level
    pub zoom_min: f32,
    /// Maximum zoom level
    pub zoom_max: f32,
}

impl Default for SpriteCanvasViewport {
    fn default() -> Self {
        Self {
            pan: glam::Vec2::ZERO,
            zoom: 8.0, // Start zoomed in for pixel editing
            zoom_min: 1.0,
            zoom_max: 64.0,
        }
    }
}

#[allow(dead_code)]
impl SpriteCanvasViewport {
    /// Create a new viewport with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Zoom in by one step (doubling)
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(self.zoom_max);
    }

    /// Zoom out by one step
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(self.zoom_min);
    }

    /// Set zoom level with clamping
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(self.zoom_min, self.zoom_max);
    }

    /// Pan by delta in screen pixels
    pub fn pan_by(&mut self, delta: glam::Vec2) {
        // Convert screen delta to canvas delta
        self.pan -= delta / self.zoom;
    }

    /// Convert screen position to canvas position
    pub fn screen_to_canvas(
        &self,
        screen_pos: glam::Vec2,
        viewport_rect: egui::Rect,
    ) -> glam::Vec2 {
        let viewport_pos = screen_pos - glam::Vec2::new(viewport_rect.left(), viewport_rect.top());
        viewport_pos / self.zoom + self.pan
    }

    /// Convert canvas position to screen position
    pub fn canvas_to_screen(
        &self,
        canvas_pos: glam::Vec2,
        viewport_rect: egui::Rect,
    ) -> glam::Vec2 {
        let viewport_pos = (canvas_pos - self.pan) * self.zoom;
        viewport_pos + glam::Vec2::new(viewport_rect.left(), viewport_rect.top())
    }

    /// Get the visible canvas rect in canvas coordinates
    pub fn visible_canvas_rect(&self, viewport_size: glam::Vec2) -> egui::Rect {
        let size = viewport_size / self.zoom;
        egui::Rect::from_min_size(
            egui::pos2(self.pan.x, self.pan.y),
            egui::vec2(size.x, size.y),
        )
    }
}

/// Selection rectangle in canvas pixel coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpriteSelection {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[allow(dead_code)]
impl SpriteSelection {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, px: u32, py: u32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// Undo/redo command for sprite editing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SpriteEditCommand {
    /// Canvas state before the edit
    pub before: SpriteCanvas,
    /// Canvas state after the edit
    pub after: SpriteCanvas,
}

/// Local undo/redo history for sprite editor (separate from scene history)
#[derive(Debug, Clone, Default)]
pub struct SpriteEditorHistory {
    undo_stack: Vec<SpriteEditCommand>,
    redo_stack: Vec<SpriteEditCommand>,
    /// Maximum number of undo steps to keep
    max_size: usize,
}

#[allow(dead_code)]
impl SpriteEditorHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size,
        }
    }

    pub fn push(&mut self, command: SpriteEditCommand) {
        self.undo_stack.push(command);
        self.redo_stack.clear();
        // Trim history if too large
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    pub fn take_undo(&mut self) -> Option<SpriteCanvas> {
        let command = self.undo_stack.pop()?;
        let before = command.before.clone();
        self.redo_stack.push(command);
        Some(before)
    }

    pub fn take_redo(&mut self) -> Option<SpriteCanvas> {
        let command = self.redo_stack.pop()?;
        let after = command.after.clone();
        self.undo_stack.push(command);
        Some(after)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Sprite editor state
#[allow(dead_code)]
pub struct SpriteEditorState {
    /// Currently active sprite asset path (JSON metadata file)
    pub active_sprite: Option<String>,
    /// In-memory canvas being edited
    pub canvas: Option<SpriteCanvas>,
    /// Viewport state (zoom, pan)
    pub viewport: SpriteCanvasViewport,
    /// Has unsaved changes
    pub dirty: bool,
    /// Current editing tool
    pub tool: SpriteEditorTool,
    /// Current foreground color
    pub foreground_color: PixelColor,
    /// Current background color (used by eraser)
    pub background_color: PixelColor,
    /// Brush size in pixels
    pub brush_size: u32,
    /// Selection rectangle (if any)
    pub selection: Option<SpriteSelection>,
    /// Local undo/redo history
    pub history: SpriteEditorHistory,
    /// Show pixel grid overlay
    pub show_grid: bool,
    /// Current cursor position in canvas coordinates (if hovering)
    pub cursor_canvas_pos: Option<glam::IVec2>,
    /// Cached texture handle for canvas preview
    pub canvas_texture: Option<egui::TextureHandle>,
    /// Recent colors palette
    pub recent_colors: Vec<PixelColor>,
    /// Maximum recent colors to remember
    pub max_recent_colors: usize,
    /// Asset kind being created/edited
    pub asset_kind: Option<SpriteAssetKind>,
    /// Grid cell size for sheet editing (width, height in pixels)
    pub cell_size: glam::UVec2,
    /// Selected cell index (for sheet editing)
    pub selected_cell: Option<usize>,
    /// Show new canvas dialog
    pub show_new_canvas_dialog: bool,
    /// New canvas dialog: width
    pub new_canvas_width: u32,
    /// New canvas dialog: height
    pub new_canvas_height: u32,
    /// New canvas dialog: cell width (for sheets)
    pub new_canvas_cell_width: u32,
    /// New canvas dialog: cell height (for sheets)
    pub new_canvas_cell_height: u32,
    /// New canvas dialog: create as sheet
    pub new_canvas_is_sheet: bool,
    /// Whether to show the sheet cell grid overlay
    pub show_cell_grid: bool,
    /// Line tool: start position when dragging
    pub line_start_pos: Option<glam::IVec2>,
    /// Selection tool: start position when dragging
    pub selection_start_pos: Option<glam::IVec2>,
    /// Canvas state before current stroke (for undo)
    pub canvas_before_stroke: Option<SpriteCanvas>,
    /// Whether currently in a paint stroke
    pub is_painting: bool,
    /// Show save sprite dialog
    pub show_save_dialog: bool,
    /// Save dialog: asset name (without extension)
    pub save_asset_name: String,
    /// Save dialog: asset type (atlas vs object sheet)
    pub save_asset_kind: SpriteAssetKind,
    /// Swap target cell index (for cell reordering UI)
    pub swap_target_cell: u32,
    /// Show confirmation dialog for risky operations
    pub show_warning_dialog: bool,
    /// Warning dialog message
    pub warning_message: String,
    /// Pending action after warning confirmation
    pub pending_warning_action: Option<WarningAction>,
}

/// Actions that require warning confirmation
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum WarningAction {
    /// Clear the selected cell
    ClearCell(usize),
    /// Change cell grid size (may cause data loss)
    ChangeCellSize { new_width: u32, new_height: u32 },
}

impl Default for SpriteEditorState {
    fn default() -> Self {
        Self {
            active_sprite: None,
            canvas: None,
            viewport: SpriteCanvasViewport::default(),
            dirty: false,
            tool: SpriteEditorTool::Drag,
            foreground_color: PixelColor::black(),
            background_color: PixelColor::transparent(),
            brush_size: 1,
            selection: None,
            history: SpriteEditorHistory::new(50),
            show_grid: true,
            cursor_canvas_pos: None,
            canvas_texture: None,
            recent_colors: Vec::new(),
            max_recent_colors: 16,
            asset_kind: None,
            cell_size: glam::UVec2::new(16, 16),
            selected_cell: None,
            show_new_canvas_dialog: false,
            new_canvas_width: 16,
            new_canvas_height: 16,
            new_canvas_cell_width: 16,
            new_canvas_cell_height: 16,
            new_canvas_is_sheet: false,
            show_cell_grid: false,
            line_start_pos: None,
            selection_start_pos: None,
            canvas_before_stroke: None,
            is_painting: false,
            show_save_dialog: false,
            save_asset_name: String::new(),
            save_asset_kind: SpriteAssetKind::ObjectSheet,
            swap_target_cell: 0,
            show_warning_dialog: false,
            warning_message: String::new(),
            pending_warning_action: None,
        }
    }
}

#[allow(dead_code)]
impl SpriteEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new empty canvas
    pub fn new_canvas(&mut self, width: u32, height: u32) {
        self.canvas = Some(SpriteCanvas::new(width, height));
        self.active_sprite = None;
        self.dirty = true;
        self.history.clear();
        self.selection = None;
        self.canvas_texture = None;
        self.viewport = SpriteCanvasViewport::default();
    }

    /// Create a new canvas filled with a color
    pub fn new_canvas_filled(&mut self, width: u32, height: u32, color: PixelColor) {
        self.canvas = Some(SpriteCanvas::filled(width, height, color));
        self.active_sprite = None;
        self.dirty = true;
        self.history.clear();
        self.selection = None;
        self.canvas_texture = None;
        self.viewport = SpriteCanvasViewport::default();
    }

    /// Check if there's an active canvas being edited
    pub fn has_canvas(&self) -> bool {
        self.canvas.is_some()
    }

    /// Get canvas dimensions if a canvas is active
    pub fn canvas_dimensions(&self) -> Option<(u32, u32)> {
        self.canvas.as_ref().map(|c| (c.width, c.height))
    }

    /// Check if this is a sheet (has cell dimensions set)
    pub fn is_sheet(&self) -> bool {
        self.cell_size.x > 0 && self.cell_size.y > 0 && self.show_cell_grid
    }

    /// Get the number of cells in the sheet (columns, rows)
    pub fn sheet_cell_count(&self) -> Option<(u32, u32)> {
        let (w, h) = self.canvas_dimensions()?;
        if self.cell_size.x == 0 || self.cell_size.y == 0 {
            return None;
        }
        Some((w / self.cell_size.x, h / self.cell_size.y))
    }

    /// Get the total number of cells in the sheet
    pub fn total_cell_count(&self) -> Option<u32> {
        let (cols, rows) = self.sheet_cell_count()?;
        Some(cols * rows)
    }

    /// Get cell index from canvas position
    pub fn cell_at_position(&self, x: u32, y: u32) -> Option<usize> {
        if !self.is_sheet() {
            return None;
        }
        let (cols, _rows) = self.sheet_cell_count()?;
        let cell_x = x / self.cell_size.x;
        let cell_y = y / self.cell_size.y;
        Some((cell_y * cols + cell_x) as usize)
    }

    /// Get cell bounds (start_x, start_y, end_x, end_y) for a cell index
    pub fn cell_bounds(&self, cell_index: usize) -> Option<(u32, u32, u32, u32)> {
        let (cols, rows) = self.sheet_cell_count()?;
        let total = cols * rows;
        if cell_index as u32 >= total {
            return None;
        }
        let col = cell_index as u32 % cols;
        let row = cell_index as u32 / cols;
        let start_x = col * self.cell_size.x;
        let start_y = row * self.cell_size.y;
        let end_x = start_x + self.cell_size.x;
        let end_y = start_y + self.cell_size.y;
        Some((start_x, start_y, end_x, end_y))
    }

    /// Create a new sheet canvas
    pub fn new_sheet(&mut self, width: u32, height: u32, cell_width: u32, cell_height: u32) {
        self.canvas = Some(SpriteCanvas::new(width, height));
        self.active_sprite = None;
        self.dirty = true;
        self.history.clear();
        self.selection = None;
        self.canvas_texture = None;
        self.viewport = SpriteCanvasViewport::default();
        self.cell_size = glam::UVec2::new(cell_width, cell_height);
        self.show_cell_grid = true;
        self.selected_cell = None;
    }

    /// Clear the selected cell to transparent pixels
    pub fn clear_selected_cell(&mut self) -> bool {
        let cell_idx = match self.selected_cell {
            Some(idx) => idx,
            None => return false,
        };

        // Get cell bounds before mutable borrow
        let Some((start_x, start_y, end_x, end_y)) = self.cell_bounds(cell_idx) else {
            return false;
        };

        let Some(canvas) = &mut self.canvas else {
            return false;
        };

        // Save state for undo
        let before = canvas.clone();

        // Clear the cell
        canvas.fill_rect(
            start_x,
            start_y,
            end_x - start_x,
            end_y - start_y,
            PixelColor::transparent(),
        );

        // Push undo state
        self.history.push(SpriteEditCommand {
            before,
            after: canvas.clone(),
        });

        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Swap the contents of two cells
    pub fn swap_cells(&mut self, cell_a: usize, cell_b: usize) -> bool {
        if cell_a == cell_b {
            return false;
        }

        // Get cell bounds before mutable borrow
        let Some((a_start_x, a_start_y, a_end_x, a_end_y)) = self.cell_bounds(cell_a) else {
            return false;
        };

        let Some((b_start_x, b_start_y, b_end_x, b_end_y)) = self.cell_bounds(cell_b) else {
            return false;
        };

        let Some(canvas) = &mut self.canvas else {
            return false;
        };

        // Cells must have the same dimensions (they should since they're from the same grid)
        let width = a_end_x - a_start_x;
        let height = a_end_y - a_start_y;

        // Save state for undo
        let before = canvas.clone();

        // Read pixels from cell A
        let mut pixels_a = Vec::with_capacity((width * height * 4) as usize);
        for y in a_start_y..a_end_y {
            for x in a_start_x..a_end_x {
                if let Some(color) = canvas.get_pixel(x, y) {
                    pixels_a.extend_from_slice(&color.to_rgba_array());
                }
            }
        }

        // Read pixels from cell B
        let mut pixels_b = Vec::with_capacity((width * height * 4) as usize);
        for y in b_start_y..b_end_y {
            for x in b_start_x..b_end_x {
                if let Some(color) = canvas.get_pixel(x, y) {
                    pixels_b.extend_from_slice(&color.to_rgba_array());
                }
            }
        }

        // Write cell B pixels to cell A
        let mut i = 0;
        for y in a_start_y..a_end_y {
            for x in a_start_x..a_end_x {
                let color = PixelColor::from_rgba_array([
                    pixels_b[i],
                    pixels_b[i + 1],
                    pixels_b[i + 2],
                    pixels_b[i + 3],
                ]);
                canvas.set_pixel(x, y, color);
                i += 4;
            }
        }

        // Write cell A pixels to cell B
        let mut i = 0;
        for y in b_start_y..b_end_y {
            for x in b_start_x..b_end_x {
                let color = PixelColor::from_rgba_array([
                    pixels_a[i],
                    pixels_a[i + 1],
                    pixels_a[i + 2],
                    pixels_a[i + 3],
                ]);
                canvas.set_pixel(x, y, color);
                i += 4;
            }
        }

        // Push undo state
        self.history.push(SpriteEditCommand {
            before,
            after: canvas.clone(),
        });

        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Mark the current state as dirty (has unsaved changes)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear the dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Push current canvas state for undo
    pub fn push_undo_state(&mut self, before: SpriteCanvas) {
        if let Some(canvas) = &self.canvas {
            self.history.push(SpriteEditCommand {
                before,
                after: canvas.clone(),
            });
        }
    }

    /// Perform undo
    pub fn undo(&mut self) -> bool {
        if let Some(before) = self.history.take_undo() {
            self.canvas = Some(before);
            self.canvas_texture = None;
            true
        } else {
            false
        }
    }

    /// Perform redo
    pub fn redo(&mut self) -> bool {
        if let Some(after) = self.history.take_redo() {
            self.canvas = Some(after);
            self.canvas_texture = None;
            true
        } else {
            false
        }
    }

    /// Add a color to the recent colors palette
    pub fn add_recent_color(&mut self, color: PixelColor) {
        // Remove if already exists
        self.recent_colors.retain(|&c| c != color);
        // Add to front
        self.recent_colors.insert(0, color);
        // Trim to max size
        while self.recent_colors.len() > self.max_recent_colors {
            self.recent_colors.pop();
        }
    }

    /// Close the current canvas
    pub fn close_canvas(&mut self) {
        self.canvas = None;
        self.active_sprite = None;
        self.dirty = false;
        self.history.clear();
        self.selection = None;
        self.canvas_texture = None;
    }

    /// Open the save dialog
    pub fn begin_save_dialog(&mut self) {
        self.show_save_dialog = true;
        // Default to the existing name if editing
        if self.save_asset_name.is_empty() {
            self.save_asset_name = "new_sprite".to_string();
        }
    }

    /// Save the current canvas as a sprite asset.
    /// Returns Ok(()) on success, Err with message on failure.
    pub fn save_as_asset(&mut self, sprites_dir: &std::path::Path) -> Result<(), String> {
        use glam::UVec2;
        use toki_core::assets::atlas::AtlasMeta;
        use toki_core::assets::object_sheet::ObjectSheetMeta;

        let canvas = self.canvas.as_ref().ok_or("No canvas to save")?;
        let name = self.save_asset_name.trim();
        if name.is_empty() {
            return Err("Asset name cannot be empty".to_string());
        }

        // Validate name (alphanumeric and underscores only)
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Asset name can only contain letters, numbers, and underscores".to_string());
        }

        // Ensure sprites directory exists
        std::fs::create_dir_all(sprites_dir)
            .map_err(|e| format!("Failed to create sprites directory: {e}"))?;

        let png_filename = format!("{name}.png");
        let json_filename = format!("{name}.json");
        let png_path = sprites_dir.join(&png_filename);
        let json_path = sprites_dir.join(&json_filename);

        // Save PNG
        toki_core::graphics::image::save_image_rgba8(
            &png_path,
            canvas.width,
            canvas.height,
            canvas.pixels(),
        )
        .map_err(|e| format!("Failed to save PNG: {e}"))?;

        // Create and save metadata based on asset kind
        match self.save_asset_kind {
            SpriteAssetKind::TileAtlas => {
                let meta = if self.is_sheet() {
                    // Create grid atlas
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    AtlasMeta::new_grid(&png_filename, self.cell_size, cols, rows)
                } else {
                    // Single tile atlas
                    AtlasMeta::new_single_tile(&png_filename, UVec2::new(canvas.width, canvas.height))
                };
                meta.save_to_file(&json_path)
                    .map_err(|e| format!("Failed to save metadata: {e}"))?;
            }
            SpriteAssetKind::ObjectSheet => {
                let meta = if self.is_sheet() {
                    // Create grid of objects
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    ObjectSheetMeta::new_grid(&png_filename, self.cell_size, cols, rows)
                } else {
                    // Single object
                    ObjectSheetMeta::new_single_object(
                        &png_filename,
                        name,
                        UVec2::new(canvas.width, canvas.height),
                    )
                };
                meta.save_to_file(&json_path)
                    .map_err(|e| format!("Failed to save metadata: {e}"))?;
            }
        }

        // Update state
        self.active_sprite = Some(json_path.to_string_lossy().to_string());
        self.dirty = false;
        self.show_save_dialog = false;

        Ok(())
    }
}

impl EditorUI {
    /// Begin showing the new canvas dialog
    pub fn begin_new_sprite_canvas_dialog(&mut self) {
        self.sprite.show_new_canvas_dialog = true;
    }

    /// Submit new canvas creation request
    #[allow(dead_code)]
    pub fn submit_new_sprite_canvas(&mut self) {
        let width = self.sprite.new_canvas_width.max(1);
        let height = self.sprite.new_canvas_height.max(1);
        self.sprite.new_canvas(width, height);
        self.sprite.show_new_canvas_dialog = false;
    }

    /// Cancel new canvas dialog
    pub fn cancel_new_sprite_canvas_dialog(&mut self) {
        self.sprite.show_new_canvas_dialog = false;
    }
}

#[cfg(test)]
#[path = "editor_ui_sprite_editor_tests.rs"]
mod tests;
