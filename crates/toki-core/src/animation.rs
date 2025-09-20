use serde::{Deserialize, Serialize};

use crate::errors::CoreError;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum AnimationState {
    Idle,
    Walk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopMode {
    Loop,    // Repeart forever
    Once,    // Play once
    PingPong, //Forward then backward
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClip {
    pub state: AnimationState,
    pub atlas_name: String,            // Reference to the atlas
    pub frame_tile_names: Vec<String>, // Which named tiles from the atlas
    pub frame_duration_ms: f32,        // How long each animation frame lasts
    pub loop_mode: LoopMode,           // Repeat?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationController {
    pub clips: HashMap<AnimationState, AnimationClip>, // All available animations
    pub current_clip_state: AnimationState,
    pub current_frame_index: usize, // Where are we at the current frame
    pub frame_timer: f32,           // Counter of the current frame
    pub is_finished: bool,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            current_clip_state: AnimationState::Idle,
            current_frame_index: 0,
            frame_timer: 0.0,
            is_finished: false,
        }
    }
    /// Add an animation clip to this controller
    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.insert(clip.state, clip);
    }

    /// Start playing a specific animation
    pub fn play(&mut self, clip_state: AnimationState) -> bool {
        if self.clips.contains_key(&clip_state) {
            self.current_clip_state = clip_state;
            self.current_frame_index = 0;
            self.frame_timer = 0.0;
            self.is_finished = false;
            true // Success
        } else {
            tracing::warn!("Animation doesn't exist.");
            false // Animation doesn't exist
        }
    }

    /// Update animation timing (call this every frame)
    pub fn update(&mut self, delta_time_ms: f32) {
        let current_clip = match self.clips.get(&self.current_clip_state) {
            Some(clip) => clip,
            None => return, // No current animation
        };
        self.frame_timer += delta_time_ms;
        while (self.frame_timer >= current_clip.frame_duration_ms) && !self.is_finished {
            self.frame_timer -= current_clip.frame_duration_ms;

            self.current_frame_index += 1;

            if self.current_frame_index >= current_clip.frame_tile_names.len() {
                match current_clip.loop_mode {
                    LoopMode::Loop => self.current_frame_index = 0,
                    LoopMode::Once => self.is_finished = true,
                    LoopMode::PingPong => self.current_frame_index = 0, //TODO we still have to implement that one
                }
            }
        }
    }

    /// Get the current tile name for rendering
    pub fn current_tile_name(&self) -> Result<String, CoreError> {
        let current_clip = self.clips.get(&self.current_clip_state).ok_or_else(|| {
            CoreError::AnimationClipNotFound {
                clip_name: format!("{:?}", self.current_clip_state),
            }
        })?;

        if self.current_frame_index >= current_clip.frame_tile_names.len() {
            return Err(CoreError::AnimationFrameOutOfBounds {
                frame_index: self.current_frame_index,
                clip_name: format!("{:?}", current_clip.state),
                max_frames: current_clip.frame_tile_names.len(),
            });
        }

        Ok(current_clip.frame_tile_names[self.current_frame_index].clone())
    }

    /// Get the current atlas name for rendering
    pub fn current_atlas_name(&self) -> Result<String, CoreError> {
        let current_clip = self.clips.get(&self.current_clip_state).ok_or_else(|| {
            CoreError::AnimationClipNotFound {
                clip_name: format!("{:?}", self.current_clip_state),
            }
        })?;

        Ok(current_clip.atlas_name.clone())
    }
}
