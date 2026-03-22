//! Cell operations and canvas transforms for SpriteEditorState.

use super::{PixelColor, ResizeAnchor, SpriteCanvas, SpriteEditCommand, SpriteEditorState};

impl SpriteEditorState {
    /// Clear the selected cell to transparent pixels
    pub fn clear_selected_cell(&mut self) -> bool {
        let cell_idx = match self.active().selected_cell {
            Some(idx) => idx,
            None => return false,
        };

        let Some((start_x, start_y, end_x, end_y)) = self.cell_bounds(cell_idx) else {
            return false;
        };

        let cs = self.active_mut();
        let Some(canvas) = &mut cs.canvas else {
            return false;
        };

        let before = canvas.clone();
        canvas.fill_rect(
            start_x,
            start_y,
            end_x - start_x,
            end_y - start_y,
            PixelColor::transparent(),
        );

        cs.history.push(SpriteEditCommand {
            before,
            after: canvas.clone(),
        });
        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Swap the contents of two cells
    #[allow(dead_code)]
    pub fn swap_cells(&mut self, cell_a: usize, cell_b: usize) -> bool {
        if cell_a == cell_b {
            return false;
        }

        let Some((a_start_x, a_start_y, a_end_x, a_end_y)) = self.cell_bounds(cell_a) else {
            return false;
        };
        let Some((b_start_x, b_start_y, b_end_x, b_end_y)) = self.cell_bounds(cell_b) else {
            return false;
        };

        let cs = self.active_mut();
        let Some(canvas) = &mut cs.canvas else {
            return false;
        };

        let width = a_end_x - a_start_x;
        let height = a_end_y - a_start_y;
        let before = canvas.clone();

        // Read pixels from both cells
        let pixels_a = read_cell_pixels(canvas, a_start_x, a_start_y, a_end_x, a_end_y);
        let pixels_b = read_cell_pixels(canvas, b_start_x, b_start_y, b_end_x, b_end_y);

        // Write swapped pixels
        write_cell_pixels(canvas, a_start_x, a_start_y, width, height, &pixels_b);
        write_cell_pixels(canvas, b_start_x, b_start_y, width, height, &pixels_a);

        cs.history.push(SpriteEditCommand {
            before,
            after: canvas.clone(),
        });
        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Append a new row of empty cells to the bottom of the sheet
    #[allow(dead_code)]
    pub fn append_row(&mut self) -> bool {
        if !self.is_sheet() {
            return false;
        }

        let cs = self.active_mut();
        let Some(canvas) = &cs.canvas else {
            return false;
        };

        let old_width = canvas.width;
        let old_height = canvas.height;
        let new_height = old_height + cs.cell_size.y;
        let before = canvas.clone();

        let mut new_canvas = SpriteCanvas::new(old_width, new_height);
        copy_canvas_pixels(&before, &mut new_canvas, old_width, old_height);

        cs.canvas = Some(new_canvas.clone());
        cs.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Append a new column of empty cells to the right of the sheet
    #[allow(dead_code)]
    pub fn append_column(&mut self) -> bool {
        if !self.is_sheet() {
            return false;
        }

        let cs = self.active_mut();
        let Some(canvas) = &cs.canvas else {
            return false;
        };

        let old_width = canvas.width;
        let old_height = canvas.height;
        let new_width = old_width + cs.cell_size.x;
        let before = canvas.clone();

        let mut new_canvas = SpriteCanvas::new(new_width, old_height);
        copy_canvas_pixels(&before, &mut new_canvas, old_width, old_height);

        cs.canvas = Some(new_canvas.clone());
        cs.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Delete the selected cell and collapse remaining cells to fill the gap
    #[allow(dead_code)]
    pub fn delete_cell_with_collapse(&mut self) -> bool {
        let cell_idx = match self.active().selected_cell {
            Some(idx) => idx,
            None => return false,
        };

        let Some((cols, rows)) = self.sheet_cell_count() else {
            return false;
        };

        let total_cells = (cols * rows) as usize;
        if cell_idx >= total_cells {
            return false;
        }

        let cs = self.active_mut();
        let Some(canvas) = &mut cs.canvas else {
            return false;
        };

        let before = canvas.clone();
        let cell_w = cs.cell_size.x;
        let cell_h = cs.cell_size.y;

        // Shift all cells after the deleted one
        for i in cell_idx..(total_cells - 1) {
            let src_col = ((i + 1) as u32) % cols;
            let src_row = ((i + 1) as u32) / cols;
            let dst_col = (i as u32) % cols;
            let dst_row = (i as u32) / cols;

            for py in 0..cell_h {
                for px in 0..cell_w {
                    let src_x = src_col * cell_w + px;
                    let src_y = src_row * cell_h + py;
                    let dst_x = dst_col * cell_w + px;
                    let dst_y = dst_row * cell_h + py;

                    let color = canvas
                        .get_pixel(src_x, src_y)
                        .unwrap_or(PixelColor::transparent());
                    canvas.set_pixel(dst_x, dst_y, color);
                }
            }
        }

        // Clear the last cell
        let last_idx = total_cells - 1;
        let last_col = (last_idx as u32) % cols;
        let last_row = (last_idx as u32) / cols;
        canvas.fill_rect(
            last_col * cell_w,
            last_row * cell_h,
            cell_w,
            cell_h,
            PixelColor::transparent(),
        );

        cs.history.push(SpriteEditCommand {
            before,
            after: canvas.clone(),
        });

        if cell_idx >= total_cells - 1 {
            cs.selected_cell = if total_cells > 1 {
                Some(total_cells - 2)
            } else {
                None
            };
        }

        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Flip the entire canvas horizontally
    #[allow(dead_code)]
    pub fn flip_horizontal(&mut self) -> bool {
        let cs = self.active_mut();
        let Some(canvas) = &cs.canvas else {
            return false;
        };

        let before = canvas.clone();
        let w = canvas.width;
        let h = canvas.height;

        let mut new_canvas = SpriteCanvas::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(w - 1 - x, y, color);
                }
            }
        }

        cs.canvas = Some(new_canvas.clone());
        cs.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Flip the entire canvas vertically
    #[allow(dead_code)]
    pub fn flip_vertical(&mut self) -> bool {
        let cs = self.active_mut();
        let Some(canvas) = &cs.canvas else {
            return false;
        };

        let before = canvas.clone();
        let w = canvas.width;
        let h = canvas.height;

        let mut new_canvas = SpriteCanvas::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(x, h - 1 - y, color);
                }
            }
        }

        cs.canvas = Some(new_canvas.clone());
        cs.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Rotate the entire canvas 90° clockwise
    #[allow(dead_code)]
    pub fn rotate_clockwise(&mut self) -> bool {
        let is_sheet = self.is_sheet();
        let cs = self.active_mut();
        let Some(canvas) = &cs.canvas else {
            return false;
        };

        let before = canvas.clone();
        let old_w = canvas.width;
        let old_h = canvas.height;

        let mut new_canvas = SpriteCanvas::new(old_h, old_w);
        for y in 0..old_h {
            for x in 0..old_w {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(old_h - 1 - y, x, color);
                }
            }
        }

        cs.canvas = Some(new_canvas.clone());
        cs.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });

        if is_sheet {
            std::mem::swap(&mut cs.cell_size.x, &mut cs.cell_size.y);
        }

        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Rotate the entire canvas 90° counter-clockwise
    #[allow(dead_code)]
    pub fn rotate_counter_clockwise(&mut self) -> bool {
        let is_sheet = self.is_sheet();
        let cs = self.active_mut();
        let Some(canvas) = &cs.canvas else {
            return false;
        };

        let before = canvas.clone();
        let old_w = canvas.width;
        let old_h = canvas.height;

        let mut new_canvas = SpriteCanvas::new(old_h, old_w);
        for y in 0..old_h {
            for x in 0..old_w {
                if let Some(color) = before.get_pixel(x, y) {
                    new_canvas.set_pixel(y, old_w - 1 - x, color);
                }
            }
        }

        cs.canvas = Some(new_canvas.clone());
        cs.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });

        if is_sheet {
            std::mem::swap(&mut cs.cell_size.x, &mut cs.cell_size.y);
        }

        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Resize the canvas with anchor positioning
    pub fn resize_canvas(&mut self, new_w: u32, new_h: u32, anchor: ResizeAnchor) -> bool {
        if new_w == 0 || new_h == 0 {
            return false;
        }

        let cs = self.active_mut();
        let Some(canvas) = &cs.canvas else {
            return false;
        };

        if new_w == canvas.width && new_h == canvas.height {
            return false;
        }

        let before = canvas.clone();
        let old_w = canvas.width;
        let old_h = canvas.height;

        let mut new_canvas = SpriteCanvas::new(new_w, new_h);

        let (offset_x, offset_y) = anchor.calculate_offset(old_w, old_h, new_w, new_h);

        for y in 0..old_h {
            for x in 0..old_w {
                let new_x = x as i32 + offset_x;
                let new_y = y as i32 + offset_y;
                if new_x >= 0 && new_x < new_w as i32 && new_y >= 0 && new_y < new_h as i32 {
                    if let Some(color) = before.get_pixel(x, y) {
                        new_canvas.set_pixel(new_x as u32, new_y as u32, color);
                    }
                }
            }
        }

        cs.canvas = Some(new_canvas.clone());
        cs.history.push(SpriteEditCommand {
            before,
            after: new_canvas,
        });
        cs.dirty = true;
        cs.canvas_texture = None;
        true
    }

    /// Begin the resize dialog
    pub fn begin_resize_dialog(&mut self) {
        let (w, h) = self.canvas_dimensions().unwrap_or((16, 16));
        let cs = self.active();
        let cell_w = cs.cell_size.x.max(1);
        let cell_h = cs.cell_size.y.max(1);
        self.resize_tiles_x = w.div_ceil(cell_w);
        self.resize_tiles_y = h.div_ceil(cell_h);
        self.resize_anchor = ResizeAnchor::default();
        self.show_resize_dialog = true;
    }
}

// Helper functions

fn read_cell_pixels(
    canvas: &SpriteCanvas,
    start_x: u32,
    start_y: u32,
    end_x: u32,
    end_y: u32,
) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(((end_x - start_x) * (end_y - start_y) * 4) as usize);
    for y in start_y..end_y {
        for x in start_x..end_x {
            if let Some(color) = canvas.get_pixel(x, y) {
                pixels.extend_from_slice(&color.to_rgba_array());
            }
        }
    }
    pixels
}

fn write_cell_pixels(
    canvas: &mut SpriteCanvas,
    start_x: u32,
    start_y: u32,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    let mut i = 0;
    for y in start_y..(start_y + height) {
        for x in start_x..(start_x + width) {
            let color = PixelColor::from_rgba_array([
                pixels[i],
                pixels[i + 1],
                pixels[i + 2],
                pixels[i + 3],
            ]);
            canvas.set_pixel(x, y, color);
            i += 4;
        }
    }
}

fn copy_canvas_pixels(src: &SpriteCanvas, dst: &mut SpriteCanvas, width: u32, height: u32) {
    for y in 0..height {
        for x in 0..width {
            if let Some(color) = src.get_pixel(x, y) {
                dst.set_pixel(x, y, color);
            }
        }
    }
}
