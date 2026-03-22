use crate::ui::editor_ui::{PixelColor, SpriteCanvas};
use glam::{IVec2, UVec2};

pub struct SpritePaintInteraction;

impl SpritePaintInteraction {
    fn contains_pos(bounds: (UVec2, UVec2), pos: UVec2) -> bool {
        let (start, end) = bounds;
        pos.x >= start.x && pos.y >= start.y && pos.x < end.x && pos.y < end.y
    }

    fn flood_replace_in_bounds(
        canvas: &mut SpriteCanvas,
        start_pos: UVec2,
        target_color: PixelColor,
        replacement_color: PixelColor,
        bounds: (UVec2, UVec2),
    ) -> bool {
        if !Self::contains_pos(bounds, start_pos) {
            return false;
        }

        if target_color == replacement_color {
            return false;
        }

        let (start, end) = bounds;
        let mut stack = vec![(start_pos.x, start_pos.y)];
        let mut changed = false;

        while let Some((x, y)) = stack.pop() {
            if x < start.x || y < start.y || x >= end.x || y >= end.y {
                continue;
            }

            let Some(current_color) = canvas.get_pixel(x, y) else {
                continue;
            };

            if current_color != target_color {
                continue;
            }

            canvas.set_pixel(x, y, replacement_color);
            changed = true;

            if x > start.x {
                stack.push((x - 1, y));
            }
            if x + 1 < end.x {
                stack.push((x + 1, y));
            }
            if y > start.y {
                stack.push((x, y - 1));
            }
            if y + 1 < end.y {
                stack.push((x, y + 1));
            }
        }

        changed
    }

