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
            assert_eq!(
                canvas.get_pixel(x, y),
                Some(expected),
                "Pixel at ({x}, {y})"
            );
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

// ============================================================================
// Rectangle Tests
// ============================================================================

#[test]
fn draw_rectangle_outline_basic() {
    let mut canvas = create_test_canvas(10, 10);
    let color = PixelColor::rgb(255, 0, 0);
    let params = ShapeParams {
        start: IVec2::new(1, 1),
        end: IVec2::new(6, 6),
        color,
        brush_size: 1,
        filled: false,
    };

    assert!(SpritePaintInteraction::draw_rectangle(&mut canvas, &params));

    // Top edge
    for x in 1..=6 {
        assert_eq!(canvas.get_pixel(x, 1), Some(color), "Top edge at ({x}, 1)");
    }
    // Bottom edge
    for x in 1..=6 {
        assert_eq!(
            canvas.get_pixel(x, 6),
            Some(color),
            "Bottom edge at ({x}, 6)"
        );
    }
    // Left edge
    for y in 1..=6 {
        assert_eq!(canvas.get_pixel(1, y), Some(color), "Left edge at (1, {y})");
    }
    // Right edge
    for y in 1..=6 {
        assert_eq!(
            canvas.get_pixel(6, y),
            Some(color),
            "Right edge at (6, {y})"
        );
    }
    // Interior should be transparent
    assert_eq!(canvas.get_pixel(3, 3), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(4, 4), Some(PixelColor::transparent()));
}

#[test]
fn draw_rectangle_outline_single_point() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(0, 255, 0);
    let params = ShapeParams {
        start: IVec2::new(4, 4),
        end: IVec2::new(4, 4),
        color,
        brush_size: 1,
        filled: false,
    };

    assert!(SpritePaintInteraction::draw_rectangle(&mut canvas, &params));
    assert_eq!(canvas.get_pixel(4, 4), Some(color));
}

#[test]
fn draw_rectangle_outline_backwards_coords() {
    let color = PixelColor::rgb(0, 0, 255);
    let params_forward = ShapeParams {
        start: IVec2::new(1, 1),
        end: IVec2::new(5, 5),
        color,
        brush_size: 1,
        filled: false,
    };
    let params_backward = ShapeParams {
        start: IVec2::new(5, 5),
        end: IVec2::new(1, 1),
        color,
        brush_size: 1,
        filled: false,
    };

    let mut canvas1 = create_test_canvas(8, 8);
    let mut canvas2 = create_test_canvas(8, 8);
    SpritePaintInteraction::draw_rectangle(&mut canvas1, &params_forward);
    SpritePaintInteraction::draw_rectangle(&mut canvas2, &params_backward);

    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(
                canvas1.get_pixel(x, y),
                canvas2.get_pixel(x, y),
                "Mismatch at ({x}, {y})"
            );
        }
    }
}

#[test]
fn draw_rectangle_outline_with_brush_size() {
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(255, 255, 0);
    let params = ShapeParams {
        start: IVec2::new(4, 4),
        end: IVec2::new(11, 11),
        color,
        brush_size: 3,
        filled: false,
    };

    SpritePaintInteraction::draw_rectangle(&mut canvas, &params);

    // Top edge at y=4 should have pixels above it at y=3 due to brush_size=3
    assert_eq!(canvas.get_pixel(4, 3), Some(color));
    // Interior well inside should still be transparent
    assert_eq!(canvas.get_pixel(7, 7), Some(PixelColor::transparent()));
}

