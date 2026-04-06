use super::*;
use crate::ui::sprite_editor::canonical_indexed_color;

// ============================================================================
// PixelColor Tests
// ============================================================================

#[test]
fn pixel_color_new_creates_rgba() {
    let color = PixelColor::new(10, 20, 30, 40);
    assert_eq!(color.r, 10);
    assert_eq!(color.g, 20);
    assert_eq!(color.b, 30);
    assert_eq!(color.a, 40);
}

#[test]
fn pixel_color_rgb_creates_opaque() {
    let color = PixelColor::rgb(100, 150, 200);
    assert_eq!(color.r, 100);
    assert_eq!(color.g, 150);
    assert_eq!(color.b, 200);
    assert_eq!(color.a, 255);
}

#[test]
fn pixel_color_transparent_is_all_zero() {
    let color = PixelColor::transparent();
    assert_eq!(color.r, 0);
    assert_eq!(color.g, 0);
    assert_eq!(color.b, 0);
    assert_eq!(color.a, 0);
}

#[test]
fn pixel_color_to_array_roundtrip() {
    let color = PixelColor::new(1, 2, 3, 4);
    let array = color.to_rgba_array();
    let restored = PixelColor::from_rgba_array(array);
    assert_eq!(color, restored);
}

#[test]
fn canonical_indexed_color_returns_expected_shades() {
    assert_eq!(
        canonical_indexed_color(0),
        PixelColor::rgb(0x00, 0x00, 0x00)
    );
    assert_eq!(
        canonical_indexed_color(1),
        PixelColor::rgb(0x55, 0x55, 0x55)
    );
    assert_eq!(
        canonical_indexed_color(2),
        PixelColor::rgb(0xAA, 0xAA, 0xAA)
    );
    assert_eq!(
        canonical_indexed_color(3),
        PixelColor::rgb(0xFF, 0xFF, 0xFF)
    );
}

#[test]
fn indexed_slot_for_authored_color_matches_palette_display_colors() {
    let palette = toki_core::palette::Palette::new(
        toki_core::palette::PaletteSize::Pal4,
        vec![
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [70, 80, 90, 255],
            [100, 110, 120, 255],
        ],
    )
    .unwrap();

    assert_eq!(
        indexed_slot_for_authored_color(PixelColor::rgb(70, 80, 90), Some(&palette)),
        Some(2)
    );
}

#[test]
fn nearest_palette_slot_prefers_closest_palette_color() {
    let palette = toki_core::palette::Palette::new(
        toki_core::palette::PaletteSize::Pal4,
        vec![
            [0, 0, 0, 255],
            [32, 32, 32, 255],
            [128, 128, 128, 255],
            [255, 255, 255, 255],
        ],
    )
    .unwrap();

    assert_eq!(
        nearest_palette_slot(PixelColor::rgb(120, 130, 125), &palette),
        2
    );
}

// ============================================================================
// SpriteCanvas Tests
// ============================================================================

#[test]
fn sprite_canvas_new_creates_transparent() {
    let canvas = SpriteCanvas::new(4, 4);
    assert_eq!(canvas.width, 4);
    assert_eq!(canvas.height, 4);
    assert_eq!(canvas.pixels().len(), 4 * 4 * 4);

    // All pixels should be transparent
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(canvas.get_pixel(x, y), Some(PixelColor::transparent()));
        }
    }
}

#[test]
fn sprite_canvas_filled_creates_solid_color() {
    let color = PixelColor::rgb(255, 0, 0);
    let canvas = SpriteCanvas::filled(2, 2, color);

    for y in 0..2 {
        for x in 0..2 {
            assert_eq!(canvas.get_pixel(x, y), Some(color));
        }
    }
}

#[test]
fn sprite_canvas_get_pixel_out_of_bounds_returns_none() {
    let canvas = SpriteCanvas::new(4, 4);
    assert_eq!(canvas.get_pixel(4, 0), None);
    assert_eq!(canvas.get_pixel(0, 4), None);
    assert_eq!(canvas.get_pixel(100, 100), None);
}

#[test]
fn sprite_canvas_set_pixel_works() {
    let mut canvas = SpriteCanvas::new(4, 4);
    let color = PixelColor::rgb(100, 150, 200);

    assert!(canvas.set_pixel(2, 3, color));
    assert_eq!(canvas.get_pixel(2, 3), Some(color));
    // Other pixels unchanged
    assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::transparent()));
}

#[test]
fn sprite_canvas_set_pixel_out_of_bounds_returns_false() {
    let mut canvas = SpriteCanvas::new(4, 4);
    let color = PixelColor::rgb(100, 150, 200);

    assert!(!canvas.set_pixel(4, 0, color));
    assert!(!canvas.set_pixel(0, 4, color));
}

#[test]
fn sprite_canvas_fill_rect_works() {
    let mut canvas = SpriteCanvas::new(8, 8);
    let color = PixelColor::rgb(50, 100, 150);

    canvas.fill_rect(2, 2, 3, 3, color);

    // Check filled region
    for y in 2..5 {
        for x in 2..5 {
            assert_eq!(canvas.get_pixel(x, y), Some(color), "Pixel at ({x}, {y})");
        }
    }
    // Check unfilled region
    assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(1, 2), Some(PixelColor::transparent()));
}

#[test]
fn sprite_canvas_fill_rect_clips_to_bounds() {
    let mut canvas = SpriteCanvas::new(4, 4);
    let color = PixelColor::rgb(255, 0, 0);

    // Fill rect that extends beyond bounds
    canvas.fill_rect(2, 2, 10, 10, color);

    // Should only fill visible portion
    assert_eq!(canvas.get_pixel(2, 2), Some(color));
    assert_eq!(canvas.get_pixel(3, 3), Some(color));
    assert_eq!(canvas.get_pixel(1, 1), Some(PixelColor::transparent()));
}

#[test]
fn sprite_canvas_clear_sets_all_transparent() {
    let mut canvas = SpriteCanvas::filled(4, 4, PixelColor::white());
    canvas.clear();

    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(canvas.get_pixel(x, y), Some(PixelColor::transparent()));
        }
    }
}

#[test]
fn sprite_canvas_clear_to_color_works() {
    let mut canvas = SpriteCanvas::new(4, 4);
    let color = PixelColor::rgb(128, 64, 32);
    canvas.clear_to_color(color);

    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(canvas.get_pixel(x, y), Some(color));
        }
    }
}

#[test]
fn sprite_canvas_from_rgba_validates_length() {
    // Correct length
    let pixels = vec![0u8; 4 * 4 * 4];
    assert!(SpriteCanvas::from_rgba(4, 4, pixels).is_some());

    // Wrong length
    let pixels = vec![0u8; 100];
    assert!(SpriteCanvas::from_rgba(4, 4, pixels).is_none());
}

// ============================================================================
// SpriteCanvasViewport Tests
// ============================================================================

#[test]
fn sprite_canvas_viewport_default_zoom() {
    let viewport = SpriteCanvasViewport::default();
    assert_eq!(viewport.zoom, 8.0);
    assert_eq!(viewport.pan, glam::Vec2::ZERO);
}

#[test]
fn sprite_canvas_viewport_zoom_in() {
    let mut viewport = SpriteCanvasViewport::default();
    let initial_zoom = viewport.zoom;
    viewport.zoom_in();
    assert!(viewport.zoom > initial_zoom);
    assert_eq!(viewport.zoom, initial_zoom * 1.2);
}

#[test]
fn sprite_canvas_viewport_zoom_out() {
    let mut viewport = SpriteCanvasViewport::default();
    let initial_zoom = viewport.zoom;
    viewport.zoom_out();
    assert!(viewport.zoom < initial_zoom);
    assert_eq!(viewport.zoom, initial_zoom / 1.2);
}

#[test]
fn sprite_canvas_viewport_zoom_clamped_to_max() {
    let mut viewport = SpriteCanvasViewport::default();
    for _ in 0..20 {
        viewport.zoom_in();
    }
    assert_eq!(viewport.zoom, viewport.zoom_max);
}

#[test]
fn sprite_canvas_viewport_zoom_clamped_to_min() {
    let mut viewport = SpriteCanvasViewport::default();
    for _ in 0..20 {
        viewport.zoom_out();
    }
    assert_eq!(viewport.zoom, viewport.zoom_min);
}

#[test]
fn sprite_canvas_viewport_pan_by_screen_delta() {
    let mut viewport = SpriteCanvasViewport {
        zoom: 2.0,
        pan: glam::Vec2::ZERO,
        ..Default::default()
    };

    // Pan by 10 screen pixels at zoom 2.0 should move 5 canvas pixels
    viewport.pan_by(glam::Vec2::new(10.0, 20.0));

    assert_eq!(viewport.pan, glam::Vec2::new(-5.0, -10.0));
}

#[test]
fn sprite_canvas_viewport_screen_to_canvas_conversion() {
    let viewport = SpriteCanvasViewport {
        zoom: 4.0,
        pan: glam::Vec2::new(10.0, 20.0),
        ..Default::default()
    };

    let rect = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 200.0));
    let screen_pos = glam::Vec2::new(140.0, 90.0); // 40 pixels into viewport

    let canvas_pos = viewport.screen_to_canvas(screen_pos, rect);
    // (40, 40) screen pixels / 4.0 zoom + (10, 20) pan = (20, 30) canvas
    assert_eq!(canvas_pos, glam::Vec2::new(20.0, 30.0));
}

