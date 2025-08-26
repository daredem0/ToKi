use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{Entity, EntityId, EntityManager};
use crate::events::{GameEvent, GameUpdateResult};
use crate::sprite::{SpriteFrame, SpriteInstance};

/// Core input keys abstraction (platform-independent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputKey {
    Up,
    Down,
    Left,
    Right,
    DebugToggle, // F4 key for toggling debug rendering
                 // Can extend with more keys as needed
}

/// Audio events that can be triggered by game logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioEvent {
    /// Player started walking
    PlayerWalk,
    /// Player collided with something
    PlayerCollision,
    /// Start background music
    BackgroundMusic(String),
}

impl GameEvent for AudioEvent {}

/// Core game state that manages entities, input, and game logic.
///
/// This is platform-independent and contains pure game logic without
/// any runtime or windowing dependencies.
#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    /// Entity manager for all game objects
    entity_manager: EntityManager,

    /// Player entity ID for quick access
    player_id: Option<EntityId>,

    /// Currently held input keys
    #[serde(default)]
    keys_held: HashSet<InputKey>,

    /// Game configuration constants
    movement_step: i32,
    sprite_size: u32,

    /// Debug rendering flags
    #[serde(default)]
    debug_collision_rendering: bool,
}

impl GameState {
    /// Create a new GameState with the given player sprite
    pub fn new(player_sprite: SpriteInstance) -> Self {
        let mut entity_manager = EntityManager::new();

        // Create player entity at the sprite's initial position
        let player_id = entity_manager.spawn_player(player_sprite.position);

        Self {
            entity_manager,
            player_id: Some(player_id),
            keys_held: HashSet::new(),
            movement_step: 1, // Move exactly 1 pixel per frame
            sprite_size: 16,  // Sprite is 16×16 pixels
            debug_collision_rendering: false,
        }
    }

    /// Create a new empty GameState with no entities
    pub fn new_empty() -> Self {
        Self {
            entity_manager: EntityManager::new(),
            player_id: None,
            keys_held: HashSet::new(),
            movement_step: 1,
            sprite_size: 16,
            debug_collision_rendering: false,
        }
    }

    /// Initialize the game with a player at the specified position
    pub fn spawn_player_at(&mut self, position: glam::IVec2) -> EntityId {
        let player_id = self.entity_manager.spawn_player(position);
        self.player_id = Some(player_id);
        player_id
    }

    /// Update game state by one tick
    pub fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        let input_result = self.process_input(world_bounds, tilemap, atlas);

        // Pick moving or idle animation
        if let Some(player_entity) = self.entity_manager.get_player_mut() {
            if let Some(animation_controller) = &mut player_entity.attributes.animation_controller {
                let desired_player_animation = if input_result.player_moved {
                    AnimationState::Walk
                } else {
                    AnimationState::Idle
                };
                if animation_controller.current_clip_state != desired_player_animation {
                    tracing::debug!(
                        "Changing clip from  {:?} to {:?}",
                        animation_controller.current_clip_state,
                        desired_player_animation
                    );
                    animation_controller.play(desired_player_animation);
                }
            }
        }

        // Update entity animation timing
        self.entity_manager.update_animations(17.0);