#[test]
fn draw_rectangle_filled_basic() {
    let mut canvas = create_test_canvas(10, 10);
    let color = PixelColor::rgb(255, 0, 0);
    let params = ShapeParams {
        start: IVec2::new(2, 2),
        end: IVec2::new(5, 5),
        color,
        brush_size: 1,
        filled: true,
    };

    assert!(SpritePaintInteraction::draw_rectangle(&mut canvas, &params));

    // All interior pixels painted
    for y in 2..=5 {
        for x in 2..=5 {
            assert_eq!(
                canvas.get_pixel(x, y),
                Some(color),
                "Interior at ({x}, {y})"
            );
        }
    }
    // Outside should be transparent
    assert_eq!(canvas.get_pixel(1, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(6, 2), Some(PixelColor::transparent()));
}

#[test]
fn draw_rectangle_filled_single_pixel() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(0, 255, 0);
    let params = ShapeParams {
        start: IVec2::new(3, 3),
        end: IVec2::new(3, 3),
        color,
        brush_size: 1,
        filled: true,
    };

    assert!(SpritePaintInteraction::draw_rectangle(&mut canvas, &params));
    assert_eq!(canvas.get_pixel(3, 3), Some(color));
    assert_eq!(canvas.get_pixel(2, 3), Some(PixelColor::transparent()));
}

// ============================================================================
// Ellipse Tests
// ============================================================================

#[test]
fn draw_ellipse_outline_circle() {
    let mut canvas = create_test_canvas(20, 20);
    let color = PixelColor::rgb(255, 0, 0);
    let params = ShapeParams {
        start: IVec2::new(2, 2),
        end: IVec2::new(17, 17),
        color,
        brush_size: 1,
        filled: false,
    };

    assert!(SpritePaintInteraction::draw_ellipse(&mut canvas, &params));

    // Boundary pixels should be painted (top/bottom/left/right of circle)
    let cx = 9;
    let cy = 9;
    let r = 7; // (17-2)/2 = 7
               // Top and bottom
    assert_eq!(
        canvas.get_pixel(cx as u32, (cy - r) as u32),
        Some(color),
        "Top"
    );
    assert_eq!(
        canvas.get_pixel(cx as u32, (cy + r) as u32),
        Some(color),
        "Bottom"
    );
    // Left and right
    assert_eq!(
        canvas.get_pixel((cx - r) as u32, cy as u32),
        Some(color),
        "Left"
    );
    assert_eq!(
        canvas.get_pixel((cx + r) as u32, cy as u32),
        Some(color),
        "Right"
    );
    // Center should be transparent (outline only)
    assert_eq!(
        canvas.get_pixel(cx as u32, cy as u32),
        Some(PixelColor::transparent())
    );
}

#[test]
fn draw_ellipse_outline_wide() {
    let mut canvas = create_test_canvas(20, 10);
    let color = PixelColor::rgb(0, 255, 0);
    // Bounding box (1,1)→(17,7): cx=9, cy=4, rx=8, ry=3
    let params = ShapeParams {
        start: IVec2::new(1, 1),
        end: IVec2::new(17, 7),
        color,
        brush_size: 1,
        filled: false,
    };

    assert!(SpritePaintInteraction::draw_ellipse(&mut canvas, &params));

    let cy = 4u32;
    // Horizontal extremes: cx±rx = 9±8 = 1 and 17
    assert_eq!(canvas.get_pixel(1, cy), Some(color), "Left extreme");
    assert_eq!(canvas.get_pixel(17, cy), Some(color), "Right extreme");
    // Center should be transparent (outline)
    assert_eq!(canvas.get_pixel(9, cy), Some(PixelColor::transparent()));
}

#[test]
fn draw_ellipse_outline_single_pixel() {
    let mut canvas = create_test_canvas(8, 8);
    let color = PixelColor::rgb(0, 0, 255);
    let params = ShapeParams {
        start: IVec2::new(4, 4),
        end: IVec2::new(4, 4),
        color,
        brush_size: 1,
        filled: false,
    };

    assert!(SpritePaintInteraction::draw_ellipse(&mut canvas, &params));
    assert_eq!(canvas.get_pixel(4, 4), Some(color));
}

