//! Basic types for the sprite editor.
//!
//! Contains fundamental types like colors, tools, and asset kinds.

use std::path::PathBuf;
use toki_core::palette::Palette4;

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
    /// Remove connected pixels of the clicked color within the clicked tile
    MagicErase,
    /// Add an outline around the clicked connected sprite region within the clicked tile
    AddOutline,
    /// Add a ground shadow below the clicked connected sprite region within the clicked tile
    AddShadow,
    /// Draw a rectangle (outline or filled)
    Rectangle,
    /// Draw an ellipse (outline or filled)
    Ellipse,
}

/// Type of sprite asset being edited
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl PixelColor {
    /// Create a new color with explicit RGBA values.
    #[cfg_attr(not(test), allow(dead_code))]
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

    /// White color constant.
    #[cfg_attr(not(test), allow(dead_code))]
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

pub const fn canonical_indexed_color(slot: usize) -> PixelColor {
    match slot {
        0 => PixelColor::rgb(0x00, 0x00, 0x00),
        1 => PixelColor::rgb(0x55, 0x55, 0x55),
        2 => PixelColor::rgb(0xAA, 0xAA, 0xAA),
        _ => PixelColor::rgb(0xFF, 0xFF, 0xFF),
    }
}

pub fn indexed_slot_for_canonical_color(color: PixelColor) -> Option<usize> {
    if color.a == 0 {
        return None;
    }

    match [color.r, color.g, color.b] {
        [0x00, 0x00, 0x00] => Some(0),
        [0x55, 0x55, 0x55] => Some(1),
        [0xAA, 0xAA, 0xAA] => Some(2),
        [0xFF, 0xFF, 0xFF] => Some(3),
        _ => None,
    }
}

pub fn indexed_slot_for_authored_color(
    color: PixelColor,
    palette: Option<Palette4>,
) -> Option<usize> {
    indexed_slot_for_canonical_color(color).or_else(|| {
        palette.and_then(|palette| {
            palette.colors.iter().position(|candidate| {
                color.a != 0
                    && [color.r, color.g, color.b] == [candidate[0], candidate[1], candidate[2]]
            })
        })
    })
}

pub fn preview_indexed_color(color: PixelColor, palette: Palette4) -> PixelColor {
    if color.a == 0 {
        return PixelColor::transparent();
    }

    let Some(slot) = indexed_slot_for_canonical_color(color) else {
        return color;
    };
    let target = palette.colors[slot];
    PixelColor::new(
        target[0],
        target[1],
        target[2],
        ((color.a as u16 * target[3] as u16) / 255) as u8,
    )
}

pub fn nearest_palette_slot(color: PixelColor, palette: Palette4) -> usize {
    let [r, g, b, _a] = color.to_rgba_array();
    palette
        .colors
        .iter()
        .enumerate()
        .min_by_key(|(_, candidate)| {
            let dr = r as i32 - candidate[0] as i32;
            let dg = g as i32 - candidate[1] as i32;
            let db = b as i32 - candidate[2] as i32;
            dr * dr + dg * dg + db * db
        })
        .map(|(slot, _)| slot)
        .unwrap_or(3)
}

/// Dithering pattern for brush tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DitherPattern {
    /// No dithering — paint all pixels
    #[default]
    None,
    /// 50% checkerboard — every other pixel
    Checker50,
    /// 25% sparse — 1 in 4 pixels
    Checker25,
    /// 75% dense — 3 in 4 pixels
    Checker75,
}

impl DitherPattern {
    pub const ALL: [Self; 4] = [
        Self::None,
        Self::Checker50,
        Self::Checker25,
        Self::Checker75,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Checker50 => "50% Checker",
            Self::Checker25 => "25% Sparse",
            Self::Checker75 => "75% Dense",
        }
    }
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

/// Which corner of a floating selection is being dragged for resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Active resize-drag state while the user is dragging a corner handle.
#[derive(Debug, Clone)]
pub struct ResizeDrag {
    /// Which corner the user grabbed.
    pub corner: ResizeCorner,
    /// The opposite (fixed) corner in canvas coordinates.
    pub anchor_canvas: glam::IVec2,
    /// Original width / height ratio for aspect-ratio locking.
    pub aspect_ratio: f32,
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
