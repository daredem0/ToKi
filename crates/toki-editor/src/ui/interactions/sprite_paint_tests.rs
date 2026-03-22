use super::*;

fn create_test_canvas(width: u32, height: u32) -> SpriteCanvas {
    SpriteCanvas::new(width, height)
}

// ============================================================================
// Brush Footprint Tests
// ============================================================================

#[test]
fn brush_footprint_bounds_single_pixel() {
    let canvas = create_test_canvas(8, 8);
    let result = SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(4, 4), 1);
    assert_eq!(result, Some((UVec2::new(4, 4), UVec2::new(5, 5))));
}

#[test]
fn brush_footprint_bounds_three_pixel_brush() {
    let canvas = create_test_canvas(8, 8);
    let result = SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(4, 4), 3);
    // 3x3 brush centered at (4,4) -> start at (3,3), end at (6,6)
    assert_eq!(result, Some((UVec2::new(3, 3), UVec2::new(6, 6))));
}

#[test]
fn brush_footprint_bounds_clips_to_canvas_edge() {
    let canvas = create_test_canvas(8, 8);
    // 3x3 brush centered at (0,0):
    // - radius = 1, start clips to (0,0), end is (0+3)=3
    let result = SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(0, 0), 3);
    assert_eq!(result, Some((UVec2::new(0, 0), UVec2::new(3, 3))));

    // At bottom-right corner (7,7) with 3x3 brush:
    // - start = (7-1, 7-1) = (6, 6), end clips to (8, 8)
    let result = SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(7, 7), 3);
    assert_eq!(result, Some((UVec2::new(6, 6), UVec2::new(8, 8))));
}

#[test]
fn brush_footprint_bounds_out_of_bounds_returns_none() {
    let canvas = create_test_canvas(8, 8);
    assert!(
        SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(-1, 4), 1).is_none()
    );
    assert!(
        SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(4, -1), 1).is_none()
    );
    assert!(SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(8, 4), 1).is_none());
    assert!(SpritePaintInteraction::brush_footprint_bounds(&canvas, IVec2::new(4, 8), 1).is_none());
}

// ============================================================================
// Paint Pixel Tests
// ============================================================================

#[test]
fn paint_pixel_sets_color() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(255, 0, 0);
    assert!(SpritePaintInteraction::paint_pixel(
        &mut canvas,
        IVec2::new(4, 4),
        color
    ));
    assert_eq!(canvas.get_pixel(4, 4), Some(color));
}

#[test]
fn paint_pixel_negative_coords_returns_false() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(255, 0, 0);
    assert!(!SpritePaintInteraction::paint_pixel(
        &mut canvas,
        IVec2::new(-1, 4),
        color
    ));
    assert!(!SpritePaintInteraction::paint_pixel(
        &mut canvas,
        IVec2::new(4, -1),
        color
    ));
}

// ============================================================================
// Paint Brush Tests
// ============================================================================

#[test]
fn paint_brush_paints_area() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(0, 255, 0);
    assert!(SpritePaintInteraction::paint_brush(
        &mut canvas,
        IVec2::new(4, 4),
        color,
        3
    ));

    // Check 3x3 area is painted
    for y in 3..6 {
        for x in 3..6 {
            assert_eq!(canvas.get_pixel(x, y), Some(color), "Pixel at ({x}, {y})");
        }
    }

    // Check outside area is not painted
    assert_eq!(canvas.get_pixel(2, 4), Some(PixelColor::transparent()));
}

// ============================================================================
// Erase Brush Tests
// ============================================================================

#[test]
fn erase_brush_sets_transparent() {
    let mut canvas = SpriteCanvas::filled(8, 8, PixelColor::rgb(255, 255, 255));
    assert!(SpritePaintInteraction::erase_brush(
        &mut canvas,
        IVec2::new(4, 4),
        1
    ));
    assert_eq!(canvas.get_pixel(4, 4), Some(PixelColor::transparent()));
}

// ============================================================================
// Flood Fill Tests
// ============================================================================

#[test]
fn flood_fill_fills_connected_region() {
    let mut canvas = create_test_canvas(8, 8);
    let fill_color = PixelColor::rgb(255, 0, 0);

    assert!(SpritePaintInteraction::flood_fill(
        &mut canvas,
        IVec2::new(0, 0),
        fill_color
    ));

    // Entire canvas should be filled
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(
                canvas.get_pixel(x, y),
                Some(fill_color),
                "Pixel at ({x}, {y})"
            );
        }
    }
}

