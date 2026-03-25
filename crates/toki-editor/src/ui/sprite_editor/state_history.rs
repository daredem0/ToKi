//! Undo/redo and clipboard operations for SpriteEditorState.

use super::{
    canonical_indexed_color, nearest_palette_slot, preview_indexed_color, CanvasSide, SpriteCanvas,
    SpriteEditCommand, SpriteEditorState,
};
use toki_core::palette::Palette4;

impl SpriteEditorState {
    /// Push current canvas state for undo
    pub fn push_undo_state(&mut self, before: SpriteCanvas) {
        let cs = self.active_mut();
        if let Some(canvas) = &cs.canvas {
            if *canvas == before {
                return;
            }
            cs.history.push(SpriteEditCommand {
                before,
                after: canvas.clone(),
            });
        }
    }

    /// Perform undo on the active canvas
    pub fn undo(&mut self) -> bool {
        let cs = self.active_mut();
        if let Some(before) = cs.history.take_undo() {
            cs.canvas = Some(before);
            cs.canvas_texture = None;
            true
        } else {
            false
        }
    }

    /// Perform redo on the active canvas
    pub fn redo(&mut self) -> bool {
        let cs = self.active_mut();
        if let Some(after) = cs.history.take_redo() {
            cs.canvas = Some(after);
            cs.canvas_texture = None;
            true
        } else {
            false
        }
    }

    /// Copy the selected region from the active canvas to the clipboard
    pub fn copy_selection(&mut self) -> bool {
        let cs = self.active();
        let Some(selection) = &cs.selection else {
            return false;
        };
        let Some(canvas) = &cs.canvas else {
            return false;
        };
        let Some(copied) = extract_masked_selection(canvas, selection) else {
            return false;
        };
        self.clipboard = Some(copied);
        true
    }

    /// Cut the selected pixels into the clipboard and clear them from the canvas.
    pub fn cut_selection(&mut self) -> bool {
        let selection = match self.active().selection.clone() {
            Some(selection) => selection,
            None => return false,
        };
        let before = match self.active().canvas.clone() {
            Some(canvas) => canvas,
            None => return false,
        };
        let Some(copied) = extract_masked_selection(&before, &selection) else {
            return false;
        };

        self.clipboard = Some(copied);
        let cs = self.active_mut();
        let Some(canvas) = &mut cs.canvas else {
            return false;
        };
        clear_masked_pixels(canvas, &selection);
        cs.dirty = true;
        cs.canvas_texture = None;
        self.push_undo_state(before);
        true
    }

    /// Delete the selected pixels without copying them.
    pub fn delete_selection(&mut self) -> bool {
        let selection = match self.active().selection.clone() {
            Some(selection) => selection,
            None => return false,
        };
        let before = match self.active().canvas.clone() {
            Some(canvas) => canvas,
            None => return false,
        };
        if selection.is_empty() {
            return false;
        }

        let cs = self.active_mut();
        let Some(canvas) = &mut cs.canvas else {
            return false;
        };
        clear_masked_pixels(canvas, &selection);
        cs.dirty = true;
        cs.canvas_texture = None;
        self.push_undo_state(before);
        true
    }

    /// Paste clipboard contents at the best position on a specific canvas.
    /// If a cell is selected, scales to fit and centers the paste in that cell.
    /// Otherwise, uses the cursor position.
    pub fn paste_at_cursor(&mut self, side: CanvasSide) -> bool {
        let clipboard = match &self.clipboard {
            Some(c) => c.clone(),
            None => return false,
        };
        let cs = self.canvas_state(side);
        if cs.canvas.is_none() {
            return false;
        }

        // Prepare clipboard (possibly scaled) and position
        let (to_paste, paste_pos) = self.prepare_paste(side, &clipboard);
        let Some(paste_pos) = paste_pos else {
            return false;
        };

        // Store canvas state before paste
        let before = self.canvas_state(side).canvas.clone().unwrap();

        // Perform the paste
        let cs = self.canvas_state_mut(side);
        if let Some(canvas) = &mut cs.canvas {
            canvas.blit(&to_paste, paste_pos.x, paste_pos.y);
            cs.dirty = true;
            cs.canvas_texture = None;
        }

        // Push undo state with before and after
        let after = self.canvas_state(side).canvas.clone().unwrap();
        self.canvas_state_mut(side)
            .history
            .push(SpriteEditCommand { before, after });
        true
    }

