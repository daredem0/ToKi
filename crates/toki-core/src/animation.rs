use serde::{Deserialize, Serialize};

use crate::errors::CoreError;
use std::collections::HashMap;

// ============================================================================
// Shared Playback Types
// ============================================================================

/// Direction of playback (for ping-pong mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayDirection {
    #[default]
    Forward,
    Backward,
}

/// Events emitted during playback updates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackEvent {
    /// No change occurred
    None,
    /// Frame changed from old to new index
    FrameChanged { old: usize, new: usize },
    /// A loop cycle completed (returned to start or reversed in ping-pong)
    LoopCompleted,
    /// Playback finished (for Once mode)
    Finished,
}

/// Shared animation playback state.
/// Handles frame timing, loop modes, and speed control.
/// Can be used by both runtime and editor for consistent behavior.
#[derive(Debug, Clone)]
pub struct ClipPlayback {
    /// Current frame index
    pub current_frame: usize,
    /// Time accumulated for current frame (milliseconds)
    pub frame_timer: f32,
    /// Whether playback is active
    pub playing: bool,
    /// Whether playback has finished (for Once mode)
    pub is_finished: bool,
    /// Playback speed multiplier (1.0 = normal)
    pub speed: f32,
    /// Current direction (for ping-pong)
    pub direction: PlayDirection,
}

impl Default for ClipPlayback {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipPlayback {
    pub fn new() -> Self {
        Self {
            current_frame: 0,
            frame_timer: 0.0,
            playing: false,
            is_finished: false,
            speed: 1.0,
            direction: PlayDirection::Forward,
        }
    }

    /// Start playback
    pub fn play(&mut self) {
        self.playing = true;
        self.is_finished = false;
    }

    /// Pause playback (keeps current position)
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Toggle between play and pause
    pub fn toggle(&mut self) {
        if self.playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Stop playback and reset to beginning
    pub fn stop(&mut self) {
        self.playing = false;
        self.reset();
    }

    /// Reset to first frame
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.frame_timer = 0.0;
        self.is_finished = false;
        self.direction = PlayDirection::Forward;
    }

    /// Go to specific frame
    pub fn go_to_frame(&mut self, frame: usize, frame_count: usize) {
        if frame_count == 0 {
            return;
        }
        self.current_frame = frame.min(frame_count - 1);
        self.frame_timer = 0.0;
    }

    /// Step forward one frame
    pub fn step_forward(&mut self, frame_count: usize) {
        if frame_count == 0 {
            return;
        }
        self.current_frame = (self.current_frame + 1) % frame_count;
        self.frame_timer = 0.0;
    }

    /// Step backward one frame
    pub fn step_backward(&mut self, frame_count: usize) {
        if frame_count == 0 {
            return;
        }
        self.current_frame = if self.current_frame == 0 {
            frame_count - 1
        } else {
            self.current_frame - 1
        };
        self.frame_timer = 0.0;
    }

    /// Update playback state given delta time and clip parameters.
    /// Returns the most significant event that occurred during this update.
    ///
    /// # Arguments
    /// * `delta_ms` - Time elapsed since last update in milliseconds
    /// * `frame_count` - Total number of frames in the clip
    /// * `frame_duration_at` - Closure that returns duration in ms for a given frame index
    /// * `loop_mode` - How to handle reaching the end of the clip
    pub fn update<F>(
        &mut self,
        delta_ms: f32,
        frame_count: usize,
        frame_duration_at: F,
        loop_mode: &LoopMode,
    ) -> PlaybackEvent
    where
        F: Fn(usize) -> f32,
    {
        if !self.playing || self.is_finished || frame_count == 0 {
            return PlaybackEvent::None;
        }

        self.frame_timer += delta_ms * self.speed;
        let mut event = PlaybackEvent::None;

        // Process frame advances (may advance multiple frames if delta is large)
        while !self.is_finished {
            let frame_duration = frame_duration_at(self.current_frame);
            if self.frame_timer < frame_duration {
                break;
            }
            self.frame_timer -= frame_duration;
            let old_frame = self.current_frame;

            event = self.advance_frame(frame_count, loop_mode);

            // If frame actually changed, update event (unless we got a more significant event)
            if self.current_frame != old_frame && event == PlaybackEvent::None {
                event = PlaybackEvent::FrameChanged {
                    old: old_frame,
                    new: self.current_frame,
                };
            }
        }

        event
    }