#[test]
fn draw_ellipse_outline_backwards_coords() {
    let color = PixelColor::rgb(255, 0, 255);
    let params_fwd = ShapeParams {
        start: IVec2::new(2, 2),
        end: IVec2::new(12, 12),
        color,
        brush_size: 1,
        filled: false,
    };
    let params_bwd = ShapeParams {
        start: IVec2::new(12, 12),
        end: IVec2::new(2, 2),
        color,
        brush_size: 1,
        filled: false,
    };

    let mut canvas1 = create_test_canvas(16, 16);
    let mut canvas2 = create_test_canvas(16, 16);
    SpritePaintInteraction::draw_ellipse(&mut canvas1, &params_fwd);
    SpritePaintInteraction::draw_ellipse(&mut canvas2, &params_bwd);

    for y in 0..16 {
        for x in 0..16 {
            assert_eq!(
                canvas1.get_pixel(x, y),
                canvas2.get_pixel(x, y),
                "Mismatch at ({x}, {y})"
            );
        }
    }
}

#[test]
fn draw_ellipse_filled_circle() {
    let mut canvas = create_test_canvas(20, 20);
    let color = PixelColor::rgb(255, 0, 0);
    let params = ShapeParams {
        start: IVec2::new(2, 2),
        end: IVec2::new(17, 17),
        color,
        brush_size: 1,
        filled: true,
    };

    assert!(SpritePaintInteraction::draw_ellipse(&mut canvas, &params));

    // Center should be painted (filled mode)
    assert_eq!(canvas.get_pixel(9, 9), Some(color));
    // Boundary should be painted too
    assert_eq!(canvas.get_pixel(9, 2), Some(color));
    // Corners outside the ellipse should be transparent
    assert_eq!(canvas.get_pixel(2, 2), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(17, 17), Some(PixelColor::transparent()));
}

#[test]
fn draw_ellipse_filled_small() {
    let mut canvas = create_test_canvas(10, 10);
    let color = PixelColor::rgb(0, 0, 255);
    let params = ShapeParams {
        start: IVec2::new(2, 3),
        end: IVec2::new(6, 5),
        color,
        brush_size: 1,
        filled: true,
    };

    assert!(SpritePaintInteraction::draw_ellipse(&mut canvas, &params));

    // Center should be filled
    assert_eq!(canvas.get_pixel(4, 4), Some(color));
    // Left and right extremes at center row
    assert_eq!(canvas.get_pixel(2, 4), Some(color));
    assert_eq!(canvas.get_pixel(6, 4), Some(color));
}

// ============================================================================
// Symmetry Tests
// ============================================================================

fn full_canvas_symmetry(w: u32, h: u32, horizontal: bool, vertical: bool) -> SymmetryConfig {
    SymmetryConfig {
        bounds: SymmetryBounds {
            origin: UVec2::new(0, 0),
            size: UVec2::new(w, h),
        },
        horizontal,
        vertical,
    }
}

#[test]
fn mirror_x_basic() {
    let bounds = SymmetryBounds {
        origin: UVec2::new(0, 0),
        size: UVec2::new(16, 16),
    };
    let result = bounds.mirror_x(IVec2::new(3, 5));
    assert_eq!(result, IVec2::new(12, 5));
}

#[test]
fn mirror_y_basic() {
    let bounds = SymmetryBounds {
        origin: UVec2::new(0, 0),
        size: UVec2::new(16, 16),
    };
    let result = bounds.mirror_y(IVec2::new(5, 3));
    assert_eq!(result, IVec2::new(5, 12));
}

#[test]
fn mirror_x_with_offset_bounds() {
    // Sheet mode: cell at (16, 0) with size 16x16
    let bounds = SymmetryBounds {
        origin: UVec2::new(16, 0),
        size: UVec2::new(16, 16),
    };
    let result = bounds.mirror_x(IVec2::new(19, 5));
    // local_x = 19 - 16 = 3, mirror = 15 - 3 = 12, result = 12 + 16 = 28
    assert_eq!(result, IVec2::new(28, 5));
}

#[test]
fn mirror_positions_both_axes() {
    let bounds = SymmetryBounds {
        origin: UVec2::new(0, 0),
        size: UVec2::new(16, 16),
    };
    let positions = bounds.mirror_positions(IVec2::new(3, 3), true, true);
    assert_eq!(positions.len(), 4);
    assert!(positions.contains(&IVec2::new(3, 3)));
    assert!(positions.contains(&IVec2::new(12, 3)));
    assert!(positions.contains(&IVec2::new(3, 12)));
    assert!(positions.contains(&IVec2::new(12, 12)));
}

