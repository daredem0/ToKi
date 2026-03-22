//! Canvas access helpers and layout methods for SpriteEditorState.

use super::{
    CanvasSide, CanvasState, DualCanvasLayout, PixelColor, SpriteCanvas, SpriteCanvasViewport,
    SpriteEditorState,
};

impl SpriteEditorState {
    // ========== Canvas Access Helpers ==========

    /// Get a reference to the active canvas state
    pub fn active(&self) -> &CanvasState {
        &self.canvases[self.active_canvas.index()]
    }

    /// Get a mutable reference to the active canvas state
    pub fn active_mut(&mut self) -> &mut CanvasState {
        let idx = self.active_canvas.index();
        &mut self.canvases[idx]
    }

    /// Get a reference to the canvas state for a specific side
    pub fn canvas_state(&self, side: CanvasSide) -> &CanvasState {
        &self.canvases[side.index()]
    }

    /// Get a mutable reference to the canvas state for a specific side
    pub fn canvas_state_mut(&mut self, side: CanvasSide) -> &mut CanvasState {
        &mut self.canvases[side.index()]
    }

    /// Switch the active canvas to the other side
    pub fn switch_active_canvas(&mut self) {
        self.active_canvas = self.active_canvas.other();
    }

    /// Set the active canvas to a specific side
    pub fn set_active_canvas(&mut self, side: CanvasSide) {
        self.active_canvas = side;
    }

    /// Cycle through layout modes
    pub fn cycle_layout(&mut self) {
        self.layout = match self.layout {
            DualCanvasLayout::Single => DualCanvasLayout::Horizontal,
            DualCanvasLayout::Horizontal => DualCanvasLayout::Vertical,
            DualCanvasLayout::Vertical => DualCanvasLayout::Single,
        };
    }

    // ========== Canvas Operations (on active canvas) ==========

    /// Create a new empty canvas on the active canvas
    pub fn new_canvas(&mut self, width: u32, height: u32) {
        let cs = self.active_mut();
        cs.canvas = Some(SpriteCanvas::new(width, height));
        cs.active_sprite = None;
        cs.dirty = true;
        cs.history.clear();
        cs.selection = None;
        cs.canvas_texture = None;
        cs.viewport = SpriteCanvasViewport::default();
        cs.original_cell_names = None;
    }

    /// Create a new canvas filled with a color on the active canvas
    #[allow(dead_code)]
    pub fn new_canvas_filled(&mut self, width: u32, height: u32, color: PixelColor) {
        let cs = self.active_mut();
        cs.canvas = Some(SpriteCanvas::filled(width, height, color));
        cs.active_sprite = None;
        cs.dirty = true;
        cs.history.clear();
        cs.selection = None;
        cs.canvas_texture = None;
        cs.viewport = SpriteCanvasViewport::default();
        cs.original_cell_names = None;
    }

    /// Check if there's an active canvas being edited
    pub fn has_canvas(&self) -> bool {
        self.active().canvas.is_some()
    }

    /// Get canvas dimensions if a canvas is active
    pub fn canvas_dimensions(&self) -> Option<(u32, u32)> {
        self.active().canvas.as_ref().map(|c| (c.width, c.height))
    }

    /// Check if this is a sheet (has cell dimensions set)
    pub fn is_sheet(&self) -> bool {
        let cs = self.active();
        cs.cell_size.x > 0 && cs.cell_size.y > 0 && cs.show_cell_grid
    }

    /// Get the number of cells in the sheet (columns, rows)
    pub fn sheet_cell_count(&self) -> Option<(u32, u32)> {
        let cs = self.active();
        let (w, h) = self.canvas_dimensions()?;
        if cs.cell_size.x == 0 || cs.cell_size.y == 0 {
            return None;
        }
        Some((w / cs.cell_size.x, h / cs.cell_size.y))
    }

    /// Get the total number of cells in the sheet
    #[allow(dead_code)]
    pub fn total_cell_count(&self) -> Option<u32> {
        let (cols, rows) = self.sheet_cell_count()?;
        Some(cols * rows)
    }

    /// Get cell index from canvas position
    pub fn cell_at_position(&self, x: u32, y: u32) -> Option<usize> {
        if !self.is_sheet() {
            return None;
        }
        let cs = self.active();
        let (cols, _rows) = self.sheet_cell_count()?;
        let cell_x = x / cs.cell_size.x;
        let cell_y = y / cs.cell_size.y;
        Some((cell_y * cols + cell_x) as usize)
    }

    /// Get cell bounds (start_x, start_y, end_x, end_y) for a cell index
    pub fn cell_bounds(&self, cell_index: usize) -> Option<(u32, u32, u32, u32)> {
        let cs = self.active();
        let (cols, rows) = self.sheet_cell_count()?;
        let total = cols * rows;
        if cell_index as u32 >= total {
            return None;
        }
        let col = cell_index as u32 % cols;
        let row = cell_index as u32 / cols;
        let start_x = col * cs.cell_size.x;
        let start_y = row * cs.cell_size.y;
        let end_x = start_x + cs.cell_size.x;
        let end_y = start_y + cs.cell_size.y;
        Some((start_x, start_y, end_x, end_y))
    }

    /// Create a new sheet canvas on the active canvas
    pub fn new_sheet(&mut self, width: u32, height: u32, cell_width: u32, cell_height: u32) {
        let cs = self.active_mut();
        cs.canvas = Some(SpriteCanvas::new(width, height));
        cs.active_sprite = None;
        cs.dirty = true;
        cs.history.clear();
        cs.selection = None;
        cs.canvas_texture = None;
        cs.viewport = SpriteCanvasViewport::default();
        cs.cell_size = glam::UVec2::new(cell_width, cell_height);
        cs.show_cell_grid = true;
        cs.selected_cell = None;
        cs.original_cell_names = None;
    }

    /// Mark the active canvas as dirty
    pub fn mark_dirty(&mut self) {
        self.active_mut().dirty = true;
    }

    /// Clear the dirty flag on the active canvas
    pub fn clear_dirty(&mut self) {
        self.active_mut().dirty = false;
    }

    /// Close the active canvas
    pub fn close_canvas(&mut self) {
        let cs = self.active_mut();
        cs.canvas = None;
        cs.active_sprite = None;
        cs.dirty = false;
        cs.history.clear();
        cs.selection = None;
        cs.canvas_texture = None;
        cs.original_cell_names = None;
    }

    /// Add a color to recent colors palette
    pub fn add_recent_color(&mut self, color: PixelColor) {
        // Don't add transparent
        if color.a == 0 {
            return;
        }
        // Remove existing if present
        self.recent_colors.retain(|c| *c != color);
        // Add at front
        self.recent_colors.insert(0, color);
        // Trim to max
        self.recent_colors.truncate(self.max_recent_colors);
    }
}
