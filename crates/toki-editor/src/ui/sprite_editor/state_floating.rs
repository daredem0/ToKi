//! Floating selection operations for SpriteEditorState.

use super::floating::{FloatingOrigin, FloatingSelection};
use super::selection::{clear_masked_pixels, extract_masked_selection};
use super::SpriteEditorState;

impl SpriteEditorState {
    /// Check if there is an active floating selection.
    pub fn has_floating(&self) -> bool {
        self.active().floating.is_some()
    }

    /// Lift the current selection into a floating selection.
    ///
    /// Extracts the selected pixels, clears them from the canvas, and stores
    /// the result as a `FloatingSelection` for repositioning.
    pub fn lift_selection(&mut self) -> bool {
        let selection = match self.active().selection.clone() {
            Some(s) if !s.is_empty() => s,
            _ => return false,
        };
        let canvas_before_lift = match self.active().canvas.clone() {
            Some(c) => c,
            None => return false,
        };

        let Some(pixels) = extract_masked_selection(&canvas_before_lift, &selection) else {
            return false;
        };
        let Some(bounds) = selection.bounding_rect() else {
            return false;
        };

        let local_mask = build_local_mask(&selection, bounds);
        let offset = glam::IVec2::new(bounds.x as i32, bounds.y as i32);
        let selection_before_float = selection.clone();

        let cs = self.active_mut();
        if let Some(canvas) = &mut cs.canvas {
            clear_masked_pixels(canvas, &selection);
        }

        cs.floating = Some(FloatingSelection {
            pixels,
            mask: local_mask,
            offset,
            canvas_before_lift,
            origin: FloatingOrigin::SelectionLift {
                selection_before_float,
            },
            resize_size: None,
        });
        cs.selection = None;
        cs.canvas_texture_dirty = true;
        true
    }

    /// Stamp the floating pixels onto the canvas and push one undo entry.
    /// If `resize_size` is set, the pixels and mask are resampled before blitting.
    pub fn commit_floating(&mut self) -> bool {
        let floating = match self.active_mut().floating.take() {
            Some(f) => f,
            None => return false,
        };

        let (final_pixels, final_mask) = resample_if_resized(&floating);
        let canvas_before_lift = floating.canvas_before_lift;

        let cs = self.active_mut();
        cs.resize_drag = None;
        if let Some(canvas) = &mut cs.canvas {
            canvas.blit(&final_pixels, floating.offset.x, floating.offset.y);
        }

        // Reconstruct selection mask at the new position
        let (cw, ch) = cs
            .canvas
            .as_ref()
            .map(|c| (c.width, c.height))
            .unwrap_or((0, 0));
        cs.selection = Some(final_mask.translated_to_canvas(cw, ch, floating.offset));
        cs.dirty = true;
        cs.canvas_texture_dirty = true;

        self.push_undo_state(canvas_before_lift);
        true
    }

    /// Cancel the floating selection and restore the canvas to its pre-lift state.
    /// Does not push an undo entry.
    pub fn cancel_floating(&mut self) -> bool {
        let floating = match self.active_mut().floating.take() {
            Some(f) => f,
            None => return false,
        };

        let selection_before_float = floating.origin.selection_before_float().cloned();
        let cs = self.active_mut();
        cs.canvas = Some(floating.canvas_before_lift);
        cs.selection = selection_before_float;
        cs.resize_drag = None;
        cs.canvas_texture_dirty = true;
        true
    }

    /// Switch the active tool, auto-committing any floating selection first.
    pub fn set_tool(&mut self, tool: super::SpriteEditorTool) {
        if self.has_floating() {
            self.commit_floating();
        }
        self.tool = tool;
    }

    /// Lift the selection into a float (if not already floating) and nudge by `delta`.
    /// This is the arrow-key workflow: one keypress lifts and moves in a single step.
    pub fn lift_and_nudge(&mut self, delta: glam::IVec2) {
        if !self.has_floating() && !self.lift_selection() {
            return;
        }
        self.nudge_floating(delta);
    }

    /// Move the floating selection by `delta` pixels.
    pub fn nudge_floating(&mut self, delta: glam::IVec2) {
        if let Some(floating) = &mut self.active_mut().floating {
            floating.offset += delta;
        }
    }

    /// Center the floating selection within its current tile (sheet mode) or the full canvas.
    ///
    /// In sheet mode the tile is whichever cell contains the floating selection's center point.
    /// In single-sprite mode the selection is centered on the whole canvas.
    pub fn center_floating_on_tile(&mut self) {
        let (float_offset, float_size) = match self.active().floating.as_ref() {
            Some(f) => (f.offset, f.display_size()),
            None => return,
        };

        let center_x = float_offset.x + float_size.x as i32 / 2;
        let center_y = float_offset.y + float_size.y as i32 / 2;
        let (tile_x, tile_y, tile_w, tile_h) = self.centering_bounds(center_x, center_y);

        let new_offset = glam::IVec2::new(
            tile_x as i32 + (tile_w as i32 - float_size.x as i32) / 2,
            tile_y as i32 + (tile_h as i32 - float_size.y as i32) / 2,
        );
        if let Some(floating) = &mut self.active_mut().floating {
            floating.offset = new_offset;
        }
    }

    /// Returns `(tile_x, tile_y, tile_w, tile_h)` — the region within which the floating
    /// selection should be centered. Uses the tile under `(cx, cy)` in sheet mode, or the
    /// full canvas otherwise.
    fn centering_bounds(&self, cx: i32, cy: i32) -> (u32, u32, u32, u32) {
        if self.is_sheet() {
            let clamped_x = cx.max(0) as u32;
            let clamped_y = cy.max(0) as u32;
            if let Some(idx) = self.cell_at_position(clamped_x, clamped_y) {
                if let Some((sx, sy, ex, ey)) = self.cell_bounds(idx) {
                    return (sx, sy, ex - sx, ey - sy);
                }
            }
        }
        let (w, h) = self.canvas_dimensions().unwrap_or((0, 0));
        (0, 0, w, h)
    }

    /// Set the display/commit size of the floating selection and update the offset
    /// so that the given anchor point stays fixed.
    pub fn resize_floating(&mut self, new_size: glam::UVec2, anchor: glam::IVec2) {
        if let Some(floating) = &mut self.active_mut().floating {
            floating.resize_size = Some(new_size);
            // Recompute offset so the anchor corner stays in place.
            // anchor = offset + display_size for bottom-right, etc.
            // We always store offset as top-left, so: offset = anchor - 0 or anchor - size
            // The caller provides the fixed anchor corner in canvas coords;
            // we derive offset from it.
            floating.offset = anchor;
        }
    }
}

/// If the floating selection has a resize target, resample pixels and mask.
/// Otherwise return clones of the originals.
fn resample_if_resized(
    floating: &FloatingSelection,
) -> (super::SpriteCanvas, super::SelectionMask) {
    match floating.resize_size {
        Some(size) => (
            floating.pixels.scaled_to(size.x, size.y),
            floating.mask.scaled_to(size.x, size.y),
        ),
        None => (floating.pixels.clone(), floating.mask.clone()),
    }
}

/// Build a local-coordinate mask from a canvas-sized selection and its bounding rect.
fn build_local_mask(
    selection: &super::SelectionMask,
    bounds: super::SpriteSelection,
) -> super::SelectionMask {
    let mut local = super::SelectionMask::new(bounds.width, bounds.height);
    for y in 0..bounds.height {
        for x in 0..bounds.width {
            if selection.is_selected(bounds.x + x, bounds.y + y) {
                local.select_pixel(x, y);
            }
        }
    }
    local
}
