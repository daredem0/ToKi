use std::collections::HashSet;

use crate::entity::{Entity, EntityId, EntityManager};
use crate::sprite::{SpriteFrame, SpriteInstance};

/// Core input keys abstraction (platform-independent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputKey {
    Up,
    Down,
    Left,
    Right,
    // Can extend with more keys as needed
}

/// Core game state that manages entities, input, and game logic.
///
/// This is platform-independent and contains pure game logic without
/// any runtime or windowing dependencies.
#[derive(Debug)]
pub struct GameState {
    /// Entity manager for all game objects
    entity_manager: EntityManager,

    /// Player entity ID for quick access
    player_id: Option<EntityId>,

    /// Currently held input keys
    keys_held: HashSet<InputKey>,

    /// Game configuration constants
    movement_step: i32,
    sprite_size: u32,
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
        }
    }

    /// Initialize the game with a player at the specified position
    pub fn spawn_player_at(&mut self, position: glam::IVec2) -> EntityId {
        let player_id = self.entity_manager.spawn_player(position);
        self.player_id = Some(player_id);
        player_id
    }

    /// Update game state by one tick
    pub fn update(&mut self, world_bounds: glam::UVec2) -> bool {
        let moved = self.process_input(world_bounds);

        // Update entity animation timing
        self.entity_manager.update_animations(17.0);

        moved
    }

    /// Process input and update player position
    /// Returns true if the player actually moved (position changed)
    fn process_input(&mut self, world_bounds: glam::UVec2) -> bool {
        let Some(player_id) = self.player_id else {
            return false; // No player entity to move
        };

        let Some(player_entity) = self.entity_manager.get_entity(player_id) else {
            return false; // Player entity doesn't exist
        };

        let initial_position = player_entity.position;

        // Get mutable reference to player entity
        let Some(player_entity) = self.entity_manager.get_entity_mut(player_id) else {
            return false;
        };

        for key in &self.keys_held {
            match key {
                InputKey::Up => {
                    tracing::trace!("Move forward");
                    let new_y = (player_entity.position.y - self.movement_step).max(0);
                    player_entity.position.y = new_y;
                }
                InputKey::Left => {
                    tracing::trace!("Move left");
                    let new_x = (player_entity.position.x - self.movement_step).max(0);
                    player_entity.position.x = new_x;
                }
                InputKey::Down => {
                    tracing::trace!("Move backward");
                    let new_y = (player_entity.position.y + self.movement_step)
                        .min(world_bounds.y as i32 - self.sprite_size as i32);
                    player_entity.position.y = new_y;
                }
                InputKey::Right => {
                    tracing::trace!("Move right");
                    let new_x = (player_entity.position.x + self.movement_step)
                        .min(world_bounds.x as i32 - self.sprite_size as i32);
                    player_entity.position.x = new_x;
                }
            }
        }

        // Only return true if position actually changed
        player_entity.position != initial_position
    }

    /// Handle key press events
    pub fn handle_key_press(&mut self, key: InputKey) {
        self.keys_held.insert(key);
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

    /// Get the current sprite frame for rendering
    pub fn current_sprite_frame(&self) -> SpriteFrame {
        if let Some(player_entity) = self.player_entity() {
            if let Some(sprite_info) = &player_entity.attributes.sprite_info {
                // Create a basic UV frame calculation
                // For now, assume 4 frames in a horizontal strip
                let frame_width = 1.0 / 4.0; // 4 frames wide
                let u0 = sprite_info.current_frame as f32 * frame_width;
                let u1 = u0 + frame_width;

                return SpriteFrame {
                    u0,
                    v0: 0.0,
                    u1,
                    v1: 1.0,
                };
            }
        }

        // Fallback to default frame
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
}