    /// Advance to next frame based on direction and loop mode.
    /// Returns PlaybackEvent for loop/finish events.
    fn advance_frame(&mut self, frame_count: usize, loop_mode: &LoopMode) -> PlaybackEvent {
        match self.direction {
            PlayDirection::Forward => self.advance_forward(frame_count, loop_mode),
            PlayDirection::Backward => self.advance_backward(frame_count, loop_mode),
        }
    }

    fn advance_forward(&mut self, frame_count: usize, loop_mode: &LoopMode) -> PlaybackEvent {
        self.current_frame += 1;

        if self.current_frame >= frame_count {
            match loop_mode {
                LoopMode::Loop => {
                    self.current_frame = 0;
                    PlaybackEvent::LoopCompleted
                }
                LoopMode::Once => {
                    self.current_frame = frame_count - 1;
                    self.is_finished = true;
                    self.playing = false;
                    PlaybackEvent::Finished
                }
                LoopMode::PingPong => {
                    // Reverse direction at end
                    self.current_frame = frame_count.saturating_sub(2);
                    self.direction = PlayDirection::Backward;
                    PlaybackEvent::LoopCompleted
                }
            }
        } else {
            PlaybackEvent::None
        }
    }

    fn advance_backward(&mut self, frame_count: usize, loop_mode: &LoopMode) -> PlaybackEvent {
        if self.current_frame == 0 {
            match loop_mode {
                LoopMode::PingPong => {
                    // Reverse direction at start
                    self.current_frame = 1.min(frame_count - 1);
                    self.direction = PlayDirection::Forward;
                    PlaybackEvent::LoopCompleted
                }
                _ => {
                    // Shouldn't happen in backward for non-ping-pong, but handle gracefully
                    self.current_frame = frame_count - 1;
                    PlaybackEvent::LoopCompleted
                }
            }
        } else {
            self.current_frame -= 1;
            PlaybackEvent::None
        }
    }

