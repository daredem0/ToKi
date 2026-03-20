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

    /// Zoom in by one step
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 2.0).min(self.zoom_max);
    }

    /// Zoom out by one step
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 2.0).max(self.zoom_min);
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
    pub fn screen_to_canvas(&self, screen_pos: glam::Vec2, viewport_rect: egui::Rect) -> glam::Vec2 {
        let viewport_pos = screen_pos - glam::Vec2::new(viewport_rect.left(), viewport_rect.top());
        viewport_pos / self.zoom + self.pan
    }

    /// Convert canvas position to screen position
    pub fn canvas_to_screen(&self, canvas_pos: glam::Vec2, viewport_rect: egui::Rect) -> glam::Vec2 {
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
    /// Line tool: start position when dragging
    pub line_start_pos: Option<glam::IVec2>,
    /// Selection tool: start position when dragging
    pub selection_start_pos: Option<glam::IVec2>,
    /// Canvas state before current stroke (for undo)
    pub canvas_before_stroke: Option<SpriteCanvas>,
    /// Whether currently in a paint stroke
    pub is_painting: bool,
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
            line_start_pos: None,
            selection_start_pos: None,
            canvas_before_stroke: None,
            is_painting: false,
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
}

impl EditorUI {
    /// Begin showing the new canvas dialog
    pub fn begin_new_sprite_canvas_dialog(&mut self) {
        self.sprite.show_new_canvas_dialog = true;
    }

    /// Submit new canvas creation request
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
