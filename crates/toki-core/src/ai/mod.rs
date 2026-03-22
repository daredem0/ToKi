//! AI system for entity behavior.
//!
//! This module provides the runtime AI system that updates entity positions
//! based on their authored AI configuration.
//!
//! # Module Structure
//!
//! - `constants`: AI behavior constants (tile size, update frequency, etc.)
//! - `types`: Core types (WanderPhase, AiRuntimeState, AiUpdateResult, etc.)
//! - `context`: AiContext for bundling movement operation parameters
//! - `movement`: Movement computation helpers
//! - `behaviors`: Behavior trait and handler implementations
//! - `system`: Core AiSystem implementation
//! - `run_and_multiply`: RunAndMultiply specific behavior

pub mod behaviors;
pub mod constants;
pub mod context;
pub mod movement;
mod run_and_multiply;
pub mod system;
pub mod types;

// Re-export commonly used items
pub use behaviors::{BehaviorHandler, BehaviorUpdate, ChaseHandler, RunHandler, WanderHandler};
pub use constants::{
    IDLE_WAIT_MAX_FRAMES, IDLE_WAIT_MIN_FRAMES, INITIAL_WAIT_MAX_FRAMES, INITIAL_WAIT_MIN_FRAMES,
    TILE_SIZE_PX, WANDER_MAX_TILES, WANDER_MIN_TILES, WANDER_SPEED_MULTIPLIER,
    WANDER_UPDATE_FREQUENCY,
};
pub use context::AiContext;
pub use system::AiSystem;
pub use types::{
    AiRuntimeState, AiSpawnRequest, AiUpdateResult, SeparationState, SpawnMode, WanderPhase,
};

// Test-only helpers
#[cfg(test)]
impl AiSystem {
    /// Set frame counter for testing
    pub fn set_frame_counter(&mut self, value: u64) {
        self.frame_counter = value;
    }

    /// Get frame counter for testing
    pub fn frame_counter(&self) -> u64 {
        self.frame_counter
    }
}

#[cfg(test)]
#[path = "../ai_tests.rs"]
mod tests;