    fn connected_opaque_region_in_bounds(
        canvas: &SpriteCanvas,
        start_pos: UVec2,
        bounds: (UVec2, UVec2),
    ) -> Option<Vec<bool>> {
        if !Self::contains_pos(bounds, start_pos) {
            return None;
        }
        let start_color = canvas.get_pixel(start_pos.x, start_pos.y)?;
        if start_color.a == 0 {
            return None;
        }

        let (start, end) = bounds;
        let width = (end.x - start.x) as usize;
        let height = (end.y - start.y) as usize;
        let mut region = vec![false; width * height];
        let mut stack = vec![(start_pos.x, start_pos.y)];

        while let Some((x, y)) = stack.pop() {
            if x < start.x || y < start.y || x >= end.x || y >= end.y {
                continue;
            }

            let local_x = (x - start.x) as usize;
            let local_y = (y - start.y) as usize;
            let idx = local_y * width + local_x;
            if region[idx] {
                continue;
            }

            let Some(color) = canvas.get_pixel(x, y) else {
                continue;
            };
            if color.a == 0 {
                continue;
            }

            region[idx] = true;

            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= start.x as i32
                        && ny >= start.y as i32
                        && nx < end.x as i32
                        && ny < end.y as i32
                    {
                        stack.push((nx as u32, ny as u32));
                    }
                }
            }
        }

        Some(region)
    }

    fn outside_transparent_mask(
        canvas: &SpriteCanvas,
        bounds: (UVec2, UVec2),
        opaque_region: &[bool],
    ) -> Vec<bool> {
        let (start, end) = bounds;
        let width = (end.x - start.x) as usize;
        let height = (end.y - start.y) as usize;
        let mut outside = vec![false; width * height];
        let mut stack = Vec::new();

        let try_push = |x: u32, y: u32, outside: &mut Vec<bool>, stack: &mut Vec<(u32, u32)>| {
            let local_x = (x - start.x) as usize;
            let local_y = (y - start.y) as usize;
            let idx = local_y * width + local_x;
            if outside[idx] || opaque_region[idx] {
                return;
            }
            let Some(color) = canvas.get_pixel(x, y) else {
                return;
            };
            if color.a != 0 {
                return;
            }
            outside[idx] = true;
            stack.push((x, y));
        };

        for x in start.x..end.x {
            try_push(x, start.y, &mut outside, &mut stack);
            if end.y > start.y + 1 {
                try_push(x, end.y - 1, &mut outside, &mut stack);
            }
        }
        for y in start.y..end.y {
            try_push(start.x, y, &mut outside, &mut stack);
            if end.x > start.x + 1 {
                try_push(end.x - 1, y, &mut outside, &mut stack);
            }
        }

        while let Some((x, y)) = stack.pop() {
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < start.x as i32
                        || ny < start.y as i32
                        || nx >= end.x as i32
                        || ny >= end.y as i32
                    {
                        continue;
                    }
                    let nx = nx as u32;
                    let ny = ny as u32;
                    let local_x = (nx - start.x) as usize;
                    let local_y = (ny - start.y) as usize;
                    let idx = local_y * width + local_x;
                    if outside[idx] || opaque_region[idx] {
                        continue;
                    }
                    let Some(color) = canvas.get_pixel(nx, ny) else {
                        continue;
                    };
                    if color.a != 0 {
                        continue;
                    }
                    outside[idx] = true;
                    stack.push((nx, ny));
                }
            }
        }

        outside
    }

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
    #[cfg_attr(not(test), allow(dead_code))]
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
        Self::flood_replace_in_bounds(
            canvas,
            UVec2::new(start_x, start_y),
            target_color,
            fill_color,
            (UVec2::ZERO, UVec2::new(canvas.width, canvas.height)),
        )
    }

    /// Remove the 4-connected region of the clicked color, limited to the provided bounds.
    /// Intended for tile-local background cleanup in sprite sheets.
    pub fn erase_connected_color_in_bounds(
        canvas: &mut SpriteCanvas,
        start_pos: IVec2,
        bounds: (UVec2, UVec2),
    ) -> bool {
        if start_pos.x < 0 || start_pos.y < 0 {
            return false;
        }
        let start_pos = UVec2::new(start_pos.x as u32, start_pos.y as u32);
        if !Self::contains_pos(bounds, start_pos) {
            return false;
        }

        let Some(target_color) = canvas.get_pixel(start_pos.x, start_pos.y) else {
            return false;
        };
        if target_color.a == 0 {
            return false;
        }

        Self::flood_replace_in_bounds(
            canvas,
            start_pos,
            target_color,
            PixelColor::transparent(),
            bounds,
        )
    }

    /// Add an outline around the clicked connected sprite region, limited to the provided bounds.
    /// Only transparent pixels connected to the outside of the bounds are outlined.
    pub fn add_outline_in_bounds(
        canvas: &mut SpriteCanvas,
        start_pos: IVec2,
        outline_color: PixelColor,
        bounds: (UVec2, UVec2),
    ) -> bool {
        if start_pos.x < 0 || start_pos.y < 0 {
            return false;
        }
        let start_pos = UVec2::new(start_pos.x as u32, start_pos.y as u32);
        let Some(region) = Self::connected_opaque_region_in_bounds(canvas, start_pos, bounds) else {
            return false;
        };

        let (start, end) = bounds;
        let width = (end.x - start.x) as usize;
        let outside = Self::outside_transparent_mask(canvas, bounds, &region);
        let mut outline_pixels = Vec::new();

        for y in start.y..end.y {
            for x in start.x..end.x {
                let local_x = (x - start.x) as usize;
                let local_y = (y - start.y) as usize;
                let idx = local_y * width + local_x;
                if !region[idx] {
                    continue;
                }

                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < start.x as i32
                            || ny < start.y as i32
                            || nx >= end.x as i32
                            || ny >= end.y as i32
                        {
                            continue;
                        }
                        let nx = nx as u32;
                        let ny = ny as u32;
                        let nlocal_x = (nx - start.x) as usize;
                        let nlocal_y = (ny - start.y) as usize;
                        let nidx = nlocal_y * width + nlocal_x;
                        if region[nidx] || !outside[nidx] {
                            continue;
                        }
                        outline_pixels.push((nx, ny));
                    }
                }
            }
        }

        let mut changed = false;
        for (x, y) in outline_pixels {
            changed |= canvas.set_pixel(x, y, outline_color);
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
