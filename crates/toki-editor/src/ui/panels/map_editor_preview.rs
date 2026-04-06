use super::*;
use crate::editor_viewport::EditorViewportContext;
use crate::ui::editor_ui::MapEditorTool;
use crate::ui::EditorUI;

impl PanelSystem {
    pub(super) fn paint_map_editor_empty_tile_checkerboard(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        viewport_ctx: &EditorViewportContext,
    ) {
        let Some(tilemap) = viewport.tilemap() else {
            return;
        };
        if tilemap.size.x == 0 || tilemap.size.y == 0 {
            return;
        }

        let tile_size = tilemap.tile_size.max(glam::UVec2::ONE);
        let (camera_position, camera_scale) = viewport.camera_state();
        let (viewport_width, viewport_height) = viewport.viewport_size();
        let world_span_x = (viewport_width as f32 * camera_scale).ceil() as i32;
        let world_span_y = (viewport_height as f32 * camera_scale).ceil() as i32;
        let visible_min = EditorViewportContext::world_to_tile_coords(camera_position, tile_size)
            .max(glam::IVec2::ZERO);
        let visible_max = EditorViewportContext::world_to_tile_coords(
            glam::IVec2::new(
                camera_position.x + world_span_x.saturating_sub(1),
                camera_position.y + world_span_y.saturating_sub(1),
            ),
            tile_size,
        )
        .min(glam::IVec2::new(
            tilemap.size.x.saturating_sub(1) as i32,
            tilemap.size.y.saturating_sub(1) as i32,
        ));
        if visible_min.x > visible_max.x || visible_min.y > visible_max.y {
            return;
        }

        let texture = Self::ensure_map_editor_empty_checkerboard_texture(
            ui_state,
            ui.ctx(),
            viewport,
            tilemap,
            visible_min,
            visible_max,
        );
        let Some(texture) = texture else {
            return;
        };
        let Some(first_tile_rect) = viewport_ctx.tile_screen_rect(
            tile_size,
            glam::UVec2::new(visible_min.x as u32, visible_min.y as u32),
        ) else {
            return;
        };
        let Some(last_tile_rect) = viewport_ctx.tile_screen_rect(
            tile_size,
            glam::UVec2::new(visible_max.x as u32, visible_max.y as u32),
        ) else {
            return;
        };
        let checkerboard_rect = egui::Rect::from_min_max(first_tile_rect.min, last_tile_rect.max);
        let painter = ui.painter().with_clip_rect(viewport_ctx.display_rect());
        painter.image(
            texture.id(),
            checkerboard_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    pub(super) fn paint_map_editor_brush_preview(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        viewport_ctx: &EditorViewportContext,
        project_path: &std::path::Path,
        renderer: Option<&mut crate::rendering::WindowRenderer>,
    ) {
        if crate::ui::editor_context::map_state(ui_state).tool != MapEditorTool::Brush {
            return;
        }
        let Some(selected_tile) = crate::ui::editor_context::map_state(ui_state)
            .selected_tile
            .clone()
        else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if !viewport_ctx.contains_screen_pos(pointer_pos) {
            return;
        }
        let Some(tilemap) = viewport.tilemap() else {
            return;
        };
        let world_pos = viewport_ctx.screen_to_world(pointer_pos);
        let Some(center_tile) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos)
        else {
            return;
        };
        let Some((start_tile, end_tile)) = MapPaintInteraction::brush_footprint_bounds(
            tilemap,
            center_tile,
            crate::ui::editor_context::map_state(ui_state).brush_size_tiles,
        ) else {
            return;
        };
        let Some(brush_source) = crate::ui::editor_ui::load_map_editor_brush_source_for_tilemap(
            ui_state,
            project_path,
            tilemap,
        ) else {
            return;
        };
        let Some(brush_entry) = crate::ui::editor_ui::selected_map_editor_brush_entry(
            &brush_source.brush_entries,
            Some(selected_tile.as_str()),
        ) else {
            return;
        };
        let Some(atlas_name) =
            crate::ui::editor_ui::map_editor_brush_entry_atlas_name(&brush_entry.id)
        else {
            return;
        };
        let Some(atlas_source) = brush_source.atlases.get(atlas_name) else {
            return;
        };
        let Some(texture_path) = atlas_source
            .path
            .parent()
            .map(|parent| parent.join(&atlas_source.meta.image))
        else {
            return;
        };
        let Some(texture) = Self::ensure_map_editor_brush_preview_texture(
            ui_state,
            ui.ctx(),
            &atlas_source.meta,
            &texture_path,
            renderer,
        ) else {
            return;
        };
        let Some(texture_size) = atlas_source.meta.image_size() else {
            return;
        };
        let Some(preview_tile_id) = brush_entry.preview_tile_id.as_deref() else {
            return;
        };
        let Some(tile_rect_px) = atlas_source.meta.get_tile_rect(preview_tile_id) else {
            return;
        };
        let uv_rect = egui::Rect::from_min_max(
            egui::pos2(
                tile_rect_px[0] as f32 / texture_size.x as f32,
                tile_rect_px[1] as f32 / texture_size.y as f32,
            ),
            egui::pos2(
                (tile_rect_px[0] + tile_rect_px[2]) as f32 / texture_size.x as f32,
                (tile_rect_px[1] + tile_rect_px[3]) as f32 / texture_size.y as f32,
            ),
        );
        let display_rect = viewport_ctx.display_rect();
        let painter = ui.painter().with_clip_rect(display_rect);
        let preview_tint = egui::Color32::from_white_alpha(170);
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(150));

        for tile_y in start_tile.y..end_tile.y {
            for tile_x in start_tile.x..end_tile.x {
                let Some(tile_screen_rect) = viewport_ctx
                    .tile_screen_rect(tilemap.tile_size, glam::UVec2::new(tile_x, tile_y))
                else {
                    continue;
                };
                painter.image(texture, tile_screen_rect, uv_rect, preview_tint);
                painter.rect_stroke(tile_screen_rect, 0.0, stroke, egui::StrokeKind::Inside);
            }
        }
    }

