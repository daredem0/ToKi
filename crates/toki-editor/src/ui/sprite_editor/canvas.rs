//! In-memory canvas for pixel editing.

use super::types::PixelColor;

/// In-memory canvas for pixel editing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteCanvas {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel data in RGBA format (row-major, top-to-bottom)
    pixels: Vec<u8>,
}

#[allow(dead_code)]
impl SpriteCanvas {
    /// Create a new canvas filled with transparent pixels
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            pixels: vec![0; pixel_count * 4],
        }
    }

    /// Create a new canvas filled with a specific color
    pub fn filled(width: u32, height: u32, color: PixelColor) -> Self {
        let pixel_count = (width * height) as usize;
        let mut pixels = Vec::with_capacity(pixel_count * 4);
        let rgba = color.to_rgba_array();
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&rgba);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    /// Create a canvas from RGBA pixel data
    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected_len = (width * height * 4) as usize;
        if pixels.len() != expected_len {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }

    /// Get pixel color at position, returns None if out of bounds
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<PixelColor> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        Some(PixelColor::from_rgba_array([
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ]))
    }

    /// Set pixel color at position, returns false if out of bounds
    pub fn set_pixel(&mut self, x: u32, y: u32, color: PixelColor) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        let rgba = color.to_rgba_array();
        self.pixels[idx..idx + 4].copy_from_slice(&rgba);
        true
    }

    /// Get raw RGBA pixel data
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Get mutable raw RGBA pixel data
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Fill a rectangle with a color
    pub fn fill_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: PixelColor) {
        let x_end = (x + w).min(self.width);
        let y_end = (y + h).min(self.height);
        for py in y..y_end {
            for px in x..x_end {
                self.set_pixel(px, py, color);
            }
        }
    }

    /// Clear entire canvas to transparent
    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Clear entire canvas to a specific color
    pub fn clear_to_color(&mut self, color: PixelColor) {
        let rgba = color.to_rgba_array();
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&rgba);
        }
    }

    /// Extract a rectangular region as a new canvas
    pub fn extract_region(&self, x: u32, y: u32, width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let mut result = Self::new(width, height);
        for dy in 0..height {
            for dx in 0..width {
                let src_x = x + dx;
                let src_y = y + dy;
                if let Some(color) = self.get_pixel(src_x, src_y) {
                    result.set_pixel(dx, dy, color);
                }
            }
        }
        Some(result)
    }

    /// Blit (copy) another canvas onto this one at the specified position
    pub fn blit(&mut self, source: &Self, x: i32, y: i32) {
        for sy in 0..source.height {
            for sx in 0..source.width {
                let dest_x = x + sx as i32;
                let dest_y = y + sy as i32;
                if dest_x >= 0 && dest_y >= 0 {
                    if let Some(color) = source.get_pixel(sx, sy) {
                        // Only blit non-transparent pixels (alpha > 0)
                        if color.a > 0 {
                            self.set_pixel(dest_x as u32, dest_y as u32, color);
                        }
                    }
                }
            }
        }
    }

    /// Scale the canvas to fit within max dimensions while maintaining aspect ratio.
    /// Uses nearest-neighbor sampling for pixel art.
    pub fn scaled_to_fit(&self, max_width: u32, max_height: u32) -> Self {
        if max_width == 0 || max_height == 0 {
            return self.clone();
        }

        // Calculate scale factor to fit within bounds
        let scale_x = max_width as f32 / self.width as f32;
        let scale_y = max_height as f32 / self.height as f32;
        let scale = scale_x.min(scale_y).min(1.0); // Don't upscale, only downscale

        if scale >= 1.0 {
            return self.clone(); // No scaling needed
        }

        let new_width = ((self.width as f32 * scale).round() as u32).max(1);
        let new_height = ((self.height as f32 * scale).round() as u32).max(1);

        let mut result = Self::new(new_width, new_height);

        // Nearest-neighbor downscaling
        for dy in 0..new_height {
            for dx in 0..new_width {
                let src_x = ((dx as f32 + 0.5) / scale).floor() as u32;
                let src_y = ((dy as f32 + 0.5) / scale).floor() as u32;
                if let Some(color) =
                    self.get_pixel(src_x.min(self.width - 1), src_y.min(self.height - 1))
                {
                    result.set_pixel(dx, dy, color);
                }
            }
        }

        result
    }

    /// Find all non-transparent pixels connected to the given starting point.
    /// Uses 8-connectivity (includes diagonals) for better sprite selection.
    /// Returns the bounding box (x, y, width, height) of all connected pixels,
    /// or None if the starting pixel is transparent or out of bounds.
    pub fn find_connected_sprite(&self, start_x: u32, start_y: u32) -> Option<(u32, u32, u32, u32)> {
        // Check bounds
        if start_x >= self.width || start_y >= self.height {
            return None;
        }

        // Check if starting pixel is non-transparent
        let start_color = self.get_pixel(start_x, start_y)?;
        if start_color.a == 0 {
            return None;
        }

        // Track visited pixels and bounding box
        let mut visited = vec![false; (self.width * self.height) as usize];
        let mut min_x = start_x;
        let mut max_x = start_x;
        let mut min_y = start_y;
        let mut max_y = start_y;

        // Flood fill using a stack (8-connectivity)
        let mut stack = vec![(start_x, start_y)];

        while let Some((x, y)) = stack.pop() {
            let idx = (y * self.width + x) as usize;
            if visited[idx] {
                continue;
            }
            visited[idx] = true;

            // Check if this pixel is non-transparent
            if let Some(color) = self.get_pixel(x, y) {
                if color.a == 0 {
                    continue;
                }

                // Update bounding box
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);

                // Add 8 neighbors to stack
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx >= 0 && ny >= 0 {
                            let nx = nx as u32;
                            let ny = ny as u32;
                            if nx < self.width && ny < self.height {
                                let nidx = (ny * self.width + nx) as usize;
                                if !visited[nidx] {
                                    stack.push((nx, ny));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Return bounding box
        let width = max_x - min_x + 1;
        let height = max_y - min_y + 1;
        Some((min_x, min_y, width, height))
    }
}