#[test]
fn mirror_positions_on_axis_deduplicates() {
    // For a 16-wide canvas, position 7 mirrors to 8 (different).
    // But for an 8-wide canvas, position 3 mirrors to 4, and position 4 mirrors to 3.
    // True on-axis: for a 15-wide canvas, position 7 mirrors to 7 (same).
    let bounds = SymmetryBounds {
        origin: UVec2::new(0, 0),
        size: UVec2::new(15, 15),
    };
    let positions = bounds.mirror_positions(IVec2::new(7, 7), true, true);
    // 7 mirrors to 14-7=7, so all 4 positions collapse to 1
    assert_eq!(positions.len(), 1);
    assert!(positions.contains(&IVec2::new(7, 7)));
}

#[test]
fn paint_brush_symmetric_horizontal() {
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(255, 0, 0);
    let sym = full_canvas_symmetry(16, 16, true, false);

    SpritePaintInteraction::paint_brush_symmetric(&mut canvas, IVec2::new(3, 5), color, 1, &sym);

    assert_eq!(canvas.get_pixel(3, 5), Some(color));
    assert_eq!(canvas.get_pixel(12, 5), Some(color));
    assert_eq!(canvas.get_pixel(3, 10), Some(PixelColor::transparent()));
}

#[test]
fn paint_brush_symmetric_vertical() {
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(0, 255, 0);
    let sym = full_canvas_symmetry(16, 16, false, true);

    SpritePaintInteraction::paint_brush_symmetric(&mut canvas, IVec2::new(5, 3), color, 1, &sym);

    assert_eq!(canvas.get_pixel(5, 3), Some(color));
    assert_eq!(canvas.get_pixel(5, 12), Some(color));
}

#[test]
fn paint_brush_symmetric_both() {
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(0, 0, 255);
    let sym = full_canvas_symmetry(16, 16, true, true);

    SpritePaintInteraction::paint_brush_symmetric(&mut canvas, IVec2::new(3, 3), color, 1, &sym);

    assert_eq!(canvas.get_pixel(3, 3), Some(color));
    assert_eq!(canvas.get_pixel(12, 3), Some(color));
    assert_eq!(canvas.get_pixel(3, 12), Some(color));
    assert_eq!(canvas.get_pixel(12, 12), Some(color));
}

#[test]
fn paint_brush_symmetric_no_symmetry() {
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(255, 0, 0);
    let sym = full_canvas_symmetry(16, 16, false, false);

    SpritePaintInteraction::paint_brush_symmetric(&mut canvas, IVec2::new(3, 5), color, 1, &sym);

    assert_eq!(canvas.get_pixel(3, 5), Some(color));
    assert_eq!(canvas.get_pixel(12, 5), Some(PixelColor::transparent()));
}

#[test]
fn erase_brush_symmetric_horizontal() {
    let color = PixelColor::rgb(255, 0, 0);
    let mut canvas = SpriteCanvas::filled(16, 16, color);
    let sym = full_canvas_symmetry(16, 16, true, false);

    SpritePaintInteraction::erase_brush_symmetric(&mut canvas, IVec2::new(3, 5), 1, &sym);

    assert_eq!(canvas.get_pixel(3, 5), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(12, 5), Some(PixelColor::transparent()));
    assert_eq!(canvas.get_pixel(5, 5), Some(color));
}

#[test]
fn paint_brush_symmetric_in_cell_bounds() {
    let mut canvas = create_test_canvas(32, 16);
    let color = PixelColor::rgb(255, 0, 0);
    // Symmetry within cell at (16, 0) size 16x16
    let sym = SymmetryConfig {
        bounds: SymmetryBounds {
            origin: UVec2::new(16, 0),
            size: UVec2::new(16, 16),
        },
        horizontal: true,
        vertical: false,
    };

    SpritePaintInteraction::paint_brush_symmetric(&mut canvas, IVec2::new(19, 5), color, 1, &sym);

    // Should paint at (19, 5) and mirror at (28, 5)
    assert_eq!(canvas.get_pixel(19, 5), Some(color));
    assert_eq!(canvas.get_pixel(28, 5), Some(color));
    // Should NOT mirror at (12, 5) — that would be outside the cell bounds
    assert_eq!(canvas.get_pixel(12, 5), Some(PixelColor::transparent()));
}

