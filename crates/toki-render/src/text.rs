use crate::RenderError;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping, Style,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport, Weight,
};
use std::collections::HashSet;
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
struct PreparedTextEntry {
    buffer_index: usize,
    left: f32,
    top: f32,
    color: Color,
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

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_width: u32,
        surface_height: u32,
        items: &[TextItem],
        world_to_screen_mvp: glam::Mat4,
    ) -> Result<Vec<TextBackgroundRect>, RenderError> {
        self.viewport.update(
            queue,
            Resolution {
                width: surface_width,
                height: surface_height,
            },
        );

        let mut sorted_items = items.to_vec();
        sorted_items.sort_by_key(|item| item.layer);

        let mut entries = Vec::new();
        let mut backgrounds = Vec::new();
        let mut used_keys = HashSet::new();

        for item in &sorted_items {
            if item.content.is_empty() {
                continue;
            }

            let Some(base_pos) = to_screen_position(
                item,
                world_to_screen_mvp,
                surface_width as f32,
                surface_height as f32,
            ) else {
                continue;
            };

            let estimated_size = estimate_text_size(item);
            let anchored_pos = apply_anchor(base_pos, estimated_size, item.anchor);

            let max_width = item
                .max_width
                .unwrap_or_else(|| (surface_width as f32 - anchored_pos.x).max(1.0));
            let key = make_buffer_key(item, max_width, surface_height as f32);
            let buffer_index = self.upsert_buffer(item, max_width, surface_height as f32, &key);
            used_keys.insert(key);
            entries.push(PreparedTextEntry {
                buffer_index,
                left: anchored_pos.x,
                top: anchored_pos.y,
                color: color_from_rgba(item.style.color),
            });

            if let Some(box_style) = &item.box_style {
                backgrounds.push(background_rect_for(anchored_pos, estimated_size, box_style));
            }
        }

        let text_areas: Vec<TextArea<'_>> = entries
            .iter()
            .map(|entry| TextArea {
                buffer: &self.cached_buffers[entry.buffer_index].buffer,
                left: entry.left,
                top: entry.top,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: surface_width as i32,
                    bottom: surface_height as i32,
                },
                default_color: entry.color,
                custom_glyphs: &[],
            })
            .collect();

        self.renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .map_err(|error| RenderError::Other(format!("text prepare failed: {error}")))?;

        self.cached_buffers
            .retain(|entry| used_keys.contains(&entry.key));

        Ok(backgrounds)
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

    fn upsert_buffer(
        &mut self,
        item: &TextItem,
        max_width: f32,
        layout_height: f32,
        key: &TextBufferKey,
    ) -> usize {
        if let Some(existing_index) = self
            .cached_buffers
            .iter()
            .position(|entry| &entry.key == key)
        {
            return existing_index;
        }

        let mut buffer = Buffer::new(
            &mut self.font_system,
            Metrics::new(item.style.size_px, item.style.size_px * 1.25),
        );
        buffer.set_size(
            &mut self.font_system,
            Some(max_width.max(1.0)),
            Some(layout_height.max(1.0)),
        );
        let attrs = attrs_for_style(&item.style);
        let shaping = if item.content.is_ascii() {
            Shaping::Basic
        } else {
            Shaping::Advanced
        };
        buffer.set_text(&mut self.font_system, &item.content, &attrs, shaping);
        buffer.shape_until_scroll(&mut self.font_system, false);

        self.cached_buffers.push(CachedTextBuffer {
            key: key.clone(),
            buffer,
        });
        self.cached_buffers.len() - 1
    }
}

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
    let mut attrs = Attrs::new().family(Family::Name(&style.font_family));
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
mod tests {
    use super::{apply_anchor, estimate_text_size, make_buffer_key, to_screen_position};
    use toki_core::text::{TextAnchor, TextItem, TextSpace, TextStyle};

    #[test]
    fn apply_anchor_center_offsets_by_half_size() {
        let anchored = apply_anchor(
            glam::Vec2::new(100.0, 100.0),
            glam::Vec2::new(40.0, 20.0),
            TextAnchor::Center,
        );
        assert_eq!(anchored, glam::Vec2::new(80.0, 90.0));
    }

    #[test]
    fn estimate_text_size_uses_max_width_for_wrapping() {
        let item = TextItem::new_screen(
            "A very long line",
            glam::Vec2::ZERO,
            TextStyle {
                size_px: 20.0,
                ..TextStyle::default()
            },
        )
        .with_max_width(40.0);
        let size = estimate_text_size(&item);
        assert!(size.x <= 40.0);
        assert!(size.y > 25.0);
    }

    #[test]
    fn to_screen_position_passes_screen_space_directly() {
        let item = TextItem::new_screen("HUD", glam::Vec2::new(8.0, 12.0), TextStyle::default());
        let screen = to_screen_position(&item, glam::Mat4::IDENTITY, 320.0, 180.0)
            .expect("screen-space text should map directly");
        assert_eq!(screen, glam::Vec2::new(8.0, 12.0));
    }

    #[test]
    fn to_screen_position_projects_world_space_coordinates() {
        let mut item = TextItem::new_world("NPC", glam::Vec2::ZERO, TextStyle::default());
        item.space = TextSpace::World;
        let screen = to_screen_position(&item, glam::Mat4::IDENTITY, 200.0, 100.0)
            .expect("origin should project into viewport");
        assert_eq!(screen, glam::Vec2::new(100.0, 50.0));
    }

    #[test]
    fn buffer_key_ignores_position_and_color_for_layout_reuse() {
        let item_a = TextItem::new_screen(
            "FPS: 60",
            glam::Vec2::new(8.0, 8.0),
            TextStyle {
                color: [1.0, 1.0, 1.0, 1.0],
                ..TextStyle::default()
            },
        );
        let item_b = TextItem::new_screen(
            "FPS: 60",
            glam::Vec2::new(200.0, 120.0),
            TextStyle {
                color: [0.2, 1.0, 0.2, 1.0],
                ..TextStyle::default()
            },
        );

        let key_a = make_buffer_key(&item_a, 180.0, 320.0);
        let key_b = make_buffer_key(&item_b, 180.0, 320.0);
        assert_eq!(key_a, key_b);
    }
}
