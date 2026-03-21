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
    /// New canvas dialog: sprite width (each cell/sprite's width)
    pub new_sprite_width: u32,
    /// New canvas dialog: sprite height (each cell/sprite's height)
    pub new_sprite_height: u32,
    /// New canvas dialog: number of columns (for sheets)
    pub new_sheet_cols: u32,
    /// New canvas dialog: number of rows (for sheets)
    pub new_sheet_rows: u32,
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
    /// Original tile/object names from loaded asset (for preserving names on re-save)
    pub original_cell_names: Option<Vec<String>>,
    /// Swap target cell index (for cell reordering UI)
    pub swap_target_cell: u32,
    /// Show confirmation dialog for risky operations
    pub show_warning_dialog: bool,
    /// Warning dialog message
    pub warning_message: String,
    /// Pending action after warning confirmation
    pub pending_warning_action: Option<WarningAction>,
    /// Show the load sprite dialog
    pub show_load_dialog: bool,
    /// Discovered sprite assets in project
    pub discovered_assets: Vec<DiscoveredSpriteAsset>,
    /// Selected asset index in the load dialog
    pub selected_asset_index: Option<usize>,
    /// Show the merge sprites dialog
    pub show_merge_dialog: bool,
    /// Selected asset indices for merging (multi-select)
    pub merge_selected_indices: Vec<usize>,
    /// Merge dialog: target columns for the resulting sheet
    pub merge_target_cols: u32,
    /// Show the resize canvas dialog
    pub show_resize_dialog: bool,
    /// Resize dialog: new tile count X (columns)
    pub resize_tiles_x: u32,
    /// Resize dialog: new tile count Y (rows)
    pub resize_tiles_y: u32,
    /// Resize dialog: anchor position
    pub resize_anchor: ResizeAnchor,
    /// Show the rename asset dialog
    pub show_rename_dialog: bool,
    /// Rename dialog: new name input
    pub rename_new_name: String,
    /// Show the delete confirmation dialog
    pub show_delete_confirm: bool,
    /// Asset name pending deletion
    pub delete_asset_name: String,
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

/// Anchor position for canvas resize operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    #[default]
    MiddleCenter,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl ResizeAnchor {
    /// Calculate pixel offset for placing old content in new canvas
    pub fn calculate_offset(self, old_w: u32, old_h: u32, new_w: u32, new_h: u32) -> (i32, i32) {
        let dw = new_w as i32 - old_w as i32;
        let dh = new_h as i32 - old_h as i32;

        match self {
            Self::TopLeft => (0, 0),
            Self::TopCenter => (dw / 2, 0),
            Self::TopRight => (dw, 0),
            Self::MiddleLeft => (0, dh / 2),
            Self::MiddleCenter => (dw / 2, dh / 2),
            Self::MiddleRight => (dw, dh / 2),
            Self::BottomLeft => (0, dh),
            Self::BottomCenter => (dw / 2, dh),
            Self::BottomRight => (dw, dh),
        }
    }

    /// Get display label for this anchor
    pub fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "TL",
            Self::TopCenter => "T",
            Self::TopRight => "TR",
            Self::MiddleLeft => "L",
            Self::MiddleCenter => "C",
            Self::MiddleRight => "R",
            Self::BottomLeft => "BL",
            Self::BottomCenter => "B",
            Self::BottomRight => "BR",
        }
    }

    /// All anchor positions in grid order
    pub fn all() -> [Self; 9] {
        [
            Self::TopLeft,
            Self::TopCenter,
            Self::TopRight,
            Self::MiddleLeft,
            Self::MiddleCenter,
            Self::MiddleRight,
            Self::BottomLeft,
            Self::BottomCenter,
            Self::BottomRight,
        ]
    }
}