#[test]
fn draw_line_symmetric_horizontal() {
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(255, 0, 0);
    let sym = full_canvas_symmetry(16, 16, true, false);
    let params = ShapeParams {
        start: IVec2::new(1, 4),
        end: IVec2::new(5, 4),
        color,
        brush_size: 1,
        filled: false,
    };

    SpritePaintInteraction::draw_line_symmetric(&mut canvas, &params, &sym);

    // Original line
    assert_eq!(canvas.get_pixel(1, 4), Some(color));
    assert_eq!(canvas.get_pixel(5, 4), Some(color));
    // Mirrored line (x mirrored: 1→14, 5→10)
    assert_eq!(canvas.get_pixel(14, 4), Some(color));
    assert_eq!(canvas.get_pixel(10, 4), Some(color));
}

#[test]
fn draw_rectangle_symmetric_both() {
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(0, 255, 0);
    let sym = full_canvas_symmetry(16, 16, true, true);
    let params = ShapeParams {
        start: IVec2::new(1, 1),
        end: IVec2::new(3, 3),
        color,
        brush_size: 1,
        filled: true,
    };

    SpritePaintInteraction::draw_rectangle_symmetric(&mut canvas, &params, &sym);

    // Original rectangle (top-left)
    assert_eq!(canvas.get_pixel(1, 1), Some(color));
    assert_eq!(canvas.get_pixel(3, 3), Some(color));
    // Mirrored top-right
    assert_eq!(canvas.get_pixel(14, 1), Some(color));
    // Mirrored bottom-left
    assert_eq!(canvas.get_pixel(1, 14), Some(color));
    // Mirrored bottom-right
    assert_eq!(canvas.get_pixel(14, 14), Some(color));
}

// ============================================================================
// Dithering Tests
// ============================================================================

#[test]
fn should_dither_none_always_true() {
    use crate::ui::sprite_editor::DitherPattern;
    for y in 0..4 {
        for x in 0..4 {
            assert!(should_dither(x, y, DitherPattern::None), "({x}, {y})");
        }
    }
}

#[test]
fn should_dither_checker50_alternates() {
    use crate::ui::sprite_editor::DitherPattern;
    assert!(should_dither(0, 0, DitherPattern::Checker50));
    assert!(!should_dither(1, 0, DitherPattern::Checker50));
    assert!(!should_dither(0, 1, DitherPattern::Checker50));
    assert!(should_dither(1, 1, DitherPattern::Checker50));
}

#[test]
fn should_dither_checker25_sparse() {
    use crate::ui::sprite_editor::DitherPattern;
    let mut count = 0;
    for y in 0..4 {
        for x in 0..4 {
            if should_dither(x, y, DitherPattern::Checker25) {
                count += 1;
            }
        }
    }
    // 25% of 16 = 4 pixels
    assert_eq!(count, 4);
    // Specific positions
    assert!(should_dither(0, 0, DitherPattern::Checker25));
    assert!(!should_dither(1, 0, DitherPattern::Checker25));
    assert!(!should_dither(0, 1, DitherPattern::Checker25));
}

#[test]
fn should_dither_checker75_dense() {
    use crate::ui::sprite_editor::DitherPattern;
    let mut count = 0;
    for y in 0..4 {
        for x in 0..4 {
            if should_dither(x, y, DitherPattern::Checker75) {
                count += 1;
            }
        }
    }
    // 75% of 16 = 12 pixels
    assert_eq!(count, 12);
    // Only odd,odd positions should be false
    assert!(!should_dither(1, 1, DitherPattern::Checker75));
    assert!(!should_dither(3, 3, DitherPattern::Checker75));
    assert!(should_dither(0, 0, DitherPattern::Checker75));
    assert!(should_dither(1, 0, DitherPattern::Checker75));
}

