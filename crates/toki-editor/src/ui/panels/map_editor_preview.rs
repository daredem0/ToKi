use super::*;
use crate::editor_sprite_preview::{
    load_texture_preview_image, resolve_indexed_preview_palette, texture_preview_cache_key,
};
use crate::editor_viewport::EditorViewportContext;
use crate::ui::editor_ui::MapEditorTool;
use crate::ui::EditorUI;

impl PanelSystem {
    pub(super) fn paint_map_editor_brush_preview(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        viewport_ctx: &EditorViewportContext,
        project_path: &std::path::Path,
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
        let Some(brush_source) =
            crate::ui::editor_ui::load_map_editor_brush_source_for_tilemap(
                ui_state,
                project_path,
                tilemap,
            )
        else {
            return;
        };
        let Some(brush_entry) = crate::ui::editor_ui::selected_map_editor_brush_entry(
            &brush_source.brush_entries,
            Some(selected_tile.as_str()),
        ) else {
            return;
        };
        let Some(atlas_name) = crate::ui::editor_ui::map_editor_brush_entry_atlas_name(&brush_entry.id)
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
                painter.image(texture.id(), tile_screen_rect, uv_rect, preview_tint);
                painter.rect_stroke(tile_screen_rect, 0.0, stroke, egui::StrokeKind::Inside);
            }
        }
    }

    pub(super) fn ensure_map_editor_brush_preview_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        atlas: &AtlasMeta,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        let resolved_palette_id = resolve_indexed_preview_palette(
            atlas.color_mode,
            &ui_state.project.available_palettes,
            ui_state.project.indexed_palette_override.as_deref(),
            None,
            atlas.palette.as_deref(),
        )
        .ok()
        .flatten()
        .map(|(palette_id, _)| palette_id);
        let cache_key = texture_preview_cache_key(
            texture_path,
            atlas.color_mode,
            resolved_palette_id.as_deref(),
        );

        if crate::ui::editor_context::map_state(ui_state)
            .brush_preview_cache_key
            .as_deref()
            == Some(cache_key.as_str())
            && crate::ui::editor_context::map_state(ui_state)
                .brush_preview_texture
                .is_some()
        {
            return crate::ui::editor_context::map_state(ui_state)
                .brush_preview_texture
                .clone();
        }

        let (decoded, _) = load_texture_preview_image(
            texture_path,
            atlas.color_mode,
            &ui_state.project.available_palettes,
            ui_state.project.indexed_palette_override.as_deref(),
            None,
            atlas.palette.as_deref(),
        )
        .ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{cache_key}");
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_cache_key =
            Some(cache_key);
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_image_path =
            Some(texture_path.to_path_buf());
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture =
            Some(texture.clone());
        Some(texture)
    }
}