#[test]
fn sprite_canvas_viewport_canvas_to_screen_conversion() {
    let viewport = SpriteCanvasViewport {
        zoom: 4.0,
        pan: glam::Vec2::new(10.0, 20.0),
        ..Default::default()
    };

    let rect = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 200.0));
    let canvas_pos = glam::Vec2::new(20.0, 30.0);

    let screen_pos = viewport.canvas_to_screen(canvas_pos, rect);
    // ((20, 30) - (10, 20)) * 4.0 + (100, 50) = (140, 90)
    assert_eq!(screen_pos, glam::Vec2::new(140.0, 90.0));
}

// ============================================================================
// SelectionMask Tests
// ============================================================================

#[test]
fn selection_mask_tracks_pixels_and_bounding_rect() {
    let mut selection = SelectionMask::new(8, 8);
    assert!(selection.is_empty());

    selection.select_pixel(5, 2);
    selection.select_pixel(3, 6);

    assert!(selection.is_selected(5, 2));
    assert!(selection.is_selected(3, 6));
    assert_eq!(
        selection.bounding_rect(),
        Some(SpriteSelection::new(3, 2, 3, 5))
    );
}

#[test]
fn selection_mask_select_and_deselect_rect_updates_contents() {
    let mut selection = SelectionMask::new(6, 6);
    selection.select_rect(1, 1, 3, 3);
    selection.deselect_rect(2, 2, 1, 1);

    assert!(selection.is_selected(1, 1));
    assert!(selection.is_selected(3, 3));
    assert!(!selection.is_selected(2, 2));
}

#[test]
fn selection_mask_translated_to_canvas_basic() {
    let mut local = SelectionMask::new(2, 2);
    local.select_rect(0, 0, 2, 2);

    let result = local.translated_to_canvas(8, 8, glam::IVec2::new(3, 3));

    assert_eq!(result.width, 8);
    assert_eq!(result.height, 8);
    assert!(result.is_selected(3, 3));
    assert!(result.is_selected(4, 4));
    assert!(result.is_selected(3, 4));
    assert!(result.is_selected(4, 3));
    assert!(!result.is_selected(2, 3));
    assert!(!result.is_selected(5, 5));
}

#[test]
fn selection_mask_translated_to_canvas_clips_negative() {
    let mut local = SelectionMask::new(2, 2);
    local.select_rect(0, 0, 2, 2);

    let result = local.translated_to_canvas(4, 4, glam::IVec2::new(-1, -1));

    // Only local (1,1) maps to canvas (0,0)
    assert!(result.is_selected(0, 0));
    assert!(!result.is_selected(1, 0));
    assert!(!result.is_selected(0, 1));
}

#[test]
fn selection_mask_translated_to_canvas_clips_overflow() {
    let mut local = SelectionMask::new(2, 2);
    local.select_rect(0, 0, 2, 2);

    let result = local.translated_to_canvas(8, 8, glam::IVec2::new(7, 7));

    // Only local (0,0) maps to canvas (7,7); (8,8) is out of bounds
    assert!(result.is_selected(7, 7));
    assert!(!result.is_selected(6, 7));
}

// ============================================================================
// Selection helper function tests
// ============================================================================

#[test]
fn extract_masked_selection_copies_only_selected_pixels() {
    use crate::ui::sprite_editor::extract_masked_selection;

    let mut canvas = SpriteCanvas::new(4, 4);
    let red = PixelColor::rgb(255, 0, 0);
    let blue = PixelColor::rgb(0, 0, 255);
    canvas.set_pixel(1, 1, red);
    canvas.set_pixel(2, 2, blue);
    canvas.set_pixel(3, 3, red); // not selected

    let mut mask = SelectionMask::new(4, 4);
    mask.select_pixel(1, 1);
    mask.select_pixel(2, 2);

    let result = extract_masked_selection(&canvas, &mask).unwrap();
    assert_eq!(result.width, 2);
    assert_eq!(result.height, 2);
    assert_eq!(result.get_pixel(0, 0), Some(red));
    assert_eq!(result.get_pixel(1, 1), Some(blue));
    // Unselected within bounding rect should be transparent
    assert_eq!(result.get_pixel(1, 0), Some(PixelColor::transparent()));
}

#[test]
fn clear_masked_pixels_clears_only_selected() {
    use crate::ui::sprite_editor::clear_masked_pixels;

    let red = PixelColor::rgb(255, 0, 0);
    let mut canvas = SpriteCanvas::filled(4, 4, red);

    let mut mask = SelectionMask::new(4, 4);
    mask.select_pixel(1, 1);
    mask.select_pixel(2, 2);

    clear_masked_pixels(&mut canvas, &mask);

    assert_eq!(canvas.get_pixel(1, 1), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(0, 0), Some(red));
    assert_eq!(canvas.get_pixel(3, 3), Some(red));
}

// ============================================================================
// SpriteEditorHistory Tests
// ============================================================================

#[test]
fn sprite_editor_history_push_and_undo() {
    let mut history = SpriteEditorHistory::new(10);

    let before = SpriteCanvas::filled(4, 4, PixelColor::white());
    let after = SpriteCanvas::filled(4, 4, PixelColor::black());

    history.push(SpriteEditCommand {
        before: before.clone(),
        after: after.clone(),
    });

    assert!(history.can_undo());
    assert!(!history.can_redo());

    let undone = history.take_undo();
    assert!(undone.is_some());
    let undone_canvas = undone.unwrap();
    assert_eq!(undone_canvas.get_pixel(0, 0), Some(PixelColor::white()));

    assert!(!history.can_undo());
    assert!(history.can_redo());
}

#[test]
fn sprite_editor_history_redo_works() {
    let mut history = SpriteEditorHistory::new(10);

    let before = SpriteCanvas::filled(4, 4, PixelColor::white());
    let after = SpriteCanvas::filled(4, 4, PixelColor::black());

    history.push(SpriteEditCommand {
        before: before.clone(),
        after: after.clone(),
    });

    history.take_undo();
    assert!(history.can_redo());

    let redone = history.take_redo();
    assert!(redone.is_some());
    let redone_canvas = redone.unwrap();
    assert_eq!(redone_canvas.get_pixel(0, 0), Some(PixelColor::black()));
}

#[test]
fn sprite_editor_history_push_clears_redo() {
    let mut history = SpriteEditorHistory::new(10);

    let canvas1 = SpriteCanvas::filled(4, 4, PixelColor::white());
    let canvas2 = SpriteCanvas::filled(4, 4, PixelColor::black());
    let canvas3 = SpriteCanvas::filled(4, 4, PixelColor::rgb(128, 128, 128));

    history.push(SpriteEditCommand {
        before: canvas1.clone(),
        after: canvas2.clone(),
    });

    history.take_undo();
    assert!(history.can_redo());

    // Push new command should clear redo stack
    history.push(SpriteEditCommand {
        before: canvas1.clone(),
        after: canvas3,
    });

    assert!(!history.can_redo());
}

#[test]
fn sprite_editor_history_respects_max_size() {
    let mut history = SpriteEditorHistory::new(3);

    for i in 0..5 {
        let before = SpriteCanvas::filled(1, 1, PixelColor::rgb(i, 0, 0));
        let after = SpriteCanvas::filled(1, 1, PixelColor::rgb(i + 1, 0, 0));
        history.push(SpriteEditCommand { before, after });
    }

    // Should only have 3 items
    let mut count = 0;
    while history.can_undo() {
        history.take_undo();
        count += 1;
    }
    assert_eq!(count, 3);
}

// ============================================================================
// SpriteEditorState Tests
// ============================================================================

#[test]
fn sprite_editor_state_default_values() {
    let state = SpriteEditorState::default();
    assert!(state.active().canvas.is_none());
    assert!(!state.active().dirty);
    assert_eq!(state.tool, SpriteEditorTool::Drag);
    assert_eq!(state.brush_size, 1);
    assert!(!state.pixel_perfect);
    assert!(state.active().show_grid);
    assert!(state.active().show_autotile_labels);
    assert!(state.active().show_autotile_guides);
}

#[test]
fn sprite_editor_state_new_canvas() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(16, 32);

    assert!(state.has_canvas());
    assert_eq!(state.canvas_dimensions(), Some((16, 32)));
    assert!(state.active().dirty);
}

#[test]
fn sprite_editor_state_new_canvas_filled() {
    let mut state = SpriteEditorState::default();
    let color = PixelColor::rgb(100, 150, 200);
    state.new_canvas_filled(8, 8, color);

    assert!(state.has_canvas());
    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(0, 0), Some(color));
}

#[test]
fn sprite_editor_state_close_canvas() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(16, 16);
    state.close_canvas();

    assert!(!state.has_canvas());
    assert!(!state.active().dirty);
}

#[test]
fn sprite_editor_state_undo_redo_integration() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    let before = state.active().canvas.clone().unwrap();
    state
        .active_mut()
        .canvas
        .as_mut()
        .unwrap()
        .set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    state.push_undo_state(before);

    // Check pixel was changed
    assert_eq!(
        state.active().canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(PixelColor::rgb(255, 0, 0))
    );

    // Undo
    assert!(state.undo());
    assert_eq!(
        state.active().canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(PixelColor::transparent())
    );

    // Redo
    assert!(state.redo());
    assert_eq!(
        state.active().canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(PixelColor::rgb(255, 0, 0))
    );
}

#[test]
fn sprite_editor_state_push_undo_state_ignores_no_op_edits() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    let before = state.active().canvas.clone().unwrap();
    state.push_undo_state(before);

    assert!(!state.active().history.can_undo());
    assert!(!state.undo());
}

