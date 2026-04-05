//! Visual collision box editor rendered over a sprite preview.

use crate::editor_sprite_preview::{load_texture_preview_image, texture_preview_cache_key};
use crate::ui::editor_ui::EntityEditState;
use crate::ui::EditorUI;
use std::path::{Path, PathBuf};
use toki_core::assets::atlas::ColorMode;

/// Cached texture state for the entity editor sprite preview.
#[derive(Clone, Default)]
pub struct CollisionPreviewState {
    pub texture: Option<egui::TextureHandle>,
    pub cache_key: Option<String>,
    pub image_size: Option<(u32, u32)>,
    pub cell_rect: Option<[u32; 4]>,
}

/// Render the collision box preview inside the collision settings section.
pub fn render_collision_preview(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    project_path: &Path,
    ctx: &egui::Context,
) {
    let Some(edit) = crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .edit_state
        .as_ref()
    else {
        return;
    };

    let sprite_size = [
        edit.definition.rendering.size[0],
        edit.definition.rendering.size[1],
    ];
    let sprite_info = resolve_sprite_source(edit, project_path);
    let Some(sprite_info) = sprite_info else {
        return;
    };

    ensure_preview_texture(ui_state, ctx, &sprite_info);

    let preview = &ui_state.collision_preview;
    let Some(texture) = preview.texture.clone() else {
        return;
    };
    let Some(img_size) = preview.image_size else {
        return;
    };

    let cell_rect = preview.cell_rect.unwrap_or([0, 0, img_size.0, img_size.1]);

    let uv = calc_uv(cell_rect, img_size);

    let preview_width = ui.available_width().min(300.0);
    let aspect = sprite_size[1] as f32 / sprite_size[0] as f32;
    let preview_height = preview_width * aspect;
    let display_size = egui::vec2(preview_width, preview_height);

    let scale_x = preview_width / sprite_size[0] as f32;
    let scale_y = preview_height / sprite_size[1] as f32;

    ui.add_space(4.0);

    let (canvas_rect, response) =
        ui.allocate_exact_size(display_size, egui::Sense::click_and_drag());

    draw_checkered_background(ui.painter(), canvas_rect);
    ui.painter()
        .image(texture.id(), canvas_rect, uv, egui::Color32::WHITE);

    // Draw and handle the collision box
    let edit = crate::ui::editor_context::entity_editor_state_mut(ui_state)
        .edit_state
        .as_mut()
        .unwrap();

    draw_and_handle_collision_box(ui, edit, canvas_rect, scale_x, scale_y, &response);

    ui.add_space(4.0);
}

/// Describes the sprite image to load for preview.
struct SpriteSource {
    png_path: PathBuf,
    color_mode: ColorMode,
    palette_id: Option<String>,
    entity_palette_override: Option<String>,
    default_palette: Option<String>,
    cell_rect: Option<[u32; 4]>,
}

fn resolve_sprite_source(edit: &EntityEditState, project_path: &Path) -> Option<SpriteSource> {
    let sprites_dir = project_path.join("assets").join("sprites");
    resolve_from_static_object(edit, &sprites_dir)
        .or_else(|| resolve_from_atlas(edit, &sprites_dir))
}

fn resolve_from_static_object(edit: &EntityEditState, sprites_dir: &Path) -> Option<SpriteSource> {
    let static_obj = edit.definition.rendering.static_object.as_ref()?;
    let sheet_path = sprites_dir.join(format!("{}.json", &static_obj.sheet));
    let sheet =
        toki_core::assets::object_sheet::ObjectSheetMeta::load_from_file(&sheet_path).ok()?;
    let png_path = sprites_dir.join(&sheet.image);

    let obj = sheet.objects.get(&static_obj.object_name)?;
    let cell_rect = [
        obj.position.x * sheet.tile_size.x,
        obj.position.y * sheet.tile_size.y,
        obj.size_tiles.x * sheet.tile_size.x,
        obj.size_tiles.y * sheet.tile_size.y,
    ];

    Some(SpriteSource {
        png_path,
        color_mode: sheet.color_mode,
        palette_id: sheet.palette.clone(),
        entity_palette_override: edit
            .definition
            .rendering
            .palette_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        default_palette: sheet.palette,
        cell_rect: Some(cell_rect),
    })
}

