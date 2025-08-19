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

    /// Player sprite instance (legacy - will be migrated to entity system)
    player_sprite: SpriteInstance,

    /// Player entity ID for quick access
    player_id: Option<EntityId>,

    /// Currently held input keys
    keys_held: HashSet<InputKey>,

    /// Game configuration constants
    movement_step: f32,
    sprite_size: f32,
}

impl GameState {
    /// Create a new GameState with the given player sprite
    pub fn new(player_sprite: SpriteInstance) -> Self {
        let mut entity_manager = EntityManager::new();
        
        // Create player entity at the sprite's initial position
        let player_id = entity_manager.spawn_player(player_sprite.position);

        Self {
            entity_manager,
            player_sprite,
            player_id: Some(player_id),
            keys_held: HashSet::new(),
            movement_step: 1.0, // Move exactly 1 pixel per frame
            sprite_size: 16.0,  // Sprite is 16×16 pixels
        }
    }

    /// Update game state by one tick
    pub fn update(&mut self, world_bounds: glam::Vec2) -> bool {
        let moved = self.process_input(world_bounds);

        // Update sprite animation
        self.player_sprite.tick(17); // ~60fps timing

        // Keep entity position synchronized with sprite position
        if let Some(player_id) = self.player_id {
            if let Some(player_entity) = self.entity_manager.get_entity_mut(player_id) {
                player_entity.position = self.player_sprite.position;
            }
        }

        moved
    }

    /// Process input and update player position
    /// Returns true if the player actually moved (position changed)
    fn process_input(&mut self, world_bounds: glam::Vec2) -> bool {
        let initial_position = self.player_sprite.position;

        for key in &self.keys_held {
            match key {
                InputKey::Up => {
                    tracing::trace!("Move forward");
                    let new_y = (self.player_sprite.position.y - self.movement_step).max(0.0);
                    self.player_sprite.position.y = new_y;
                }
                InputKey::Left => {
                    tracing::trace!("Move left");
                    let new_x = (self.player_sprite.position.x - self.movement_step).max(0.0);
                    self.player_sprite.position.x = new_x;
                }
                InputKey::Down => {
                    tracing::trace!("Move backward");
                    let new_y = (self.player_sprite.position.y + self.movement_step)
                        .min(world_bounds.y - self.sprite_size);
                    self.player_sprite.position.y = new_y;
                }
                InputKey::Right => {
                    tracing::trace!("Move right");
                    let new_x = (self.player_sprite.position.x + self.movement_step)
                        .min(world_bounds.x - self.sprite_size);
                    self.player_sprite.position.x = new_x;
                }
            }
        }

        // Only return true if position actually changed
        self.player_sprite.position != initial_position
    }

    /// Handle key press events
    pub fn handle_key_press(&mut self, key: InputKey) {
        self.keys_held.insert(key);
    }

    /// Handle key release events
    pub fn handle_key_release(&mut self, key: InputKey) {
        self.keys_held.remove(&key);
    }

    /// Get reference to player sprite
    pub fn player_sprite(&self) -> &SpriteInstance {
        &self.player_sprite
    }

    /// Get reference to all entities (legacy method - preserved for compatibility)
    pub fn entities(&self) -> Vec<&Entity> {
        self.entity_manager.active_entities()
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
        self.player_id.and_then(|id| self.entity_manager.get_entity(id))
    }

    /// Get entities as owned Vec for camera system compatibility
    pub fn entities_owned(&self) -> Vec<Entity> {
        self.entity_manager.active_entities()
            .iter()
            .filter_map(|&id| self.entity_manager.get_entity(id))
            .cloned()
            .collect()
    }

    /// Get the current sprite frame for rendering
    pub fn current_sprite_frame(&self) -> SpriteFrame {
        self.player_sprite.current_frame()
    }

    /// Get player position for rendering
    pub fn player_position(&self) -> glam::Vec2 {
        self.player_sprite.position
    }

    /// Get sprite size for rendering calculations
    pub fn sprite_size(&self) -> f32 {
        self.sprite_size
    }
}
