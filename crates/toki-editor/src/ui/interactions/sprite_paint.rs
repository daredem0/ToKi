use crate::ui::editor_ui::{PixelColor, SpriteCanvas};
use crate::ui::sprite_editor::DitherPattern;
use glam::{IVec2, UVec2};

/// Bounds within which symmetry mirroring is computed.
pub struct SymmetryBounds {
    pub origin: UVec2,
    pub size: UVec2,
}

impl SymmetryBounds {
    pub fn mirror_x(&self, pos: IVec2) -> IVec2 {
        let local_x = pos.x - self.origin.x as i32;
        let mirrored_x = (self.size.x as i32 - 1) - local_x + self.origin.x as i32;
        IVec2::new(mirrored_x, pos.y)
    }

    pub fn mirror_y(&self, pos: IVec2) -> IVec2 {
        let local_y = pos.y - self.origin.y as i32;
        let mirrored_y = (self.size.y as i32 - 1) - local_y + self.origin.y as i32;
        IVec2::new(pos.x, mirrored_y)
    }

    /// Returns all positions to paint for the given symmetry flags (deduplicated).
    pub fn mirror_positions(&self, pos: IVec2, horizontal: bool, vertical: bool) -> Vec<IVec2> {
        let mut positions = vec![pos];
        if horizontal {
            let mx = self.mirror_x(pos);
            if !positions.contains(&mx) {
                positions.push(mx);
            }
        }
        if vertical {
            let my = self.mirror_y(pos);
            if !positions.contains(&my) {
                positions.push(my);
            }
        }
        if horizontal && vertical {
            let mxy = self.mirror_y(self.mirror_x(pos));
            if !positions.contains(&mxy) {
                positions.push(mxy);
            }
        }
        positions
    }
}

/// Configuration for symmetric drawing operations.
pub struct SymmetryConfig {
    pub bounds: SymmetryBounds,
    pub horizontal: bool,
    pub vertical: bool,
}

