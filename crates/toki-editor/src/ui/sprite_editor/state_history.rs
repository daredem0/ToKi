//! Undo/redo and clipboard operations for SpriteEditorState.

use super::{
    CanvasSide, SpriteCanvas, SpriteEditCommand, SpriteEditorState,
};

impl SpriteEditorState {
    /// Push current canvas state for undo
    pub fn push_undo_state(&mut self, before: SpriteCanvas) {
        let cs = self.active_mut();
        if let Some(canvas) = &cs.canvas {
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
        let Some(copied) =
            canvas.extract_region(selection.x, selection.y, selection.width, selection.height)
        else {
            return false;
        };
        self.clipboard = Some(copied);
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