fn resolve_from_atlas(edit: &EntityEditState, sprites_dir: &Path) -> Option<SpriteSource> {
    let atlas_name = &edit.definition.animations.atlas_name;
    if atlas_name.is_empty() {
        return None;
    }
    let atlas_filename = if atlas_name.ends_with(".json") {
        atlas_name.clone()
    } else {
        format!("{atlas_name}.json")
    };
    let atlas_path = sprites_dir.join(&atlas_filename);
    let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_path).ok()?;
    let png_path = sprites_dir.join(&atlas.image);

    let first_tile = edit
        .definition
        .animations
        .clips
        .first()
        .and_then(|c| c.frame_tiles.first());

    let cell_rect = first_tile.and_then(|tile_name| {
        let tile = atlas.tiles.get(tile_name)?;
        let ts = atlas.tile_size;
        Some([tile.position.x * ts.x, tile.position.y * ts.y, ts.x, ts.y])
    });

    Some(SpriteSource {
        png_path,
        color_mode: atlas.color_mode,
        palette_id: None,
        entity_palette_override: edit
            .definition
            .rendering
            .palette_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        default_palette: None,
        cell_rect,
    })
}

fn ensure_preview_texture(ui_state: &mut EditorUI, ctx: &egui::Context, source: &SpriteSource) {
    let cache_key = texture_preview_cache_key(
        &source.png_path,
        source.color_mode,
        source.palette_id.as_deref(),
    );

    if ui_state.collision_preview.texture.is_some()
        && ui_state.collision_preview.cache_key.as_deref() == Some(cache_key.as_str())
    {
        return;
    }

    let available_palettes = ui_state.project.available_palettes.clone();
    let indexed_override = ui_state.project.indexed_palette_override.clone();

    let Ok((decoded, _palette_id)) = load_texture_preview_image(
        &source.png_path,
        source.color_mode,
        &available_palettes,
        indexed_override.as_deref(),
        source.entity_palette_override.as_deref(),
        source.default_palette.as_deref(),
    ) else {
        return;
    };

    ui_state.collision_preview.image_size = Some((decoded.width, decoded.height));
    ui_state.collision_preview.cell_rect = source.cell_rect;
    ui_state.collision_preview.cache_key = Some(cache_key);

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [decoded.width as usize, decoded.height as usize],
        &decoded.data,
    );
    let texture = ctx.load_texture(
        "entity_collision_preview",
        color_image,
        egui::TextureOptions::NEAREST,
    );
    ui_state.collision_preview.texture = Some(texture);
}

fn calc_uv(cell_rect: [u32; 4], img_size: (u32, u32)) -> egui::Rect {
    let u_min = cell_rect[0] as f32 / img_size.0 as f32;
    let v_min = cell_rect[1] as f32 / img_size.1 as f32;
    let u_max = (cell_rect[0] + cell_rect[2]) as f32 / img_size.0 as f32;
    let v_max = (cell_rect[1] + cell_rect[3]) as f32 / img_size.1 as f32;
    egui::Rect::from_min_max(egui::pos2(u_min, v_min), egui::pos2(u_max, v_max))
}

fn draw_checkered_background(painter: &egui::Painter, rect: egui::Rect) {
    let check_size = 8.0;
    let light = egui::Color32::from_gray(200);
    let dark = egui::Color32::from_gray(150);

    let cols = ((rect.width() / check_size).ceil() as usize).max(1);
    let rows = ((rect.height() / check_size).ceil() as usize).max(1);

    for row in 0..rows {
        for col in 0..cols {
            let color = if (row + col) % 2 == 0 { light } else { dark };
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + col as f32 * check_size,
                    rect.top() + row as f32 * check_size,
                ),
                egui::vec2(check_size, check_size),
            )
            .intersect(rect);
            painter.rect_filled(cell_rect, 0.0, color);
        }
    }
}

const EDGE_GRAB_MARGIN: f32 = 6.0;

fn draw_and_handle_collision_box(
    ui: &egui::Ui,
    edit: &mut EntityEditState,
    canvas_rect: egui::Rect,
    scale_x: f32,
    scale_y: f32,
    response: &egui::Response,
) {
    let offset = edit.definition.collision.offset;
    let size = edit.definition.collision.size;

    let box_screen = egui::Rect::from_min_size(
        egui::pos2(
            canvas_rect.left() + offset[0] as f32 * scale_x,
            canvas_rect.top() + offset[1] as f32 * scale_y,
        ),
        egui::vec2(size[0] as f32 * scale_x, size[1] as f32 * scale_y),
    );

    // Semi-transparent fill
    ui.painter().rect_filled(
        box_screen,
        0.0,
        egui::Color32::from_rgba_unmultiplied(60, 180, 60, 50),
    );
    // Border
    ui.painter().rect_stroke(
        box_screen,
        0.0,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 220, 60)),
        egui::StrokeKind::Inside,
    );

    // Edge labels
    draw_edge_handles(ui.painter(), box_screen);

    if !response.dragged() {
        return;
    }

    let Some(pointer) = response.interact_pointer_pos() else {
        return;
    };

    let drag_start = pointer - response.drag_delta();
    let edge = detect_edge(drag_start, box_screen);

    // Convert pointer position to pixel coordinates and snap to grid
    let pixel_x = ((pointer.x - canvas_rect.left()) / scale_x).floor() as i32;
    let pixel_y = ((pointer.y - canvas_rect.top()) / scale_y).floor() as i32;

    apply_drag_snapped(edit, edge, pixel_x, pixel_y);
}

