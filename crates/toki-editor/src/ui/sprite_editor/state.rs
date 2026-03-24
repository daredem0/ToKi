//! Sprite editor state - the main container for sprite editing state.

use super::{
    canonical_indexed_color, indexed_slot_for_authored_color, CanvasSide, CanvasState,
    DiscoveredSpriteAsset, DualCanvasLayout, PixelColor, ResizeAnchor, SpriteCanvas,
    SpriteEditorTool,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use toki_core::assets::atlas::ColorMode;
use toki_core::palette::Palette4;

/// Sprite editor state
pub struct SpriteEditorState {
    /// Dual canvas states (left and right)
    pub canvases: [CanvasState; 2],
    /// Which canvas is currently active
    pub active_canvas: CanvasSide,
    /// Layout mode for canvas display
    pub layout: DualCanvasLayout,
    /// Split ratio for dual canvas layout (0.0 to 1.0, where 0.5 is equal split)
    pub split_ratio: f32,
    /// Current editing tool (shared across canvases)
    pub tool: SpriteEditorTool,
    /// Clipboard for copy/paste operations (shared across canvases)
    pub clipboard: Option<SpriteCanvas>,
    /// Current foreground color (shared across canvases)
    pub foreground_color: PixelColor,
    /// Current color interpretation mode for atlas authoring.
    pub color_mode: ColorMode,
    /// Selected palette id when authoring indexed atlas art.
    pub selected_palette_id: Option<String>,
    /// Brush size in pixels (shared across canvases)
    pub brush_size: u32,
    /// Recent colors palette (shared across canvases)
    pub recent_colors: Vec<PixelColor>,
    /// Maximum recent colors to remember
    pub max_recent_colors: usize,
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
    /// Optional source image for creating a new canvas from an existing PNG.
    pub new_canvas_source_image: Option<PathBuf>,
    /// Cached dimensions of the source image selected for new-canvas-from-image flow.
    pub new_canvas_source_image_size: Option<glam::UVec2>,
    /// Inline error shown in the new canvas dialog.
    pub new_canvas_error: Option<String>,
    /// Show save sprite dialog
    pub show_save_dialog: bool,
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
    /// Flag to indicate project assets need rescanning (after save/delete/rename)
    pub needs_asset_rescan: bool,
}

impl Default for SpriteEditorState {
    fn default() -> Self {
        Self {
            canvases: [CanvasState::default(), CanvasState::default()],
            active_canvas: CanvasSide::default(),
            layout: DualCanvasLayout::default(),
            split_ratio: 0.5,
            tool: SpriteEditorTool::Drag,
            clipboard: None,
            foreground_color: PixelColor::black(),
            color_mode: ColorMode::TrueColor,
            selected_palette_id: None,
            brush_size: 1,
            recent_colors: Vec::new(),
            max_recent_colors: 16,
            show_new_canvas_dialog: false,
            new_sprite_width: 16,
            new_sprite_height: 16,
            new_sheet_cols: 4,
            new_sheet_rows: 4,
            new_canvas_is_sheet: false,
            new_canvas_source_image: None,
            new_canvas_source_image_size: None,
            new_canvas_error: None,
            show_save_dialog: false,
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
            needs_asset_rescan: false,
        }
    }
}

impl SpriteEditorState {
    pub fn sync_palette_selection(&mut self, available_palettes: &BTreeMap<String, Palette4>) {
        if self.color_mode != ColorMode::PaletteIndexed {
            self.selected_palette_id = None;
            return;
        }

        if self
            .selected_palette_id
            .as_ref()
            .is_none_or(|palette_id| !available_palettes.contains_key(palette_id))
        {
            self.selected_palette_id = available_palettes.keys().next().cloned();
        }

        let selected_palette = self
            .selected_palette_id
            .as_ref()
            .and_then(|palette_id| available_palettes.get(palette_id))
            .copied();

        if indexed_slot_for_authored_color(self.foreground_color, selected_palette).is_none() {
            self.foreground_color = canonical_indexed_color(3);
        }
    }
}