    /// Calculate normalized progress through current frame (0.0 to 1.0)
    pub fn frame_progress(&self, frame_duration_at: impl Fn(usize) -> f32) -> f32 {
        let duration = frame_duration_at(self.current_frame);
        if duration <= 0.0 {
            return 0.0;
        }
        (self.frame_timer / duration).clamp(0.0, 1.0)
    }
}

// ============================================================================
// Animation State Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum AnimationState {
    Idle,
    Walk,
    Attack,
    IdleDown,
    IdleUp,
    IdleLeft,
    IdleRight,
    WalkDown,
    WalkUp,
    WalkLeft,
    WalkRight,
    AttackDown,
    AttackUp,
    AttackLeft,
    AttackRight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopMode {
    Loop,     // Repeart forever
    Once,     // Play once
    PingPong, //Forward then backward
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClip {
    pub state: AnimationState,
    pub atlas_name: String,
    /// Legacy name-based frame references (may be empty if using positions)
    pub frame_tile_names: Vec<String>,
    /// Position-based frame references as grid [column, row] pairs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_positions: Option<Vec<[u32; 2]>>,
    /// Uniform frame duration in milliseconds (used unless per-frame overrides exist)
    pub frame_duration_ms: f32,
    /// Optional per-frame duration overrides in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_durations_ms: Option<Vec<f32>>,
    pub loop_mode: LoopMode,
}

impl AnimationClip {
    /// Returns the number of frames in this clip.
    /// Prefers frame_positions if available, otherwise uses frame_tile_names.
    pub fn frame_count(&self) -> usize {
        self.frame_positions
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(self.frame_tile_names.len())
    }

    /// Returns the duration for a specific frame index.
    /// Uses per-frame override if available, otherwise falls back to uniform duration.
    pub fn frame_duration_at(&self, frame_index: usize) -> f32 {
        self.frame_durations_ms
            .as_ref()
            .and_then(|durations| durations.get(frame_index).copied())
            .unwrap_or(self.frame_duration_ms)
    }
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

    pub fn has_clip(&self, clip_state: AnimationState) -> bool {
        self.clips.contains_key(&clip_state)
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
    pub fn update(&mut self, delta_time_ms: f32) -> u32 {
        let current_clip = match self.clips.get(&self.current_clip_state) {
            Some(clip) => clip,
            None => return 0,
        };

        let frame_count = current_clip.frame_count();
        if frame_count == 0 {
            return 0;
        }

        let mut completed_loops = 0;
        self.frame_timer += delta_time_ms;

        while !self.is_finished {
            let frame_duration = current_clip.frame_duration_at(self.current_frame_index);
            if self.frame_timer < frame_duration {
                break;
            }
            self.frame_timer -= frame_duration;
            self.current_frame_index += 1;

            if self.current_frame_index >= frame_count {
                match current_clip.loop_mode {
                    LoopMode::Loop => {
                        self.current_frame_index = 0;
                        completed_loops += 1;
                    }
                    LoopMode::Once => self.is_finished = true,
                    LoopMode::PingPong => {
                        self.current_frame_index = 0; // TODO: implement ping-pong
                        completed_loops += 1;
                    }
                }
            }
        }
        completed_loops
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ClipPlayback Tests
    // ========================================================================

    fn uniform_duration(duration: f32) -> impl Fn(usize) -> f32 {
        move |_| duration
    }

    fn varying_durations(durations: Vec<f32>) -> impl Fn(usize) -> f32 {
        move |i| durations.get(i).copied().unwrap_or(100.0)
    }

    // --- Basic State Tests ---

    #[test]
    fn playback_starts_paused() {
        let playback = ClipPlayback::new();
        assert!(!playback.playing);
        assert_eq!(playback.current_frame, 0);
        assert_eq!(playback.speed, 1.0);
        assert!(!playback.is_finished);
    }

    #[test]
    fn play_sets_playing_flag() {
        let mut playback = ClipPlayback::new();
        playback.play();
        assert!(playback.playing);
    }

    #[test]
    fn pause_clears_playing_flag() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.pause();
        assert!(!playback.playing);
    }

    #[test]
    fn toggle_flips_playing_state() {
        let mut playback = ClipPlayback::new();
        assert!(!playback.playing);
        playback.toggle();
        assert!(playback.playing);
        playback.toggle();
        assert!(!playback.playing);
    }

    #[test]
    fn stop_resets_state() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.current_frame = 5;
        playback.frame_timer = 50.0;
        playback.stop();

        assert!(!playback.playing);
        assert_eq!(playback.current_frame, 0);
        assert_eq!(playback.frame_timer, 0.0);
    }

    // --- Frame Navigation Tests ---

    #[test]
    fn go_to_frame_sets_frame_and_clears_timer() {
        let mut playback = ClipPlayback::new();
        playback.frame_timer = 50.0;
        playback.go_to_frame(3, 5);

        assert_eq!(playback.current_frame, 3);
        assert_eq!(playback.frame_timer, 0.0);
    }

    #[test]
    fn go_to_frame_clamps_to_valid_range() {
        let mut playback = ClipPlayback::new();
        playback.go_to_frame(10, 5);
        assert_eq!(playback.current_frame, 4); // 5 frames = indices 0-4
    }

    #[test]
    fn go_to_frame_handles_empty_clip() {
        let mut playback = ClipPlayback::new();
        playback.go_to_frame(3, 0);
        assert_eq!(playback.current_frame, 0); // No change
    }

    #[test]
    fn step_forward_wraps_around() {
        let mut playback = ClipPlayback::new();
        playback.current_frame = 2;
        playback.step_forward(3); // 3 frames total
        assert_eq!(playback.current_frame, 0); // Wraps to 0
    }

    #[test]
    fn step_backward_wraps_around() {
        let mut playback = ClipPlayback::new();
        playback.step_backward(3);
        assert_eq!(playback.current_frame, 2); // Wraps to last frame
    }

    // --- Frame Timing Tests ---

    #[test]
    fn update_does_nothing_when_paused() {
        let mut playback = ClipPlayback::new();
        let event = playback.update(100.0, 3, uniform_duration(50.0), &LoopMode::Loop);

        assert_eq!(event, PlaybackEvent::None);
        assert_eq!(playback.current_frame, 0);
    }

    #[test]
    fn update_accumulates_time() {
        let mut playback = ClipPlayback::new();
        playback.play();

        playback.update(30.0, 3, uniform_duration(100.0), &LoopMode::Loop);
        assert_eq!(playback.frame_timer, 30.0);
        assert_eq!(playback.current_frame, 0);
    }

    #[test]
    fn update_advances_frame_when_duration_exceeded() {
        let mut playback = ClipPlayback::new();
        playback.play();

        let event = playback.update(110.0, 3, uniform_duration(100.0), &LoopMode::Loop);

        assert!(matches!(event, PlaybackEvent::FrameChanged { old: 0, new: 1 }));
        assert_eq!(playback.current_frame, 1);
        assert_eq!(playback.frame_timer, 10.0); // Remainder carried over
    }

    #[test]
    fn update_advances_multiple_frames_for_large_delta() {
        let mut playback = ClipPlayback::new();
        playback.play();

        let event = playback.update(250.0, 5, uniform_duration(100.0), &LoopMode::Loop);

        // Should advance 2 full frames
        assert_eq!(playback.current_frame, 2);
        assert_eq!(playback.frame_timer, 50.0);
        // Event reflects the most recent change
        assert!(matches!(event, PlaybackEvent::FrameChanged { old: 1, new: 2 }));
    }

    #[test]
    fn update_respects_per_frame_durations() {
        let mut playback = ClipPlayback::new();
        playback.play();

        let durations = vec![50.0, 100.0, 200.0];
        // Advance past first frame (50ms), should be on frame 1
        playback.update(60.0, 3, varying_durations(durations.clone()), &LoopMode::Loop);
        assert_eq!(playback.current_frame, 1);
        assert_eq!(playback.frame_timer, 10.0);

        // Advance past second frame (100ms), should be on frame 2
        playback.update(100.0, 3, varying_durations(durations), &LoopMode::Loop);
        assert_eq!(playback.current_frame, 2);
        assert_eq!(playback.frame_timer, 10.0);
    }

    // --- Speed Control Tests ---

    #[test]
    fn update_applies_speed_multiplier() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.speed = 2.0;

        // At 2x speed, 50ms real time = 100ms animation time
        let event = playback.update(50.0, 3, uniform_duration(100.0), &LoopMode::Loop);

        assert!(matches!(event, PlaybackEvent::FrameChanged { .. }));
        assert_eq!(playback.current_frame, 1);
    }

    #[test]
    fn update_with_slow_speed() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.speed = 0.5;

        // At 0.5x speed, 100ms real time = 50ms animation time
        playback.update(100.0, 3, uniform_duration(100.0), &LoopMode::Loop);

        assert_eq!(playback.current_frame, 0); // Not enough time to advance
        assert_eq!(playback.frame_timer, 50.0);
    }