    pub(super) fn ensure_map_editor_brush_preview_texture(
        ui_state: &mut EditorUI,
        _ctx: &egui::Context,
        atlas: &AtlasMeta,
        texture_path: &std::path::Path,
        renderer: Option<&mut crate::rendering::WindowRenderer>,
    ) -> Option<egui::TextureId> {
        let renderer = renderer?;
        let presentation_settings = ui_state.project.indexed_presentation_settings();
        let resolved_palette_id = toki_core::indexed_presentation::resolve_indexed_palette(
            atlas.color_mode,
            &ui_state.project.available_palettes,
            &presentation_settings,
            None,
            atlas.palette.as_deref(),
        )
        .ok()
        .flatten()
        .map(|(palette_id, _)| palette_id);
        let cache_key = toki_core::indexed_presentation::texture_preview_cache_key(
            &texture_path.display().to_string(),
            atlas.color_mode,
            resolved_palette_id.as_deref(),
            &presentation_settings.resolve_post_process(&ui_state.project.available_palettes),
        );

        if crate::ui::editor_context::map_state(ui_state)
            .brush_preview_cache_key
            .as_deref()
            == Some(cache_key.as_str())
            && crate::ui::editor_context::map_state(ui_state)
                .brush_preview_texture
                .is_some()
        {
            return crate::ui::editor_context::map_state(ui_state).brush_preview_texture;
        }

        let texture = renderer
            .preview_texture_from_path(
                texture_path,
                atlas.color_mode,
                &ui_state.project.available_palettes,
                &presentation_settings,
                None,
                atlas.palette.as_deref(),
            )
            .ok()?;
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_cache_key =
            Some(cache_key);
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_image_path =
            Some(texture_path.to_path_buf());
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture =
            Some(texture.texture_id);
        Some(texture.texture_id)
    }

    pub(super) fn map_editor_tile_is_empty(
        tilemap: &toki_core::assets::tilemap::TileMap,
        tile_x: u32,
        tile_y: u32,
    ) -> bool {
        if tile_x >= tilemap.size.x || tile_y >= tilemap.size.y {
            return true;
        }
        let index = (tile_y * tilemap.size.x + tile_x) as usize;
        !tilemap
            .layers
            .iter()
            .filter(|layer| layer.visible)
            .any(|layer| layer.tiles.get(index).is_some_and(|tile| !tile.is_empty()))
    }

    pub(super) fn map_editor_checkerboard_color(tile_x: u32, tile_y: u32) -> egui::Color32 {
        let light = egui::Color32::from_rgb(98, 98, 98);
        let dark = egui::Color32::from_rgb(82, 82, 82);
        if (tile_x + tile_y).is_multiple_of(2) {
            light
        } else {
            dark
        }
    }

    fn ensure_map_editor_empty_checkerboard_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        viewport: &SceneViewport,
        tilemap: &toki_core::assets::tilemap::TileMap,
        visible_min: glam::IVec2,
        visible_max: glam::IVec2,
    ) -> Option<egui::TextureHandle> {
        let width = (visible_max.x - visible_min.x + 1).max(0) as usize;
        let height = (visible_max.y - visible_min.y + 1).max(0) as usize;
        if width == 0 || height == 0 {
            return None;
        }

        let (camera_position, camera_scale) = viewport.camera_state();
        let viewport_size = viewport.viewport_size();
        let cache_key = format!(
            "rev:{}:cam:{}:{}:scale:{}:vp:{}x{}:tiles:{}:{}-{}:{}",
            viewport.tilemap_revision(),
            camera_position.x,
            camera_position.y,
            camera_scale.to_bits(),
            viewport_size.0,
            viewport_size.1,
            visible_min.x,
            visible_min.y,
            visible_max.x,
            visible_max.y
        );
        let state = crate::ui::editor_context::map_state_mut(ui_state);
        if state.empty_checkerboard_cache_key.as_deref() == Some(cache_key.as_str()) {
            if let Some(texture) = state.empty_checkerboard_texture.clone() {
                return Some(texture);
            }
        }

        let mut pixels = vec![0_u8; width * height * 4];
        for (row, tile_y) in (visible_min.y as u32..=visible_max.y as u32).enumerate() {
            for (col, tile_x) in (visible_min.x as u32..=visible_max.x as u32).enumerate() {
                if !Self::map_editor_tile_is_empty(tilemap, tile_x, tile_y) {
                    continue;
                }
                let color = Self::map_editor_checkerboard_color(tile_x, tile_y);
                let pixel_index = (row * width + col) * 4;
                pixels[pixel_index] = color.r();
                pixels[pixel_index + 1] = color.g();
                pixels[pixel_index + 2] = color.b();
                pixels[pixel_index + 3] = color.a();
            }
        }

        let image = egui::ColorImage::from_rgba_unmultiplied([width, height], &pixels);
        let texture = ctx.load_texture(
            "map_editor_empty_checkerboard",
            image,
            egui::TextureOptions::NEAREST,
        );
        let state = crate::ui::editor_context::map_state_mut(ui_state);
        state.empty_checkerboard_cache_key = Some(cache_key);
        state.empty_checkerboard_texture = Some(texture.clone());
        Some(texture)
    }
}
