use std::collections::HashSet;

use crate::{sprite::{SpriteInstance, SpriteFrame}, camera::Entity};

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
    /// Player sprite instance
    player_sprite: SpriteInstance,
    
    /// All entities in the game world (for now just the player)
    entities: Vec<Entity>,
    
    /// Currently held input keys
    keys_held: HashSet<InputKey>,
    
    /// Game configuration constants
    movement_step: f32,
    sprite_size: f32,
}

impl GameState {
    /// Create a new GameState with the given player sprite
    pub fn new(player_sprite: SpriteInstance) -> Self {
        let player_entity = Entity {
            id: 1,
            position: player_sprite.position,
        };
        
        Self {
            player_sprite,
            entities: vec![player_entity],
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
        
        // Update entity position to match sprite
        if let Some(entity) = self.entities.get_mut(0) {
            entity.position = self.player_sprite.position;
        }
        
        moved
    }
    
    /// Process input and update player position
    /// Returns true if the player moved
    fn process_input(&mut self, world_bounds: glam::Vec2) -> bool {
        let mut moved = false;
        
        for key in &self.keys_held {
            match key {
                InputKey::Up => {
                    tracing::trace!("Move forward");
                    self.player_sprite.position.y = 
                        (self.player_sprite.position.y - self.movement_step).max(0.0);
                    moved = true;
                }
                InputKey::Left => {
                    tracing::trace!("Move left");
                    self.player_sprite.position.x = 
                        (self.player_sprite.position.x - self.movement_step).max(0.0);
                    moved = true;
                }
                InputKey::Down => {
                    tracing::trace!("Move backward");
                    self.player_sprite.position.y = 
                        (self.player_sprite.position.y + self.movement_step)
                        .min(world_bounds.y - self.sprite_size);
                    moved = true;
                }
                InputKey::Right => {
                    tracing::trace!("Move right");
                    self.player_sprite.position.x = 
                        (self.player_sprite.position.x + self.movement_step)
                        .min(world_bounds.x - self.sprite_size);
                    moved = true;
                }
            }
        }
        
        moved
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
    
    /// Get reference to all entities
    pub fn entities(&self) -> &[Entity] {
        &self.entities
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