#[test]
fn sprite_editor_state_convert_active_canvas_to_palette_maps_pixels_and_records_undo() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(2, 1);
    let palette = toki_core::palette::Palette::new(
        toki_core::palette::PaletteSize::Pal4,
        vec![
            [0, 0, 0, 255],
            [64, 64, 64, 255],
            [128, 128, 128, 255],
            [255, 255, 255, 255],
        ],
    )
    .unwrap();

    let canvas = state.active_mut().canvas.as_mut().unwrap();
    canvas.set_pixel(0, 0, PixelColor::rgb(70, 70, 70));
    canvas.set_pixel(1, 0, PixelColor::new(250, 250, 250, 128));

    assert!(state.convert_active_canvas_to_palette(&palette));
    assert_eq!(
        state.active().canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(canonical_indexed_color(1))
    );
    assert_eq!(
        state.active().canvas.as_ref().unwrap().get_pixel(1, 0),
        Some(PixelColor::new(0xFF, 0xFF, 0xFF, 128))
    );
    assert!(state.active().history.can_undo());

    assert!(state.undo());
    assert_eq!(
        state.active().canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(PixelColor::rgb(70, 70, 70))
    );
}

#[test]
fn sprite_editor_state_convert_active_canvas_to_palette_is_no_op_when_already_canonical() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(1, 1);
    let palette = toki_core::palette::Palette::new(
        toki_core::palette::PaletteSize::Pal4,
        vec![
            [10, 20, 30, 255],
            [40, 50, 60, 255],
            [70, 80, 90, 255],
            [100, 110, 120, 255],
        ],
    )
    .unwrap();

    state
        .active_mut()
        .canvas
        .as_mut()
        .unwrap()
        .set_pixel(0, 0, canonical_indexed_color(2));

    assert!(!state.convert_active_canvas_to_palette(&palette));
    assert!(!state.active().history.can_undo());
}

#[test]
fn sprite_editor_state_recent_colors() {
    let mut state = SpriteEditorState {
        max_recent_colors: 3,
        ..Default::default()
    };

    let color1 = PixelColor::rgb(1, 0, 0);
    let color2 = PixelColor::rgb(2, 0, 0);
    let color3 = PixelColor::rgb(3, 0, 0);
    let color4 = PixelColor::rgb(4, 0, 0);

    state.add_recent_color(color1);
    state.add_recent_color(color2);
    state.add_recent_color(color3);

    assert_eq!(state.recent_colors.len(), 3);
    assert_eq!(state.recent_colors[0], color3); // Most recent first

    // Adding 4th color should evict oldest
    state.add_recent_color(color4);
    assert_eq!(state.recent_colors.len(), 3);
    assert_eq!(state.recent_colors[0], color4);
    assert!(!state.recent_colors.contains(&color1));

    // Re-adding existing color moves it to front
    state.add_recent_color(color2);
    assert_eq!(state.recent_colors[0], color2);
    assert_eq!(state.recent_colors.len(), 3);
}

#[test]
fn begin_new_sprite_canvas_dialog_uses_active_asset_name_for_autotile_group_default() {
    let mut ui = EditorUI::new();
    crate::ui::editor_context::sprite_state_mut(&mut ui)
        .active_mut()
        .save_asset_name = "AutoTile_Grass".to_string();
    crate::ui::editor_context::sprite_state_mut(&mut ui).new_autotile_group_name =
        "terrain".to_string();

    crate::ui::editor_ui::begin_new_sprite_canvas_dialog(&mut ui);

    assert!(
        crate::ui::editor_context::sprite_state(&ui).show_new_canvas_dialog,
        "new canvas dialog should be shown"
    );
    assert_eq!(
        crate::ui::editor_context::sprite_state(&ui).new_autotile_group_name,
        "AutoTile_Grass"
    );
}

// ============================================================================
// Import/Export Tests
// ============================================================================

fn create_test_png(path: &std::path::Path, width: u32, height: u32, data: &[u8]) {
    toki_core::graphics::image::save_image_rgba8(path, width, height, data).unwrap();
}

#[test]
fn sprite_editor_state_import_external_image() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();

    // Create a simple 2x2 PNG image
    let png_path = temp.path().join("test.png");
    let pixel_data = vec![
        255, 0, 0, 255, // Red
        0, 255, 0, 255, // Green
        0, 0, 255, 255, // Blue
        255, 255, 0, 255, // Yellow
    ];
    create_test_png(&png_path, 2, 2, &pixel_data);

    let mut state = SpriteEditorState::default();
    let result = state.import_external_image(&png_path);

    assert!(result.is_ok(), "Import should succeed");
    assert!(state.has_canvas());
    assert!(state.active().dirty); // Should be marked dirty since it's newly imported

    let (w, h) = state.canvas_dimensions().unwrap();
    assert_eq!(w, 2);
    assert_eq!(h, 2);

    // Check that name is derived from filename
    assert_eq!(state.active().save_asset_name, "test");
}

#[test]
fn sprite_editor_state_import_external_image_as_sheet() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let png_path = temp.path().join("tiles.png");
    let pixel_data = vec![255; 32 * 16 * 4];
    create_test_png(&png_path, 32, 16, &pixel_data);

    let mut state = SpriteEditorState::default();
    let result = state.import_external_image_as_sheet(&png_path, 16, 16);

    assert!(result.is_ok(), "Import as sheet should succeed");
    assert!(state.has_canvas());
    assert!(state.is_sheet());
    assert_eq!(state.canvas_dimensions(), Some((32, 16)));
    assert_eq!(state.sheet_cell_count(), Some((2, 1)));
    assert_eq!(state.active().save_asset_name, "tiles");
    assert_eq!(state.active().save_asset_kind, SpriteAssetKind::TileAtlas);
}

#[test]
fn sprite_editor_state_import_external_image_as_sheet_rejects_non_divisible_dimensions() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let png_path = temp.path().join("tiles.png");
    let pixel_data = vec![255; 30 * 16 * 4];
    create_test_png(&png_path, 30, 16, &pixel_data);

    let mut state = SpriteEditorState::default();
    let result = state.import_external_image_as_sheet(&png_path, 16, 16);

    assert!(result.is_err());
    assert!(!state.has_canvas());
}

#[test]
fn sprite_editor_state_import_nonexistent_file_fails() {
    let mut state = SpriteEditorState::default();
    let result = state.import_external_image(std::path::Path::new("/nonexistent/path/file.png"));

    assert!(result.is_err());
    assert!(!state.has_canvas());
}

#[test]
fn sprite_editor_state_export_as_png() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let png_path = temp.path().join("export.png");

    // Create a canvas with some content
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    // Draw a red pixel
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    }

    let result = state.export_as_png(&png_path);
    assert!(result.is_ok(), "Export should succeed");
    assert!(png_path.exists(), "PNG file should exist");

    // Verify file size is reasonable (non-empty)
    let metadata = std::fs::metadata(&png_path).unwrap();
    assert!(metadata.len() > 0, "PNG file should not be empty");
}

#[test]
fn sprite_editor_state_export_without_canvas_fails() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let png_path = temp.path().join("export.png");

    let state = SpriteEditorState::default();
    let result = state.export_as_png(&png_path);

    assert!(result.is_err());
    assert!(!png_path.exists());
}

// ============================================================================
// Asset Discovery Tests
// ============================================================================

#[test]
fn sprite_editor_state_scan_sprite_assets_empty_dir() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();
    let assets = SpriteEditorState::scan_sprite_assets(temp.path());

    assert!(assets.is_empty());
}

#[test]
fn sprite_editor_state_scan_sprite_assets_finds_atlas() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();

    // Create a valid atlas JSON
    let json_content = r#"{
        "image": "test.png",
        "tile_size": [16, 16],
        "tiles": {
            "default": {
                "position": [0, 0],
                "properties": { "solid": false }
            }
        }
    }"#;
    std::fs::write(temp.path().join("test.json"), json_content).unwrap();

    // Create a matching PNG
    create_test_png(
        &temp.path().join("test.png"),
        16,
        16,
        &vec![0u8; 16 * 16 * 4],
    );

    let assets = SpriteEditorState::scan_sprite_assets(temp.path());

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].name, "test");
    assert_eq!(assets[0].kind, SpriteAssetKind::TileAtlas);
}

#[test]
fn sprite_editor_state_scan_sprite_assets_finds_object_sheet() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();

    // Create a valid object sheet JSON
    let json_content = r#"{
        "sheet_type": "objects",
        "image": "objects.png",
        "tile_size": [32, 32],
        "objects": {
            "object_0": {
                "position": [0, 0],
                "size_tiles": [1, 1]
            }
        }
    }"#;
    std::fs::write(temp.path().join("objects.json"), json_content).unwrap();

    // Create a matching PNG
    create_test_png(
        &temp.path().join("objects.png"),
        32,
        32,
        &vec![0u8; 32 * 32 * 4],
    );

    let assets = SpriteEditorState::scan_sprite_assets(temp.path());

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].name, "objects");
    assert_eq!(assets[0].kind, SpriteAssetKind::ObjectSheet);
}

#[test]
fn sprite_editor_state_scan_sprite_assets_ignores_json_without_png() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();

    // Create a valid atlas JSON but no PNG
    let json_content = r#"{
        "image": "missing.png",
        "tile_size": [16, 16],
        "tiles": {}
    }"#;
    std::fs::write(temp.path().join("missing.json"), json_content).unwrap();

    let assets = SpriteEditorState::scan_sprite_assets(temp.path());

    assert!(assets.is_empty());
}

