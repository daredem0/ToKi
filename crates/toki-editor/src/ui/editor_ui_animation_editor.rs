// Animation editor state for the dedicated animation editor tab
// Provides visual animation editing with preview playback

use super::editor_ui_animation_authoring::{AnimationAuthoringState, AuthoredClip};
use std::path::PathBuf;
use toki_core::animation::ClipPlayback;

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
    pub fn screen_to_canvas(
        &self,
        screen_pos: glam::Vec2,
        viewport_rect: egui::Rect,
    ) -> glam::Vec2 {
        let viewport_pos = screen_pos - glam::Vec2::new(viewport_rect.left(), viewport_rect.top());
        viewport_pos / self.zoom + self.pan
    }
}

/// Playback state for animation preview.
/// Wraps the shared ClipPlayback from toki-core for consistent behavior
/// between editor preview and runtime animation playback.
#[derive(Debug, Clone, Default)]
pub struct AnimationPreviewState {
    /// Core playback state (shared with runtime)
    playback: ClipPlayback,
}

impl AnimationPreviewState {
    /// Whether the animation is currently playing
    pub fn playing(&self) -> bool {
        self.playback.playing
    }

    /// Current frame index in the active clip
    pub fn current_frame(&self) -> usize {
        self.playback.current_frame
    }

    /// Playback speed multiplier (1.0 = normal)
    pub fn speed(&self) -> f32 {
        self.playback.speed
    }

    /// Set playback speed multiplier
    pub fn set_speed(&mut self, speed: f32) {
        self.playback.speed = speed;
    }

    /// Toggle play/pause
    pub fn toggle_playback(&mut self) {
        self.playback.toggle();
    }

    /// Stop playback and reset
    pub fn stop(&mut self) {
        self.playback.stop();
    }

    /// Update playback state given delta time and clip info.
    /// Returns true if frame changed.
    pub fn update(&mut self, delta_seconds: f32, clip: &AuthoredClip) -> bool {
        use toki_core::animation::PlaybackEvent;

        if clip.frames.is_empty() {
            return false;
        }

        // Convert seconds to milliseconds for ClipPlayback
        let delta_ms = delta_seconds * 1000.0;
        let loop_mode = clip.loop_mode_enum();

        let event = self.playback.update(
            delta_ms,
            clip.frame_count(),
            |i| clip.frame_duration_at(i),
            &loop_mode,
        );

        matches!(
            event,
            PlaybackEvent::FrameChanged { .. }
                | PlaybackEvent::LoopCompleted
                | PlaybackEvent::Finished
        )
    }

    /// Go to specific frame
    pub fn go_to_frame(&mut self, frame: usize, frame_count: usize) {
        self.playback.go_to_frame(frame, frame_count);
    }

    /// Step to next frame (manual)
    pub fn step_forward(&mut self, frame_count: usize) {
        self.playback.step_forward(frame_count);
    }

    /// Step to previous frame (manual)
    pub fn step_backward(&mut self, frame_count: usize) {
        self.playback.step_backward(frame_count);
    }

    /// Get normalized progress through current frame (0.0 to 1.0)
    pub fn frame_progress(&self, clip: &AuthoredClip) -> f32 {
        self.playback.frame_progress(|i| clip.frame_duration_at(i))
    }
}

/// State for the animation editor tab
#[derive(Clone)]
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
    pub show_new_clip_dialog: bool,
    /// New clip state name input
    pub new_clip_state_input: String,
    // Panel layout sizes (draggable dividers)
    /// Width of the clip list panel (left)
    pub clip_list_width: f32,
    /// Width of the frame sequence panel (right)
    pub frame_sequence_width: f32,
    /// Height of the preview area (center top)
    pub preview_height: f32,
    /// Height ratio of clip list vs default state selector (0.0-1.0)
    pub clip_list_ratio: f32,
}

impl std::fmt::Debug for AnimationEditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationEditorState")
            .field("active_entity", &self.active_entity)
            .field("entity_file_path", &self.entity_file_path)
            .field("authoring", &self.authoring)
            .field("preview", &self.preview)
            .field(
                "atlas_texture",
                &self.atlas_texture.as_ref().map(|_| "TextureHandle"),
            )
            .field("atlas_texture_path", &self.atlas_texture_path)
            .field("atlas_image_size", &self.atlas_image_size)
            .field("atlas_grid_size", &self.atlas_grid_size)
            .field("atlas_cell_size", &self.atlas_cell_size)
            .field("atlas_viewport", &self.atlas_viewport)
            .field("preview_zoom", &self.preview_zoom)
            .field("show_grid", &self.show_grid)
            .field("clip_list_width", &self.clip_list_width)
            .field("frame_sequence_width", &self.frame_sequence_width)
            .field("preview_height", &self.preview_height)
            .field("clip_list_ratio", &self.clip_list_ratio)
            .finish()
    }
}

impl Default for AnimationEditorState {
    fn default() -> Self {
        Self {
            active_entity: None,
            entity_file_path: None,
            authoring: AnimationAuthoringState::default(),
            preview: AnimationPreviewState::default(),
            atlas_texture: None,
            atlas_texture_path: None,
            atlas_image_size: None,
            atlas_grid_size: None,
            atlas_cell_size: None,
            atlas_viewport: AtlasViewport::default(),
            preview_zoom: 2.0,
            show_grid: true,
            show_new_clip_dialog: false,
            new_clip_state_input: String::new(),
            clip_list_width: 180.0,
            frame_sequence_width: 200.0,
            preview_height: 180.0,
            clip_list_ratio: 0.7,
        }
    }
}

