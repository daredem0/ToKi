mod prepare;

use crate::RenderError;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Style, SwashCache, TextAtlas,
    TextRenderer, Viewport, Weight,
};
use toki_core::fonts::{builtin_font_family, BuiltinFontFamily};
use toki_core::text::{TextAnchor, TextBoxStyle, TextItem, TextSlant, TextSpace, TextStyle};

#[derive(Debug, Clone, PartialEq)]
pub struct TextBackgroundRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub background_color: [f32; 4],
    pub border_color: Option<[f32; 4]>,
}

#[derive(Debug)]
pub(super) struct PreparedTextEntry {
    buffer_index: usize,
    left: f32,
    top: f32,
    color: Color,
}

#[derive(Debug)]
pub(super) struct PreparedTextLayout {
    entries: Vec<PreparedTextEntry>,
    backgrounds: Vec<TextBackgroundRect>,
    used_buffer_indices: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextBufferKey {
    content: String,
    font_family: String,
    size_px_bits: u32,
    weight: toki_core::text::TextWeight,
    slant: TextSlant,
    max_width_px: u32,
    layout_height_px: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BorrowedTextBufferKey<'a> {
    content: &'a str,
    font_family: &'a str,
    size_px_bits: u32,
    weight: toki_core::text::TextWeight,
    slant: TextSlant,
    max_width_px: u32,
    layout_height_px: u32,
}

struct CachedTextBuffer {
    key: TextBufferKey,
    buffer: Buffer,
}

pub struct GlyphonTextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    viewport: Viewport,
    renderer: TextRenderer,
    cached_buffers: Vec<CachedTextBuffer>,
}

impl GlyphonTextRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let mut font_system = FontSystem::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let viewport = Viewport::new(device, &cache);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        // Preload fallback font metrics context.
        let _ = Buffer::new(&mut font_system, Metrics::new(16.0, 20.0));

        Self {
            font_system,
            swash_cache: SwashCache::new(),
            atlas,
            viewport,
            renderer,
            cached_buffers: Vec::new(),
        }
    }

    pub fn render<'a>(
        &'a mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) -> Result<(), RenderError> {
        self.renderer
            .render(&self.atlas, &self.viewport, render_pass)
            .map_err(|error| RenderError::Other(format!("text render failed: {error}")))?;
        Ok(())
    }

    pub fn trim_cache(&mut self) {
        self.atlas.trim();
    }

    pub fn load_font_file(&mut self, path: &std::path::Path) -> Result<(), RenderError> {
        self.font_system
            .db_mut()
            .load_font_file(path)
            .map(|_| ())
            .map_err(|error| {
                RenderError::Other(format!("failed to load font '{}': {error}", path.display()))
            })
    }
}

#[cfg(test)]
fn make_buffer_key(item: &TextItem, max_width: f32, layout_height: f32) -> TextBufferKey {
    TextBufferKey {
        content: item.content.clone(),
        font_family: item.style.font_family.clone(),
        size_px_bits: item.style.size_px.to_bits(),
        weight: item.style.weight,
        slant: item.style.slant,
        max_width_px: max_width.round().max(1.0) as u32,
        layout_height_px: layout_height.round().max(1.0) as u32,
    }
}

fn borrowed_buffer_key(
    item: &TextItem,
    max_width: f32,
    layout_height: f32,
) -> BorrowedTextBufferKey<'_> {
    BorrowedTextBufferKey {
        content: item.content.as_str(),
        font_family: item.style.font_family.as_str(),
        size_px_bits: item.style.size_px.to_bits(),
        weight: item.style.weight,
        slant: item.style.slant,
        max_width_px: max_width.round().max(1.0) as u32,
        layout_height_px: layout_height.round().max(1.0) as u32,
    }
}

impl<'a> BorrowedTextBufferKey<'a> {
    fn into_owned(self) -> TextBufferKey {
        TextBufferKey {
            content: self.content.to_string(),
            font_family: self.font_family.to_string(),
            size_px_bits: self.size_px_bits,
            weight: self.weight,
            slant: self.slant,
            max_width_px: self.max_width_px,
            layout_height_px: self.layout_height_px,
        }
    }
}

impl TextBufferKey {
    fn matches(&self, other: BorrowedTextBufferKey<'_>) -> bool {
        self.content == other.content
            && self.font_family == other.font_family
            && self.size_px_bits == other.size_px_bits
            && self.weight == other.weight
            && self.slant == other.slant
            && self.max_width_px == other.max_width_px
            && self.layout_height_px == other.layout_height_px
    }
}

fn prune_cached_buffers(cached_buffers: &mut Vec<CachedTextBuffer>, used_buffer_indices: &[bool]) {
    let old_buffers = std::mem::take(cached_buffers);
    *cached_buffers = old_buffers
        .into_iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            used_buffer_indices
                .get(index)
                .copied()
                .unwrap_or(false)
                .then_some(entry)
        })
        .collect();
}