#[derive(Debug, Clone, Copy)]
enum Edge {
    Left,
    Right,
    Top,
    Bottom,
    Body,
}

fn detect_edge(pointer: egui::Pos2, rect: egui::Rect) -> Edge {
    let expanded = rect.expand(EDGE_GRAB_MARGIN);
    if !expanded.contains(pointer) {
        return Edge::Body;
    }

    let dl = (pointer.x - rect.left()).abs();
    let dr = (pointer.x - rect.right()).abs();
    let dt = (pointer.y - rect.top()).abs();
    let db = (pointer.y - rect.bottom()).abs();

    let min_dist = dl.min(dr).min(dt).min(db);

    if min_dist > EDGE_GRAB_MARGIN {
        return Edge::Body;
    }

    if dl == min_dist {
        Edge::Left
    } else if dr == min_dist {
        Edge::Right
    } else if dt == min_dist {
        Edge::Top
    } else {
        Edge::Bottom
    }
}

fn apply_drag_snapped(edit: &mut EntityEditState, edge: Edge, pixel_x: i32, pixel_y: i32) {
    let offset = &mut edit.definition.collision.offset;
    let size = &mut edit.definition.collision.size;

    let old_offset = *offset;
    let old_size = *size;

    match edge {
        Edge::Left => {
            let right = offset[0] + size[0] as i32;
            let new_left = pixel_x.min(right - 1);
            offset[0] = new_left;
            size[0] = (right - new_left).max(1) as u32;
        }
        Edge::Right => {
            let new_right = pixel_x.max(offset[0] + 1);
            size[0] = (new_right - offset[0]).max(1) as u32;
        }
        Edge::Top => {
            let bottom = offset[1] + size[1] as i32;
            let new_top = pixel_y.min(bottom - 1);
            offset[1] = new_top;
            size[1] = (bottom - new_top).max(1) as u32;
        }
        Edge::Bottom => {
            let new_bottom = pixel_y.max(offset[1] + 1);
            size[1] = (new_bottom - offset[1]).max(1) as u32;
        }
        Edge::Body => {
            let dx = pixel_x - (old_offset[0] + old_size[0] as i32 / 2);
            let dy = pixel_y - (old_offset[1] + old_size[1] as i32 / 2);
            offset[0] = old_offset[0] + dx;
            offset[1] = old_offset[1] + dy;
        }
    }

    if *offset != old_offset || *size != old_size {
        sync_grounding_footprint(edit);
        edit.mark_dirty();
    }
}

fn sync_grounding_footprint(edit: &mut EntityEditState) {
    edit.definition.rendering.grounding.footprint = Some(toki_core::entity::EntityFootprint::new(
        edit.definition.collision.offset,
        edit.definition.collision.size,
    ));
}

fn draw_edge_handles(painter: &egui::Painter, rect: egui::Rect) {
    let handle_color = egui::Color32::from_rgb(200, 255, 200);
    let handle_len = 8.0_f32;
    let mid_x = rect.center().x;
    let mid_y = rect.center().y;

    // Small tick marks at edge midpoints
    let stroke = egui::Stroke::new(2.0, handle_color);
    // Top
    painter.line_segment(
        [
            egui::pos2(mid_x - handle_len, rect.top()),
            egui::pos2(mid_x + handle_len, rect.top()),
        ],
        stroke,
    );
    // Bottom
    painter.line_segment(
        [
            egui::pos2(mid_x - handle_len, rect.bottom()),
            egui::pos2(mid_x + handle_len, rect.bottom()),
        ],
        stroke,
    );
    // Left
    painter.line_segment(
        [
            egui::pos2(rect.left(), mid_y - handle_len),
            egui::pos2(rect.left(), mid_y + handle_len),
        ],
        stroke,
    );
    // Right
    painter.line_segment(
        [
            egui::pos2(rect.right(), mid_y - handle_len),
            egui::pos2(rect.right(), mid_y + handle_len),
        ],
        stroke,
    );
}
