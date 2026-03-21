// Animation authoring state and logic for the entity editor
// This module provides the UI state management for visual animation clip authoring
//
// Note: Some methods are currently only used in tests but will be used when
// additional UI features (per-frame duration controls, cell grid picker) are added.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use toki_core::animation::LoopMode;
use toki_core::entity::{AnimationClipDef, AnimationsDef};

/// A single frame in an animation clip, using position-based references
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoredFrame {
    /// Grid position [column, row] in the atlas
    pub position: [u32; 2],
    /// Optional per-frame duration override (None = use clip default)
    pub duration_ms: Option<f32>,
}

impl AuthoredFrame {
    pub fn new(column: u32, row: u32) -> Self {
        Self {
            position: [column, row],
            duration_ms: None,
        }
    }

    pub fn with_duration(column: u32, row: u32, duration_ms: f32) -> Self {
        Self {
            position: [column, row],
            duration_ms: Some(duration_ms),
        }
    }
}

/// A clip being authored in the animation editor
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoredClip {
    /// Animation state name (e.g., "idle_down", "walk_up")
    pub state: String,
    /// Frames in sequence order
    pub frames: Vec<AuthoredFrame>,
    /// Default frame duration in milliseconds
    pub default_duration_ms: f32,
    /// Loop mode: "loop", "once", or "ping_pong"
    pub loop_mode: String,
}

impl Default for AuthoredClip {
    fn default() -> Self {
        Self {
            state: "idle".to_string(),
            frames: Vec::new(),
            default_duration_ms: 100.0,
            loop_mode: "loop".to_string(),
        }
    }
}

impl AuthoredClip {
    pub fn new(state: &str) -> Self {
        Self {
            state: state.to_string(),
            ..Default::default()
        }
    }

    /// Add a frame at the end of the sequence
    pub fn add_frame(&mut self, column: u32, row: u32) {
        self.frames.push(AuthoredFrame::new(column, row));
    }

    /// Add a frame with custom duration at the end
    pub fn add_frame_with_duration(&mut self, column: u32, row: u32, duration_ms: f32) {
        self.frames
            .push(AuthoredFrame::with_duration(column, row, duration_ms));
    }

    /// Remove a frame at the given index
    pub fn remove_frame(&mut self, index: usize) -> bool {
        if index < self.frames.len() {
            self.frames.remove(index);
            true
        } else {
            false
        }
    }

    /// Move a frame from one index to another (for drag-and-drop reordering)
    pub fn move_frame(&mut self, from_index: usize, to_index: usize) -> bool {
        if from_index >= self.frames.len() || to_index >= self.frames.len() {
            return false;
        }
        if from_index == to_index {
            return true;
        }

        let frame = self.frames.remove(from_index);
        self.frames.insert(to_index, frame);
        true
    }

    /// Set per-frame duration for a specific frame
    pub fn set_frame_duration(&mut self, index: usize, duration_ms: Option<f32>) -> bool {
        if let Some(frame) = self.frames.get_mut(index) {
            frame.duration_ms = duration_ms;
            true
        } else {
            false
        }
    }

    /// Get the effective duration for a frame (per-frame override or default)
    pub fn effective_duration(&self, index: usize) -> Option<f32> {
        self.frames.get(index).map(|frame| {
            frame.duration_ms.unwrap_or(self.default_duration_ms)
        })
    }

    /// Check if any frame has a per-frame duration override
    pub fn has_per_frame_durations(&self) -> bool {
        self.frames.iter().any(|f| f.duration_ms.is_some())
    }

    /// Get the number of frames in this clip
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Parse the loop_mode string into the LoopMode enum
    pub fn loop_mode_enum(&self) -> LoopMode {
        match self.loop_mode.as_str() {
            "once" => LoopMode::Once,
            "ping_pong" => LoopMode::PingPong,
            _ => LoopMode::Loop, // Default to Loop
        }
    }

    /// Get frame duration at index (for ClipPlayback compatibility)
    pub fn frame_duration_at(&self, index: usize) -> f32 {
        self.frames
            .get(index)
            .and_then(|f| f.duration_ms)
            .unwrap_or(self.default_duration_ms)
    }

    /// Clear all per-frame duration overrides
    pub fn clear_per_frame_durations(&mut self) {
        for frame in &mut self.frames {
            frame.duration_ms = None;
        }
    }

    /// Convert to AnimationClipDef for saving
    pub fn to_clip_def(&self) -> AnimationClipDef {
        let frame_positions: Vec<[u32; 2]> = self.frames.iter().map(|f| f.position).collect();

        let frame_durations_ms: Option<Vec<f32>> = if self.has_per_frame_durations() {
            Some(self.frames.iter().map(|f| {
                f.duration_ms.unwrap_or(self.default_duration_ms)
            }).collect())
        } else {
            None
        };

        AnimationClipDef {
            state: self.state.clone(),
            frame_tiles: Vec::new(), // Position-based, not name-based
            frame_positions: if frame_positions.is_empty() { None } else { Some(frame_positions) },
            frame_duration_ms: self.default_duration_ms,
            frame_durations_ms,
            loop_mode: self.loop_mode.clone(),
        }
    }

