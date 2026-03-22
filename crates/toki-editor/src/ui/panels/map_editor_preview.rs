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
        if ui_state.map.tool != MapEditorTool::Brush {
            return;
        }
        let Some(selected_tile) = ui_state.map.selected_tile.clone() else {
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
            ui_state.map.brush_size_tiles,
        ) else {
            return;
        };
        let Some((atlas, texture_path)) =
            Self::load_map_editor_preview_assets(project_path, tilemap).ok()
        else {
            return;
        };
        let Some(texture) =
            Self::ensure_map_editor_brush_preview_texture(ui_state, ui.ctx(), &texture_path)
        else {
            return;
        };
        let Some(texture_size) = atlas.image_size() else {
            return;
        };
        let Some(tile_rect_px) = atlas.get_tile_rect(&selected_tile) else {
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

    pub(super) fn paint_map_editor_object_preview(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        viewport_ctx: &EditorViewportContext,
        project_path: &std::path::Path,
    ) {
        if ui_state.map.tool != MapEditorTool::PlaceObject {
            return;
        }
        let Some(object_sheet_name) = ui_state.map.selected_object_sheet.clone() else {
            return;
        };
        let Some(object_name) = ui_state.map.selected_object_name.clone() else {
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
        let Some((object_sheet, texture_path)) =
            Self::load_map_editor_object_preview_assets(project_path, &object_sheet_name).ok()
        else {
            return;
        };
        let Some(object_info) = object_sheet.objects.get(&object_name) else {
            return;
        };
        let world_pos = viewport_ctx.screen_to_world(pointer_pos);
        let Some(world_anchor) = MapObjectInteraction::object_anchor_at_world(tilemap, world_pos)
        else {
            return;
        };
        let Some(texture) =
            Self::ensure_map_editor_preview_texture(ui_state, ui.ctx(), &texture_path)
        else {
            return;
        };
        let Some(texture_size) = object_sheet.image_size() else {
            return;
        };
        let Some(rect_px) = object_sheet.get_object_rect(&object_name) else {
            return;
        };
        let uv_rect = egui::Rect::from_min_max(
            egui::pos2(
                rect_px[0] as f32 / texture_size.x as f32,
                rect_px[1] as f32 / texture_size.y as f32,
            ),
            egui::pos2(
                (rect_px[0] + rect_px[2]) as f32 / texture_size.x as f32,
                (rect_px[1] + rect_px[3]) as f32 / texture_size.y as f32,
            ),
        );
        let display_rect = viewport_ctx.display_rect();
        let Some(object_screen_rect) = viewport_ctx.world_rect_to_screen_rect(
            world_anchor,
            glam::UVec2::new(
                object_info.size_tiles.x * object_sheet.tile_size.x,
                object_info.size_tiles.y * object_sheet.tile_size.y,
            ),
        ) else {
            return;
        };
        let painter = ui.painter().with_clip_rect(display_rect);
        painter.image(
            texture.id(),
            object_screen_rect,
            uv_rect,
            egui::Color32::from_white_alpha(180),
        );
        painter.rect_stroke(
            object_screen_rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_white_alpha(180)),
            egui::StrokeKind::Outside,
        );
    }

    pub(super) fn load_map_editor_object_preview_assets(
        project_path: &std::path::Path,
        object_sheet_name: &str,
    ) -> anyhow::Result<(ObjectSheetMeta, std::path::PathBuf)> {
        let sheet_file = if object_sheet_name.ends_with(".json") {
            object_sheet_name.to_string()
        } else {
            format!("{}.json", object_sheet_name)
        };
        let object_sheet_path = project_path.join("assets").join("sprites").join(sheet_file);
        let object_sheet = ObjectSheetMeta::load_from_file(&object_sheet_path)
            .map_err(|error| anyhow::anyhow!("failed to load object sheet: {}", error))?;
        let texture_path = object_sheet_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("object sheet path has no parent"))?
            .join(&object_sheet.image);
        Ok((object_sheet, texture_path))
    }

    pub(super) fn ensure_map_editor_preview_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if ui_state.map.brush_preview_image_path.as_deref() == Some(texture_path)
            && ui_state.map.brush_preview_texture.is_some()
        {
            return ui_state.map.brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        ui_state.map.brush_preview_image_path = Some(texture_path.to_path_buf());
        ui_state.map.brush_preview_texture = Some(texture.clone());
        Some(texture)
    }

    pub(super) fn load_map_editor_tile_names(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<Vec<String>> {
        let atlas = Self::load_map_editor_atlas(project_path, tilemap)?;
        let mut tile_names = atlas.tiles.keys().cloned().collect::<Vec<_>>();
        tile_names.sort();
        Ok(tile_names)
    }

    pub(super) fn load_map_editor_atlas(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<AtlasMeta> {
        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(&tilemap.atlas);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path
                    .join("assets")
                    .join("sprites")
                    .join(&tilemap.atlas)
            }
        };
        AtlasMeta::load_from_file(&atlas_path)
            .map_err(|e| anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e))
    }

    pub(super) fn load_map_editor_preview_assets(
        project_path: &std::path::Path,
        tilemap: &TileMap,
    ) -> anyhow::Result<(AtlasMeta, std::path::PathBuf)> {
        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(&tilemap.atlas);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path
                    .join("assets")
                    .join("sprites")
                    .join(&tilemap.atlas)
            }
        };
        let atlas = AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
            anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
        })?;
        let texture_path = atlas_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Atlas path '{}' has no parent", atlas_path.display()))?
            .join(&atlas.image);
        Ok((atlas, texture_path))
    }

    pub(super) fn ensure_map_editor_brush_preview_texture(
        ui_state: &mut EditorUI,
        ctx: &egui::Context,
        texture_path: &std::path::Path,
    ) -> Option<egui::TextureHandle> {
        if ui_state.map.brush_preview_image_path.as_deref() == Some(texture_path)
            && ui_state.map.brush_preview_texture.is_some()
        {
            return ui_state.map.brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        ui_state.map.brush_preview_image_path = Some(texture_path.to_path_buf());
        ui_state.map.brush_preview_texture = Some(texture.clone());
        Some(texture)
    }
}
