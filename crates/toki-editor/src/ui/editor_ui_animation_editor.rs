// Animation editor state for the dedicated animation editor tab
// Provides visual animation editing with preview playback
//
// Note: Some methods are currently only used in tests but will be used when
// additional UI features (atlas texture rendering, full preview) are added.

#![allow(dead_code)]

use super::editor_ui_animation_authoring::{AnimationAuthoringState, AuthoredClip};
use std::path::PathBuf;

/// Viewport state for the atlas canvas view
#[derive(Debug, Clone)]
pub struct AtlasViewport {
    /// Camera offset in canvas pixels (top-left corner of view)
    pub pan: glam::Vec2,
    /// Zoom level (1.0 = 1 canvas pixel = 1 screen pixel)
    pub zoom: f32,
    /// Current cursor position in canvas coordinates (if hovering)
    pub cursor_canvas_pos: Option<glam::IVec2>,
}

impl Default for AtlasViewport {
    fn default() -> Self {
        Self {
            pan: glam::Vec2::ZERO,
            zoom: 2.0, // Start zoomed in for visibility
            cursor_canvas_pos: None,
        }
    }
}

impl AtlasViewport {
    /// Zoom in by one step
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(32.0);
    }

    /// Zoom out by one step
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.5);
    }

    /// Pan by delta in screen pixels
    pub fn pan_by(&mut self, delta: glam::Vec2) {
        self.pan -= delta / self.zoom;
    }

    /// Convert screen position to canvas position
    pub fn screen_to_canvas(&self, screen_pos: glam::Vec2, viewport_rect: egui::Rect) -> glam::Vec2 {
        let viewport_pos = screen_pos - glam::Vec2::new(viewport_rect.left(), viewport_rect.top());
        viewport_pos / self.zoom + self.pan
    }
}

/// Playback state for animation preview
#[derive(Debug, Clone, Default)]
pub struct AnimationPreviewState {
    /// Whether the animation is currently playing
    pub playing: bool,
    /// Current frame index in the active clip
    pub current_frame: usize,
    /// Time elapsed since last frame change (in seconds)
    pub elapsed_time: f32,
    /// Playback speed multiplier (1.0 = normal speed)
    pub speed: f32,
}

impl AnimationPreviewState {
    pub fn new() -> Self {
        Self {
            playing: false,
            current_frame: 0,
            elapsed_time: 0.0,
            speed: 1.0,
        }
    }

    /// Start playback
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Toggle play/pause
    pub fn toggle_playback(&mut self) {
        self.playing = !self.playing;
    }

    /// Reset to first frame
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.elapsed_time = 0.0;
    }

    /// Stop playback and reset
    pub fn stop(&mut self) {
        self.playing = false;
        self.reset();
    }

    /// Update playback state given delta time and clip info.
    /// Returns true if frame changed.
    pub fn update(&mut self, delta_seconds: f32, clip: &AuthoredClip) -> bool {
        if !self.playing || clip.frames.is_empty() {
            return false;
        }

        self.elapsed_time += delta_seconds * self.speed;

        let frame_duration_secs = clip.effective_duration(self.current_frame)
            .unwrap_or(clip.default_duration_ms) / 1000.0;

        if self.elapsed_time >= frame_duration_secs {
            self.elapsed_time -= frame_duration_secs;
            let old_frame = self.current_frame;
            self.advance_frame(clip);
            return self.current_frame != old_frame;
        }

        false
    }

    /// Advance to next frame based on loop mode
    fn advance_frame(&mut self, clip: &AuthoredClip) {
        if clip.frames.is_empty() {
            return;
        }

        match clip.loop_mode.as_str() {
            "loop" => {
                self.current_frame = (self.current_frame + 1) % clip.frames.len();
            }
            "once" => {
                if self.current_frame + 1 < clip.frames.len() {
                    self.current_frame += 1;
                } else {
                    self.playing = false;
                }
            }
            "ping_pong" => {
                // Simplified ping-pong: just loop for now
                self.current_frame = (self.current_frame + 1) % clip.frames.len();
            }
            _ => {
                self.current_frame = (self.current_frame + 1) % clip.frames.len();
            }
        }
    }

    /// Go to specific frame
    pub fn go_to_frame(&mut self, frame: usize) {
        self.current_frame = frame;
        self.elapsed_time = 0.0;
    }

    /// Step to next frame (manual)
    pub fn step_forward(&mut self, frame_count: usize) {
        if frame_count == 0 {
            return;
        }
        self.current_frame = (self.current_frame + 1) % frame_count;
        self.elapsed_time = 0.0;
    }

    /// Step to previous frame (manual)
    pub fn step_backward(&mut self, frame_count: usize) {
        if frame_count == 0 {
            return;
        }
        self.current_frame = if self.current_frame == 0 {
            frame_count - 1
        } else {
            self.current_frame - 1
        };
        self.elapsed_time = 0.0;
    }
}