    pub fn convert_active_canvas_to_palette(&mut self, palette: Palette4) -> bool {
        let before = match self.active().canvas.clone() {
            Some(canvas) => canvas,
            None => return false,
        };

        let mut changed = false;
        if let Some(canvas) = &mut self.active_mut().canvas {
            for y in 0..canvas.height {
                for x in 0..canvas.width {
                    let Some(color) = canvas.get_pixel(x, y) else {
                        continue;
                    };
                    if color.a == 0 {
                        continue;
                    }

                    let visible = preview_indexed_color(color, palette);
                    let slot = nearest_palette_slot(visible, palette);
                    let mut canonical = canonical_indexed_color(slot);
                    canonical.a = color.a;
                    if canonical != color {
                        canvas.set_pixel(x, y, canonical);
                        changed = true;
                    }
                }
            }
        }

        if changed {
            let cs = self.active_mut();
            cs.dirty = true;
            cs.canvas_texture = None;
            self.push_undo_state(before);
            true
        } else {
            false
        }
    }

    /// Prepare clipboard for pasting: scale if needed and calculate position.
    fn prepare_paste(
        &self,
        side: CanvasSide,
        clipboard: &SpriteCanvas,
    ) -> (SpriteCanvas, Option<glam::IVec2>) {
        let cs = self.canvas_state(side);
        let Some(canvas) = cs.canvas.as_ref() else {
            return (clipboard.clone(), None);
        };

        // If a cell is selected, scale to fit and center in that cell
        if let Some(cell_idx) = cs.selected_cell {
            if let Some(scaled_paste) = self.prepare_cell_paste(canvas, cs, cell_idx, clipboard) {
                return scaled_paste;
            }
        }

        // Fall back to cursor position without scaling
        (clipboard.clone(), cs.cursor_canvas_pos)
    }

    /// Prepare paste for a selected cell - scale and center.
    fn prepare_cell_paste(
        &self,
        canvas: &SpriteCanvas,
        cs: &super::CanvasState,
        cell_idx: usize,
        clipboard: &SpriteCanvas,
    ) -> Option<(SpriteCanvas, Option<glam::IVec2>)> {
        let cell_w = cs.cell_size.x;
        let cell_h = cs.cell_size.y;

        if cell_w == 0 || cell_h == 0 {
            return None;
        }

        let cols = canvas.width / cell_w;
        if cols == 0 {
            return None;
        }

        let cell_x = (cell_idx as u32 % cols) * cell_w;
        let cell_y = (cell_idx as u32 / cols) * cell_h;

        // Scale clipboard to fit in cell if larger
        let scaled = clipboard.scaled_to_fit(cell_w, cell_h);

        // Center the scaled clipboard in the cell
        let center_x = cell_x as i32 + (cell_w as i32 - scaled.width as i32) / 2;
        let center_y = cell_y as i32 + (cell_h as i32 - scaled.height as i32) / 2;

        Some((scaled, Some(glam::IVec2::new(center_x, center_y))))
    }
}

fn extract_masked_selection(
    canvas: &SpriteCanvas,
    selection: &super::SelectionMask,
) -> Option<SpriteCanvas> {
    let bounds = selection.bounding_rect()?;
    let mut result = SpriteCanvas::new(bounds.width, bounds.height);
    for y in 0..bounds.height {
        for x in 0..bounds.width {
            let src_x = bounds.x + x;
            let src_y = bounds.y + y;
            if selection.is_selected(src_x, src_y) {
                if let Some(color) = canvas.get_pixel(src_x, src_y) {
                    result.set_pixel(x, y, color);
                }
            }
        }
    }
    Some(result)
}

fn clear_masked_pixels(canvas: &mut SpriteCanvas, selection: &super::SelectionMask) {
    if let Some(bounds) = selection.bounding_rect() {
        for y in bounds.y..(bounds.y + bounds.height) {
            for x in bounds.x..(bounds.x + bounds.width) {
                if selection.is_selected(x, y) {
                    canvas.set_pixel(x, y, super::PixelColor::transparent());
                }
            }
        }
    }
}
