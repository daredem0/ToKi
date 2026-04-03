use glyphon::{Buffer, Metrics, Resolution, Shaping, TextArea, TextBounds};

use super::*;

impl GlyphonTextRenderer {
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

        let prepared = self.layout_text_items(
            items,
            world_to_screen_mvp,
            surface_width as f32,
            surface_height as f32,
        );
        let text_areas = build_text_areas(
            &self.cached_buffers,
            &prepared.entries,
            surface_width,
            surface_height,
        );

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

        self.prune_unused_buffers(&prepared.used_buffer_indices);
        Ok(prepared.backgrounds)
    }

    fn layout_text_items(
        &mut self,
        items: &[TextItem],
        world_to_screen_mvp: glam::Mat4,
        surface_width: f32,
        surface_height: f32,
    ) -> PreparedTextLayout {
        let mut sorted_items = items.to_vec();
        sorted_items.sort_by_key(|item| item.layer);

        let mut entries = Vec::new();
        let mut backgrounds = Vec::new();
        let mut used_buffer_indices = vec![false; self.cached_buffers.len()];

        for item in &sorted_items {
            if item.content.is_empty() {
                continue;
            }

            let Some(base_pos) =
                to_screen_position(item, world_to_screen_mvp, surface_width, surface_height)
            else {
                continue;
            };

            let estimated_size = estimate_text_size(item);
            let estimated_anchored_pos = apply_anchor(base_pos, estimated_size, item.anchor);
            let max_width = item
                .max_width
                .unwrap_or_else(|| (surface_width - estimated_anchored_pos.x).max(1.0));
            let key = borrowed_buffer_key(item, max_width, surface_height);
            let buffer_index = self.upsert_buffer(item, max_width, surface_height, key);
            if buffer_index >= used_buffer_indices.len() {
                used_buffer_indices.resize(self.cached_buffers.len(), false);
            }
            used_buffer_indices[buffer_index] = true;
            let actual_size = measure_buffer_size(&self.cached_buffers[buffer_index].buffer);
            let anchored_pos = apply_anchor(base_pos, actual_size, item.anchor);
            entries.push(PreparedTextEntry {
                buffer_index,
                left: anchored_pos.x,
                top: anchored_pos.y,
                color: color_from_rgba(item.style.color),
            });

            if let Some(box_style) = &item.box_style {
                backgrounds.push(background_rect_for(anchored_pos, actual_size, box_style));
            }
        }

        PreparedTextLayout {
            entries,
            backgrounds,
            used_buffer_indices,
        }
    }

    fn prune_unused_buffers(&mut self, used_buffer_indices: &[bool]) {
        prune_cached_buffers(&mut self.cached_buffers, used_buffer_indices);
    }

    fn upsert_buffer(
        &mut self,
        item: &TextItem,
        max_width: f32,
        layout_height: f32,
        key: BorrowedTextBufferKey<'_>,
    ) -> usize {
        if let Some(existing_index) = self
            .cached_buffers
            .iter()
            .position(|entry| entry.key.matches(key))
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
            key: key.into_owned(),
            buffer,
        });
        self.cached_buffers.len() - 1
    }
}

fn build_text_areas<'a>(
    cached_buffers: &'a [CachedTextBuffer],
    entries: &'a [PreparedTextEntry],
    surface_width: u32,
    surface_height: u32,
) -> Vec<TextArea<'a>> {
    entries
        .iter()
        .map(|entry| TextArea {
            buffer: &cached_buffers[entry.buffer_index].buffer,
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
        .collect()
}
