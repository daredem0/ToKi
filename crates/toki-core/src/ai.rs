//! AI system for entity behavior.
//!
//! This module provides the runtime AI system that updates entity positions
//! based on their authored AI configuration.

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::collision::can_entity_move_to_position;
use crate::entity::{AiBehavior, Entity, EntityId, EntityManager};
use glam::{IVec2, UVec2};
use std::collections::HashMap;

/// Context for AI movement operations, grouping related parameters.
///
/// This reduces parameter counts for AI methods by bundling commonly-used
/// references together: entity manager, world bounds, tilemap, and atlas.
#[derive(Clone, Copy)]
pub struct AiContext<'a> {
    pub entity_manager: &'a EntityManager,
    pub world_bounds: UVec2,
    pub tilemap: &'a TileMap,
    pub atlas: &'a AtlasMeta,
}

impl<'a> AiContext<'a> {
    /// Create a new AI context with all required references.
    pub fn new(
        entity_manager: &'a EntityManager,
        world_bounds: UVec2,
        tilemap: &'a TileMap,
        atlas: &'a AtlasMeta,
    ) -> Self {
        Self {
            entity_manager,
            world_bounds,
            tilemap,
            atlas,
        }
    }

    /// Compute maximum position for an entity of the given size.
    /// Returns (max_x, max_y) clamped to at least 0.
    pub fn max_position(&self, entity_size: UVec2) -> (i32, i32) {
        let max_x = (self.world_bounds.x as i32 - entity_size.x as i32).max(0);
        let max_y = (self.world_bounds.y as i32 - entity_size.y as i32).max(0);
        (max_x, max_y)
    }

    /// Check if movement to new position is valid (no collisions with tiles or entities).
    pub fn is_movement_valid(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        new_position: IVec2,
    ) -> bool {
        can_entity_move_to_position(entity, new_position, self.tilemap, self.atlas)
            && !self
                .entity_manager
                .would_collide_with_solid_entity(entity_id, new_position)
    }
}

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
            wait_frames_remaining: fastrand::u32(30..=90),
            seeking_mate: None,
            separation_state: None,
        }
    }
}

/// Manages AI state for all entities.
#[derive(Debug, Clone, Default)]
pub struct AiSystem {
    /// Per-entity runtime AI state
    entity_states: HashMap<EntityId, AiRuntimeState>,
    /// Global frame counter for periodic updates
    frame_counter: u64,
}

// ============================================================================
// Behavior Handlers - Strategy Pattern Implementation
// ============================================================================

/// Trait for AI behavior update logic.
/// Each behavior type implements this trait to provide polymorphic dispatch.
pub trait BehaviorUpdate {
    /// Update the entity's AI state and return movement/animation changes.
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult>;
}

/// Chase behavior handler - moves toward player when in detection radius.
#[derive(Debug, Clone, Copy)]
pub struct ChaseHandler;

impl BehaviorUpdate for ChaseHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let player_pos = player_position?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        let distance = AiSystem::distance_between(current_position, player_pos);

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return IdleWanderHandler.update(entity, entity_id, player_position, ctx, ai_state);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions =
            AiSystem::compute_directions_toward(current_position, player_pos, movement_step);

        AiSystem::try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }
}

/// Run behavior handler - moves away from player when in detection radius.
#[derive(Debug, Clone, Copy)]
pub struct RunHandler;

impl BehaviorUpdate for RunHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let player_pos = player_position?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        let distance = AiSystem::distance_between(current_position, player_pos);

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return IdleWanderHandler.update(entity, entity_id, player_position, ctx, ai_state);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions =
            AiSystem::compute_directions_away(current_position, player_pos, movement_step);

        AiSystem::try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }
}

/// Wander behavior handler - random movement every 60 frames.
#[derive(Debug, Clone, Copy)]
pub struct WanderHandler {
    frame_counter: u64,
}

impl WanderHandler {
    pub fn new(frame_counter: u64) -> Self {
        Self { frame_counter }
    }
}