/// Discovered sprite asset in the project
#[derive(Debug, Clone)]
pub struct DiscoveredSpriteAsset {
    /// Asset name (filename without extension)
    pub name: String,
    /// Full path to JSON metadata file
    pub json_path: std::path::PathBuf,
    /// Full path to PNG image file
    pub png_path: std::path::PathBuf,
    /// Asset kind (atlas or object sheet)
    pub kind: SpriteAssetKind,
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
            new_sprite_width: 16,
            new_sprite_height: 16,
            new_sheet_cols: 4,
            new_sheet_rows: 4,
            new_canvas_is_sheet: false,
            show_cell_grid: false,
            line_start_pos: None,
            selection_start_pos: None,
            canvas_before_stroke: None,
            is_painting: false,
            show_save_dialog: false,
            save_asset_name: String::new(),
            save_asset_kind: SpriteAssetKind::ObjectSheet,
            original_cell_names: None,
            swap_target_cell: 0,
            show_warning_dialog: false,
            warning_message: String::new(),
            pending_warning_action: None,
            show_load_dialog: false,
            discovered_assets: Vec::new(),
            selected_asset_index: None,
            show_merge_dialog: false,
            merge_selected_indices: Vec::new(),
            merge_target_cols: 4,
            show_resize_dialog: false,
            resize_tiles_x: 1,
            resize_tiles_y: 1,
            resize_anchor: ResizeAnchor::default(),
            show_rename_dialog: false,
            rename_new_name: String::new(),
            show_delete_confirm: false,
            delete_asset_name: String::new(),
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
        self.original_cell_names = None;
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
        self.original_cell_names = None;
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
        self.original_cell_names = None;
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

    /// Append a new row of empty cells to the bottom of the sheet
    pub fn append_row(&mut self) -> bool {
        if !self.is_sheet() {
            return false;
        }

        let Some(canvas) = &self.canvas else {
            return false;
        };

        let old_width = canvas.width;
        let old_height = canvas.height;
        let new_height = old_height + self.cell_size.y;

        // Save state for undo
        let before = canvas.clone();

        // Create new larger canvas
        let mut new_canvas = SpriteCanvas::new(old_width, new_height);

        // Copy old pixels to new canvas
        for y in 0..old_height {
            for x in 0..old_width {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(x, y, color);
                }
            }
        }

        self.canvas = Some(new_canvas.clone());
        self.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Append a new column of empty cells to the right of the sheet
    pub fn append_column(&mut self) -> bool {
        if !self.is_sheet() {
            return false;
        }

        let Some(canvas) = &self.canvas else {
            return false;
        };

        let old_width = canvas.width;
        let old_height = canvas.height;
        let new_width = old_width + self.cell_size.x;

        // Save state for undo
        let before = canvas.clone();

        // Create new larger canvas
        let mut new_canvas = SpriteCanvas::new(new_width, old_height);

        // Copy old pixels to new canvas
        for y in 0..old_height {
            for x in 0..old_width {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(x, y, color);
                }
            }
        }