#[test]
fn flood_fill_respects_boundaries() {
    let mut canvas = create_test_canvas(8, 8);
    let barrier_color = PixelColor::rgb(0, 0, 255);
    let fill_color = PixelColor::rgb(255, 0, 0);

    // Create a vertical barrier at x=4
    for y in 0..8 {
        canvas.set_pixel(4, y, barrier_color);
    }

    // Fill left side
    assert!(SpritePaintInteraction::flood_fill(
        &mut canvas,
        IVec2::new(0, 0),
        fill_color
    ));

    // Left side should be filled
    for y in 0..8 {
        for x in 0..4 {
            assert_eq!(
                canvas.get_pixel(x, y),
                Some(fill_color),
                "Left side ({x}, {y})"
            );
        }
    }

    // Right side should still be transparent
    for y in 0..8 {
        for x in 5..8 {
            assert_eq!(
                canvas.get_pixel(x, y),
                Some(PixelColor::transparent()),
                "Right side ({x}, {y})"
            );
        }
    }

    // Barrier should be unchanged
    for y in 0..8 {
        assert_eq!(
            canvas.get_pixel(4, y),
            Some(barrier_color),
            "Barrier at y={y}"
        );
    }
}

#[test]
fn flood_fill_same_color_returns_false() {
    let mut canvas = SpriteCanvas::filled(8, 8, PixelColor::rgb(255, 0, 0));
    let fill_color = PixelColor::rgb(255, 0, 0);
    assert!(!SpritePaintInteraction::flood_fill(
        &mut canvas,
        IVec2::new(0, 0),
        fill_color
    ));
}

#[test]
fn flood_fill_out_of_bounds_returns_false() {
    let mut canvas = create_test_canvas(8, 8);
    let fill_color = PixelColor::rgb(255, 0, 0);
    assert!(!SpritePaintInteraction::flood_fill(
        &mut canvas,
        IVec2::new(-1, 0),
        fill_color
    ));
    assert!(!SpritePaintInteraction::flood_fill(
        &mut canvas,
        IVec2::new(0, -1),
        fill_color
    ));
}

#[test]
fn erase_connected_color_in_bounds_erases_only_connected_region() {
    let mut canvas = SpriteCanvas::filled(4, 4, PixelColor::rgb(10, 20, 30));
    let island = PixelColor::rgb(200, 10, 10);
    canvas.set_pixel(1, 1, island);
    canvas.set_pixel(2, 1, island);

    assert!(SpritePaintInteraction::erase_connected_color_in_bounds(
        &mut canvas,
        IVec2::new(0, 0),
        (UVec2::ZERO, UVec2::new(4, 4)),
    ));

    assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(3, 3), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(1, 1), Some(island));
    assert_eq!(canvas.get_pixel(2, 1), Some(island));
}

#[test]
fn erase_connected_color_in_bounds_respects_tile_bounds() {
    let background = PixelColor::rgb(40, 50, 60);
    let mut canvas = SpriteCanvas::filled(4, 2, background);

    assert!(SpritePaintInteraction::erase_connected_color_in_bounds(
        &mut canvas,
        IVec2::new(0, 0),
        (UVec2::new(0, 0), UVec2::new(2, 2)),
    ));

    assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(1, 1), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(2, 0), Some(background));
    assert_eq!(canvas.get_pixel(3, 1), Some(background));
}

#[test]
fn erase_connected_color_in_bounds_uses_exact_color_matching() {
    let mut canvas = SpriteCanvas::filled(3, 3, PixelColor::rgb(5, 5, 5));
    let different = PixelColor::rgb(5, 5, 6);
    canvas.set_pixel(1, 0, different);

    assert!(SpritePaintInteraction::erase_connected_color_in_bounds(
        &mut canvas,
        IVec2::new(0, 0),
        (UVec2::ZERO, UVec2::new(3, 3)),
    ));

    assert_eq!(canvas.get_pixel(1, 0), Some(different));
    assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::transparent()));
}

