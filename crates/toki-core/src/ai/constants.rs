//! AI system constants.

/// Standard tile size in pixels used for movement calculations.
pub const TILE_SIZE_PX: i32 = 16;

/// Wander behavior update frequency in frames.
/// Lower values = more frequent updates = more erratic movement.
pub const WANDER_UPDATE_FREQUENCY: u64 = 60;

/// Speed multiplier for wander behavior (compensates for less frequent updates).
pub const WANDER_SPEED_MULTIPLIER: f32 = 5.0;

/// Minimum tiles to walk during wander movement.
pub const WANDER_MIN_TILES: u32 = 1;

/// Maximum tiles to walk during wander movement.
pub const WANDER_MAX_TILES: u32 = 3;

/// Minimum frames to wait when idle (at 60fps, ~0.5 seconds).
pub const IDLE_WAIT_MIN_FRAMES: u32 = 30;

/// Maximum frames to wait when idle (at 60fps, ~3 seconds).
pub const IDLE_WAIT_MAX_FRAMES: u32 = 180;

/// Minimum frames for initial entity wait (at 60fps, ~0.5 seconds).
pub const INITIAL_WAIT_MIN_FRAMES: u32 = 30;

/// Maximum frames for initial entity wait (at 60fps, ~1.5 seconds).
pub const INITIAL_WAIT_MAX_FRAMES: u32 = 90;