#[test]
fn sprite_editor_state_load_sprite_asset() {
    use tempfile::tempdir;

    let temp = tempdir().unwrap();

    // Create a valid atlas with 2x2 grid
    let json_content = r#"{
        "image": "sprite.png",
        "tile_size": [8, 8],
        "tiles": {
            "tile_0": { "position": [0, 0], "properties": { "solid": false } },
            "tile_1": { "position": [1, 0], "properties": { "solid": false } },
            "tile_2": { "position": [0, 1], "properties": { "solid": false } },
            "tile_3": { "position": [1, 1], "properties": { "solid": false } }
        }
    }"#;
    std::fs::write(temp.path().join("sprite.json"), json_content).unwrap();

    // Create a 16x16 PNG (2x2 tiles of 8x8)
    create_test_png(
        &temp.path().join("sprite.png"),
        16,
        16,
        &vec![128u8; 16 * 16 * 4],
    );

    let mut state = SpriteEditorState::default();
    let assets = SpriteEditorState::scan_sprite_assets(temp.path());

    assert_eq!(assets.len(), 1);

    let result = state.load_sprite_asset(&assets[0]);
    assert!(result.is_ok());
    assert!(state.has_canvas());
    assert!(!state.active().dirty); // Should not be dirty - loaded from file
    assert!(state.active().show_cell_grid); // Should show grid for multi-tile sprite
    assert_eq!(state.active().cell_size.x, 8);
    assert_eq!(state.active().cell_size.y, 8);
    assert_eq!(state.active().save_asset_name, "sprite");
}

#[test]
fn sprite_editor_state_load_palette_indexed_asset_reads_mode_and_palette() {
    use tempfile::tempdir;
    use toki_core::assets::atlas::ColorMode;

    let temp = tempdir().unwrap();

    let json_content = r#"{
        "image": "sprite.png",
        "tile_size": [8, 8],
        "color_mode": "palette_indexed",
        "palette": "sepia",
        "tiles": {
            "tile_0": { "position": [0, 0], "properties": { "solid": false } }
        }
    }"#;
    std::fs::write(temp.path().join("sprite.json"), json_content).unwrap();
    create_test_png(
        &temp.path().join("sprite.png"),
        8,
        8,
        &vec![128u8; 8 * 8 * 4],
    );

    let mut state = SpriteEditorState::default();
    let assets = SpriteEditorState::scan_sprite_assets(temp.path());

    state.load_sprite_asset(&assets[0]).unwrap();

    assert_eq!(state.color_mode, ColorMode::PaletteIndexed);
    assert_eq!(state.authored_palette_id.as_deref(), Some("sepia"));
}

#[test]
fn sprite_editor_state_sync_palette_selection_keeps_valid_loaded_palette() {
    let mut state = SpriteEditorState {
        color_mode: toki_core::assets::atlas::ColorMode::PaletteIndexed,
        authored_palette_id: Some("sepia".to_string()),
        ..Default::default()
    };

    let builtins = toki_core::palette::builtin_palettes();
    let palettes = std::collections::BTreeMap::from([
        ("gb_default".to_string(), builtins["gb_default"].clone()),
        ("sepia".to_string(), builtins["sepia"].clone()),
    ]);

    state.sync_palette_selection(&palettes);

    assert_eq!(state.authored_palette_id.as_deref(), Some("sepia"));
}

#[test]
fn sprite_editor_state_sync_palette_selection_falls_back_to_first_available_palette() {
    let mut state = SpriteEditorState {
        color_mode: toki_core::assets::atlas::ColorMode::PaletteIndexed,
        authored_palette_id: Some("missing".to_string()),
        ..Default::default()
    };

    let builtins = toki_core::palette::builtin_palettes();
    let palettes = std::collections::BTreeMap::from([
        ("gb_default".to_string(), builtins["gb_default"].clone()),
        ("sepia".to_string(), builtins["sepia"].clone()),
    ]);

    state.sync_palette_selection(&palettes);

    assert_eq!(state.authored_palette_id.as_deref(), Some("gb_default"));
}

#[test]
fn sprite_editor_state_save_current_atlas_preserves_existing_tile_metadata() {
    use tempfile::tempdir;
    use toki_core::assets::atlas::AtlasMeta;

    let temp = tempdir().unwrap();

    let json_content = r#"{
        "image": "sprite.png",
        "tile_size": [8, 8],
        "tiles": {
            "idle_0": { "position": [0, 0], "properties": { "solid": true, "trigger": false } },
            "idle_1": { "position": [1, 0], "properties": { "solid": false, "trigger": true } }
        }
    }"#;
    std::fs::write(temp.path().join("sprite.json"), json_content).unwrap();
    create_test_png(
        &temp.path().join("sprite.png"),
        16,
        8,
        &vec![128u8; 16 * 8 * 4],
    );

    let mut state = SpriteEditorState::default();
    let assets = SpriteEditorState::scan_sprite_assets(temp.path());
    state.load_sprite_asset(&assets[0]).unwrap();
    assert!(state.append_column());
    state.color_mode = toki_core::assets::atlas::ColorMode::PaletteIndexed;
    state.authored_palette_id = Some("gb_default".to_string());

    state.save_current_asset().unwrap();

    let saved = AtlasMeta::load_from_file(temp.path().join("sprite.json")).unwrap();
    assert_eq!(
        saved.color_mode,
        toki_core::assets::atlas::ColorMode::PaletteIndexed
    );
    assert_eq!(saved.palette.as_deref(), Some("gb_default"));

    let idle_0 = saved.tiles.get("idle_0").unwrap();
    assert_eq!(idle_0.position, glam::UVec2::new(0, 0));
    assert!(idle_0.properties.solid);
    assert!(!idle_0.properties.trigger);

    let idle_1 = saved.tiles.get("idle_1").unwrap();
    assert_eq!(idle_1.position, glam::UVec2::new(1, 0));
    assert!(!idle_1.properties.solid);
    assert!(idle_1.properties.trigger);

    let added = saved.tiles.get("tile_2").unwrap();
    assert_eq!(added.position, glam::UVec2::new(2, 0));
    assert_eq!(
        added.properties,
        toki_core::assets::atlas::TileProperties::default()
    );
}

#[test]
fn sprite_editor_state_save_current_atlas_preserves_all_aliases_for_same_cell() {
    use tempfile::tempdir;
    use toki_core::assets::atlas::AtlasMeta;

    let temp = tempdir().unwrap();

    let json_content = r#"{
        "image": "sprite.png",
        "tile_size": [8, 8],
        "tiles": {
            "slime/idle_a": { "position": [0, 0], "properties": { "solid": false, "trigger": false } },
            "slime/walk_a": { "position": [0, 0], "properties": { "solid": false, "trigger": false } },
            "slime/idle_b": { "position": [1, 0], "properties": { "solid": false, "trigger": false } },
            "slime/walk_b": { "position": [1, 0], "properties": { "solid": false, "trigger": false } }
        }
    }"#;
    std::fs::write(temp.path().join("sprite.json"), json_content).unwrap();
    create_test_png(
        &temp.path().join("sprite.png"),
        16,
        8,
        &vec![128u8; 16 * 8 * 4],
    );

    let mut state = SpriteEditorState::default();
    let assets = SpriteEditorState::scan_sprite_assets(temp.path());
    state.load_sprite_asset(&assets[0]).unwrap();

    state.save_current_asset().unwrap();

    let saved = AtlasMeta::load_from_file(temp.path().join("sprite.json")).unwrap();
    assert_eq!(saved.tiles.len(), 4);
    assert_eq!(
        saved.tiles.get("slime/idle_a").unwrap().position,
        glam::UVec2::new(0, 0)
    );
    assert_eq!(
        saved.tiles.get("slime/walk_a").unwrap().position,
        glam::UVec2::new(0, 0)
    );
    assert_eq!(
        saved.tiles.get("slime/idle_b").unwrap().position,
        glam::UVec2::new(1, 0)
    );
    assert_eq!(
        saved.tiles.get("slime/walk_b").unwrap().position,
        glam::UVec2::new(1, 0)
    );
}

#[test]
fn sprite_editor_state_save_current_atlas_preserves_remaining_tile_names_after_collapse() {
    use tempfile::tempdir;
    use toki_core::assets::atlas::AtlasMeta;

    let temp = tempdir().unwrap();

    let json_content = r#"{
        "image": "sprite.png",
        "tile_size": [8, 8],
        "tiles": {
            "walk_0": { "position": [0, 0], "properties": { "solid": true, "trigger": false } },
            "walk_1": { "position": [1, 0], "properties": { "solid": false, "trigger": true } }
        }
    }"#;
    std::fs::write(temp.path().join("sprite.json"), json_content).unwrap();
    create_test_png(
        &temp.path().join("sprite.png"),
        16,
        8,
        &vec![128u8; 16 * 8 * 4],
    );

    let mut state = SpriteEditorState::default();
    let assets = SpriteEditorState::scan_sprite_assets(temp.path());
    state.load_sprite_asset(&assets[0]).unwrap();
    state.active_mut().selected_cell = Some(0);
    assert!(state.delete_cell_with_collapse());

    state.save_current_asset().unwrap();

    let saved = AtlasMeta::load_from_file(temp.path().join("sprite.json")).unwrap();
    assert!(!saved.tiles.contains_key("walk_0"));
    let moved = saved.tiles.get("walk_1").unwrap();
    assert_eq!(moved.position, glam::UVec2::new(0, 0));
    assert!(!moved.properties.solid);
    assert!(moved.properties.trigger);
}