impl BehaviorUpdate for WanderHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        _player_position: Option<IVec2>,
        ctx: &AiContext,
        _ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        // Wander only updates every 60 frames
        if !self.frame_counter.is_multiple_of(60) {
            return None;
        }

        let current_position = entity.position;
        let movement_step = (entity.attributes.speed * 5.0).round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

        let random_direction = fastrand::u32(0..5);
        let new_position = match random_direction {
            0 => IVec2::new(current_position.x, (current_position.y - movement_step).max(0)),
            1 => IVec2::new(current_position.x, (current_position.y + movement_step).min(max_y)),
            2 => IVec2::new((current_position.x - movement_step).max(0), current_position.y),
            3 => IVec2::new((current_position.x + movement_step).min(max_x), current_position.y),
            _ => current_position,
        };

        let entity_moved =
            new_position != current_position && ctx.is_movement_valid(entity, entity_id, new_position);

        AiSystem::build_wander_result(entity_id, current_position, new_position, entity_moved)
    }
}

/// Idle wander handler - state machine for waiting/walking when not pursuing.
#[derive(Debug, Clone, Copy)]
pub struct IdleWanderHandler;

impl BehaviorUpdate for IdleWanderHandler {
    fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        _player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let current_position = entity.position;

        match &ai_state.wander_phase {
            WanderPhase::Waiting => {
                if ai_state.wait_frames_remaining > 0 {
                    ai_state.wait_frames_remaining -= 1;
                    return Some(AiUpdateResult {
                        entity_id,
                        new_position: None,
                        new_animation: Some(AnimationState::Idle),
                        movement_distance: 0.0,
                        spawn_request: None,
                    });
                }

                // Done waiting - start walking
                let direction = AiSystem::random_cardinal_direction();
                let tiles = fastrand::u32(1..=3);
                let tile_size = 16;

                ai_state.wander_phase = WanderPhase::Walking {
                    direction,
                    remaining_distance: (tiles * tile_size) as i32,
                };

                Some(AiUpdateResult {
                    entity_id,
                    new_position: None,
                    new_animation: Some(AnimationState::Walk),
                    movement_distance: 0.0,
                    spawn_request: None,
                })
            }
            WanderPhase::Walking {
                direction,
                remaining_distance,
            } => {
                let dir = *direction;
                let remaining = *remaining_distance;
                self.handle_walking(entity, entity_id, current_position, dir, remaining, ctx, ai_state)
            }
        }
    }
}

impl IdleWanderHandler {
    #[allow(clippy::too_many_arguments)]
    fn handle_walking(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        current_position: IVec2,
        direction: IVec2,
        remaining_distance: i32,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        let movement_step = entity.attributes.speed.round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

        let scaled_direction = IVec2::new(direction.x * movement_step, direction.y * movement_step);
        let new_position = IVec2::new(
            (current_position.x + scaled_direction.x).clamp(0, max_x),
            (current_position.y + scaled_direction.y).clamp(0, max_y),
        );

        let can_move =
            new_position != current_position && ctx.is_movement_valid(entity, entity_id, new_position);

        let new_remaining = remaining_distance - movement_step;

        if can_move && new_remaining > 0 {
            ai_state.wander_phase = WanderPhase::Walking {
                direction,
                remaining_distance: new_remaining,
            };
            return Some(AiSystem::build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ));
        }

        // Transition to waiting
        ai_state.wander_phase = WanderPhase::Waiting;
        ai_state.wait_frames_remaining = fastrand::u32(30..=180);

        if can_move {
            Some(AiSystem::build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ))
        } else {
            Some(AiUpdateResult {
                entity_id,
                new_position: None,
                new_animation: Some(AnimationState::Idle),
                movement_distance: 0.0,
                spawn_request: None,
            })
        }
    }
}

/// Enum wrapper for behavior handlers enabling factory method.
#[derive(Debug)]
pub enum BehaviorHandler {
    Chase(ChaseHandler),
    Run(RunHandler),
    Wander(WanderHandler),
}