        input_result
    }

    /// Check if an entity can move to a position without colliding with solid tiles
    /// Returns true if movement is allowed, false if blocked
    fn can_entity_move_to_position(
        entity: &crate::entity::Entity,
        new_position: glam::IVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> bool {
        // If entity has no collision box, allow movement
        let Some(collision_box) = &entity.collision_box else {
            return true;
        };

        // Skip collision for trigger boxes (they don't block movement)
        if collision_box.trigger {
            return true;
        }

        // Get the world bounds of the collision box at the new position
        let (box_pos, box_size) = collision_box.world_bounds(new_position);

        // Handle negative coordinates - treat as blocked
        if box_pos.x < 0 || box_pos.y < 0 {
            tracing::debug!("Movement blocked - collision box would be at negative coordinates");
            return false;
        }

        // Convert collision box bounds to tile coordinates
        let tile_size = tilemap.tile_size;
        let min_tile_x = (box_pos.x as u32) / tile_size.x;
        let min_tile_y = (box_pos.y as u32) / tile_size.y;
        let max_tile_x = ((box_pos.x + box_size.x as i32 - 1) as u32) / tile_size.x;
        let max_tile_y = ((box_pos.y + box_size.y as i32 - 1) as u32) / tile_size.y;

        // Check all tiles that the collision box would overlap
        for tile_y in min_tile_y..=max_tile_y {
            for tile_x in min_tile_x..=max_tile_x {
                match tilemap.is_tile_solid_at(atlas, tile_x, tile_y) {
                    Ok(is_solid) => {
                        if is_solid {
                            tracing::trace!(
                                "Collision blocked movement - solid tile at ({}, {}) would overlap collision box at ({}, {})",
                                tile_x, tile_y, box_pos.x, box_pos.y
                            );
                            return false;
                        }
                    }
                    Err(err) => {
                        // Out of bounds or other error - treat as blocking
                        tracing::debug!(
                            "Collision check failed for tile ({}, {}): {}",
                            tile_x,
                            tile_y,
                            err
                        );
                        return false;
                    }
                }
            }
        }

        tracing::trace!("Movement allowed - no collision detected");
        true
    }

    /// Process input and update player position
    /// Returns GameUpdateResult with movement info and audio events
    fn process_input(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        let Some(player_id) = self.player_id else {
            return GameUpdateResult::new(); // No player entity to move
        };

        let Some(player_entity) = self.entity_manager.get_entity(player_id) else {
            return GameUpdateResult::new(); // Player entity doesn't exist
        };

        let initial_position = player_entity.position;
        let mut result = GameUpdateResult::new();

        // Get mutable reference to player entity
        let Some(player_entity) = self.entity_manager.get_entity_mut(player_id) else {
            return GameUpdateResult::new();
        };

        for key in &self.keys_held {
            match key {
                InputKey::Up => {
                    tracing::trace!("Move forward");
                    let new_y = (player_entity.position.y - self.movement_step).max(0);
                    let new_position = glam::IVec2::new(player_entity.position.x, new_y);
                    if Self::can_entity_move_to_position(
                        player_entity,
                        new_position,
                        tilemap,
                        atlas,
                    ) {
                        player_entity.position.y = new_y;
                        player_entity.last_collision_state = false;
                    } else {
                        // Only trigger audio on state change
                        if !player_entity.last_collision_state {
                            result.add_event(AudioEvent::PlayerCollision);
                        }
                        player_entity.last_collision_state = true;
                    }
                }
                InputKey::Left => {
                    tracing::trace!("Move left");
                    let new_x = (player_entity.position.x - self.movement_step).max(0);
                    let new_position = glam::IVec2::new(new_x, player_entity.position.y);
                    if Self::can_entity_move_to_position(
                        player_entity,
                        new_position,
                        tilemap,
                        atlas,
                    ) {
                        player_entity.position.x = new_x;
                        player_entity.last_collision_state = false;
                    } else {
                        // Only trigger audio on state change
                        if !player_entity.last_collision_state {
                            result.add_event(AudioEvent::PlayerCollision);
                        }
                        player_entity.last_collision_state = true;
                    }
                }
                InputKey::Down => {
                    tracing::trace!("Move backward");
                    let new_y = (player_entity.position.y + self.movement_step)
                        .min(world_bounds.y as i32 - self.sprite_size as i32);
                    let new_position = glam::IVec2::new(player_entity.position.x, new_y);
                    if Self::can_entity_move_to_position(
                        player_entity,
                        new_position,
                        tilemap,
                        atlas,
                    ) {
                        player_entity.position.y = new_y;
                        player_entity.last_collision_state = false;
                    } else {
                        // Only trigger audio on state change
                        if !player_entity.last_collision_state {
                            result.add_event(AudioEvent::PlayerCollision);
                        }
                        player_entity.last_collision_state = true;
                    }
                }
                InputKey::Right => {
                    tracing::trace!("Move right");
                    let new_x = (player_entity.position.x + self.movement_step)
                        .min(world_bounds.x as i32 - self.sprite_size as i32);
                    let new_position = glam::IVec2::new(new_x, player_entity.position.y);
                    if Self::can_entity_move_to_position(
                        player_entity,
                        new_position,
                        tilemap,
                        atlas,
                    ) {
                        player_entity.position.x = new_x;
                        player_entity.last_collision_state = false;
                    } else {
                        // Only trigger audio on state change
                        if !player_entity.last_collision_state {
                            result.add_event(AudioEvent::PlayerCollision);
                        }
                        player_entity.last_collision_state = true;
                    }
                }
                InputKey::DebugToggle => {
                    // Debug toggle is handled in key press event, not as held key
                }
            }
        }

        // Check if position actually changed
        let player_moved = player_entity.position != initial_position;
        result.player_moved = player_moved;

        // Distance-based footstep tracking
        if player_moved {
            // Calculate distance moved
            let distance_moved = (((player_entity.position.x - initial_position.x).pow(2)
                + (player_entity.position.y - initial_position.y).pow(2))
                as f32)
                .sqrt();

            player_entity.footstep_distance_accumulator += distance_moved;

            // Trigger footstep when accumulated distance exceeds threshold
            if player_entity.footstep_distance_accumulator
                >= player_entity.footstep_trigger_distance
            {
                result.add_event(AudioEvent::PlayerWalk);
                player_entity.footstep_distance_accumulator -=
                    player_entity.footstep_trigger_distance;
            }
        }

        result
    }

    /// Handle key press events
    pub fn handle_key_press(&mut self, key: InputKey) {
        match key {
            InputKey::DebugToggle => {
                self.debug_collision_rendering = !self.debug_collision_rendering;
                tracing::info!(
                    "Debug collision rendering: {}",
                    self.debug_collision_rendering
                );
            }
            _ => {
                self.keys_held.insert(key);
            }
        }
    }

    /// Handle key release events
    pub fn handle_key_release(&mut self, key: InputKey) {
        self.keys_held.remove(&key);
    }

    /// Get reference to all entities (legacy method - preserved for compatibility)
    pub fn entities(&self) -> Vec<&Entity> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&id| self.entity_manager.get_entity(id))
            .collect()
    }

    /// Get access to the entity manager
    pub fn entity_manager(&self) -> &EntityManager {
        &self.entity_manager
    }

    /// Get mutable access to the entity manager
    pub fn entity_manager_mut(&mut self) -> &mut EntityManager {
        &mut self.entity_manager
    }

    /// Get the player entity ID
    pub fn player_id(&self) -> Option<EntityId> {
        self.player_id
    }

    /// Get reference to player entity
    pub fn player_entity(&self) -> Option<&Entity> {
        self.player_id
            .and_then(|id| self.entity_manager.get_entity(id))
    }

    /// Get entities as owned Vec for camera system compatibility
    pub fn entities_owned(&self) -> Vec<Entity> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&id| self.entity_manager.get_entity(id))
            .cloned()
            .collect()
    }

    /// Get the current sprite frame for rendering with proper atlas lookup
    pub fn current_sprite_frame(
        &self,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> SpriteFrame {
        if let Some(player_entity) = self.player_entity() {
            if let Some(animation_controller) = &player_entity.attributes.animation_controller {
                if let Ok(tile_name) = animation_controller.current_tile_name() {
                    // Look up the tile in the atlas to get UV coordinates
                    if let Some(uvs) = atlas.get_tile_uvs(&tile_name, texture_size) {
                        return SpriteFrame {
                            u0: uvs[0],
                            v0: uvs[1],
                            u1: uvs[2],
                            v1: uvs[3],
                        };
                    }
                }
            }
        }

        // Fallback to default frame if animation or atlas lookup fails
        SpriteFrame {
            u0: 0.0,
            v0: 0.0,
            u1: 0.25,
            v1: 1.0,
        }
    }

    /// Get player position for rendering
    pub fn player_position(&self) -> glam::IVec2 {
        if let Some(player_entity) = self.player_entity() {
            player_entity.position
        } else {
            glam::IVec2::ZERO // Fallback
        }
    }

    /// Get sprite size for rendering calculations
    pub fn sprite_size(&self) -> u32 {
        self.sprite_size
    }

    /// Check if debug collision rendering is enabled
    pub fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.debug_collision_rendering
    }

    /// Get entity collision boxes for debug rendering
    /// Returns Vec of (position, size, is_trigger) tuples
    pub fn get_entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut boxes = Vec::new();

        for entity_id in self.entity_manager.active_entities() {
            if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                if let Some(collision_box) = &entity.collision_box {
                    let (world_pos, size) = collision_box.world_bounds(entity.position);
                    boxes.push((world_pos, size, collision_box.trigger));
                }
            }
        }

        boxes
    }

    /// Get solid tile positions for debug rendering
    /// Returns Vec of (tile_x, tile_y) coordinates of solid tiles
    pub fn get_solid_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut solid_tiles = Vec::new();

        for y in 0..tilemap.size.y {
            for x in 0..tilemap.size.x {
                if let Ok(is_solid) = tilemap.is_tile_solid_at(atlas, x, y) {
                    if is_solid {
                        solid_tiles.push((x, y));
                    }
                }
            }
        }

        solid_tiles
    }

    /// Get trigger tile positions for debug rendering  
    /// Returns Vec of (tile_x, tile_y) coordinates of trigger tiles
    pub fn get_trigger_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut trigger_tiles = Vec::new();

        for y in 0..tilemap.size.y {
            for x in 0..tilemap.size.x {
                if let Ok(tile_name) = tilemap.get_tile_name(x, y) {
                    if atlas.is_tile_trigger(tile_name) {
                        trigger_tiles.push((x, y));
                    }
                }
            }
        }

        trigger_tiles
    }
}