/// Parameters for shape drawing operations (line, rectangle, ellipse).
pub struct ShapeParams {
    pub start: IVec2,
    pub end: IVec2,
    pub color: PixelColor,
    pub brush_size: u32,
    pub filled: bool,
}

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
        let Some(region) = Self::connected_opaque_region_in_bounds(canvas, start_pos, bounds)
        else {
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

    /// Add a simple ground shadow under the clicked connected sprite region, limited to bounds.
    /// The shadow is projected one pixel downward and spread one pixel horizontally.
    /// Only transparent pixels connected to the outside are painted.
    pub fn add_ground_shadow_in_bounds(
        canvas: &mut SpriteCanvas,
        start_pos: IVec2,
        shadow_color: PixelColor,
        bounds: (UVec2, UVec2),
    ) -> bool {
        if start_pos.x < 0 || start_pos.y < 0 {
            return false;
        }
        let start_pos = UVec2::new(start_pos.x as u32, start_pos.y as u32);
        let Some(region) = Self::connected_opaque_region_in_bounds(canvas, start_pos, bounds)
        else {
            return false;
        };

        let (start, end) = bounds;
        let width = (end.x - start.x) as usize;
        let outside = Self::outside_transparent_mask(canvas, bounds, &region);
        let mut bottom_by_col = vec![None; width];

        for y in start.y..end.y {
            for x in start.x..end.x {
                let local_x = (x - start.x) as usize;
                let local_y = (y - start.y) as usize;
                let idx = local_y * width + local_x;
                if region[idx] {
                    bottom_by_col[local_x] = Some(y);
                }
            }
        }

        let mut targets = vec![false; region.len()];
        for (local_x, bottom_y) in bottom_by_col.into_iter().enumerate() {
            let Some(bottom_y) = bottom_y else {
                continue;
            };
            let x = start.x + local_x as u32;
            let shadow_y = bottom_y + 1;
            if shadow_y >= end.y {
                continue;
            }

            for shadow_x in [x.checked_sub(1), Some(x), x.checked_add(1)] {
                let Some(shadow_x) = shadow_x else {
                    continue;
                };
                if shadow_x < start.x || shadow_x >= end.x {
                    continue;
                }
                let target_local_x = (shadow_x - start.x) as usize;
                let target_local_y = (shadow_y - start.y) as usize;
                let target_idx = target_local_y * width + target_local_x;
                if region[target_idx] || !outside[target_idx] || targets[target_idx] {
                    continue;
                }
                let Some(color) = canvas.get_pixel(shadow_x, shadow_y) else {
                    continue;
                };
                if color.a != 0 {
                    continue;
                }
                targets[target_idx] = true;
            }
        }

        let mut changed = false;
        for y in start.y..end.y {
            for x in start.x..end.x {
                let local_x = (x - start.x) as usize;
                let local_y = (y - start.y) as usize;
                let idx = local_y * width + local_x;
                if targets[idx] {
                    changed |= canvas.set_pixel(x, y, shadow_color);
                }
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

    /// Draw a rectangle on the canvas using `ShapeParams`.
    /// Outline mode draws 4 edges using `draw_line`. Filled mode paints all interior pixels.
    pub fn draw_rectangle(canvas: &mut SpriteCanvas, params: &ShapeParams) -> bool {
        let min = IVec2::new(params.start.x.min(params.end.x), params.start.y.min(params.end.y));
        let max = IVec2::new(params.start.x.max(params.end.x), params.start.y.max(params.end.y));
        if params.filled {
            return Self::draw_rectangle_filled(canvas, min, max, params.color, params.brush_size);
        }
        Self::draw_rectangle_outline(canvas, min, max, params.color, params.brush_size)
    }

    fn draw_rectangle_outline(
        canvas: &mut SpriteCanvas,
        min: IVec2,
        max: IVec2,
        color: PixelColor,
        brush_size: u32,
    ) -> bool {
        let tl = min;
        let tr = IVec2::new(max.x, min.y);
        let bl = IVec2::new(min.x, max.y);
        let br = max;
        let mut changed = false;
        changed |= Self::draw_line(canvas, tl, tr, color, brush_size);
        changed |= Self::draw_line(canvas, bl, br, color, brush_size);
        changed |= Self::draw_line(canvas, tl, bl, color, brush_size);
        changed |= Self::draw_line(canvas, tr, br, color, brush_size);
        changed
    }

    fn draw_rectangle_filled(
        canvas: &mut SpriteCanvas,
        min: IVec2,
        max: IVec2,
        color: PixelColor,
        brush_size: u32,
    ) -> bool {
        let mut changed = false;
        for y in min.y..=max.y {
            changed |= Self::draw_line(canvas, IVec2::new(min.x, y), IVec2::new(max.x, y), color, brush_size);
        }
        changed
    }

    /// Draw an ellipse on the canvas using `ShapeParams`.
    /// Outline mode draws boundary pixels. Filled mode fills interior with horizontal spans.
    pub fn draw_ellipse(canvas: &mut SpriteCanvas, params: &ShapeParams) -> bool {
        let min = IVec2::new(params.start.x.min(params.end.x), params.start.y.min(params.end.y));
        let max = IVec2::new(params.start.x.max(params.end.x), params.start.y.max(params.end.y));
        let cx = (min.x + max.x) / 2;
        let cy = (min.y + max.y) / 2;
        let rx = (max.x - min.x) / 2;
        let ry = (max.y - min.y) / 2;

        if rx == 0 && ry == 0 {
            return Self::paint_brush(canvas, IVec2::new(cx, cy), params.color, params.brush_size);
        }
        if rx == 0 {
            return Self::draw_line(canvas, IVec2::new(cx, min.y), IVec2::new(cx, max.y), params.color, params.brush_size);
        }
        if ry == 0 {
            return Self::draw_line(canvas, IVec2::new(min.x, cy), IVec2::new(max.x, cy), params.color, params.brush_size);
        }

        if params.filled {
            draw_ellipse_filled(canvas, cx, cy, rx, ry, params.color, params.brush_size)
        } else {
            draw_ellipse_outline(canvas, cx, cy, rx, ry, params.color, params.brush_size)
        }
    }

    /// Paint with symmetry. Paints at all mirrored positions.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn paint_brush_symmetric(
        canvas: &mut SpriteCanvas,
        center_pos: IVec2,
        color: PixelColor,
        brush_size: u32,
        symmetry: &SymmetryConfig,
    ) -> bool {
        let positions = symmetry.bounds.mirror_positions(
            center_pos, symmetry.horizontal, symmetry.vertical,
        );
        let mut changed = false;
        for pos in positions {
            changed |= Self::paint_brush(canvas, pos, color, brush_size);
        }
        changed
    }

    /// Erase with symmetry. Erases at all mirrored positions.
    pub fn erase_brush_symmetric(
        canvas: &mut SpriteCanvas,
        center_pos: IVec2,
        brush_size: u32,
        symmetry: &SymmetryConfig,
    ) -> bool {
        let positions = symmetry.bounds.mirror_positions(
            center_pos, symmetry.horizontal, symmetry.vertical,
        );
        let mut changed = false;
        for pos in positions {
            changed |= Self::erase_brush(canvas, pos, brush_size);
        }
        changed
    }

    /// Draw a line with symmetry. Draws at all mirrored start/end pairs.
    pub fn draw_line_symmetric(
        canvas: &mut SpriteCanvas,
        params: &ShapeParams,
        symmetry: &SymmetryConfig,
    ) -> bool {
        let starts = symmetry.bounds.mirror_positions(
            params.start, symmetry.horizontal, symmetry.vertical,
        );
        let ends = symmetry.bounds.mirror_positions(
            params.end, symmetry.horizontal, symmetry.vertical,
        );
        let mut changed = false;
        for (s, e) in starts.iter().zip(ends.iter()) {
            changed |= Self::draw_line(canvas, *s, *e, params.color, params.brush_size);
        }
        changed
    }

    /// Draw a rectangle with symmetry.
    pub fn draw_rectangle_symmetric(
        canvas: &mut SpriteCanvas,
        params: &ShapeParams,
        symmetry: &SymmetryConfig,
    ) -> bool {
        let starts = symmetry.bounds.mirror_positions(
            params.start, symmetry.horizontal, symmetry.vertical,
        );
        let ends = symmetry.bounds.mirror_positions(
            params.end, symmetry.horizontal, symmetry.vertical,
        );
        let mut changed = false;
        for (s, e) in starts.iter().zip(ends.iter()) {
            let mirrored = ShapeParams {
                start: *s, end: *e, color: params.color,
                brush_size: params.brush_size, filled: params.filled,
            };
            changed |= Self::draw_rectangle(canvas, &mirrored);
        }
        changed
    }

    /// Draw an ellipse with symmetry.
    pub fn draw_ellipse_symmetric(
        canvas: &mut SpriteCanvas,
        params: &ShapeParams,
        symmetry: &SymmetryConfig,
    ) -> bool {
        let starts = symmetry.bounds.mirror_positions(
            params.start, symmetry.horizontal, symmetry.vertical,
        );
        let ends = symmetry.bounds.mirror_positions(
            params.end, symmetry.horizontal, symmetry.vertical,
        );
        let mut changed = false;
        for (s, e) in starts.iter().zip(ends.iter()) {
            let mirrored = ShapeParams {
                start: *s, end: *e, color: params.color,
                brush_size: params.brush_size, filled: params.filled,
            };
            changed |= Self::draw_ellipse(canvas, &mirrored);
        }
        changed
    }

    /// Paint with dithering. Skips pixels where the dither pattern says no.
    pub fn paint_brush_dithered(
        canvas: &mut SpriteCanvas,
        center_pos: IVec2,
        color: PixelColor,
        brush_size: u32,
        pattern: DitherPattern,
    ) -> bool {
        let Some((start, end)) = Self::brush_footprint_bounds(canvas, center_pos, brush_size)
        else {
            return false;
        };
        let mut changed = false;
        for y in start.y..end.y {
            for x in start.x..end.x {
                if should_dither(x, y, pattern) {
                    changed |= canvas.set_pixel(x, y, color);
                }
            }
        }
        changed
    }

    /// Paint with dithering and symmetry combined.
    pub fn paint_brush_dithered_symmetric(
        canvas: &mut SpriteCanvas,
        center_pos: IVec2,
        color: PixelColor,
        brush_size: u32,
        pattern: DitherPattern,
        symmetry: &SymmetryConfig,
    ) -> bool {
        let positions = symmetry.bounds.mirror_positions(
            center_pos, symmetry.horizontal, symmetry.vertical,
        );
        let mut changed = false;
        for pos in positions {
            changed |= Self::paint_brush_dithered(canvas, pos, color, brush_size, pattern);
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

/// Draw ellipse outline using the midpoint algorithm.
fn draw_ellipse_outline(
    canvas: &mut SpriteCanvas,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: PixelColor,
    brush_size: u32,
) -> bool {
    let mut changed = false;
    for p in midpoint_ellipse_points(rx, ry) {
        changed |= SpritePaintInteraction::paint_brush(
            canvas,
            IVec2::new(cx + p.x, cy + p.y),
            color,
            brush_size,
        );
    }
    changed
}

/// Draw filled ellipse using horizontal spans between boundary points.
fn draw_ellipse_filled(
    canvas: &mut SpriteCanvas,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: PixelColor,
    brush_size: u32,
) -> bool {
    let mut changed = false;
    let spans = ellipse_horizontal_spans(rx, ry);
    for (dy, x_extent) in spans {
        changed |= SpritePaintInteraction::draw_line(
            canvas,
            IVec2::new(cx - x_extent, cy + dy),
            IVec2::new(cx + x_extent, cy + dy),
            color,
            brush_size,
        );
        if dy != 0 {
            changed |= SpritePaintInteraction::draw_line(
                canvas,
                IVec2::new(cx - x_extent, cy - dy),
                IVec2::new(cx + x_extent, cy - dy),
                color,
                brush_size,
            );
        }
    }
    changed
}

/// Compute boundary points for an ellipse using the midpoint algorithm.
/// Returns offsets relative to center (all 4 quadrants).
fn midpoint_ellipse_points(rx: i32, ry: i32) -> Vec<IVec2> {
    let mut points = Vec::new();
    let (rx2, ry2) = (rx as i64 * rx as i64, ry as i64 * ry as i64);
    let (mut x, mut y) = (0i32, ry);

    // Region 1: slope < 1
    let mut d1 = ry2 - rx2 * ry as i64 + rx2 / 4;
    while 2 * ry2 * x as i64 <= 2 * rx2 * y as i64 {
        push_four_quadrants(&mut points, x, y);
        x += 1;
        if d1 < 0 {
            d1 += 2 * ry2 * x as i64 + ry2;
        } else {
            y -= 1;
            d1 += 2 * ry2 * x as i64 - 2 * rx2 * y as i64 + ry2;
        }
    }

    // Region 2: slope >= 1
    let mut d2 = ry2 * (x as i64 * 2 + 1).pow(2) / 4 + rx2 * (y as i64 - 1).pow(2) - rx2 * ry2;
    while y >= 0 {
        push_four_quadrants(&mut points, x, y);
        y -= 1;
        if d2 > 0 {
            d2 += rx2 - 2 * rx2 * y as i64;
        } else {
            x += 1;
            d2 += 2 * ry2 * x as i64 - 2 * rx2 * y as i64 + rx2;
        }
    }
    points
}

fn push_four_quadrants(points: &mut Vec<IVec2>, x: i32, y: i32) {
    points.push(IVec2::new(x, y));
    if x != 0 {
        points.push(IVec2::new(-x, y));
    }
    if y != 0 {
        points.push(IVec2::new(x, -y));
    }
    if x != 0 && y != 0 {
        points.push(IVec2::new(-x, -y));
    }
}

/// Compute horizontal spans for a filled ellipse.
/// Returns (dy, x_extent) pairs where dy >= 0, and the span goes from -x_extent to +x_extent.
fn ellipse_horizontal_spans(rx: i32, ry: i32) -> Vec<(i32, i32)> {
    let mut max_x_for_y = vec![0i32; (ry + 1) as usize];
    for point in midpoint_ellipse_points(rx, ry) {
        let ay = point.y.unsigned_abs() as usize;
        if ay < max_x_for_y.len() {
            max_x_for_y[ay] = max_x_for_y[ay].max(point.x.abs());
        }
    }
    max_x_for_y.into_iter().enumerate().map(|(dy, x)| (dy as i32, x)).collect()
}

/// Check if a pixel should be painted for the given dither pattern.
/// Uses canvas-global coordinates so patterns are consistent across strokes.
pub fn should_dither(x: u32, y: u32, pattern: DitherPattern) -> bool {
    match pattern {
        DitherPattern::None => true,
        DitherPattern::Checker50 => (x + y).is_multiple_of(2),
        DitherPattern::Checker25 => x.is_multiple_of(2) && y.is_multiple_of(2),
        DitherPattern::Checker75 => x.is_multiple_of(2) || y.is_multiple_of(2),
    }
}

#[cfg(test)]
#[path = "sprite_paint_tests.rs"]
mod tests;