        self.canvas = Some(new_canvas.clone());
        self.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Delete the selected cell and collapse remaining cells to fill the gap
    pub fn delete_cell_with_collapse(&mut self) -> bool {
        let cell_idx = match self.selected_cell {
            Some(idx) => idx,
            None => return false,
        };

        let Some((cols, rows)) = self.sheet_cell_count() else {
            return false;
        };

        let total_cells = (cols * rows) as usize;
        if cell_idx >= total_cells {
            return false;
        }

        let Some(canvas) = &mut self.canvas else {
            return false;
        };

        // Save state for undo
        let before = canvas.clone();

        let cell_w = self.cell_size.x;
        let cell_h = self.cell_size.y;

        // Shift all cells after the deleted one to the left
        for i in cell_idx..(total_cells - 1) {
            let src_col = ((i + 1) as u32) % cols;
            let src_row = ((i + 1) as u32) / cols;
            let dst_col = (i as u32) % cols;
            let dst_row = (i as u32) / cols;

            // Copy pixels from source cell to destination cell
            for py in 0..cell_h {
                for px in 0..cell_w {
                    let src_x = src_col * cell_w + px;
                    let src_y = src_row * cell_h + py;
                    let dst_x = dst_col * cell_w + px;
                    let dst_y = dst_row * cell_h + py;

                    let color = canvas
                        .get_pixel(src_x, src_y)
                        .unwrap_or(PixelColor::transparent());
                    canvas.set_pixel(dst_x, dst_y, color);
                }
            }
        }

        // Clear the last cell (now empty after collapse)
        let last_idx = total_cells - 1;
        let last_col = (last_idx as u32) % cols;
        let last_row = (last_idx as u32) / cols;
        canvas.fill_rect(
            last_col * cell_w,
            last_row * cell_h,
            cell_w,
            cell_h,
            PixelColor::transparent(),
        );

        self.history.push(SpriteEditCommand {
            before,
            after: canvas.clone(),
        });

        // Deselect or select the same index (now contains next cell's content)
        if cell_idx >= total_cells - 1 {
            self.selected_cell = if total_cells > 1 {
                Some(total_cells - 2)
            } else {
                None
            };
        }

        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Flip the entire canvas horizontally
    pub fn flip_horizontal(&mut self) -> bool {
        let Some(canvas) = &self.canvas else {
            return false;
        };

        let before = canvas.clone();
        let w = canvas.width;
        let h = canvas.height;

        let mut new_canvas = SpriteCanvas::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(w - 1 - x, y, color);
                }
            }
        }

        self.canvas = Some(new_canvas.clone());
        self.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Flip the entire canvas vertically
    pub fn flip_vertical(&mut self) -> bool {
        let Some(canvas) = &self.canvas else {
            return false;
        };

        let before = canvas.clone();
        let w = canvas.width;
        let h = canvas.height;

        let mut new_canvas = SpriteCanvas::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(x, h - 1 - y, color);
                }
            }
        }

        self.canvas = Some(new_canvas.clone());
        self.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Rotate the entire canvas 90° clockwise
    pub fn rotate_clockwise(&mut self) -> bool {
        let Some(canvas) = &self.canvas else {
            return false;
        };

        let before = canvas.clone();
        let old_w = canvas.width;
        let old_h = canvas.height;

        // After 90° CW rotation: new_w = old_h, new_h = old_w
        let mut new_canvas = SpriteCanvas::new(old_h, old_w);
        for y in 0..old_h {
            for x in 0..old_w {
                if let Some(color) = before.get_pixel(x, y) {
                    // (x, y) -> (old_h - 1 - y, x)
                    new_canvas.set_pixel(old_h - 1 - y, x, color);
                }
            }
        }

        self.canvas = Some(new_canvas.clone());
        self.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });

        // Swap cell size dimensions for sheets
        if self.is_sheet() {
            std::mem::swap(&mut self.cell_size.x, &mut self.cell_size.y);
        }

        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Rotate the entire canvas 90° counter-clockwise
    pub fn rotate_counter_clockwise(&mut self) -> bool {
        let Some(canvas) = &self.canvas else {
            return false;
        };

        let before = canvas.clone();
        let old_w = canvas.width;
        let old_h = canvas.height;

        // After 90° CCW rotation: new_w = old_h, new_h = old_w
        let mut new_canvas = SpriteCanvas::new(old_h, old_w);
        for y in 0..old_h {
            for x in 0..old_w {
                if let Some(color) = before.get_pixel(x, y) {
                    // (x, y) -> (y, old_w - 1 - x)
                    new_canvas.set_pixel(y, old_w - 1 - x, color);
                }
            }
        }

        self.canvas = Some(new_canvas.clone());
        self.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });

        // Swap cell size dimensions for sheets
        if self.is_sheet() {
            std::mem::swap(&mut self.cell_size.x, &mut self.cell_size.y);
        }

        self.dirty = true;
        self.canvas_texture = None;
        true
    }

    /// Resize the canvas with anchor positioning
    pub fn resize_canvas(&mut self, new_w: u32, new_h: u32, anchor: ResizeAnchor) -> bool {
        let Some(canvas) = &self.canvas else {
            return false;
        };

        if new_w == 0 || new_h == 0 {
            return false;
        }

        let before = canvas.clone();
        let old_w = canvas.width;
        let old_h = canvas.height;

        // Calculate offset based on anchor
        let (offset_x, offset_y) = anchor.calculate_offset(old_w, old_h, new_w, new_h);

        let mut new_canvas = SpriteCanvas::new(new_w, new_h);

        // Copy pixels from old canvas to new position
        for y in 0..old_h {
            for x in 0..old_w {
                let new_x = x as i32 + offset_x;
                let new_y = y as i32 + offset_y;

                if new_x >= 0 && new_x < new_w as i32 && new_y >= 0 && new_y < new_h as i32 {
                    if let Some(color) = before.get_pixel(x, y) {
                        new_canvas.set_pixel(new_x as u32, new_y as u32, color);
                    }
                }
            }
        }

        self.canvas = Some(new_canvas.clone());
        self.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        self.dirty = true;
        self.canvas_texture = None;
        self.selected_cell = None;
        true
    }

    /// Open the resize canvas dialog
    pub fn begin_resize_dialog(&mut self) {
        if let Some((w, h)) = self.canvas_dimensions() {
            // Calculate current tile counts from canvas dimensions and cell size
            let cell_w = self.cell_size.x.max(1);
            let cell_h = self.cell_size.y.max(1);
            self.resize_tiles_x = w.div_ceil(cell_w);
            self.resize_tiles_y = h.div_ceil(cell_h);
            self.resize_anchor = ResizeAnchor::default();
            self.show_resize_dialog = true;
        }
    }

    /// Rename a sprite asset (both PNG and JSON files)
    pub fn rename_asset(
        sprites_dir: &std::path::Path,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), String> {
        // Validate new name
        if new_name.is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        if new_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']) {
            return Err("Name contains invalid characters".to_string());
        }
        if old_name == new_name {
            return Ok(()); // No change needed
        }

        let old_png = sprites_dir.join(format!("{old_name}.png"));
        let old_json = sprites_dir.join(format!("{old_name}.json"));
        let new_png = sprites_dir.join(format!("{new_name}.png"));
        let new_json = sprites_dir.join(format!("{new_name}.json"));

        // Check source files exist
        if !old_png.exists() {
            return Err(format!("Source PNG not found: {}", old_png.display()));
        }

        // Check target doesn't already exist
        if new_png.exists() {
            return Err(format!("Target already exists: {}", new_png.display()));
        }

        // Rename PNG
        std::fs::rename(&old_png, &new_png).map_err(|e| format!("Failed to rename PNG: {e}"))?;

        // Rename JSON if it exists
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

        // Delete PNG
        if png_path.exists() {
            std::fs::remove_file(&png_path).map_err(|e| format!("Failed to delete PNG: {e}"))?;
        }

        // Delete JSON if it exists
        if json_path.exists() {
            std::fs::remove_file(&json_path).map_err(|e| format!("Failed to delete JSON: {e}"))?;
        }

        Ok(())
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
        self.original_cell_names = None;
    }

    /// Open the save dialog
    pub fn begin_save_dialog(&mut self) {
        self.show_save_dialog = true;
        // Default to the existing name if editing
        if self.save_asset_name.is_empty() {
            self.save_asset_name = "new_sprite".to_string();
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

    /// Merge selected sprites into a new sheet canvas
    pub fn merge_sprites_into_sheet(&mut self) -> Result<(), String> {
        use toki_core::graphics::image::load_image_rgba8;

        if self.merge_selected_indices.is_empty() {
            return Err("No sprites selected for merge".to_string());
        }

        // Load all selected sprites
        let mut images: Vec<(u32, u32, Vec<u8>)> = Vec::new();
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

        // Calculate sheet dimensions
        let cols = self.merge_target_cols.max(1);
        let rows = (images.len() as u32).div_ceil(cols);
        let cell_w = max_width;
        let cell_h = max_height;
        let sheet_w = cols * cell_w;
        let sheet_h = rows * cell_h;

        // Create the merged canvas
        let mut canvas = SpriteCanvas::new(sheet_w, sheet_h);

        // Copy each image into its cell
        for (i, (img_w, img_h, data)) in images.iter().enumerate() {
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            let start_x = col * cell_w;
            let start_y = row * cell_h;

            // Copy pixels (centered if smaller than cell)
            let offset_x = (cell_w - img_w) / 2;
            let offset_y = (cell_h - img_h) / 2;

            for py in 0..*img_h {
                for px in 0..*img_w {
                    let src_idx = ((py * img_w + px) * 4) as usize;
                    let color = PixelColor::from_rgba_array([
                        data[src_idx],
                        data[src_idx + 1],
                        data[src_idx + 2],
                        data[src_idx + 3],
                    ]);
                    canvas.set_pixel(start_x + offset_x + px, start_y + offset_y + py, color);
                }
            }
        }

        // Update state
        self.canvas = Some(canvas);
        self.active_sprite = None;
        self.asset_kind = None;
        self.cell_size = glam::UVec2::new(cell_w, cell_h);
        self.show_cell_grid = true;
        self.dirty = true;
        self.history.clear();
        self.selection = None;
        self.canvas_texture = None;
        self.viewport = SpriteCanvasViewport::default();
        self.selected_cell = None;
        self.show_merge_dialog = false;
        self.original_cell_names = None; // Merged sheet has new names

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
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension() else {
                continue;
            };
            if ext != "json" {
                continue;
            }

            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };

            // Check for matching PNG file
            let png_path = sprites_dir.join(format!("{stem}.png"));
            if !png_path.exists() {
                continue;
            }

            // Classify the JSON file
            let kind = match classify_sprite_metadata_file(&path) {
                Ok(SpriteMetadataFileKind::Atlas) => SpriteAssetKind::TileAtlas,
                Ok(SpriteMetadataFileKind::ObjectSheet) => SpriteAssetKind::ObjectSheet,
                _ => continue,
            };

            assets.push(DiscoveredSpriteAsset {
                name: stem.to_string(),
                json_path: path,
                png_path,
                kind,
            });
        }

        // Sort by name
        assets.sort_by(|a, b| a.name.cmp(&b.name));
        assets
    }

    /// Load an existing sprite asset into the canvas
    pub fn load_sprite_asset(&mut self, asset: &DiscoveredSpriteAsset) -> Result<(), String> {
        use toki_core::assets::atlas::AtlasMeta;
        use toki_core::assets::object_sheet::ObjectSheetMeta;
        use toki_core::graphics::image::load_image_rgba8;

        // Load the PNG image
        let decoded =
            load_image_rgba8(&asset.png_path).map_err(|e| format!("Failed to load image: {e}"))?;

        // Create canvas from image data
        let canvas = SpriteCanvas::from_rgba(decoded.width, decoded.height, decoded.data)
            .ok_or_else(|| "Failed to create canvas from image data".to_string())?;

        // Load metadata to get cell size and original names
        let (cell_size, is_sheet, original_names) = match asset.kind {
            SpriteAssetKind::TileAtlas => {
                let meta = AtlasMeta::load_from_file(&asset.json_path)
                    .map_err(|e| format!("Failed to load atlas metadata: {e}"))?;
                let is_sheet = meta.tiles.len() > 1;
                // Extract tile names in order
                let mut names: Vec<_> = meta.tiles.keys().cloned().collect();
                names.sort(); // Ensure consistent order
                (meta.tile_size, is_sheet, names)
            }
            SpriteAssetKind::ObjectSheet => {
                let meta = ObjectSheetMeta::load_from_file(&asset.json_path)
                    .map_err(|e| format!("Failed to load object sheet metadata: {e}"))?;
                let is_sheet = meta.objects.len() > 1;
                // Extract object names in order
                let mut names: Vec<_> = meta.objects.keys().cloned().collect();
                names.sort(); // Ensure consistent order
                (meta.tile_size, is_sheet, names)
            }
        };

        // Update state
        self.canvas = Some(canvas);
        self.active_sprite = Some(asset.json_path.to_string_lossy().to_string());
        self.asset_kind = Some(asset.kind);
        self.save_asset_name = asset.name.clone();
        self.save_asset_kind = asset.kind;
        self.original_cell_names = Some(original_names);
        self.cell_size = cell_size;
        self.show_cell_grid = is_sheet;
        self.dirty = false;
        self.history.clear();
        self.selection = None;
        self.canvas_texture = None;
        self.viewport = SpriteCanvasViewport::default();
        self.selected_cell = None;
        self.show_load_dialog = false;

        Ok(())
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
            return Err(
                "Asset name can only contain letters, numbers, and underscores".to_string(),
            );
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
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    self.create_atlas_with_names(&png_filename, cols, rows)
                } else {
                    // Single tile atlas
                    AtlasMeta::new_single_tile(
                        &png_filename,
                        UVec2::new(canvas.width, canvas.height),
                    )
                };
                meta.save_to_file(&json_path)
                    .map_err(|e| format!("Failed to save metadata: {e}"))?;
            }
            SpriteAssetKind::ObjectSheet => {
                let meta = if self.is_sheet() {
                    let (cols, rows) = self.sheet_cell_count().unwrap_or((1, 1));
                    self.create_object_sheet_with_names(&png_filename, cols, rows)
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

    /// Create atlas metadata using original names if available
    fn create_atlas_with_names(
        &self,
        png_filename: &str,
        cols: u32,
        rows: u32,
    ) -> toki_core::assets::atlas::AtlasMeta {
        use std::collections::HashMap;
        use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};

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
            tile_size: self.cell_size,
            tiles,
        }
    }

    /// Create object sheet metadata using original names if available
    fn create_object_sheet_with_names(
        &self,
        png_filename: &str,
        cols: u32,
        rows: u32,
    ) -> toki_core::assets::object_sheet::ObjectSheetMeta {
        use std::collections::HashMap;
        use toki_core::assets::object_sheet::{ObjectSheetMeta, ObjectSheetType, ObjectSpriteInfo};

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
            tile_size: self.cell_size,
            objects,
        }
    }

    /// Get the name for a cell, using original name if available
    fn get_cell_name(&self, index: usize, total_cells: usize, prefix: &str) -> String {
        // Use original name if available and cell count matches
        if let Some(ref names) = self.original_cell_names {
            if names.len() == total_cells {
                if let Some(name) = names.get(index) {
                    return name.clone();
                }
            }
        }
        // Fall back to generated name
        format!("{}_{}", prefix, index)
    }

    /// Import an external image file (png, jpg, bmp) into the canvas
    pub fn import_external_image(&mut self, path: &std::path::Path) -> Result<(), String> {
        use toki_core::graphics::image::load_image_rgba8;

        // Load the image
        let decoded = load_image_rgba8(path).map_err(|e| format!("Failed to load image: {e}"))?;

        // Create canvas from image data
        let canvas = SpriteCanvas::from_rgba(decoded.width, decoded.height, decoded.data)
            .ok_or_else(|| "Failed to create canvas from image data".to_string())?;

        // Update state - treat as new unsaved sprite
        self.canvas = Some(canvas);
        self.active_sprite = None;
        self.asset_kind = None;
        self.save_asset_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported")
            .to_string();
        self.cell_size = glam::UVec2::new(decoded.width, decoded.height);
        self.show_cell_grid = false;
        self.dirty = true; // Mark as needing save
        self.history.clear();
        self.selection = None;
        self.canvas_texture = None;
        self.viewport = SpriteCanvasViewport::default();
        self.selected_cell = None;
        self.original_cell_names = None; // New import has no original names

        Ok(())
    }

    /// Export the current canvas as PNG
    pub fn export_as_png(&self, path: &std::path::Path) -> Result<(), String> {
        let canvas = self.canvas.as_ref().ok_or("No canvas to export")?;

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

impl EditorUI {
    /// Begin showing the new canvas dialog
    pub fn begin_new_sprite_canvas_dialog(&mut self) {
        self.sprite.show_new_canvas_dialog = true;
    }

    /// Submit new canvas creation request
    #[allow(dead_code)]
    pub fn submit_new_sprite_canvas(&mut self) {
        let width = self.sprite.new_sprite_width.max(1);
        let height = self.sprite.new_sprite_height.max(1);
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
