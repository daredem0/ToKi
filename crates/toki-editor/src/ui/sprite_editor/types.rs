//! Basic types for the sprite editor.
//!
//! Contains fundamental types like colors, tools, and asset kinds.

use std::path::PathBuf;

/// Tool for sprite/pixel editing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpriteEditorTool {
    #[default]
    Drag,
    Brush,
    Eraser,
    Fill,
    Eyedropper,
    Select,
    Line,
    /// Magic wand: select all connected non-transparent pixels
    MagicWand,
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
    pub json_path: PathBuf,
    /// Full path to PNG image file
    pub png_path: PathBuf,
    /// Asset kind (atlas or object sheet)
    pub kind: SpriteAssetKind,
}