impl BehaviorHandler {
    /// Create a handler for the given behavior type.
    /// Returns None for behaviors that don't need active updates (e.g., None).
    pub fn for_behavior(behavior: AiBehavior, frame_counter: u64) -> Option<Self> {
        match behavior {
            AiBehavior::Chase => Some(Self::Chase(ChaseHandler)),
            AiBehavior::Run => Some(Self::Run(RunHandler)),
            AiBehavior::Wander => Some(Self::Wander(WanderHandler::new(frame_counter))),
            AiBehavior::RunAndMultiply => None, // Complex behavior still handled in AiSystem
            AiBehavior::None => None,
        }
    }

    /// Delegate to the appropriate handler.
    pub fn update(
        &self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
        ai_state: &mut AiRuntimeState,
    ) -> Option<AiUpdateResult> {
        match self {
            Self::Chase(h) => h.update(entity, entity_id, player_position, ctx, ai_state),
            Self::Run(h) => h.update(entity, entity_id, player_position, ctx, ai_state),
            Self::Wander(h) => h.update(entity, entity_id, player_position, ctx, ai_state),
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

impl AiSystem {
    /// Create a new AI system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all runtime AI state.
    pub fn reset(&mut self) {
        self.entity_states.clear();
        self.frame_counter = 0;
    }

    /// Update AI for all entities.
    /// Returns a list of updates to apply to entities.
    pub fn update(
        &mut self,
        entity_manager: &EntityManager,
        player_id: Option<EntityId>,
        world_bounds: UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<AiUpdateResult> {
        self.frame_counter += 1;

        let player_position = player_id
            .and_then(|id| entity_manager.get_entity(id))
            .map(|e| e.position);

        let ctx = AiContext::new(entity_manager, world_bounds, tilemap, atlas);
        let mut results = Vec::new();

        // Collect entities with active AI behaviors
        let ai_entities: Vec<_> = entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                // Skip player
                if Some(entity_id) == player_id {
                    return None;
                }

                let entity = entity_manager.get_entity(entity_id)?;
                let behavior = entity.attributes.ai_config.behavior;
                if matches!(
                    behavior,
                    AiBehavior::Wander
                        | AiBehavior::Chase
                        | AiBehavior::Run
                        | AiBehavior::RunAndMultiply
                ) {
                    Some((entity_id, behavior))
                } else {
                    None
                }
            })
            .collect();

        for (entity_id, behavior) in ai_entities {
            let result = match behavior {
                AiBehavior::Wander => self.update_wander_entity(entity_id, &ctx),
                AiBehavior::Chase => self.update_chase_entity(entity_id, player_position, &ctx),
                AiBehavior::Run => self.update_run_entity(entity_id, player_position, &ctx),
                AiBehavior::RunAndMultiply => {
                    self.update_run_and_multiply_entity(entity_id, player_position, &ctx)
                }
                _ => None,
            };
            if let Some(r) = result {
                results.push(r);
            }
        }

        results
    }

    fn update_wander_entity(
        &mut self,
        entity_id: EntityId,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        // Wander only updates every 60 frames to avoid chaotic movement
        if !self.frame_counter.is_multiple_of(60) {
            return None;
        }

        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let current_position = entity.position;

        // Wander uses larger steps since it updates less frequently
        let movement_step = (entity.attributes.speed * 5.0).round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

        // Choose random direction: 0=up, 1=down, 2=left, 3=right, 4=stay
        let random_direction = fastrand::u32(0..5);

        let new_position = match random_direction {
            0 => IVec2::new(
                current_position.x,
                (current_position.y - movement_step).max(0),
            ),
            1 => IVec2::new(
                current_position.x,
                (current_position.y + movement_step).min(max_y),
            ),
            2 => IVec2::new(
                (current_position.x - movement_step).max(0),
                current_position.y,
            ),
            3 => IVec2::new(
                (current_position.x + movement_step).min(max_x),
                current_position.y,
            ),
            _ => current_position, // Stay in place
        };

        let entity_moved =
            new_position != current_position && ctx.is_movement_valid(entity, entity_id, new_position);

        let final_position = if entity_moved {
            new_position
        } else {
            current_position
        };

        let movement_distance = if entity_moved {
            let dx = (final_position.x - current_position.x) as f32;
            let dy = (final_position.y - current_position.y) as f32;
            (dx * dx + dy * dy).sqrt()
        } else {
            0.0
        };

        let desired_animation = if entity_moved {
            AnimationState::Walk
        } else {
            AnimationState::Idle
        };

        Some(AiUpdateResult {
            entity_id,
            new_position: if entity_moved {
                Some(final_position)
            } else {
                None
            },
            new_animation: Some(desired_animation),
            movement_distance,
            spawn_request: None,
        })
    }

    fn update_chase_entity(
        &mut self,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let player_pos = player_position?;

        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        // Calculate distance to player
        let dx = player_pos.x - current_position.x;
        let dy = player_pos.y - current_position.y;
        let distance = ((dx * dx + dy * dy) as f32).sqrt();

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return self.idle_wander(entity, entity_id, ctx);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions =
            Self::compute_directions_toward(current_position, player_pos, movement_step);

        Self::try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }

    fn update_run_entity(
        &mut self,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let player_pos = player_position?;

        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;

        // Calculate distance to player
        let dx = player_pos.x - current_position.x;
        let dy = player_pos.y - current_position.y;
        let distance = ((dx * dx + dy * dy) as f32).sqrt();

        // If player is outside detection radius, wander randomly
        if distance > detection_radius as f32 {
            return self.idle_wander(entity, entity_id, ctx);
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions = Self::compute_directions_away(current_position, player_pos, movement_step);

        Self::try_movement_with_fallback(entity, entity_id, current_position, &directions, ctx)
    }

    /// Update RunAndMultiply entity behavior.
    /// Priority: separation > flee from player > seek mate > idle wander
    fn update_run_and_multiply_entity(
        &mut self,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;
        let definition_name = entity.definition_name.clone();

        // Handle separation state first
        if let Some(result) = self.handle_separation(entity, entity_id, ctx) {
            return Some(result);
        }

        let player_in_range =
            self.is_player_in_range(player_position, current_position, detection_radius);
        let mate = self.find_compatible_entity(
            entity_id,
            &definition_name,
            ctx.entity_manager,
            detection_radius,
        );

        // Check for mating collision
        if let Some(mate_id) = mate {
            if let Some(result) = self.handle_mating_collision(
                entity,
                entity_id,
                mate_id,
                ctx.entity_manager,
                detection_radius,
            ) {
                return Some(result);
            }
        }

        // Flee from player if in range
        if player_in_range {
            return self.flee_and_seek(entity, entity_id, player_position, mate, ctx);
        }

        // Seek mate if one exists
        if let Some(mate_id) = mate {
            return self.seek_entity(entity, entity_id, mate_id, ctx);
        }

        // No threats or mates - idle wander
        self.idle_wander(entity, entity_id, ctx)
    }

    /// Check if player is within detection radius.
    fn is_player_in_range(
        &self,
        player_pos: Option<IVec2>,
        entity_pos: IVec2,
        radius: u32,
    ) -> bool {
        player_pos.is_some_and(|pos| Self::distance_between(entity_pos, pos) <= radius as f32)
    }

    /// Calculate distance between two positions.
    fn distance_between(a: IVec2, b: IVec2) -> f32 {
        let dx = (b.x - a.x) as f32;
        let dy = (b.y - a.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Find a compatible entity (same definition_name and entity_kind) within detection radius.
    fn find_compatible_entity(
        &self,
        entity_id: EntityId,
        definition_name: &Option<String>,
        entity_manager: &EntityManager,
        detection_radius: u32,
    ) -> Option<EntityId> {
        let def_name = definition_name.as_ref()?;
        let entity = entity_manager.get_entity(entity_id)?;
        let current_pos = entity.position;
        let entity_kind = entity.entity_kind;

        entity_manager
            .active_entities()
            .iter()
            .filter(|&&other_id| other_id != entity_id)
            .filter_map(|&other_id| {
                let other = entity_manager.get_entity(other_id)?;
                // Must have same definition_name AND same entity_kind (excludes player, items, etc.)
                let other_def = other.definition_name.as_ref()?;
                if other_def == def_name
                    && other.entity_kind == entity_kind
                    && !self.is_entity_separating(other_id)
                {
                    let dist = Self::distance_between(current_pos, other.position);
                    if dist <= detection_radius as f32 {
                        return Some(other_id);
                    }
                }
                None
            })
            .next()
    }

    /// Check if an entity is currently in separation state.
    fn is_entity_separating(&self, entity_id: EntityId) -> bool {
        self.entity_states
            .get(&entity_id)
            .is_some_and(|state| state.separation_state.is_some())
    }

    /// Handle separation state - move away from all tracked entities until distance threshold is met.
    fn handle_separation(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let state = self.entity_states.get(&entity_id)?;
        let separation = state.separation_state.as_ref()?;
        let other_ids = separation.other_entity_ids.clone();
        let required_distance = separation.required_distance;

        // Find the closest entity we need to separate from
        let closest = other_ids
            .iter()
            .filter_map(|&id| {
                let other = ctx.entity_manager.get_entity(id)?;
                let dist = Self::distance_between(entity.position, other.position);
                Some((id, other.position, dist))
            })
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        let Some((_, closest_pos, _)) = closest else {
            // No valid entities to separate from - clear state
            let state = self.entity_states.get_mut(&entity_id)?;
            state.separation_state = None;
            return None;
        };

        // Check if ALL entities are far enough away
        let all_separated = other_ids.iter().all(|&id| {
            ctx.entity_manager
                .get_entity(id)
                .map(|other| {
                    Self::distance_between(entity.position, other.position) >= required_distance
                })
                .unwrap_or(true) // Treat missing entities as separated
        });

        if all_separated {
            // Separation complete
            let state = self.entity_states.get_mut(&entity_id)?;
            state.separation_state = None;
            return None;
        }

        // Continue moving away from the closest entity
        let movement_step = entity.attributes.speed.round() as i32;
        let directions = Self::compute_directions_away(entity.position, closest_pos, movement_step);

        Self::try_movement_with_fallback(entity, entity_id, entity.position, &directions, ctx)
    }

    /// Find a free position to spawn a new entity adjacent to the parents.
    /// Checks cardinal directions around both parents for a free tile.
    fn find_free_spawn_position(
        entity: &crate::entity::Entity,
        mate: &crate::entity::Entity,
        entity_manager: &EntityManager,
    ) -> Option<IVec2> {
        let tile_size = 16i32;
        let size = entity.size;
        let offsets = [
            IVec2::new(tile_size, 0),  // Right
            IVec2::new(-tile_size, 0), // Left
            IVec2::new(0, tile_size),  // Down
            IVec2::new(0, -tile_size), // Up
        ];

        // Check positions around both parents
        for parent_pos in [entity.position, mate.position] {
            for offset in &offsets {
                let candidate = parent_pos + *offset;
                if candidate.x >= 0
                    && candidate.y >= 0
                    && entity_manager.is_spawn_position_free(candidate, size)
                {
                    return Some(candidate);
                }
            }
        }
        None
    }

    /// Handle mating collision - check adjacency and trigger spawn.
    fn handle_mating_collision(
        &mut self,
        entity: &crate::entity::Entity,
        entity_id: EntityId,
        mate_id: EntityId,
        entity_manager: &EntityManager,
        detection_radius: u32,
    ) -> Option<AiUpdateResult> {
        let mate = entity_manager.get_entity(mate_id)?;
        if !Self::entities_adjacent(entity, mate) {
            return None;
        }

        // Find a free spawn position adjacent to the parents
        let spawn_pos = Self::find_free_spawn_position(entity, mate, entity_manager)?;

        // Enter separation state for BOTH parent entities
        let required_distance = (detection_radius * 2) as f32;
        self.enter_separation_state(entity_id, vec![mate_id], required_distance);
        self.enter_separation_state(mate_id, vec![entity_id], required_distance);

        Some(AiUpdateResult {
            entity_id,
            new_position: None,
            new_animation: Some(AnimationState::Idle),
            movement_distance: 0.0,
            spawn_request: Some(AiSpawnRequest {
                position: spawn_pos,
                parent_entity_ids: vec![entity_id, mate_id],
                separation_distance: required_distance,
                mode: SpawnMode::Clone {
                    source_entity_id: entity_id,
                },
            }),
        })
    }

    /// Check if two entities are adjacent (touching at edges).
    /// For solid entities that can't overlap, this detects when they are colliding.
    fn entities_adjacent(a: &crate::entity::Entity, b: &crate::entity::Entity) -> bool {
        let a_min = a.position;
        let a_max = a.position + a.size.as_ivec2();
        let b_min = b.position;
        let b_max = b.position + b.size.as_ivec2();

        // Check if they overlap in Y axis (for horizontal adjacency)
        let y_overlap = a_min.y < b_max.y && a_max.y > b_min.y;
        // Check if they overlap in X axis (for vertical adjacency)
        let x_overlap = a_min.x < b_max.x && a_max.x > b_min.x;

        // Adjacent horizontally: touching on left/right edges (within 2px tolerance)
        let h_adjacent =
            y_overlap && ((a_max.x - b_min.x).abs() <= 2 || (b_max.x - a_min.x).abs() <= 2);
        // Adjacent vertically: touching on top/bottom edges (within 2px tolerance)
        let v_adjacent =
            x_overlap && ((a_max.y - b_min.y).abs() <= 2 || (b_max.y - a_min.y).abs() <= 2);

        h_adjacent || v_adjacent
    }

    /// Enter separation state for an entity.
    pub fn enter_separation_state(
        &mut self,
        entity_id: EntityId,
        other_ids: Vec<EntityId>,
        required_distance: f32,
    ) {
        let state = self.entity_states.entry(entity_id).or_default();
        state.separation_state = Some(SeparationState {
            other_entity_ids: other_ids,
            required_distance,
        });
    }

    /// Flee from player while also seeking mate if available.
    fn flee_and_seek(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        mate: Option<EntityId>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let player_pos = player_position?;
        let movement_step = entity.attributes.speed.round() as i32;

        // If we have a mate, try to move toward them while avoiding player
        if let Some(mate_id) = mate {
            if let Some(mate_entity) = ctx.entity_manager.get_entity(mate_id) {
                let directions = Self::compute_directions_toward(
                    entity.position,
                    mate_entity.position,
                    movement_step,
                );
                let result = Self::try_movement_with_fallback(
                    entity,
                    entity_id,
                    entity.position,
                    &directions,
                    ctx,
                );
                if result.as_ref().is_some_and(|r| r.new_position.is_some()) {
                    return result;
                }
            }
        }

        // Fall back to fleeing from player
        let directions = Self::compute_directions_away(entity.position, player_pos, movement_step);
        Self::try_movement_with_fallback(entity, entity_id, entity.position, &directions, ctx)
    }

    /// Seek toward a specific entity.
    fn seek_entity(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        target_id: EntityId,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let target = ctx.entity_manager.get_entity(target_id)?;
        let movement_step = entity.attributes.speed.round() as i32;
        let directions =
            Self::compute_directions_toward(entity.position, target.position, movement_step);

        Self::try_movement_with_fallback(entity, entity_id, entity.position, &directions, ctx)
    }

    /// Idle wandering behavior for Chase/Run when player is outside detection radius.
    /// Uses a state machine: walk random tiles in one direction, then wait, repeat.
    fn idle_wander(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let state = self.entity_states.entry(entity_id).or_default();
        let current_position = entity.position;

        match &state.wander_phase {
            WanderPhase::Waiting => self.handle_wander_waiting(entity_id),
            WanderPhase::Walking {
                direction,
                remaining_distance,
            } => {
                let dir = *direction;
                let remaining = *remaining_distance;
                self.handle_wander_walking(entity, entity_id, current_position, dir, remaining, ctx)
            }
        }
    }

    /// Handle the waiting phase of idle wandering.
    fn handle_wander_waiting(&mut self, entity_id: EntityId) -> Option<AiUpdateResult> {
        let state = self.entity_states.get_mut(&entity_id)?;

        if state.wait_frames_remaining > 0 {
            state.wait_frames_remaining -= 1;
            return Some(AiUpdateResult {
                entity_id,
                new_position: None,
                new_animation: Some(AnimationState::Idle),
                movement_distance: 0.0,
                spawn_request: None,
            });
        }

        // Done waiting - start walking in a random direction
        let direction = Self::random_cardinal_direction();
        let tiles = fastrand::u32(1..=3); // Walk 1-3 tiles
        let tile_size = 16; // Standard tile size in pixels

        state.wander_phase = WanderPhase::Walking {
            direction,
            remaining_distance: (tiles * tile_size) as i32,
        };

        // Return walking animation for this frame
        Some(AiUpdateResult {
            entity_id,
            new_position: None,
            new_animation: Some(AnimationState::Walk),
            movement_distance: 0.0,
            spawn_request: None,
        })
    }

    /// Handle the walking phase of idle wandering.
    fn handle_wander_walking(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        current_position: IVec2,
        direction: IVec2,
        remaining_distance: i32,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let movement_step = entity.attributes.speed.round() as i32;
        let (max_x, max_y) = ctx.max_position(entity.size);

        let scaled_direction = IVec2::new(direction.x * movement_step, direction.y * movement_step);
        let new_position = IVec2::new(
            (current_position.x + scaled_direction.x).clamp(0, max_x),
            (current_position.y + scaled_direction.y).clamp(0, max_y),
        );

        let can_move =
            new_position != current_position && ctx.is_movement_valid(entity, entity_id, new_position);

        let state = self.entity_states.get_mut(&entity_id)?;
        let new_remaining = remaining_distance - movement_step;

        if can_move && new_remaining > 0 {
            // Continue walking
            state.wander_phase = WanderPhase::Walking {
                direction,
                remaining_distance: new_remaining,
            };
            return Some(Self::build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ));
        }

        // Either blocked or finished walking - transition to waiting
        let wait_frames = fastrand::u32(30..=180); // Wait 1-3 seconds at 60fps
        state.wander_phase = WanderPhase::Waiting;
        state.wait_frames_remaining = wait_frames;

        if can_move {
            // Final step before waiting
            Some(Self::build_movement_result(
                entity_id,
                current_position,
                new_position,
                true,
            ))
        } else {
            // Blocked - just start waiting
            Some(AiUpdateResult {
                entity_id,
                new_position: None,
                new_animation: Some(AnimationState::Idle),
                movement_distance: 0.0,
                spawn_request: None,
            })
        }
    }

    /// Generate a random cardinal direction (up, down, left, right).
    fn random_cardinal_direction() -> IVec2 {
        match fastrand::u32(0..4) {
            0 => IVec2::new(0, -1), // Up
            1 => IVec2::new(0, 1),  // Down
            2 => IVec2::new(-1, 0), // Left
            _ => IVec2::new(1, 0),  // Right
        }
    }

    /// Compute movement directions toward a target, ordered by priority.
    /// Returns primary direction first, then perpendicular directions.
    fn compute_directions_toward(current: IVec2, target: IVec2, step: i32) -> Vec<IVec2> {
        let dx = target.x - current.x;
        let dy = target.y - current.y;

        let mut directions = Vec::with_capacity(3);

        // Primary direction (dominant axis)
        if dx.abs() >= dy.abs() {
            directions.push(IVec2::new(dx.signum() * step, 0));
            // Perpendicular directions
            if dy != 0 {
                directions.push(IVec2::new(0, dy.signum() * step));
            } else {
                directions.push(IVec2::new(0, step));
                directions.push(IVec2::new(0, -step));
            }
        } else {
            directions.push(IVec2::new(0, dy.signum() * step));
            // Perpendicular directions
            if dx != 0 {
                directions.push(IVec2::new(dx.signum() * step, 0));
            } else {
                directions.push(IVec2::new(step, 0));
                directions.push(IVec2::new(-step, 0));
            }
        }

        directions
    }

    /// Compute movement directions away from a threat, ordered by priority.
    /// Returns primary direction first, then perpendicular directions.
    fn compute_directions_away(current: IVec2, threat: IVec2, step: i32) -> Vec<IVec2> {
        let dx = current.x - threat.x;
        let dy = current.y - threat.y;

        let mut directions = Vec::with_capacity(3);

        // Primary direction (dominant axis away from threat)
        if dx.abs() >= dy.abs() {
            let dir = if dx == 0 { 1 } else { dx.signum() };
            directions.push(IVec2::new(dir * step, 0));
            // Perpendicular directions
            if dy != 0 {
                directions.push(IVec2::new(0, dy.signum() * step));
            } else {
                directions.push(IVec2::new(0, step));
                directions.push(IVec2::new(0, -step));
            }
        } else {
            let dir = if dy == 0 { 1 } else { dy.signum() };
            directions.push(IVec2::new(0, dir * step));
            // Perpendicular directions
            if dx != 0 {
                directions.push(IVec2::new(dx.signum() * step, 0));
            } else {
                directions.push(IVec2::new(step, 0));
                directions.push(IVec2::new(-step, 0));
            }
        }

        directions
    }

    /// Try movement in multiple directions, falling back to perpendicular if blocked.
    fn try_movement_with_fallback(
        entity: &Entity,
        entity_id: EntityId,
        current_position: IVec2,
        directions: &[IVec2],
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let (max_x, max_y) = ctx.max_position(entity.size);

        for &direction in directions {
            let new_position = IVec2::new(
                (current_position.x + direction.x).clamp(0, max_x),
                (current_position.y + direction.y).clamp(0, max_y),
            );

            if new_position == current_position {
                continue; // Clamped to same position, try next direction
            }

            if ctx.is_movement_valid(entity, entity_id, new_position) {
                return Some(Self::build_movement_result(
                    entity_id,
                    current_position,
                    new_position,
                    true,
                ));
            }
        }

        // All directions blocked, stay in place
        Some(Self::build_movement_result(
            entity_id,
            current_position,
            current_position,
            false,
        ))
    }

    /// Build the final AI update result after computing movement.
    fn build_movement_result(
        entity_id: EntityId,
        current_position: IVec2,
        new_position: IVec2,
        movement_valid: bool,
    ) -> AiUpdateResult {
        let entity_moved = movement_valid && new_position != current_position;

        let final_position = if entity_moved {
            new_position
        } else {
            current_position
        };

        let movement_distance = if entity_moved {
            let dx = (final_position.x - current_position.x) as f32;
            let dy = (final_position.y - current_position.y) as f32;
            (dx * dx + dy * dy).sqrt()
        } else {
            0.0
        };

        let desired_animation = if entity_moved {
            AnimationState::Walk
        } else {
            AnimationState::Idle
        };

        AiUpdateResult {
            entity_id,
            new_position: if entity_moved {
                Some(final_position)
            } else {
                None
            },
            new_animation: Some(desired_animation),
            movement_distance,
            spawn_request: None,
        }
    }

    /// Build a wander-specific result with optional movement.
    fn build_wander_result(
        entity_id: EntityId,
        current_position: IVec2,
        new_position: IVec2,
        entity_moved: bool,
    ) -> Option<AiUpdateResult> {
        let final_position = if entity_moved { new_position } else { current_position };

        let movement_distance = if entity_moved {
            let dx = (final_position.x - current_position.x) as f32;
            let dy = (final_position.y - current_position.y) as f32;
            (dx * dx + dy * dy).sqrt()
        } else {
            0.0
        };

        let desired_animation = if entity_moved {
            AnimationState::Walk
        } else {
            AnimationState::Idle
        };

        Some(AiUpdateResult {
            entity_id,
            new_position: if entity_moved { Some(final_position) } else { None },
            new_animation: Some(desired_animation),
            movement_distance,
            spawn_request: None,
        })
    }

    /// Get or create runtime state for an entity.
    pub fn get_or_create_state(&mut self, entity_id: EntityId) -> &mut AiRuntimeState {
        self.entity_states.entry(entity_id).or_default()
    }

    /// Remove runtime state for an entity.
    pub fn remove_state(&mut self, entity_id: EntityId) {
        self.entity_states.remove(&entity_id);
    }
}

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;