#[test]
fn sprite_editor_state_save_current_object_sheet_preserves_existing_object_names() {
    use tempfile::tempdir;
    use toki_core::assets::object_sheet::ObjectSheetMeta;

    let temp = tempdir().unwrap();

    let json_content = r#"{
        "sheet_type": "objects",
        "image": "objects.png",
        "tile_size": [8, 8],
        "objects": {
            "torch": { "position": [0, 0], "size_tiles": [1, 1] },
            "barrel": { "position": [1, 0], "size_tiles": [1, 1] }
        }
    }"#;
    std::fs::write(temp.path().join("objects.json"), json_content).unwrap();
    create_test_png(
        &temp.path().join("objects.png"),
        16,
        8,
        &vec![64u8; 16 * 8 * 4],
    );

    let mut state = SpriteEditorState::default();
    let assets = SpriteEditorState::scan_sprite_assets(temp.path());
    state.load_sprite_asset(&assets[0]).unwrap();
    assert!(state.append_column());

    state.save_current_asset().unwrap();

    let saved = ObjectSheetMeta::load_from_file(temp.path().join("objects.json")).unwrap();
    assert_eq!(
        saved.objects.get("torch").unwrap().position,
        glam::UVec2::new(0, 0)
    );
    assert_eq!(
        saved.objects.get("barrel").unwrap().position,
        glam::UVec2::new(1, 0)
    );
    assert_eq!(
        saved.objects.get("object_2").unwrap().position,
        glam::UVec2::new(2, 0)
    );
}

// ============================================================================
// Sheet Append/Delete Tests
// ============================================================================

#[test]
fn sprite_editor_state_append_row_expands_canvas() {
    let mut state = SpriteEditorState::default();
    // Create 2x2 sheet with 8x8 cells (16x16 canvas)
    state.new_sheet(16, 16, 8, 8);

    assert_eq!(state.canvas_dimensions(), Some((16, 16)));
    assert_eq!(state.sheet_cell_count(), Some((2, 2)));

    // Append a row - should expand to 16x24 (2x3 grid)
    assert!(state.append_row());

    assert_eq!(state.canvas_dimensions(), Some((16, 24)));
    assert_eq!(state.sheet_cell_count(), Some((2, 3)));
    assert!(state.active().dirty);
    assert!(state.active().history.can_undo());
}

#[test]
fn sprite_editor_state_append_column_expands_canvas() {
    let mut state = SpriteEditorState::default();
    // Create 2x2 sheet with 8x8 cells (16x16 canvas)
    state.new_sheet(16, 16, 8, 8);

    assert_eq!(state.canvas_dimensions(), Some((16, 16)));
    assert_eq!(state.sheet_cell_count(), Some((2, 2)));

    // Append a column - should expand to 24x16 (3x2 grid)
    assert!(state.append_column());

    assert_eq!(state.canvas_dimensions(), Some((24, 16)));
    assert_eq!(state.sheet_cell_count(), Some((3, 2)));
    assert!(state.active().dirty);
    assert!(state.active().history.can_undo());
}

#[test]
fn sprite_editor_state_append_row_preserves_existing_pixels() {
    let mut state = SpriteEditorState::default();
    state.new_sheet(8, 8, 8, 8); // 1x1 cell

    // Draw a red pixel in the original cell
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    }

    state.append_row();

    // Check the red pixel is still there
    if let Some(canvas) = &state.active().canvas {
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(255, 0, 0)));
        // New row should be transparent
        assert_eq!(canvas.get_pixel(0, 8), Some(PixelColor::transparent()));
    }
}

#[test]
fn sprite_editor_state_delete_cell_with_collapse_shifts_cells() {
    let mut state = SpriteEditorState::default();
    // Create 2x2 sheet with 4x4 cells
    state.new_sheet(8, 8, 4, 4);

    // Draw distinct colors in each cell
    if let Some(canvas) = &mut state.active_mut().canvas {
        // Cell 0 (top-left): Red
        canvas.fill_rect(0, 0, 4, 4, PixelColor::rgb(255, 0, 0));
        // Cell 1 (top-right): Green
        canvas.fill_rect(4, 0, 4, 4, PixelColor::rgb(0, 255, 0));
        // Cell 2 (bottom-left): Blue
        canvas.fill_rect(0, 4, 4, 4, PixelColor::rgb(0, 0, 255));
        // Cell 3 (bottom-right): Yellow
        canvas.fill_rect(4, 4, 4, 4, PixelColor::rgb(255, 255, 0));
    }

    // Select and delete cell 0 (red)
    state.active_mut().selected_cell = Some(0);
    assert!(state.delete_cell_with_collapse());

    // After collapse: cell 0 should now have green (was cell 1)
    if let Some(canvas) = &state.active().canvas {
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(0, 255, 0)));
        // Cell 1 should have blue (was cell 2)
        assert_eq!(canvas.get_pixel(4, 0), Some(PixelColor::rgb(0, 0, 255)));
        // Cell 2 should have yellow (was cell 3)
        assert_eq!(canvas.get_pixel(0, 4), Some(PixelColor::rgb(255, 255, 0)));
        // Cell 3 (last) should be transparent
        assert_eq!(canvas.get_pixel(4, 4), Some(PixelColor::transparent()));
    }

    assert!(state.active().dirty);
    assert!(state.active().history.can_undo());
}

#[test]
fn sprite_editor_state_delete_cell_without_selection_fails() {
    let mut state = SpriteEditorState::default();
    state.new_sheet(8, 8, 4, 4);
    state.active_mut().selected_cell = None;

    assert!(!state.delete_cell_with_collapse());
}

#[test]
fn sprite_editor_state_append_on_non_sheet_fails() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(16, 16); // Not a sheet

    assert!(!state.append_row());
    assert!(!state.append_column());
}

// ============================================================================
// Flip/Rotate/Resize Tests
// ============================================================================

#[test]
fn sprite_editor_state_flip_horizontal() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 2);

    // Draw red on left, green on right
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
        canvas.set_pixel(3, 0, PixelColor::rgb(0, 255, 0));
    }

    assert!(state.flip_horizontal());

    // After flip: red should be on right, green on left
    if let Some(canvas) = &state.active().canvas {
        assert_eq!(canvas.get_pixel(3, 0), Some(PixelColor::rgb(255, 0, 0)));
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(0, 255, 0)));
    }
    assert!(state.active().dirty);
    assert!(state.active().history.can_undo());
}

#[test]
fn sprite_editor_state_flip_horizontal_only_affects_selected_region() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 2);

    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
        canvas.set_pixel(1, 0, PixelColor::rgb(0, 255, 0));
        canvas.set_pixel(3, 0, PixelColor::rgb(0, 0, 255));
    }

    let mut selection = SelectionMask::new(4, 2);
    selection.select_rect(0, 0, 2, 1);
    state.active_mut().selection = Some(selection);

    assert!(state.flip_horizontal());

    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(0, 255, 0)));
    assert_eq!(canvas.get_pixel(1, 0), Some(PixelColor::rgb(255, 0, 0)));
    assert_eq!(canvas.get_pixel(3, 0), Some(PixelColor::rgb(0, 0, 255)));
}

#[test]
fn sprite_editor_state_flip_vertical() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(2, 4);

    // Draw red on top, green on bottom
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
        canvas.set_pixel(0, 3, PixelColor::rgb(0, 255, 0));
    }

    assert!(state.flip_vertical());

    // After flip: red should be on bottom, green on top
    if let Some(canvas) = &state.active().canvas {
        assert_eq!(canvas.get_pixel(0, 3), Some(PixelColor::rgb(255, 0, 0)));
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(0, 255, 0)));
    }
    assert!(state.active().dirty);
}

#[test]
fn sprite_editor_state_rotate_clockwise() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 2); // 4 wide, 2 tall

    // Draw red at top-left
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    }

    assert!(state.rotate_clockwise());

    // After 90° CW: canvas should be 2 wide, 4 tall
    // top-left (0,0) -> top-right (1, 0) in new coords
    assert_eq!(state.canvas_dimensions(), Some((2, 4)));
    if let Some(canvas) = &state.active().canvas {
        // Original (0,0) should now be at (1, 0)
        assert_eq!(canvas.get_pixel(1, 0), Some(PixelColor::rgb(255, 0, 0)));
    }
    assert!(state.active().dirty);
}

#[test]
fn sprite_editor_state_rotate_clockwise_only_affects_square_selection() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(1, 1, PixelColor::rgb(255, 0, 0));
        canvas.set_pixel(2, 1, PixelColor::rgb(0, 255, 0));
        canvas.set_pixel(3, 3, PixelColor::rgb(0, 0, 255));
    }

    let mut selection = SelectionMask::new(4, 4);
    selection.select_rect(1, 1, 2, 2);
    state.active_mut().selection = Some(selection);

    assert!(state.rotate_clockwise());

    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(2, 1), Some(PixelColor::rgb(255, 0, 0)));
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::rgb(0, 255, 0)));
    assert_eq!(canvas.get_pixel(3, 3), Some(PixelColor::rgb(0, 0, 255)));
    assert_eq!(state.canvas_dimensions(), Some((4, 4)));
}

#[test]
fn sprite_editor_state_rotate_clockwise_rejects_non_square_selection() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    let before = state.active().canvas.clone().unwrap();
    let mut selection = SelectionMask::new(4, 4);
    selection.select_rect(0, 0, 2, 3);
    state.active_mut().selection = Some(selection);

    assert!(!state.rotate_clockwise());
    assert_eq!(state.active().canvas.as_ref().unwrap(), &before);
}

