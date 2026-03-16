use super::*;
use crate::ui::editor_ui::MapEditorTool;
use crate::ui::EditorUI;

impl PanelSystem {
    pub(super) fn paint_map_editor_brush_preview(
        ui: &egui::Ui,
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        rect: egui::Rect,
        project_path: &std::path::Path,
    ) {
        if ui_state.map_editor_tool != MapEditorTool::Brush {
            return;
        }
        let Some(selected_tile) = ui_state.map_editor_selected_tile.clone() else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if !rect.contains(pointer_pos) {
            return;
        }
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
            return;
        };
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
        let Some(center_tile) = MapPaintInteraction::tile_position_at_world(tilemap, world_pos)
        else {
            return;
        };
        let Some((start_tile, end_tile)) = MapPaintInteraction::brush_footprint_bounds(
            tilemap,
            center_tile,
            ui_state.map_editor_brush_size_tiles,
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
        let (viewport_width, viewport_height) = viewport.viewport_size();
        let display_rect = Self::compute_viewport_display_rect(
            rect,
            (viewport_width, viewport_height),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
        );
        let (camera_position, camera_scale) = viewport.camera_state();
        let painter = ui.painter().with_clip_rect(display_rect);
        let preview_tint = egui::Color32::from_white_alpha(170);
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(150));

        for tile_y in start_tile.y..end_tile.y {
            for tile_x in start_tile.x..end_tile.x {
                let Some(tile_screen_rect) = Self::map_editor_tile_screen_rect(
                    display_rect,
                    (viewport_width, viewport_height),
                    camera_position,
                    camera_scale,
                    tilemap.tile_size,
                    glam::UVec2::new(tile_x, tile_y),
                ) else {
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
        rect: egui::Rect,
        project_path: &std::path::Path,
    ) {
        if ui_state.map_editor_tool != MapEditorTool::PlaceObject {
            return;
        }
        let Some(object_sheet_name) = ui_state.map_editor_selected_object_sheet.clone() else {
            return;
        };
        let Some(object_name) = ui_state.map_editor_selected_object_name.clone() else {
            return;
        };
        let Some(pointer_pos) = ui.input(|input| input.pointer.hover_pos()) else {
            return;
        };
        if !rect.contains(pointer_pos) {
            return;
        }
        let Some(tilemap) = viewport.scene_manager().tilemap() else {
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
        let world_pos = viewport.screen_to_world_pos_raw(pointer_pos, rect);
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
        let (viewport_width, viewport_height) = viewport.viewport_size();
        let display_rect = Self::compute_viewport_display_rect(
            rect,
            (viewport_width, viewport_height),
            viewport.sizing_mode() == crate::scene::viewport::ViewportSizingMode::Responsive,
        );
        let (camera_position, camera_scale) = viewport.camera_state();
        let Some(object_screen_rect) = Self::world_rect_to_screen_rect(
            display_rect,
            camera_position,
            camera_scale,
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
        if ui_state.map_editor_brush_preview_image_path.as_deref() == Some(texture_path)
            && ui_state.map_editor_brush_preview_texture.is_some()
        {
            return ui_state.map_editor_brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        ui_state.map_editor_brush_preview_image_path = Some(texture_path.to_path_buf());
        ui_state.map_editor_brush_preview_texture = Some(texture.clone());
        Some(texture)
    }

    pub(super) fn world_rect_to_screen_rect(
        display_rect: egui::Rect,
        camera_position: glam::IVec2,
        camera_scale: f32,
        world_top_left: glam::UVec2,
        world_size: glam::UVec2,
    ) -> Option<egui::Rect> {
        if camera_scale <= 0.0 {
            return None;
        }

        let screen_min_x = display_rect.min.x
            + (world_top_left.x as f32 - camera_position.x as f32) / camera_scale;
        let screen_min_y = display_rect.min.y
            + (world_top_left.y as f32 - camera_position.y as f32) / camera_scale;
        let screen_size = egui::vec2(
            world_size.x as f32 / camera_scale,
            world_size.y as f32 / camera_scale,
        );
        Some(egui::Rect::from_min_size(
            egui::pos2(screen_min_x, screen_min_y),
            screen_size,
        ))
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
        if ui_state.map_editor_brush_preview_image_path.as_deref() == Some(texture_path)
            && ui_state.map_editor_brush_preview_texture.is_some()
        {
            return ui_state.map_editor_brush_preview_texture.clone();
        }

        let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.data,
        );
        let key = format!("map_editor_brush_preview:{}", texture_path.display());
        let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
        ui_state.map_editor_brush_preview_image_path = Some(texture_path.to_path_buf());
        ui_state.map_editor_brush_preview_texture = Some(texture.clone());
        Some(texture)
    }

    pub(super) fn map_editor_tile_screen_rect(
        display_rect: egui::Rect,
        viewport_size: (u32, u32),
        camera_position: glam::IVec2,
        camera_scale: f32,
        tile_size: glam::UVec2,
        tile_pos: glam::UVec2,
    ) -> Option<egui::Rect> {
        let world_span_x = viewport_size.0 as f32 * camera_scale;
        let world_span_y = viewport_size.1 as f32 * camera_scale;
        if world_span_x <= 0.0 || world_span_y <= 0.0 {
            return None;
        }

        let world_min_x = camera_position.x as f32;
        let world_min_y = camera_position.y as f32;
        let world_left = tile_pos.x as f32 * tile_size.x as f32;
        let world_top = tile_pos.y as f32 * tile_size.y as f32;
        let world_right = world_left + tile_size.x as f32;
        let world_bottom = world_top + tile_size.y as f32;

        let left_t = (world_left - world_min_x) / world_span_x;
        let top_t = (world_top - world_min_y) / world_span_y;
        let right_t = (world_right - world_min_x) / world_span_x;
        let bottom_t = (world_bottom - world_min_y) / world_span_y;

        Some(egui::Rect::from_min_max(
            egui::pos2(
                egui::lerp(display_rect.left()..=display_rect.right(), left_t),
                egui::lerp(display_rect.top()..=display_rect.bottom(), top_t),
            ),
            egui::pos2(
                egui::lerp(display_rect.left()..=display_rect.right(), right_t),
                egui::lerp(display_rect.top()..=display_rect.bottom(), bottom_t),
            ),
        ))
    }
}
