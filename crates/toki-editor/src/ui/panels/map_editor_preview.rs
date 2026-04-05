use super::*;
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
        let Some((brush_entries, atlas, texture_path)) =
            Self::load_map_editor_preview_assets(ui_state, project_path, tilemap).ok()
        else {
            return;
        };
        let Some(texture) =
            Self::ensure_map_editor_brush_preview_texture(ui_state, ui.ctx(), &texture_path)
        else {
            return;
        };
        let Some(brush_entry) = crate::ui::editor_ui::selected_map_editor_brush_entry(
            &brush_entries,
            Some(selected_tile.as_str()),
        ) else {
            return;
        };
        let Some(texture_size) = atlas.image_size() else {
            return;
        };
        let Some(preview_tile_id) = brush_entry.preview_tile_id.as_deref() else {
            return;
        };
        let Some(tile_rect_px) = atlas.get_tile_rect(preview_tile_id) else {
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

    pub(super) fn load_map_editor_atlas(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<AtlasMeta> {
        let atlas_path = crate::ui::editor_ui::resolve_map_editor_atlas_path(project_path, tilemap);
        AtlasMeta::load_from_file(&atlas_path)
            .map_err(|e| anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e))
    }

    pub(super) fn load_map_editor_preview_assets(
        ui_state: &EditorUI,
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<(
        Vec<crate::ui::editor_ui::MapEditorBrushEntry>,
        AtlasMeta,
        std::path::PathBuf,
    )> {
        let atlas_path = crate::ui::editor_ui::resolve_map_editor_atlas_path(project_path, tilemap);
        let atlas = if crate::ui::editor_context::map_state(ui_state)
            .atlas_path
            .as_deref()
            == Some(atlas_path.as_path())
        {
            if let Some(cached) = &crate::ui::editor_context::map_state(ui_state).modified_atlas {
                cached.clone()
            } else {
                AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
                    anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
                })?
            }
        } else {
            AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
                anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
            })?
        };
        let texture_path = atlas_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Atlas path '{}' has no parent", atlas_path.display()))?
            .join(&atlas.image);
        let brush_entries = crate::ui::editor_ui::build_map_editor_brush_entries(&atlas);
        Ok((brush_entries, atlas, texture_path))
    }

    pub(super) fn ensure_map_editor_brush_preview_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if crate::ui::editor_context::map_state_mut(ui_state)
            .brush_preview_image_path
            .as_deref()
            == Some(texture_path)
            && crate::ui::editor_context::map_state_mut(ui_state)
                .brush_preview_texture
                .is_some()
        {
            return crate::ui::editor_context::map_state_mut(ui_state)
                .brush_preview_texture
                .clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_image_path =
            Some(texture_path.to_path_buf());
        crate::ui::editor_context::map_state_mut(ui_state).brush_preview_texture =
            Some(texture.clone());
        Some(texture)
    }
}