/// State for the animation editor tab
#[derive(Clone, Default)]
pub struct AnimationEditorState {
    /// Currently loaded entity definition name
    pub active_entity: Option<String>,
    /// Path to the entity definition file
    pub entity_file_path: Option<PathBuf>,
    /// Animation authoring state for the active entity
    pub authoring: AnimationAuthoringState,
    /// Preview playback state
    pub preview: AnimationPreviewState,
    /// Cached atlas texture handle (not Debug)
    pub atlas_texture: Option<egui::TextureHandle>,
    /// Cached atlas texture path (to detect changes)
    pub atlas_texture_path: Option<PathBuf>,
    /// Atlas image dimensions (width, height in pixels)
    pub atlas_image_size: Option<(u32, u32)>,
    /// Atlas grid dimensions (columns, rows)
    pub atlas_grid_size: Option<(u32, u32)>,
    /// Cell size in the atlas (width, height in pixels)
    pub atlas_cell_size: Option<(u32, u32)>,
    /// Viewport for the atlas canvas view
    pub atlas_viewport: AtlasViewport,
    /// Preview zoom level
    pub preview_zoom: f32,
    /// Show grid overlay on preview
    pub show_grid: bool,
    /// Dialog flags
    pub show_load_dialog: bool,
    pub show_new_clip_dialog: bool,
    /// New clip state name input
    pub new_clip_state_input: String,
    /// Discovered entity definitions for load dialog
    pub discovered_entities: Vec<String>,
}

impl std::fmt::Debug for AnimationEditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationEditorState")
            .field("active_entity", &self.active_entity)
            .field("entity_file_path", &self.entity_file_path)
            .field("authoring", &self.authoring)
            .field("preview", &self.preview)
            .field("atlas_texture", &self.atlas_texture.as_ref().map(|_| "TextureHandle"))
            .field("atlas_texture_path", &self.atlas_texture_path)
            .field("atlas_image_size", &self.atlas_image_size)
            .field("atlas_grid_size", &self.atlas_grid_size)
            .field("atlas_cell_size", &self.atlas_cell_size)
            .field("atlas_viewport", &self.atlas_viewport)
            .field("preview_zoom", &self.preview_zoom)
            .field("show_grid", &self.show_grid)
            .finish()
    }
}

impl AnimationEditorState {
    pub fn new() -> Self {
        Self {
            preview_zoom: 2.0,
            show_grid: true,
            new_clip_state_input: String::new(),
            ..Default::default()
        }
    }

    /// Load an entity definition for editing
    pub fn load_entity(&mut self, entity_name: &str, file_path: PathBuf, authoring: AnimationAuthoringState) {
        self.active_entity = Some(entity_name.to_string());
        self.entity_file_path = Some(file_path);
        self.authoring = authoring;
        self.preview.stop();
        self.clear_atlas_cache();
    }

    /// Unload the current entity
    pub fn unload(&mut self) {
        self.active_entity = None;
        self.entity_file_path = None;
        self.authoring = AnimationAuthoringState::default();
        self.preview.stop();
        self.clear_atlas_cache();
    }

    /// Check if an entity is loaded
    pub fn has_entity(&self) -> bool {
        self.active_entity.is_some()
    }

    /// Clear cached atlas texture
    pub fn clear_atlas_cache(&mut self) {
        self.atlas_texture = None;
        self.atlas_texture_path = None;
        self.atlas_image_size = None;
        self.atlas_grid_size = None;
        self.atlas_cell_size = None;
        self.atlas_viewport = AtlasViewport::default();
    }

    /// Get the currently selected clip for preview
    pub fn selected_clip(&self) -> Option<&AuthoredClip> {
        self.authoring.selected_clip()
    }

    /// Get frame count for current clip
    pub fn frame_count(&self) -> usize {
        self.selected_clip().map(|c| c.frames.len()).unwrap_or(0)
    }

    /// Check if preview is currently playing
    pub fn is_playing(&self) -> bool {
        self.preview.playing
    }

