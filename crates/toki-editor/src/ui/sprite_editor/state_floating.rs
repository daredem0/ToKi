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
        });
        cs.selection = None;
        cs.canvas_texture_dirty = true;
        true
    }

    /// Stamp the floating pixels onto the canvas and push one undo entry.
    pub fn commit_floating(&mut self) -> bool {
        let floating = match self.active_mut().floating.take() {
            Some(f) => f,
            None => return false,
        };

        let canvas_before_lift = floating.canvas_before_lift;
        let cs = self.active_mut();
        if let Some(canvas) = &mut cs.canvas {
            canvas.blit(&floating.pixels, floating.offset.x, floating.offset.y);
        }

        // Reconstruct selection mask at the new position
        let (cw, ch) = cs
            .canvas
            .as_ref()
            .map(|c| (c.width, c.height))
            .unwrap_or((0, 0));
        cs.selection = Some(floating.mask.translated_to_canvas(cw, ch, floating.offset));
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
