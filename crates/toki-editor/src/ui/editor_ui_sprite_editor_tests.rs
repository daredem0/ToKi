use super::*;

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
    let mut viewport = SpriteCanvasViewport::default();
    viewport.zoom = 2.0;
    viewport.pan = glam::Vec2::ZERO;

    // Pan by 10 screen pixels at zoom 2.0 should move 5 canvas pixels
    viewport.pan_by(glam::Vec2::new(10.0, 20.0));

    assert_eq!(viewport.pan, glam::Vec2::new(-5.0, -10.0));
}

#[test]
fn sprite_canvas_viewport_screen_to_canvas_conversion() {
    let mut viewport = SpriteCanvasViewport::default();
    viewport.zoom = 4.0;
    viewport.pan = glam::Vec2::new(10.0, 20.0);

    let rect = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 200.0));
    let screen_pos = glam::Vec2::new(140.0, 90.0); // 40 pixels into viewport

    let canvas_pos = viewport.screen_to_canvas(screen_pos, rect);
    // (40, 40) screen pixels / 4.0 zoom + (10, 20) pan = (20, 30) canvas
    assert_eq!(canvas_pos, glam::Vec2::new(20.0, 30.0));
}

#[test]
fn sprite_canvas_viewport_canvas_to_screen_conversion() {
    let mut viewport = SpriteCanvasViewport::default();
    viewport.zoom = 4.0;
    viewport.pan = glam::Vec2::new(10.0, 20.0);

    let rect = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(200.0, 200.0));
    let canvas_pos = glam::Vec2::new(20.0, 30.0);

    let screen_pos = viewport.canvas_to_screen(canvas_pos, rect);
    // ((20, 30) - (10, 20)) * 4.0 + (100, 50) = (140, 90)
    assert_eq!(screen_pos, glam::Vec2::new(140.0, 90.0));
}

// ============================================================================
// SpriteSelection Tests
// ============================================================================

#[test]
fn sprite_selection_contains_works() {
    let sel = SpriteSelection::new(5, 10, 20, 15);

    // Inside
    assert!(sel.contains(5, 10));
    assert!(sel.contains(24, 24));
    assert!(sel.contains(15, 17));

    // Outside
    assert!(!sel.contains(4, 10));
    assert!(!sel.contains(5, 9));
    assert!(!sel.contains(25, 10));
    assert!(!sel.contains(5, 25));
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
    assert!(state.canvas.is_none());
    assert!(!state.dirty);
    assert_eq!(state.tool, SpriteEditorTool::Drag);
    assert_eq!(state.brush_size, 1);
    assert!(state.show_grid);
}

#[test]
fn sprite_editor_state_new_canvas() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(16, 32);

    assert!(state.has_canvas());
    assert_eq!(state.canvas_dimensions(), Some((16, 32)));
    assert!(state.dirty);
}

#[test]
fn sprite_editor_state_new_canvas_filled() {
    let mut state = SpriteEditorState::default();
    let color = PixelColor::rgb(100, 150, 200);
    state.new_canvas_filled(8, 8, color);

    assert!(state.has_canvas());
    let canvas = state.canvas.as_ref().unwrap();
    assert_eq!(canvas.get_pixel(0, 0), Some(color));
}

#[test]
fn sprite_editor_state_close_canvas() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(16, 16);
    state.close_canvas();

    assert!(!state.has_canvas());
    assert!(!state.dirty);
}

#[test]
fn sprite_editor_state_undo_redo_integration() {
    let mut state = SpriteEditorState::default();
    state.new_canvas(4, 4);

    let before = state.canvas.clone().unwrap();
    state
        .canvas
        .as_mut()
        .unwrap()
        .set_pixel(0, 0, PixelColor::rgb(255, 0, 0));
    state.push_undo_state(before);

    // Check pixel was changed
    assert_eq!(
        state.canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(PixelColor::rgb(255, 0, 0))
    );

    // Undo
    assert!(state.undo());
    assert_eq!(
        state.canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(PixelColor::transparent())
    );

    // Redo
    assert!(state.redo());
    assert_eq!(
        state.canvas.as_ref().unwrap().get_pixel(0, 0),
        Some(PixelColor::rgb(255, 0, 0))
    );
}

#[test]
fn sprite_editor_state_recent_colors() {
    let mut state = SpriteEditorState::default();
    state.max_recent_colors = 3;

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
    assert!(state.dirty); // Should be marked dirty since it's newly imported

    let (w, h) = state.canvas_dimensions().unwrap();
    assert_eq!(w, 2);
    assert_eq!(h, 2);

    // Check that name is derived from filename
    assert_eq!(state.save_asset_name, "test");
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
    if let Some(canvas) = &mut state.canvas {
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
    assert!(!state.dirty); // Should not be dirty - loaded from file
    assert!(state.show_cell_grid); // Should show grid for multi-tile sprite
    assert_eq!(state.cell_size.x, 8);
    assert_eq!(state.cell_size.y, 8);
    assert_eq!(state.save_asset_name, "sprite");
}