pub fn to_screen_position(
    item: &TextItem,
    world_to_screen_mvp: glam::Mat4,
    surface_width: f32,
    surface_height: f32,
) -> Option<glam::Vec2> {
    match item.space {
        TextSpace::Screen => Some(item.position),
        TextSpace::World => {
            let world = glam::Vec4::new(item.position.x, item.position.y, 0.0, 1.0);
            let clip = world_to_screen_mvp * world;
            if clip.w.abs() < f32::EPSILON {
                return None;
            }
            let ndc = clip / clip.w;
            if !(-1.25..=1.25).contains(&ndc.x) || !(-1.25..=1.25).contains(&ndc.y) {
                return None;
            }
            let x = (ndc.x * 0.5 + 0.5) * surface_width;
            let y = (1.0 - (ndc.y * 0.5 + 0.5)) * surface_height;
            Some(glam::Vec2::new(x, y))
        }
    }
}

pub fn estimate_text_size(item: &TextItem) -> glam::Vec2 {
    let char_width = item.style.size_px * 0.55;
    let line_height = item.style.size_px * 1.25;
    let estimated_width = item.content.chars().count() as f32 * char_width;
    let width = item
        .max_width
        .map(|limit| estimated_width.min(limit))
        .unwrap_or(estimated_width)
        .max(1.0);
    let line_count = if let Some(max_width) = item.max_width {
        (estimated_width / max_width.max(1.0)).ceil().max(1.0)
    } else {
        1.0
    };
    let height = (line_count * line_height).max(line_height);
    glam::Vec2::new(width, height)
}

fn measure_buffer_size(buffer: &Buffer) -> glam::Vec2 {
    let mut width = 0.0f32;
    let mut height = 0.0f32;
    for run in buffer.layout_runs() {
        width = width.max(run.line_w);
        height = height.max(run.line_top + run.line_height);
    }
    if height <= 0.0 {
        height = buffer.metrics().line_height.max(1.0);
    }
    glam::Vec2::new(width.max(1.0), height.max(1.0))
}

pub fn apply_anchor(position: glam::Vec2, size: glam::Vec2, anchor: TextAnchor) -> glam::Vec2 {
    let x = match anchor {
        TextAnchor::TopLeft | TextAnchor::CenterLeft | TextAnchor::BottomLeft => position.x,
        TextAnchor::TopCenter | TextAnchor::Center | TextAnchor::BottomCenter => {
            position.x - size.x * 0.5
        }
        TextAnchor::TopRight | TextAnchor::CenterRight | TextAnchor::BottomRight => {
            position.x - size.x
        }
    };
    let y = match anchor {
        TextAnchor::TopLeft | TextAnchor::TopCenter | TextAnchor::TopRight => position.y,
        TextAnchor::CenterLeft | TextAnchor::Center | TextAnchor::CenterRight => {
            position.y - size.y * 0.5
        }
        TextAnchor::BottomLeft | TextAnchor::BottomCenter | TextAnchor::BottomRight => {
            position.y - size.y
        }
    };
    glam::Vec2::new(x, y)
}

fn background_rect_for(
    anchored_pos: glam::Vec2,
    estimated_text_size: glam::Vec2,
    box_style: &TextBoxStyle,
) -> TextBackgroundRect {
    let padded_origin = anchored_pos - box_style.padding;
    let padded_size = estimated_text_size + box_style.padding * 2.0;
    TextBackgroundRect {
        x: padded_origin.x,
        y: padded_origin.y,
        width: padded_size.x.max(1.0),
        height: padded_size.y.max(1.0),
        background_color: box_style.background_color,
        border_color: box_style.border_color,
    }
}

fn attrs_for_style(style: &TextStyle) -> Attrs<'_> {
    let family = match builtin_font_family(&style.font_family) {
        Some(BuiltinFontFamily::Sans) => Family::SansSerif,
        Some(BuiltinFontFamily::Serif) => Family::Serif,
        Some(BuiltinFontFamily::Mono) => Family::Monospace,
        None => Family::Name(&style.font_family),
    };
    let mut attrs = Attrs::new().family(family);
    attrs = attrs.weight(match style.weight {
        toki_core::text::TextWeight::Normal => Weight::NORMAL,
        toki_core::text::TextWeight::Bold => Weight::BOLD,
    });
    attrs = attrs.style(match style.slant {
        TextSlant::Normal => Style::Normal,
        TextSlant::Italic => Style::Italic,
    });
    attrs
}

fn color_from_rgba(rgba: [f32; 4]) -> Color {
    let to_u8 = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::rgba(
        to_u8(rgba[0]),
        to_u8(rgba[1]),
        to_u8(rgba[2]),
        to_u8(rgba[3]),
    )
}

#[cfg(test)]
#[path = "../text_tests.rs"]
mod tests;
