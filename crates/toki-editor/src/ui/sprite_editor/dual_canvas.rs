//! Dual canvas layout and per-canvas state.

use super::canvas::SpriteCanvas;
use super::history::SpriteEditorHistory;
use super::selection::SpriteSelection;
use super::types::SpriteAssetKind;
use super::viewport::SpriteCanvasViewport;

/// Layout mode for dual canvas view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DualCanvasLayout {
    /// Only show the active canvas
    #[default]
    Single,
    /// Side-by-side horizontal split
    Horizontal,
    /// Stacked vertical split
    Vertical,
}

/// Which canvas is active/focused
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CanvasSide {
    #[default]
    Left,
    Right,
}

impl CanvasSide {
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
}

/// Per-canvas state for the sprite editor
pub struct CanvasState {
    /// Currently active sprite asset path (JSON metadata file)
    pub active_sprite: Option<String>,
    /// In-memory canvas being edited
    pub canvas: Option<SpriteCanvas>,
    /// Viewport state (zoom, pan)
    pub viewport: SpriteCanvasViewport,
    /// Has unsaved changes
    pub dirty: bool,
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
    /// Asset kind being created/edited
    pub asset_kind: Option<SpriteAssetKind>,
    /// Grid cell size for sheet editing (width, height in pixels)
    pub cell_size: glam::UVec2,
    /// Selected cell index (for sheet editing)
    pub selected_cell: Option<usize>,
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
    /// Save dialog: asset name (without extension)
    pub save_asset_name: String,
    /// Save dialog: asset type (atlas vs object sheet)
    pub save_asset_kind: SpriteAssetKind,
    /// Original tile/object names from loaded asset (for preserving names on re-save)
    pub original_cell_names: Option<Vec<String>>,
    /// Swap target cell index (for cell reordering UI)
    pub swap_target_cell: u32,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            active_sprite: None,
            canvas: None,
            viewport: SpriteCanvasViewport::default(),
            dirty: false,
            selection: None,
            history: SpriteEditorHistory::new(50),
            show_grid: true,
            cursor_canvas_pos: None,
            canvas_texture: None,
            asset_kind: None,
            cell_size: glam::UVec2::new(16, 16),
            selected_cell: None,
            show_cell_grid: false,
            line_start_pos: None,
            selection_start_pos: None,
            canvas_before_stroke: None,
            is_painting: false,
            save_asset_name: String::new(),
            save_asset_kind: SpriteAssetKind::ObjectSheet,
            original_cell_names: None,
            swap_target_cell: 0,
        }
    }
}

#[allow(dead_code)]
impl CanvasState {
    /// Clear the canvas state (for new/close operations)
    pub fn clear(&mut self) {
        self.canvas = None;
        self.active_sprite = None;
        self.asset_kind = None;
        self.dirty = false;
        self.selection = None;
        self.history.clear();
        self.cursor_canvas_pos = None;
        self.canvas_texture = None;
        self.selected_cell = None;
        self.line_start_pos = None;
        self.selection_start_pos = None;
        self.canvas_before_stroke = None;
        self.is_painting = false;
        self.save_asset_name.clear();
        self.original_cell_names = None;
        self.swap_target_cell = 0;
    }

    /// Check if this canvas has content
    pub fn has_canvas(&self) -> bool {
        self.canvas.is_some()
    }

    /// Start a paint stroke (saves undo state)
    pub fn start_stroke(&mut self) {
        if !self.is_painting {
            self.canvas_before_stroke = self.canvas.clone();
            self.is_painting = true;
        }
    }

    /// End a paint stroke (pushes undo state if canvas changed)
    pub fn end_stroke(&mut self) {
        if self.is_painting {
            if let Some(before) = self.canvas_before_stroke.take() {
                if let Some(after) = &self.canvas {
                    if after != &before {
                        self.history.push(super::history::SpriteEditCommand {
                            before,
                            after: after.clone(),
                        });
                        self.dirty = true;
                    }
                }
            }
            self.is_painting = false;
        }
    }

    /// Perform undo operation
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.history.take_undo() {
            self.canvas = Some(prev);
            self.canvas_texture = None;
            self.dirty = true;
            return true;
        }
        false
    }

    /// Perform redo operation
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.history.take_redo() {
            self.canvas = Some(next);
            self.canvas_texture = None;
            self.dirty = true;
            return true;
        }
        false
    }
}