    /// Get current frame position in atlas
    pub fn current_frame_position(&self) -> Option<[u32; 2]> {
        let clip = self.selected_clip()?;
        let frame = clip.frames.get(self.preview.current_frame)?;
        Some(frame.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AnimationPreviewState tests

    #[test]
    fn preview_state_defaults_to_paused() {
        let state = AnimationPreviewState::new();
        assert!(!state.playing);
        assert_eq!(state.current_frame, 0);
        assert_eq!(state.speed, 1.0);
    }

    #[test]
    fn preview_play_starts_playback() {
        let mut state = AnimationPreviewState::new();
        state.play();
        assert!(state.playing);
    }

    #[test]
    fn preview_pause_stops_playback() {
        let mut state = AnimationPreviewState::new();
        state.play();
        state.pause();
        assert!(!state.playing);
    }

    #[test]
    fn preview_toggle_flips_playback() {
        let mut state = AnimationPreviewState::new();
        assert!(!state.playing);
        state.toggle_playback();
        assert!(state.playing);
        state.toggle_playback();
        assert!(!state.playing);
    }

    #[test]
    fn preview_stop_resets_state() {
        let mut state = AnimationPreviewState::new();
        state.play();
        state.current_frame = 5;
        state.elapsed_time = 0.5;
        state.stop();
        assert!(!state.playing);
        assert_eq!(state.current_frame, 0);
        assert_eq!(state.elapsed_time, 0.0);
    }

    #[test]
    fn preview_go_to_frame_sets_frame() {
        let mut state = AnimationPreviewState::new();
        state.go_to_frame(3);
        assert_eq!(state.current_frame, 3);
        assert_eq!(state.elapsed_time, 0.0);
    }

    #[test]
    fn preview_step_forward_wraps() {
        let mut state = AnimationPreviewState::new();
        state.current_frame = 2;
        state.step_forward(3); // 3 frames total
        assert_eq!(state.current_frame, 0); // wraps to 0
    }

    #[test]
    fn preview_step_backward_wraps() {
        let mut state = AnimationPreviewState::new();
        state.current_frame = 0;
        state.step_backward(3); // 3 frames total
        assert_eq!(state.current_frame, 2); // wraps to last frame
    }

    #[test]
    fn preview_update_advances_frame_when_time_exceeds_duration() {
        let mut state = AnimationPreviewState::new();
        state.play();

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0; // 0.1 seconds

        // First update - not enough time
        let changed = state.update(0.05, &clip); // 50ms
        assert!(!changed);
        assert_eq!(state.current_frame, 0);

        // Second update - enough time to advance
        let changed = state.update(0.06, &clip); // 60ms more = 110ms total
        assert!(changed);
        assert_eq!(state.current_frame, 1);
    }

    #[test]
    fn preview_update_respects_loop_mode_once() {
        let mut state = AnimationPreviewState::new();
        state.play();

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0;
        clip.loop_mode = "once".to_string();

        // Advance to last frame
        state.current_frame = 1;
        state.update(0.2, &clip); // Should stop at last frame

        assert!(!state.playing); // Playback should stop
        assert_eq!(state.current_frame, 1); // Should stay on last frame
    }

    #[test]
    fn preview_update_loops_in_loop_mode() {
        let mut state = AnimationPreviewState::new();
        state.play();

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0;
        clip.loop_mode = "loop".to_string();

        // Advance to last frame
        state.current_frame = 1;
        state.update(0.2, &clip); // Should loop back

        assert!(state.playing);
        assert_eq!(state.current_frame, 0);
    }

    #[test]
    fn preview_update_applies_speed_multiplier() {
        let mut state = AnimationPreviewState::new();
        state.play();
        state.speed = 2.0; // Double speed

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0;

        // At 2x speed, 50ms of real time = 100ms of animation time
        let changed = state.update(0.05, &clip);
        assert!(changed);
        assert_eq!(state.current_frame, 1);
    }

    // AnimationEditorState tests

    #[test]
    fn editor_state_defaults() {
        let state = AnimationEditorState::new();
        assert!(state.active_entity.is_none());
        assert!(!state.has_entity());
        assert_eq!(state.preview_zoom, 2.0);
        assert!(state.show_grid);
    }

    #[test]
    fn editor_load_entity() {
        let mut state = AnimationEditorState::new();
        let authoring = AnimationAuthoringState::new();

        state.load_entity("test_entity", PathBuf::from("/test/path"), authoring);

        assert!(state.has_entity());
        assert_eq!(state.active_entity, Some("test_entity".to_string()));
        assert_eq!(state.entity_file_path, Some(PathBuf::from("/test/path")));
    }

    #[test]
    fn editor_unload_clears_state() {
        let mut state = AnimationEditorState::new();
        let authoring = AnimationAuthoringState::new();
        state.load_entity("test", PathBuf::from("/test"), authoring);

        state.unload();

        assert!(!state.has_entity());
        assert!(state.active_entity.is_none());
        assert!(state.entity_file_path.is_none());
    }

    #[test]
    fn editor_frame_count_returns_clip_frames() {
        let mut state = AnimationEditorState::new();
        state.authoring.create_clip("test");
        state.authoring.add_frame_to_selected(0, 0);
        state.authoring.add_frame_to_selected(1, 0);
        state.authoring.add_frame_to_selected(2, 0);

        assert_eq!(state.frame_count(), 3);
    }

    #[test]
    fn editor_current_frame_position() {
        let mut state = AnimationEditorState::new();
        state.authoring.create_clip("test");
        state.authoring.add_frame_to_selected(5, 3);

        let pos = state.current_frame_position();
        assert_eq!(pos, Some([5, 3]));
    }

    #[test]
    fn editor_is_playing_reflects_preview_state() {
        let mut state = AnimationEditorState::new();
        assert!(!state.is_playing());

        state.preview.play();
        assert!(state.is_playing());
    }
}