impl AnimationEditorState {
    /// Load an entity definition for editing
    pub fn load_entity(
        &mut self,
        entity_name: &str,
        file_path: PathBuf,
        authoring: AnimationAuthoringState,
    ) {
        self.active_entity = Some(entity_name.to_string());
        self.entity_file_path = Some(file_path);
        self.authoring = authoring;
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
        self.preview.playing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // AnimationPreviewState tests

    #[test]
    fn preview_state_defaults_to_paused() {
        let state = AnimationPreviewState::default();
        assert!(!state.playing());
        assert_eq!(state.current_frame(), 0);
        assert_eq!(state.speed(), 1.0);
    }

    #[test]
    fn preview_toggle_starts_playback() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();
        assert!(state.playing());
    }

    #[test]
    fn preview_toggle_stops_playback() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();
        state.toggle_playback();
        assert!(!state.playing());
    }

    #[test]
    fn preview_toggle_flips_playback() {
        let mut state = AnimationPreviewState::default();
        assert!(!state.playing());
        state.toggle_playback();
        assert!(state.playing());
        state.toggle_playback();
        assert!(!state.playing());
    }

    #[test]
    fn preview_stop_resets_state() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();
        // Use go_to_frame to set position
        state.go_to_frame(5, 10);
        state.stop();
        assert!(!state.playing());
        assert_eq!(state.current_frame(), 0);
    }

    #[test]
    fn preview_go_to_frame_sets_frame() {
        let mut state = AnimationPreviewState::default();
        state.go_to_frame(3, 5);
        assert_eq!(state.current_frame(), 3);
    }

    #[test]
    fn preview_step_forward_wraps() {
        let mut state = AnimationPreviewState::default();
        state.go_to_frame(2, 3);
        state.step_forward(3); // 3 frames total
        assert_eq!(state.current_frame(), 0); // wraps to 0
    }

    #[test]
    fn preview_step_backward_wraps() {
        let mut state = AnimationPreviewState::default();
        state.step_backward(3); // 3 frames total
        assert_eq!(state.current_frame(), 2); // wraps to last frame
    }

    #[test]
    fn preview_update_advances_frame_when_time_exceeds_duration() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0; // 0.1 seconds

        // First update - not enough time
        let changed = state.update(0.05, &clip); // 50ms
        assert!(!changed);
        assert_eq!(state.current_frame(), 0);

        // Second update - enough time to advance
        let changed = state.update(0.06, &clip); // 60ms more = 110ms total
        assert!(changed);
        assert_eq!(state.current_frame(), 1);
    }

    #[test]
    fn preview_update_respects_loop_mode_once() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0;
        clip.loop_mode = "once".to_string();

        // Start at last frame
        state.go_to_frame(1, 2);
        state.toggle_playback(); // Re-enable playback after go_to_frame
        state.update(0.2, &clip); // Should stop at last frame

        assert!(!state.playing()); // Playback should stop
        assert_eq!(state.current_frame(), 1); // Should stay on last frame
    }

    #[test]
    fn preview_update_loops_in_loop_mode() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0;
        clip.loop_mode = "loop".to_string();

        // Start at last frame
        state.go_to_frame(1, 2);
        // 150ms: frame 1 completes (100ms), loops to frame 0, 50ms remaining
        state.update(0.15, &clip);

        assert!(state.playing());
        assert_eq!(state.current_frame(), 0);
    }

    #[test]
    fn preview_update_applies_speed_multiplier() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();
        state.set_speed(2.0); // Double speed

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.default_duration_ms = 100.0;

        // At 2x speed, 50ms of real time = 100ms of animation time
        let changed = state.update(0.05, &clip);
        assert!(changed);
        assert_eq!(state.current_frame(), 1);
    }

    #[test]
    fn preview_ping_pong_mode_works() {
        let mut state = AnimationPreviewState::default();
        state.toggle_playback();

        let mut clip = AuthoredClip::new("test");
        clip.add_frame(0, 0);
        clip.add_frame(1, 0);
        clip.add_frame(2, 0);
        clip.add_frame(3, 0);
        clip.default_duration_ms = 100.0;
        clip.loop_mode = "ping_pong".to_string();

        // Advance through: 0 -> 1 -> 2 -> 3 -> reverse to 2
        state.update(0.45, &clip); // 450ms
        assert_eq!(state.current_frame(), 2);
        assert!(state.playing());
    }

    // AnimationEditorState tests

    #[test]
    fn editor_state_defaults() {
        let state = AnimationEditorState::default();
        assert!(state.active_entity.is_none());
        assert!(!state.has_entity());
        assert_eq!(state.preview_zoom, 2.0);
        assert!(state.show_grid);
    }

    #[test]
    fn editor_load_entity() {
        let mut state = AnimationEditorState::default();
        let authoring = AnimationAuthoringState::default();

        state.load_entity("test_entity", PathBuf::from("/test/path"), authoring);

        assert!(state.has_entity());
        assert_eq!(state.active_entity, Some("test_entity".to_string()));
        assert_eq!(state.entity_file_path, Some(PathBuf::from("/test/path")));
    }

    #[test]
    fn editor_frame_count_returns_clip_frames() {
        let mut state = AnimationEditorState::default();
        state.authoring.create_clip("test");
        state.authoring.add_frame_to_selected(0, 0);
        state.authoring.add_frame_to_selected(1, 0);
        state.authoring.add_frame_to_selected(2, 0);

        assert_eq!(state.frame_count(), 3);
    }

    #[test]
    fn editor_is_playing_reflects_preview_state() {
        let mut state = AnimationEditorState::default();
        assert!(!state.is_playing());

        state.preview.toggle_playback();
        assert!(state.is_playing());
    }
}