    /// Create from an AnimationClipDef (for editing existing clips)
    pub fn from_clip_def(def: &AnimationClipDef) -> Self {
        Self::from_clip_def_with_tile_lookup(def, None)
    }

    /// Create from an AnimationClipDef with optional tile name to position lookup
    pub fn from_clip_def_with_tile_lookup(
        def: &AnimationClipDef,
        tile_lookup: Option<&std::collections::HashMap<String, [u32; 2]>>,
    ) -> Self {
        let default_duration = def.frame_duration_ms;

        // First try frame_positions, then fall back to frame_tiles with lookup
        let frames = if let Some(positions) = &def.frame_positions {
            positions
                .iter()
                .enumerate()
                .map(|(i, pos)| {
                    let duration_ms = def
                        .frame_durations_ms
                        .as_ref()
                        .and_then(|durations| durations.get(i).copied())
                        .filter(|&d| (d - default_duration).abs() > f32::EPSILON);
                    AuthoredFrame {
                        position: *pos,
                        duration_ms,
                    }
                })
                .collect()
        } else if !def.frame_tiles.is_empty() {
            // Convert frame_tiles to positions using the lookup
            def.frame_tiles
                .iter()
                .enumerate()
                .filter_map(|(i, tile_name)| {
                    let position = tile_lookup
                        .and_then(|lookup| lookup.get(tile_name).copied())?;
                    let duration_ms = def
                        .frame_durations_ms
                        .as_ref()
                        .and_then(|durations| durations.get(i).copied())
                        .filter(|&d| (d - default_duration).abs() > f32::EPSILON);
                    Some(AuthoredFrame {
                        position,
                        duration_ms,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        Self {
            state: def.state.clone(),
            frames,
            default_duration_ms: default_duration,
            loop_mode: def.loop_mode.clone(),
        }
    }
}

/// State for the animation authoring panel
#[derive(Debug, Clone, Default)]
pub struct AnimationAuthoringState {
    /// List of clips being authored
    pub clips: Vec<AuthoredClip>,
    /// Index of the currently selected clip (for editing)
    pub selected_clip_index: Option<usize>,
    /// Index of the currently selected frame within the clip
    pub selected_frame_index: Option<usize>,
    /// The atlas name selected for this entity's animations
    pub atlas_name: String,
    /// Default animation state name
    pub default_state: String,
    /// Whether the authoring state has unsaved changes
    pub dirty: bool,
}

impl AnimationAuthoringState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from an AnimationsDef (for editing existing entity animations)
    pub fn from_animations_def(def: &AnimationsDef) -> Self {
        Self::from_animations_def_with_tile_lookup(def, None)
    }

    /// Load from an AnimationsDef with optional tile name to position lookup
    pub fn from_animations_def_with_tile_lookup(
        def: &AnimationsDef,
        tile_lookup: Option<&std::collections::HashMap<String, [u32; 2]>>,
    ) -> Self {
        let clips = def
            .clips
            .iter()
            .map(|clip| AuthoredClip::from_clip_def_with_tile_lookup(clip, tile_lookup))
            .collect();
        Self {
            clips,
            selected_clip_index: None,
            selected_frame_index: None,
            atlas_name: def.atlas_name.clone(),
            default_state: def.default_state.clone(),
            dirty: false,
        }
    }

    /// Convert to AnimationsDef for saving
    pub fn to_animations_def(&self) -> AnimationsDef {
        AnimationsDef {
            atlas_name: self.atlas_name.clone(),
            clips: self.clips.iter().map(|c| c.to_clip_def()).collect(),
            default_state: self.default_state.clone(),
        }
    }

    /// Get the currently selected clip (immutable)
    pub fn selected_clip(&self) -> Option<&AuthoredClip> {
        self.selected_clip_index.and_then(|i| self.clips.get(i))
    }

    /// Get the currently selected clip (mutable)
    pub fn selected_clip_mut(&mut self) -> Option<&mut AuthoredClip> {
        self.selected_clip_index.and_then(|i| self.clips.get_mut(i))
    }

    /// Create a new clip with the given state
    pub fn create_clip(&mut self, state: &str) -> usize {
        let clip = AuthoredClip::new(state);
        self.clips.push(clip);
        let index = self.clips.len() - 1;
        self.selected_clip_index = Some(index);
        self.selected_frame_index = None;
        self.dirty = true;
        index
    }

    /// Delete the clip at the given index
    pub fn delete_clip(&mut self, index: usize) -> bool {
        if index >= self.clips.len() {
            return false;
        }

        self.clips.remove(index);
        self.dirty = true;

        // Adjust selection
        if let Some(selected) = self.selected_clip_index {
            if selected == index {
                // Deleted the selected clip
                self.selected_clip_index = if self.clips.is_empty() {
                    None
                } else {
                    Some(selected.min(self.clips.len() - 1))
                };
                self.selected_frame_index = None;
            } else if selected > index {
                // Selected clip shifted down
                self.selected_clip_index = Some(selected - 1);
            }
        }

        true
    }

    /// Select a clip by index
    pub fn select_clip(&mut self, index: usize) -> bool {
        if index < self.clips.len() {
            self.selected_clip_index = Some(index);
            self.selected_frame_index = None;
            true
        } else {
            false
        }
    }

    /// Add a frame to the currently selected clip
    pub fn add_frame_to_selected(&mut self, column: u32, row: u32) -> bool {
        if let Some(clip) = self.selected_clip_mut() {
            clip.add_frame(column, row);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// Remove the selected frame from the selected clip
    pub fn remove_selected_frame(&mut self) -> bool {
        let (clip_idx, frame_idx) = match (self.selected_clip_index, self.selected_frame_index) {
            (Some(c), Some(f)) => (c, f),
            _ => return false,
        };

        if let Some(clip) = self.clips.get_mut(clip_idx) {
            if clip.remove_frame(frame_idx) {
                self.dirty = true;
                // Adjust frame selection
                if clip.frames.is_empty() {
                    self.selected_frame_index = None;
                } else {
                    self.selected_frame_index = Some(frame_idx.min(clip.frames.len() - 1));
                }
                return true;
            }
        }
        false
    }

    /// Select the next frame in the sequence
    pub fn select_next_frame(&mut self) -> bool {
        let Some(clip_idx) = self.selected_clip_index else {
            return false;
        };
        let Some(clip) = self.clips.get(clip_idx) else {
            return false;
        };

        if clip.frames.is_empty() {
            return false;
        }

        let current = self.selected_frame_index.unwrap_or(0);
        let next = (current + 1).min(clip.frames.len() - 1);
        self.selected_frame_index = Some(next);
        true
    }

    /// Select the previous frame in the sequence
    pub fn select_prev_frame(&mut self) -> bool {
        let Some(clip_idx) = self.selected_clip_index else {
            return false;
        };
        let Some(clip) = self.clips.get(clip_idx) else {
            return false;
        };

        if clip.frames.is_empty() {
            return false;
        }

        let current = self.selected_frame_index.unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.selected_frame_index = Some(prev);
        true
    }

    /// Move the selected frame up (earlier in sequence)
    pub fn move_selected_frame_up(&mut self) -> bool {
        let (clip_idx, frame_idx) = match (self.selected_clip_index, self.selected_frame_index) {
            (Some(c), Some(f)) if f > 0 => (c, f),
            _ => return false,
        };

        if let Some(clip) = self.clips.get_mut(clip_idx) {
            if clip.move_frame(frame_idx, frame_idx - 1) {
                self.selected_frame_index = Some(frame_idx - 1);
                self.dirty = true;
                return true;
            }
        }
        false
    }

    /// Move the selected frame down (later in sequence)
    pub fn move_selected_frame_down(&mut self) -> bool {
        let (clip_idx, frame_idx) = match (self.selected_clip_index, self.selected_frame_index) {
            (Some(c), Some(f)) => (c, f),
            _ => return false,
        };

        if let Some(clip) = self.clips.get_mut(clip_idx) {
            if frame_idx + 1 >= clip.frames.len() {
                return false;
            }
            if clip.move_frame(frame_idx, frame_idx + 1) {
                self.selected_frame_index = Some(frame_idx + 1);
                self.dirty = true;
                return true;
            }
        }
        false
    }

    /// Find a clip by state name
    pub fn find_clip_by_state(&self, state: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.state == state)
    }

    /// Get available animation states that don't have clips yet
    pub fn available_states(&self) -> Vec<&'static str> {
        const ALL_STATES: &[&str] = &[
            "idle", "walk", "attack",
            "idle_down", "idle_up", "idle_left", "idle_right",
            "walk_down", "walk_up", "walk_left", "walk_right",
            "attack_down", "attack_up", "attack_left", "attack_right",
        ];

        ALL_STATES
            .iter()
            .filter(|&state| self.find_clip_by_state(state).is_none())
            .copied()
            .collect()
    }

    /// Check if the given state already has a clip
    pub fn has_clip_for_state(&self, state: &str) -> bool {
        self.find_clip_by_state(state).is_some()
    }
}

#[cfg(test)]
#[path = "editor_ui_animation_authoring_tests.rs"]
mod tests;