#[test]
fn sprite_editor_state_rotate_counter_clockwise() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 2); // 4 wide, 2 tall

    // Draw red at top-left
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    }

    assert!(state.rotate_counter_clockwise());

    // After 90° CCW: canvas should be 2 wide, 4 tall
    assert_eq!(state.canvas_dimensions(), Some((2, 4)));
    if let Some(canvas) = &state.active().canvas {
        // Original (0,0) should now be at (0, 3)
        assert_eq!(canvas.get_pixel(0, 3), Some(PixelColor::rgb(255, 0, 0)));
    }
    assert!(state.active().dirty);
}

#[test]
fn sprite_editor_state_resize_canvas_expand_center() {
    use super::ResizeAnchor;

    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    // Draw red at center
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(1, 1, PixelColor::rgb(255, 0, 0));
    }

    // Expand to 8x8 with center anchor
    assert!(state.resize_canvas(8, 8, ResizeAnchor::MiddleCenter));

    assert_eq!(state.canvas_dimensions(), Some((8, 8)));
    if let Some(canvas) = &state.active().canvas {
        // Original (1,1) should now be at (3,3) - shifted by (2,2)
        assert_eq!(canvas.get_pixel(3, 3), Some(PixelColor::rgb(255, 0, 0)));
    }
    assert!(state.active().dirty);
}

#[test]
fn sprite_editor_state_resize_canvas_shrink_top_left() {
    use super::ResizeAnchor;

    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // Draw red at top-left
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    }

    // Shrink to 4x4 with top-left anchor
    assert!(state.resize_canvas(4, 4, ResizeAnchor::TopLeft));

    assert_eq!(state.canvas_dimensions(), Some((4, 4)));
    if let Some(canvas) = &state.active().canvas {
        // Red pixel should still be at (0,0)
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(255, 0, 0)));
    }
}

#[test]
fn sprite_editor_state_resize_zero_size_fails() {
    use super::ResizeAnchor;

    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    assert!(!state.resize_canvas(0, 4, ResizeAnchor::MiddleCenter));
    assert!(!state.resize_canvas(4, 0, ResizeAnchor::MiddleCenter));
}

// ============================================================================
// Copy/Paste Tests
// ============================================================================

#[test]
fn sprite_editor_copy_selection_copies_to_clipboard() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // Draw a red square at (2,2) to (4,4)
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(2, 2, 2, 2, PixelColor::rgb(255, 0, 0));
    }

    // Create a selection covering the red square
    let mut selection = SelectionMask::new(8, 8);
    selection.select_rect(2, 2, 2, 2);
    state.active_mut().selection = Some(selection);

    // Copy should succeed
    assert!(state.copy_selection());
    assert!(state.clipboard.is_some());

    // Clipboard should have correct dimensions
    let clipboard = state.clipboard.as_ref().unwrap();
    assert_eq!(clipboard.width, 2);
    assert_eq!(clipboard.height, 2);

    // Clipboard should contain the red pixels
    assert_eq!(clipboard.get_pixel(0, 0), Some(PixelColor::rgb(255, 0, 0)));
}

#[test]
fn sprite_editor_copy_without_selection_fails() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // No selection
    state.active_mut().selection = None;

    assert!(!state.copy_selection());
    assert!(state.clipboard.is_none());
}

#[test]
fn sprite_editor_copy_selection_only_copies_masked_pixels() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(1, 1, 2, 2, PixelColor::rgb(255, 0, 0));
    }

    let mut selection = SelectionMask::new(4, 4);
    selection.select_pixel(1, 1);
    selection.select_pixel(2, 2);
    state.active_mut().selection = Some(selection);

    assert!(state.copy_selection());

    let clipboard = state.clipboard.as_ref().unwrap();
    assert_eq!(clipboard.width, 2);
    assert_eq!(clipboard.height, 2);
    assert_eq!(clipboard.get_pixel(0, 0), Some(PixelColor::rgb(255, 0, 0)));
    assert_eq!(clipboard.get_pixel(1, 0), Some(PixelColor::transparent()));
    assert_eq!(clipboard.get_pixel(0, 1), Some(PixelColor::transparent()));
    assert_eq!(clipboard.get_pixel(1, 1), Some(PixelColor::rgb(255, 0, 0)));
}

#[test]
fn sprite_editor_paste_at_cursor() {
    use super::CanvasSide;

    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // Draw red square and copy it
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(0, 0, 2, 2, PixelColor::rgb(255, 0, 0));
    }
    let mut selection = SelectionMask::new(8, 8);
    selection.select_rect(0, 0, 2, 2);
    state.active_mut().selection = Some(selection);
    assert!(state.copy_selection());

    // Set cursor position for paste
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));

    // Paste creates a floating selection
    assert!(state.paste_at_cursor(CanvasSide::Left));
    // Commit the float to stamp pixels
    assert!(state.commit_floating());

    // Check pixels were pasted at (4,4)
    if let Some(canvas) = &state.active().canvas {
        assert_eq!(canvas.get_pixel(4, 4), Some(PixelColor::rgb(255, 0, 0)));
        assert_eq!(canvas.get_pixel(5, 5), Some(PixelColor::rgb(255, 0, 0)));
    }

    // Should be marked dirty and have undo history
    assert!(state.active().dirty);
    assert!(state.active().history.can_undo());
}

#[test]
fn sprite_editor_paste_without_cursor_fails() {
    use super::CanvasSide;

    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // Copy something
    let mut selection = SelectionMask::new(8, 8);
    selection.select_rect(0, 0, 2, 2);
    state.active_mut().selection = Some(selection);
    state.copy_selection();

    // No cursor position
    state.active_mut().cursor_canvas_pos = None;

    assert!(!state.paste_at_cursor(CanvasSide::Left));
}

#[test]
fn sprite_editor_paste_without_clipboard_fails() {
    use super::CanvasSide;

    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // Set cursor but no clipboard
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(0, 0));
    state.clipboard = None;

    assert!(!state.paste_at_cursor(CanvasSide::Left));
}

#[test]
fn sprite_editor_paste_centers_in_selected_cell() {
    use super::CanvasSide;

    let mut state = SpriteEditorState::default();
    // Create a 16x16 sheet with 8x8 cells (2x2 grid)
    state.new_sheet(16, 16, 8, 8);

    // Draw a 2x2 red square at (0,0) and copy it
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(0, 0, 2, 2, PixelColor::rgb(255, 0, 0));
    }
    let mut selection = SelectionMask::new(16, 16);
    selection.select_rect(0, 0, 2, 2);
    state.active_mut().selection = Some(selection);
    assert!(state.copy_selection());

    // Select cell 3 (bottom-right, at position 8,8)
    state.active_mut().selected_cell = Some(3);

    // Paste - should center the 2x2 clipboard in the 8x8 cell
    assert!(state.paste_at_cursor(CanvasSide::Left));
    assert!(state.commit_floating());

    // Check that pixels were pasted centered in cell 3
    if let Some(canvas) = &state.active().canvas {
        // The 2x2 paste should be at (11, 11) to (12, 12)
        assert_eq!(canvas.get_pixel(11, 11), Some(PixelColor::rgb(255, 0, 0)));
        assert_eq!(canvas.get_pixel(12, 12), Some(PixelColor::rgb(255, 0, 0)));
        // Pixels outside the paste area in cell 3 should be transparent
        assert_eq!(canvas.get_pixel(8, 8), Some(PixelColor::transparent()));
    }
}

#[test]
fn sprite_editor_paste_scales_to_fit_cell() {
    use super::CanvasSide;

    let mut state = SpriteEditorState::default();
    // Create a 16x16 sheet with 4x4 cells (4x4 grid)
    state.new_sheet(16, 16, 4, 4);

    // Draw a 8x8 red square at (0,0) and copy it (larger than cell size)
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(0, 0, 8, 8, PixelColor::rgb(255, 0, 0));
    }
    let mut selection = SelectionMask::new(16, 16);
    selection.select_rect(0, 0, 8, 8);
    state.active_mut().selection = Some(selection);
    assert!(state.copy_selection());

    // Verify clipboard is 8x8
    assert_eq!(state.clipboard.as_ref().unwrap().width, 8);
    assert_eq!(state.clipboard.as_ref().unwrap().height, 8);

    // Select cell 15 (bottom-right, at position 12,12)
    state.active_mut().selected_cell = Some(15);

    // Paste - should scale the 8x8 clipboard down to 4x4 to fit the cell
    assert!(state.paste_at_cursor(CanvasSide::Left));
    assert!(state.commit_floating());

    // Check that pixels were pasted in cell 15 (scaled down)
    // The 8x8 source scaled to 4x4 should fill the entire cell
    if let Some(canvas) = &state.active().canvas {
        // Cell 15 starts at (12, 12), and scaled content should fill it
        assert_eq!(canvas.get_pixel(12, 12), Some(PixelColor::rgb(255, 0, 0)));
        assert_eq!(canvas.get_pixel(15, 15), Some(PixelColor::rgb(255, 0, 0)));
    }
}

#[test]
fn sprite_editor_cut_selection_copies_then_clears_only_selected_pixels() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(1, 1, 2, 2, PixelColor::rgb(255, 0, 0));
    }

    let mut selection = SelectionMask::new(4, 4);
    selection.select_pixel(1, 1);
    selection.select_pixel(2, 2);
    state.active_mut().selection = Some(selection);

    assert!(state.cut_selection());

    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(1, 1), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(1, 2), Some(PixelColor::rgb(255, 0, 0)));
    assert_eq!(canvas.get_pixel(2, 1), Some(PixelColor::rgb(255, 0, 0)));
    assert!(state.clipboard.is_some());
    assert!(state.active().history.can_undo());
}