#[test]
fn paint_brush_dithered_none_paints_all() {
    use crate::ui::sprite_editor::DitherPattern;
    let mut canvas = create_test_canvas(10, 10);
    let color = PixelColor::rgb(255, 0, 0);

    SpritePaintInteraction::paint_brush_dithered(
        &mut canvas,
        IVec2::new(4, 4),
        color,
        3,
        DitherPattern::None,
    );

    // All 9 pixels should be painted
    for y in 3..=5 {
        for x in 3..=5 {
            assert_eq!(canvas.get_pixel(x, y), Some(color), "({x}, {y})");
        }
    }
}

#[test]
fn paint_brush_dithered_checker50() {
    use crate::ui::sprite_editor::DitherPattern;
    let mut canvas = create_test_canvas(10, 10);
    let color = PixelColor::rgb(255, 0, 0);

    SpritePaintInteraction::paint_brush_dithered(
        &mut canvas,
        IVec2::new(4, 4),
        color,
        3,
        DitherPattern::Checker50,
    );

    // Checkerboard within 3x3 area (3,3)→(5,5)
    // (3,3) → 3+3=6 even → painted
    assert_eq!(canvas.get_pixel(3, 3), Some(color));
    // (4,3) → 4+3=7 odd → not painted
    assert_eq!(canvas.get_pixel(4, 3), Some(PixelColor::transparent()));
    // (4,4) → 4+4=8 even → painted
    assert_eq!(canvas.get_pixel(4, 4), Some(color));
}

#[test]
fn paint_brush_dithered_checker50_position_matters() {
    use crate::ui::sprite_editor::DitherPattern;
    let mut canvas = create_test_canvas(10, 10);
    let color = PixelColor::rgb(255, 0, 0);

    // Paint at (4,4) and (5,5) — both use canvas-global coords for dither
    SpritePaintInteraction::paint_brush_dithered(
        &mut canvas,
        IVec2::new(4, 4),
        color,
        1,
        DitherPattern::Checker50,
    );
    SpritePaintInteraction::paint_brush_dithered(
        &mut canvas,
        IVec2::new(5, 5),
        color,
        1,
        DitherPattern::Checker50,
    );

    // (4,4) sum=8 even → painted
    assert_eq!(canvas.get_pixel(4, 4), Some(color));
    // (5,5) sum=10 even → painted
    assert_eq!(canvas.get_pixel(5, 5), Some(color));
}

#[test]
fn paint_brush_dithered_symmetric_horizontal() {
    use crate::ui::sprite_editor::DitherPattern;
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(255, 0, 0);
    let sym = full_canvas_symmetry(16, 16, true, false);

    SpritePaintInteraction::paint_brush_dithered_symmetric(
        &mut canvas,
        IVec2::new(3, 4),
        color,
        1,
        DitherPattern::Checker50,
        &sym,
    );

    // (3,4) sum=7 odd → NOT painted
    assert_eq!(canvas.get_pixel(3, 4), Some(PixelColor::transparent()));
    // Mirrored (12,4) sum=16 even → painted
    assert_eq!(canvas.get_pixel(12, 4), Some(color));
}

#[test]
fn paint_brush_dithered_symmetric_no_symmetry() {
    use crate::ui::sprite_editor::DitherPattern;
    let mut canvas = create_test_canvas(16, 16);
    let color = PixelColor::rgb(0, 255, 0);
    let sym = full_canvas_symmetry(16, 16, false, false);

    SpritePaintInteraction::paint_brush_dithered_symmetric(
        &mut canvas,
        IVec2::new(4, 4),
        color,
        1,
        DitherPattern::None,
        &sym,
    );

    assert_eq!(canvas.get_pixel(4, 4), Some(color));
    assert_eq!(canvas.get_pixel(11, 4), Some(PixelColor::transparent()));
}
