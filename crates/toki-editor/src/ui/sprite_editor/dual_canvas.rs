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
