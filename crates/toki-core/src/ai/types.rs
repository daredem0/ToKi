//! AI type definitions.

use crate::animation::AnimationState;
use crate::entity::EntityId;
use glam::IVec2;

use super::constants::{INITIAL_WAIT_MAX_FRAMES, INITIAL_WAIT_MIN_FRAMES};

/// Wandering state for idle AI behavior.
#[derive(Debug, Clone, Default)]
pub enum WanderPhase {
    /// Entity is waiting/idle for a number of frames
    #[default]
    Waiting,
    /// Entity is walking in a direction for remaining distance (in pixels)
    Walking {
        direction: IVec2,
        remaining_distance: i32,
    },
}

/// State for RunAndMultiply separation phase after spawning.
#[derive(Debug, Clone)]
pub struct SeparationState {
    /// The entities we're separating from
    pub other_entity_ids: Vec<EntityId>,
    /// Required distance to exit separation (detection_radius * 2)
    pub required_distance: f32,
}

/// A request to spawn a new entity.
#[derive(Debug, Clone)]
pub struct AiSpawnRequest {
    /// Position to spawn at (pixels)
    pub position: IVec2,
    /// Parent entity IDs (for setting up separation state on the spawned entity)
    pub parent_entity_ids: Vec<EntityId>,
    /// Required separation distance for the spawned entity
    pub separation_distance: f32,
    /// Spawn mode: clone from existing entity or create from definition
    pub mode: SpawnMode,
}

/// How to spawn a new entity.
#[derive(Debug, Clone)]
pub enum SpawnMode {
    /// Clone an existing entity (copies all attributes including AI config)
    Clone { source_entity_id: EntityId },
    /// Create from an entity definition
    FromDefinition { definition_name: String },
}

/// Runtime AI state for an entity.
/// This is separate from the authored `AiConfig` and tracks transient runtime data.
#[derive(Debug, Clone)]
pub struct AiRuntimeState {
    /// Frame counter for update frequency control
    pub frame_counter: u64,
    /// Current wandering phase for idle behavior
    pub wander_phase: WanderPhase,
    /// Frames remaining in current wait period
    pub wait_frames_remaining: u32,
    /// RunAndMultiply: entity we're currently seeking to mate with
    pub seeking_mate: Option<EntityId>,
    /// RunAndMultiply: separation state after spawning
    pub separation_state: Option<SeparationState>,
}

impl Default for AiRuntimeState {
    fn default() -> Self {
        Self {
            frame_counter: 0,
            wander_phase: WanderPhase::Waiting,
            // Start with random wait so entities don't all move at once
            wait_frames_remaining: fastrand::u32(INITIAL_WAIT_MIN_FRAMES..=INITIAL_WAIT_MAX_FRAMES),
            seeking_mate: None,
            separation_state: None,
        }
    }
}

/// Result of an AI update for a single entity.
#[derive(Debug, Clone)]
pub struct AiUpdateResult {
    pub entity_id: EntityId,
    pub new_position: Option<IVec2>,
    pub new_animation: Option<AnimationState>,
    pub movement_distance: f32,
    /// Optional spawn request (used by RunAndMultiply)
    pub spawn_request: Option<AiSpawnRequest>,
}