    // --- Loop Mode Tests ---

    #[test]
    fn loop_mode_wraps_to_beginning() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.current_frame = 2; // Last frame of 3

        let event = playback.update(150.0, 3, uniform_duration(100.0), &LoopMode::Loop);

        assert_eq!(event, PlaybackEvent::LoopCompleted);
        assert_eq!(playback.current_frame, 0);
        assert!(playback.playing);
        assert!(!playback.is_finished);
    }

    #[test]
    fn once_mode_stops_at_end() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.current_frame = 2; // Last frame of 3

        let event = playback.update(150.0, 3, uniform_duration(100.0), &LoopMode::Once);

        assert_eq!(event, PlaybackEvent::Finished);
        assert_eq!(playback.current_frame, 2); // Stays on last frame
        assert!(!playback.playing);
        assert!(playback.is_finished);
    }

    #[test]
    fn once_mode_does_not_restart_after_finished() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.current_frame = 2;
        playback.update(150.0, 3, uniform_duration(100.0), &LoopMode::Once);

        // Try to update again - should do nothing
        let event = playback.update(150.0, 3, uniform_duration(100.0), &LoopMode::Once);

        assert_eq!(event, PlaybackEvent::None);
        assert_eq!(playback.current_frame, 2);
    }

    #[test]
    fn ping_pong_reverses_at_end() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.current_frame = 3; // Last frame of 4

        let event = playback.update(150.0, 4, uniform_duration(100.0), &LoopMode::PingPong);

        assert_eq!(event, PlaybackEvent::LoopCompleted);
        assert_eq!(playback.direction, PlayDirection::Backward);
        assert_eq!(playback.current_frame, 2); // Goes to second-to-last
    }

    #[test]
    fn ping_pong_reverses_at_start() {
        let mut playback = ClipPlayback::new();
        playback.play();
        playback.direction = PlayDirection::Backward;
        playback.current_frame = 0;

        let event = playback.update(150.0, 4, uniform_duration(100.0), &LoopMode::PingPong);

        assert_eq!(event, PlaybackEvent::LoopCompleted);
        assert_eq!(playback.direction, PlayDirection::Forward);
        assert_eq!(playback.current_frame, 1);
    }

    #[test]
    fn ping_pong_full_cycle() {
        let mut playback = ClipPlayback::new();
        playback.play();

        // 4 frames (indices 0,1,2,3), 100ms each
        // Forward: 0 -> 1 -> 2 -> 3 -> reverse to 2
        // Backward: 2 -> 1 -> 0 -> reverse to 1
        // Forward: 1 -> 2 -> ...

        // 450ms = 4 frame transitions + 50ms into next frame
        // 0 -> 1 (100ms) -> 2 (200ms) -> 3 (300ms) -> reverse to 2 (400ms) + 50ms
        playback.update(450.0, 4, uniform_duration(100.0), &LoopMode::PingPong);
        assert_eq!(playback.current_frame, 2); // At frame 2, going backward
        assert_eq!(playback.direction, PlayDirection::Backward);

        // 350ms more: 2 -> 1 (50+100=150ms done) -> 0 (250ms) -> reverse to 1 (350ms)
        playback.update(300.0, 4, uniform_duration(100.0), &LoopMode::PingPong);
        assert_eq!(playback.current_frame, 1);
        assert_eq!(playback.direction, PlayDirection::Forward);
    }

    // --- Edge Cases ---

    #[test]
    fn update_handles_empty_clip() {
        let mut playback = ClipPlayback::new();
        playback.play();

        let event = playback.update(100.0, 0, uniform_duration(100.0), &LoopMode::Loop);

        assert_eq!(event, PlaybackEvent::None);
    }

    #[test]
    fn update_handles_single_frame_clip_loop() {
        let mut playback = ClipPlayback::new();
        playback.play();

        let event = playback.update(150.0, 1, uniform_duration(100.0), &LoopMode::Loop);

        assert_eq!(event, PlaybackEvent::LoopCompleted);
        assert_eq!(playback.current_frame, 0);
    }

    #[test]
    fn update_handles_single_frame_clip_once() {
        let mut playback = ClipPlayback::new();
        playback.play();

        let event = playback.update(150.0, 1, uniform_duration(100.0), &LoopMode::Once);

        assert_eq!(event, PlaybackEvent::Finished);
        assert_eq!(playback.current_frame, 0);
        assert!(playback.is_finished);
    }

    #[test]
    fn frame_progress_calculates_correctly() {
        let mut playback = ClipPlayback::new();
        playback.frame_timer = 50.0;

        let progress = playback.frame_progress(uniform_duration(100.0));
        assert!((progress - 0.5).abs() < 0.001);
    }

    #[test]
    fn frame_progress_clamps_to_valid_range() {
        let mut playback = ClipPlayback::new();
        playback.frame_timer = 150.0; // Exceeds duration

        let progress = playback.frame_progress(uniform_duration(100.0));
        assert_eq!(progress, 1.0);
    }
}
