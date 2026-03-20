use crate::ui::editor_ui::{PixelColor, SpriteCanvas};
use glam::{IVec2, UVec2};

pub struct SpritePaintInteraction;

#[allow(dead_code)]
impl SpritePaintInteraction {
    /// Calculate brush footprint bounds for a given center pixel position.
    /// Returns (start, end) where end is exclusive.
    pub fn brush_footprint_bounds(
        canvas: &SpriteCanvas,
        center_pos: IVec2,
        brush_size: u32,
    ) -> Option<(UVec2, UVec2)> {
        if center_pos.x < 0
            || center_pos.y < 0
            || center_pos.x >= canvas.width as i32
            || center_pos.y >= canvas.height as i32
        {
            return None;
        }

        let brush_size = brush_size.max(1);
        let radius = (brush_size - 1) / 2;
        let start_x = (center_pos.x as u32).saturating_sub(radius);
        let start_y = (center_pos.y as u32).saturating_sub(radius);
        let end_x = (start_x + brush_size).min(canvas.width);
        let end_y = (start_y + brush_size).min(canvas.height);
        Some((UVec2::new(start_x, start_y), UVec2::new(end_x, end_y)))
    }

    /// Paint a single pixel with a color.
    pub fn paint_pixel(canvas: &mut SpriteCanvas, pos: IVec2, color: PixelColor) -> bool {
        if pos.x < 0 || pos.y < 0 {
            return false;
        }
        canvas.set_pixel(pos.x as u32, pos.y as u32, color)
    }

    /// Paint with a brush at the given center position.
    pub fn paint_brush(
        canvas: &mut SpriteCanvas,
        center_pos: IVec2,
        color: PixelColor,
        brush_size: u32,
    ) -> bool {
        let Some((start, end)) = Self::brush_footprint_bounds(canvas, center_pos, brush_size)
        else {
            return false;
        };

        let mut changed = false;
        for y in start.y..end.y {
            for x in start.x..end.x {
                changed |= canvas.set_pixel(x, y, color);
            }
        }
        changed
    }

    /// Erase (set to transparent) with a brush at the given center position.
    pub fn erase_brush(canvas: &mut SpriteCanvas, center_pos: IVec2, brush_size: u32) -> bool {
        Self::paint_brush(canvas, center_pos, PixelColor::transparent(), brush_size)
    }

    /// Flood fill starting from a position with a new color.
    /// Uses 4-way connectivity (up, down, left, right).
    pub fn flood_fill(canvas: &mut SpriteCanvas, start_pos: IVec2, fill_color: PixelColor) -> bool {
        if start_pos.x < 0 || start_pos.y < 0 {
            return false;
        }
        let start_x = start_pos.x as u32;
        let start_y = start_pos.y as u32;

        let Some(target_color) = canvas.get_pixel(start_x, start_y) else {
            return false;
        };

        // Don't fill if already the same color
        if target_color == fill_color {
            return false;
        }

        let mut stack = vec![(start_x, start_y)];
        let mut changed = false;

        while let Some((x, y)) = stack.pop() {
            let Some(current_color) = canvas.get_pixel(x, y) else {
                continue;
            };

            if current_color != target_color {
                continue;
            }

            canvas.set_pixel(x, y, fill_color);
            changed = true;

            // Add neighbors (4-way connectivity)
            if x > 0 {
                stack.push((x - 1, y));
            }
            if x + 1 < canvas.width {
                stack.push((x + 1, y));
            }
            if y > 0 {
                stack.push((x, y - 1));
            }
            if y + 1 < canvas.height {
                stack.push((x, y + 1));
            }
        }

        changed
    }

    /// Draw a line between two points using Bresenham's algorithm.
    pub fn draw_line(
        canvas: &mut SpriteCanvas,
        start: IVec2,
        end: IVec2,
        color: PixelColor,
        brush_size: u32,
    ) -> bool {
        let mut changed = false;

        let dx = (end.x - start.x).abs();
        let dy = -(end.y - start.y).abs();
        let sx = if start.x < end.x { 1 } else { -1 };
        let sy = if start.y < end.y { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = start.x;
        let mut y = start.y;

        loop {
            changed |= Self::paint_brush(canvas, IVec2::new(x, y), color, brush_size);

            if x == end.x && y == end.y {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                if x == end.x {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == end.y {
                    break;
                }
                err += dx;
                y += sy;
            }
        }

        changed
    }

    /// Pick color from canvas at the given position.
    pub fn pick_color(canvas: &SpriteCanvas, pos: IVec2) -> Option<PixelColor> {
        if pos.x < 0 || pos.y < 0 {
            return None;
        }
        canvas.get_pixel(pos.x as u32, pos.y as u32)
    }
}

#[cfg(test)]
#[path = "sprite_paint_tests.rs"]
mod tests;