#[test]
fn sprite_editor_delete_selection_clears_only_selected_pixels() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(1, 1, 2, 2, PixelColor::rgb(255, 0, 0));
    }

    let mut selection = SelectionMask::new(4, 4);
    selection.select_pixel(1, 1);
    selection.select_pixel(2, 2);
    state.active_mut().selection = Some(selection);

    assert!(state.delete_selection());

    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(1, 1), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(1, 2), Some(PixelColor::rgb(255, 0, 0)));
    assert_eq!(canvas.get_pixel(2, 1), Some(PixelColor::rgb(255, 0, 0)));
    assert!(state.active().history.can_undo());
}

#[test]
fn sprite_canvas_scaled_to_fit_downscales_correctly() {
    use super::SpriteCanvas;

    // Create an 8x8 canvas with a checkerboard pattern
    let mut canvas = SpriteCanvas::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            if (x + y) % 2 == 0 {
                canvas.set_pixel(x, y, PixelColor::rgb(255, 0, 0));
            }
        }
    }

    // Scale to 4x4
    let scaled = canvas.scaled_to_fit(4, 4);
    assert_eq!(scaled.width, 4);
    assert_eq!(scaled.height, 4);

    // The scaled version should have sampled pixels from the original
    // At scale 0.5, each output pixel samples from 2x2 input area
    // The center of output (0,0) maps to input (0.5, 0.5) -> samples (0,0)
    assert!(scaled.get_pixel(0, 0).is_some());
}

#[test]
fn sprite_canvas_scaled_to_fit_no_upscale() {
    use super::SpriteCanvas;

    // Create a 4x4 canvas
    let canvas = SpriteCanvas::new(4, 4);

    // Scaling to 8x8 should not upscale (returns same size)
    let scaled = canvas.scaled_to_fit(8, 8);
    assert_eq!(scaled.width, 4);
    assert_eq!(scaled.height, 4);
}

// ============================================================================
// Magic Wand / Find Connected Sprite Tests
// ============================================================================

#[test]
fn sprite_canvas_find_connected_sprite_simple() {
    use super::SpriteCanvas;

    // Create an 8x8 canvas with a 3x3 sprite in the center
    let mut canvas = SpriteCanvas::new(8, 8);
    for y in 2..5 {
        for x in 2..5 {
            canvas.set_pixel(x, y, PixelColor::rgb(255, 0, 0));
        }
    }

    // Click on the sprite should return its bounding box
    let result = canvas.find_connected_sprite(3, 3);
    assert_eq!(result, Some((2, 2, 3, 3)));

    // Click on a different part of the sprite should return the same box
    let result = canvas.find_connected_sprite(2, 2);
    assert_eq!(result, Some((2, 2, 3, 3)));
}

#[test]
fn sprite_canvas_find_connected_sprite_transparent_returns_none() {
    use super::SpriteCanvas;

    let canvas = SpriteCanvas::new(8, 8);

    // Clicking on transparent area returns None
    let result = canvas.find_connected_sprite(4, 4);
    assert_eq!(result, None);
}

#[test]
fn sprite_canvas_find_connected_sprite_out_of_bounds_returns_none() {
    use super::SpriteCanvas;

    let canvas = SpriteCanvas::new(8, 8);

    // Out of bounds returns None
    assert_eq!(canvas.find_connected_sprite(10, 10), None);
    assert_eq!(canvas.find_connected_sprite(8, 0), None);
    assert_eq!(canvas.find_connected_sprite(0, 8), None);
}

#[test]
fn sprite_canvas_find_connected_sprite_diagonal_connection() {
    use super::SpriteCanvas;

    // Create a diagonal line - should be connected via 8-connectivity
    let mut canvas = SpriteCanvas::new(8, 8);
    canvas.set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    canvas.set_pixel(1, 1, PixelColor::rgb(255, 0, 0));
    canvas.set_pixel(2, 2, PixelColor::rgb(255, 0, 0));

    // All three should be connected
    let result = canvas.find_connected_sprite(1, 1);
    assert_eq!(result, Some((0, 0, 3, 3)));
}

#[test]
fn sprite_canvas_find_connected_sprite_separate_sprites() {
    use super::SpriteCanvas;

    // Create two separate sprites
    let mut canvas = SpriteCanvas::new(16, 8);

    // Sprite 1: 2x2 at (0, 0)
    canvas.fill_rect(0, 0, 2, 2, PixelColor::rgb(255, 0, 0));

    // Sprite 2: 2x2 at (10, 0) - not connected to sprite 1
    canvas.fill_rect(10, 0, 2, 2, PixelColor::rgb(0, 255, 0));

    // Clicking sprite 1 should only select sprite 1
    let result = canvas.find_connected_sprite(0, 0);
    assert_eq!(result, Some((0, 0, 2, 2)));

    // Clicking sprite 2 should only select sprite 2
    let result = canvas.find_connected_sprite(10, 0);
    assert_eq!(result, Some((10, 0, 2, 2)));
}

// ============================================================================
// Floating Selection Tests
// ============================================================================

fn setup_canvas_with_selection() -> SpriteEditorState {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    let red = PixelColor::rgb(255, 0, 0);
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.set_pixel(2, 2, red);
        canvas.set_pixel(3, 3, red);
    }
    let mut mask = SelectionMask::new(8, 8);
    mask.select_pixel(2, 2);
    mask.select_pixel(3, 3);
    state.active_mut().selection = Some(mask);
    state
}

#[test]
fn lift_selection_creates_floating_with_correct_pixels() {
    let mut state = setup_canvas_with_selection();
    let red = PixelColor::rgb(255, 0, 0);

    assert!(state.lift_selection());

    let floating = state.active().floating.as_ref().unwrap();
    assert_eq!(floating.offset, glam::IVec2::new(2, 2));
    assert_eq!(floating.pixels.width, 2);
    assert_eq!(floating.pixels.height, 2);
    assert_eq!(floating.pixels.get_pixel(0, 0), Some(red));
    assert_eq!(floating.pixels.get_pixel(1, 1), Some(red));
    assert_eq!(
        floating.pixels.get_pixel(1, 0),
        Some(PixelColor::transparent())
    );
}

#[test]
fn lift_selection_clears_lifted_pixels_from_canvas() {
    let mut state = setup_canvas_with_selection();

    state.lift_selection();

    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(3, 3), Some(PixelColor::transparent()));
}

#[test]
fn lift_selection_clears_selection_mask() {
    let mut state = setup_canvas_with_selection();

    state.lift_selection();

    assert!(state.active().selection.is_none());
}

#[test]
fn lift_selection_stores_canvas_before_lift() {
    let mut state = setup_canvas_with_selection();
    let red = PixelColor::rgb(255, 0, 0);

    state.lift_selection();

    let floating = state.active().floating.as_ref().unwrap();
    // canvas_before_lift should have the original red pixels
    assert_eq!(floating.canvas_before_lift.get_pixel(2, 2), Some(red));
    assert_eq!(floating.canvas_before_lift.get_pixel(3, 3), Some(red));
}

#[test]
fn lift_selection_fails_without_selection() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    assert!(!state.lift_selection());
    assert!(state.active().floating.is_none());
}

#[test]
fn lift_selection_fails_without_canvas() {
    let mut state = SpriteEditorState::default();

    assert!(!state.lift_selection());
}

#[test]
fn lift_selection_marks_texture_dirty() {
    let mut state = setup_canvas_with_selection();
    state.active_mut().canvas_texture_dirty = false;

    state.lift_selection();

    assert!(state.active().canvas_texture_dirty);
}

// --- commit_floating tests ---

#[test]
fn commit_stamps_pixels_at_current_offset() {
    let mut state = setup_canvas_with_selection();
    let red = PixelColor::rgb(255, 0, 0);
    state.lift_selection();

    state.nudge_floating(glam::IVec2::new(2, 2));
    assert!(state.commit_floating());

    let canvas = state.active().canvas.as_ref().unwrap();
    // Original position should be transparent (was cleared on lift)
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(3, 3), Some(PixelColor::transparent()));
    // New position should have the red pixels
    assert_eq!(canvas.get_pixel(4, 4), Some(red));
    assert_eq!(canvas.get_pixel(5, 5), Some(red));
}

#[test]
fn commit_pushes_one_undo_entry() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();
    state.nudge_floating(glam::IVec2::new(1, 1));

    state.commit_floating();

    assert!(state.active().history.can_undo());
}

#[test]
fn commit_clears_floating() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();

    state.commit_floating();

    assert!(state.active().floating.is_none());
}

#[test]
fn commit_reconstructs_selection_at_new_position() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();
    state.nudge_floating(glam::IVec2::new(2, 2));

    state.commit_floating();

    let selection = state.active().selection.as_ref().unwrap();
    assert!(selection.is_selected(4, 4));
    assert!(selection.is_selected(5, 5));
    assert!(!selection.is_selected(2, 2));
}

#[test]
fn commit_marks_dirty() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();
    state.active_mut().dirty = false;
    state.active_mut().canvas_texture_dirty = false;

    state.commit_floating();

    assert!(state.active().dirty);
    assert!(state.active().canvas_texture_dirty);
}

#[test]
fn commit_without_floating_returns_false() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    assert!(!state.commit_floating());
    assert!(!state.active().history.can_undo());
}