#[test]
fn erase_connected_color_in_bounds_transparent_click_is_no_op() {
    let mut canvas = create_test_canvas(3, 3);

    assert!(!SpritePaintInteraction::erase_connected_color_in_bounds(
        &mut canvas,
        IVec2::new(1, 1),
        (UVec2::ZERO, UVec2::new(3, 3)),
    ));
}

#[test]
fn add_outline_in_bounds_adds_outline_around_clicked_sprite() {
    let mut canvas = create_test_canvas(5, 5);
    let sprite = PixelColor::rgb(200, 100, 50);
    let outline = PixelColor::rgb(0, 0, 0);
    canvas.set_pixel(2, 2, sprite);

    assert!(SpritePaintInteraction::add_outline_in_bounds(
        &mut canvas,
        IVec2::new(2, 2),
        outline,
        (UVec2::ZERO, UVec2::new(5, 5)),
    ));

    for y in 1..=3 {
        for x in 1..=3 {
            let expected = if x == 2 && y == 2 { sprite } else { outline };
            assert_eq!(canvas.get_pixel(x, y), Some(expected), "Pixel at ({x}, {y})");
        }
    }
}

#[test]
fn add_outline_in_bounds_respects_tile_bounds() {
    let mut canvas = create_test_canvas(6, 3);
    let sprite = PixelColor::rgb(220, 50, 50);
    let outline = PixelColor::rgb(10, 10, 10);
    canvas.set_pixel(1, 1, sprite);
    canvas.set_pixel(4, 1, sprite);

    assert!(SpritePaintInteraction::add_outline_in_bounds(
        &mut canvas,
        IVec2::new(1, 1),
        outline,
        (UVec2::new(0, 0), UVec2::new(3, 3)),
    ));

    assert_eq!(canvas.get_pixel(0, 1), Some(outline));
    assert_eq!(canvas.get_pixel(2, 1), Some(outline));
    assert_eq!(canvas.get_pixel(3, 1), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(4, 1), Some(sprite));
}

#[test]
fn add_outline_in_bounds_does_not_fill_internal_holes() {
    let mut canvas = create_test_canvas(5, 5);
    let sprite = PixelColor::rgb(120, 120, 120);
    let outline = PixelColor::rgb(0, 0, 0);

    for y in 1..=3 {
        for x in 1..=3 {
            if x == 2 && y == 2 {
                continue;
            }
            canvas.set_pixel(x, y, sprite);
        }
    }

    assert!(SpritePaintInteraction::add_outline_in_bounds(
        &mut canvas,
        IVec2::new(1, 1),
        outline,
        (UVec2::ZERO, UVec2::new(5, 5)),
    ));

    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(0, 0), Some(outline));
    assert_eq!(canvas.get_pixel(4, 4), Some(outline));
}

#[test]
fn add_outline_in_bounds_transparent_click_is_no_op() {
    let mut canvas = create_test_canvas(4, 4);

    assert!(!SpritePaintInteraction::add_outline_in_bounds(
        &mut canvas,
        IVec2::new(1, 1),
        PixelColor::black(),
        (UVec2::ZERO, UVec2::new(4, 4)),
    ));
}

#[test]
fn add_ground_shadow_in_bounds_projects_shadow_below_bottom_contour() {
    let mut canvas = create_test_canvas(5, 5);
    let sprite = PixelColor::rgb(200, 100, 50);
    let shadow = PixelColor::rgb(20, 20, 20);
    canvas.set_pixel(2, 1, sprite);
    canvas.set_pixel(2, 2, sprite);

    assert!(SpritePaintInteraction::add_ground_shadow_in_bounds(
        &mut canvas,
        IVec2::new(2, 1),
        shadow,
        (UVec2::ZERO, UVec2::new(5, 5)),
    ));

    assert_eq!(canvas.get_pixel(1, 3), Some(shadow));
    assert_eq!(canvas.get_pixel(2, 3), Some(shadow));
    assert_eq!(canvas.get_pixel(3, 3), Some(shadow));
    assert_eq!(canvas.get_pixel(2, 2), Some(sprite));
}