#[test]
fn commit_then_undo_restores_pre_lift_canvas() {
    let mut state = setup_canvas_with_selection();
    let red = PixelColor::rgb(255, 0, 0);
    state.lift_selection();
    state.nudge_floating(glam::IVec2::new(2, 2));
    state.commit_floating();

    state.undo();

    let canvas = state.active().canvas.as_ref().unwrap();
    // Should be back to original (pre-lift) state with red at (2,2) and (3,3)
    assert_eq!(canvas.get_pixel(2, 2), Some(red));
    assert_eq!(canvas.get_pixel(3, 3), Some(red));
    assert_eq!(canvas.get_pixel(4, 4), Some(PixelColor::transparent()));
}

// --- cancel_floating tests ---

#[test]
fn cancel_restores_original_canvas() {
    let mut state = setup_canvas_with_selection();
    let red = PixelColor::rgb(255, 0, 0);
    state.lift_selection();
    state.nudge_floating(glam::IVec2::new(2, 2));

    assert!(state.cancel_floating());

    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(2, 2), Some(red));
    assert_eq!(canvas.get_pixel(3, 3), Some(red));
}

#[test]
fn cancel_does_not_push_undo() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();

    state.cancel_floating();

    assert!(!state.active().history.can_undo());
}

#[test]
fn cancel_clears_floating() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();

    state.cancel_floating();

    assert!(state.active().floating.is_none());
}

#[test]
fn cancel_restores_selection_at_original_position() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();
    state.nudge_floating(glam::IVec2::new(2, 2));

    state.cancel_floating();

    let selection = state.active().selection.as_ref().unwrap();
    assert!(selection.is_selected(2, 2));
    assert!(selection.is_selected(3, 3));
    assert!(!selection.is_selected(4, 4));
}

#[test]
fn cancel_without_floating_returns_false() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    assert!(!state.cancel_floating());
}

// --- nudge_floating tests ---

#[test]
fn nudge_moves_offset() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();

    state.nudge_floating(glam::IVec2::new(1, -1));

    let floating = state.active().floating.as_ref().unwrap();
    assert_eq!(floating.offset, glam::IVec2::new(3, 1));
}

#[test]
fn nudge_allows_negative_offset() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();

    state.nudge_floating(glam::IVec2::new(-5, -5));

    let floating = state.active().floating.as_ref().unwrap();
    assert_eq!(floating.offset, glam::IVec2::new(-3, -3));
}

#[test]
fn nudge_no_op_without_floating() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // Should not panic
    state.nudge_floating(glam::IVec2::new(1, 1));
}

#[test]
fn nudge_does_not_push_undo() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();

    state.nudge_floating(glam::IVec2::new(1, 1));

    assert!(!state.active().history.can_undo());
}

#[test]
fn lift_and_nudge_lifts_then_moves_in_one_step() {
    let mut state = setup_canvas_with_selection();

    // No float exists yet, just a selection
    assert!(!state.has_floating());

    state.lift_and_nudge(glam::IVec2::new(1, 0));

    // Should now have a floating selection, offset moved by (1,0)
    assert!(state.has_floating());
    let floating = state.active().floating.as_ref().unwrap();
    assert_eq!(floating.offset, glam::IVec2::new(3, 2));

    // Canvas should have pixels cleared at original position
    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
}

#[test]
fn lift_and_nudge_just_nudges_when_already_floating() {
    let mut state = setup_canvas_with_selection();
    state.lift_selection();
    let initial_offset = state.active().floating.as_ref().unwrap().offset;

    state.lift_and_nudge(glam::IVec2::new(0, -1));

    let floating = state.active().floating.as_ref().unwrap();
    assert_eq!(floating.offset, initial_offset + glam::IVec2::new(0, -1));
}

#[test]
fn lift_and_nudge_no_op_without_selection_or_float() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    // Should not panic, should not create a float
    state.lift_and_nudge(glam::IVec2::new(1, 0));
    assert!(!state.has_floating());
}

// --- paste creates float tests ---

#[test]
fn paste_creates_floating_instead_of_stamping() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    let blue = PixelColor::rgb(0, 0, 255);
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(0, 0, 2, 2, blue);
    }
    // Select and copy
    let mut mask = SelectionMask::new(8, 8);
    mask.select_rect(0, 0, 2, 2);
    state.active_mut().selection = Some(mask);
    state.copy_selection();
    // Set cursor position for paste target
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));

    state.paste_at_cursor(CanvasSide::Left);

    // Should have a floating selection, NOT stamped onto canvas
    assert!(state.active().floating.is_some());
    let canvas = state.active().canvas.as_ref().unwrap();
    // Paste target area should still be empty (not stamped yet)
    assert_eq!(canvas.get_pixel(4, 4), Some(PixelColor::transparent()));
}

#[test]
fn paste_then_commit_stamps_pixels() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    let blue = PixelColor::rgb(0, 0, 255);
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(0, 0, 2, 2, blue);
    }
    let mut mask = SelectionMask::new(8, 8);
    mask.select_rect(0, 0, 2, 2);
    state.active_mut().selection = Some(mask);
    state.copy_selection();
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));
    state.paste_at_cursor(CanvasSide::Left);

    state.commit_floating();

    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(4, 4), Some(blue));
    assert_eq!(canvas.get_pixel(5, 5), Some(blue));
}

#[test]
fn paste_then_cancel_leaves_canvas_unchanged() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    let blue = PixelColor::rgb(0, 0, 255);
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(0, 0, 2, 2, blue);
    }
    let mut mask = SelectionMask::new(8, 8);
    mask.select_rect(0, 0, 2, 2);
    state.active_mut().selection = Some(mask);
    state.copy_selection();
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));

    let canvas_before = state.active().canvas.clone().unwrap();
    state.paste_at_cursor(CanvasSide::Left);
    state.cancel_floating();

    assert_eq!(state.active().canvas.as_ref().unwrap(), &canvas_before);
}

#[test]
fn paste_then_cancel_restores_previous_selection_exactly() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    let blue = PixelColor::rgb(0, 0, 255);
    if let Some(canvas) = &mut state.active_mut().canvas {
        canvas.fill_rect(0, 0, 2, 2, blue);
    }

    let mut mask = SelectionMask::new(8, 8);
    mask.select_rect(0, 0, 2, 2);
    state.active_mut().selection = Some(mask.clone());
    state.copy_selection();
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));

    state.paste_at_cursor(CanvasSide::Left);
    state.cancel_floating();

    let selection = state.active().selection.as_ref().unwrap();
    assert_eq!(selection, &mask);
    assert!(selection.is_selected(0, 0));
    assert!(selection.is_selected(1, 1));
    assert!(!selection.is_selected(4, 4));
}

#[test]
fn paste_then_cancel_without_prior_selection_restores_none() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    state.clipboard = Some(SpriteCanvas::filled(2, 2, PixelColor::rgb(0, 0, 255)));
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));

    state.paste_at_cursor(CanvasSide::Left);
    state.cancel_floating();

    assert!(state.active().selection.is_none());
}

#[test]
fn paste_then_cancel_does_not_push_undo() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    state.clipboard = Some(SpriteCanvas::filled(2, 2, PixelColor::rgb(0, 0, 255)));
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));

    state.paste_at_cursor(CanvasSide::Left);
    state.cancel_floating();

    assert!(!state.active().history.can_undo());
}

#[test]
fn paste_then_commit_selects_pasted_pixels_at_committed_position() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    state.clipboard = Some(SpriteCanvas::filled(2, 2, PixelColor::rgb(0, 0, 255)));
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(4, 4));

    state.paste_at_cursor(CanvasSide::Left);
    state.commit_floating();

    let selection = state.active().selection.as_ref().unwrap();
    assert!(selection.is_selected(4, 4));
    assert!(selection.is_selected(5, 5));
    assert!(!selection.is_selected(0, 0));
}

#[test]
fn paste_auto_commits_existing_float() {
    let mut state = setup_canvas_with_selection();
    let red = PixelColor::rgb(255, 0, 0);
    // Lift selection to create first float
    state.lift_selection();
    state.nudge_floating(glam::IVec2::new(1, 1));

    // Copy something to clipboard for paste
    state.clipboard = Some(SpriteCanvas::filled(1, 1, PixelColor::rgb(0, 255, 0)));
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(0, 0));

    state.paste_at_cursor(CanvasSide::Left);

    // First float should have been committed (red pixels at nudged position)
    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(3, 3), Some(red));
    assert_eq!(canvas.get_pixel(4, 4), Some(red));
    // New float from paste should exist
    assert!(state.active().floating.is_some());
}

#[test]
fn paste_without_clipboard_returns_false() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);
    state.active_mut().cursor_canvas_pos = Some(glam::IVec2::new(0, 0));

    assert!(!state.paste_at_cursor(CanvasSide::Left));
    assert!(state.active().floating.is_none());
}

// --- set_tool tests ---

#[test]
fn set_tool_commits_floating_before_switching() {
    use super::SpriteEditorTool;
    let mut state = setup_canvas_with_selection();
    let red = PixelColor::rgb(255, 0, 0);
    state.lift_selection();
    state.nudge_floating(glam::IVec2::new(1, 1));

    state.set_tool(SpriteEditorTool::Brush);

    assert_eq!(state.tool, SpriteEditorTool::Brush);
    assert!(state.active().floating.is_none());
    let canvas = state.active().canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(3, 3), Some(red));
    assert_eq!(canvas.get_pixel(4, 4), Some(red));
}

#[test]
fn set_tool_without_float_just_switches() {
    use super::SpriteEditorTool;
    let mut state = SpriteEditorState::default();
    state.new_canvas(8, 8);

    state.set_tool(SpriteEditorTool::Eraser);

    assert_eq!(state.tool, SpriteEditorTool::Eraser);
    assert!(!state.active().history.can_undo());
}