#[test]
fn add_ground_shadow_in_bounds_respects_tile_bounds() {
    let mut canvas = create_test_canvas(6, 4);
    let sprite = PixelColor::rgb(120, 120, 220);
    let shadow = PixelColor::rgb(15, 15, 15);
    canvas.set_pixel(1, 1, sprite);
    canvas.set_pixel(4, 1, sprite);

    assert!(SpritePaintInteraction::add_ground_shadow_in_bounds(
        &mut canvas,
        IVec2::new(1, 1),
        shadow,
        (UVec2::new(0, 0), UVec2::new(3, 4)),
    ));

    assert_eq!(canvas.get_pixel(0, 2), Some(shadow));
    assert_eq!(canvas.get_pixel(1, 2), Some(shadow));
    assert_eq!(canvas.get_pixel(2, 2), Some(shadow));
    assert_eq!(canvas.get_pixel(3, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(4, 1), Some(sprite));
}

#[test]
fn add_ground_shadow_in_bounds_does_not_fill_internal_holes() {
    let mut canvas = create_test_canvas(5, 5);
    let sprite = PixelColor::rgb(180, 180, 180);
    let shadow = PixelColor::rgb(0, 0, 0);

    for y in 1..=3 {
        for x in 1..=3 {
            if x == 2 && y == 2 {
                continue;
            }
            canvas.set_pixel(x, y, sprite);
        }
    }

    assert!(SpritePaintInteraction::add_ground_shadow_in_bounds(
        &mut canvas,
        IVec2::new(1, 1),
        shadow,
        (UVec2::ZERO, UVec2::new(5, 5)),
    ));

    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
}

#[test]
fn add_ground_shadow_in_bounds_transparent_click_is_no_op() {
    let mut canvas = create_test_canvas(4, 4);

    assert!(!SpritePaintInteraction::add_ground_shadow_in_bounds(
        &mut canvas,
        IVec2::new(1, 1),
        PixelColor::black(),
        (UVec2::ZERO, UVec2::new(4, 4)),
    ));
}

// ============================================================================
// Draw Line Tests
// ============================================================================

#[test]
fn draw_line_horizontal() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(255, 0, 0);

    assert!(SpritePaintInteraction::draw_line(
        &mut canvas,
        IVec2::new(1, 4),
        IVec2::new(6, 4),
        color,
        1
    ));

    // Check horizontal line
    for x in 1..=6 {
        assert_eq!(canvas.get_pixel(x, 4), Some(color), "Pixel at ({x}, 4)");
    }

    // Check pixels above and below are not painted
    assert_eq!(canvas.get_pixel(3, 3), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(3, 5), Some(PixelColor::transparent()));
}

#[test]
fn draw_line_vertical() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(0, 255, 0);

    assert!(SpritePaintInteraction::draw_line(
        &mut canvas,
        IVec2::new(4, 1),
        IVec2::new(4, 6),
        color,
        1
    ));

    // Check vertical line
    for y in 1..=6 {
        assert_eq!(canvas.get_pixel(4, y), Some(color), "Pixel at (4, {y})");
    }
}

#[test]
fn draw_line_diagonal() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(0, 0, 255);

    assert!(SpritePaintInteraction::draw_line(
        &mut canvas,
        IVec2::new(0, 0),
        IVec2::new(7, 7),
        color,
        1
    ));

    // Check diagonal line
    for i in 0..=7 {
        assert_eq!(canvas.get_pixel(i, i), Some(color), "Pixel at ({i}, {i})");
    }
}

#[test]
fn draw_line_single_point() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(255, 255, 0);

    assert!(SpritePaintInteraction::draw_line(
        &mut canvas,
        IVec2::new(4, 4),
        IVec2::new(4, 4),
        color,
        1
    ));

    assert_eq!(canvas.get_pixel(4, 4), Some(color));
}

// ============================================================================
// Pick Color Tests
// ============================================================================

#[test]
fn pick_color_returns_pixel_color() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(123, 45, 67);
    canvas.set_pixel(3, 5, color);

    assert_eq!(
        SpritePaintInteraction::pick_color(&canvas, IVec2::new(3, 5)),
        Some(color)
    );
}

#[test]
fn pick_color_out_of_bounds_returns_none() {
    let canvas = create_test_canvas(8, 8);
    assert!(SpritePaintInteraction::pick_color(&canvas, IVec2::new(-1, 0)).is_none());
    assert!(SpritePaintInteraction::pick_color(&canvas, IVec2::new(0, -1)).is_none());
    assert!(SpritePaintInteraction::pick_color(&canvas, IVec2::new(8, 0)).is_none());
    assert!(SpritePaintInteraction::pick_color(&canvas, IVec2::new(0, 8)).is_none());
}
