use toki_core::{
    assets::atlas::AtlasMeta, assets::tilemap::TileMap, entity::Entity, sprite::SpriteFrame,
    GameState, InputKey,
};
use winit::keyboard::KeyCode;

/// Game system that wraps the core GameState and provides runtime integration.
///
/// Handles translation between platform-specific events (winit) and core game logic,
/// providing a clean interface for the App to coordinate game state with other systems.
#[derive(Debug)]
pub struct GameSystem {
    game_state: GameState,
}

impl GameSystem {
    /// Create a new GameSystem with the given core GameState
    pub fn new(game_state: GameState) -> Self {
        Self { game_state }
    }

    /// Update the game state by one tick
    /// Returns true if the player moved (indicating camera/rendering updates needed)
    pub fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> bool {
        self.game_state.update(world_bounds, tilemap, atlas)
    }

    /// Handle winit keyboard input events, translating to core InputKey events
    pub fn handle_keyboard_input(&mut self, key: KeyCode, pressed: bool) {
        if let Some(input_key) = self.translate_keycode(key) {
            if pressed {
                self.game_state.handle_key_press(input_key);
            } else {
                self.game_state.handle_key_release(input_key);
            }
        }
    }

    /// Translate winit KeyCode to core InputKey
    fn translate_keycode(&self, key: KeyCode) -> Option<InputKey> {
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => Some(InputKey::Up),
            KeyCode::KeyA | KeyCode::ArrowLeft => Some(InputKey::Left),
            KeyCode::KeyS | KeyCode::ArrowDown => Some(InputKey::Down),
            KeyCode::KeyD | KeyCode::ArrowRight => Some(InputKey::Right),
            KeyCode::F4 => Some(InputKey::DebugToggle),
            _ => None,
        }
    }

    /// Get the current sprite frame for rendering
    pub fn current_sprite_frame(&self) -> SpriteFrame {
        self.game_state.current_sprite_frame()
    }

    /// Get player position for rendering
    pub fn player_position(&self) -> glam::IVec2 {
        self.game_state.player_position()
    }

    /// Get sprite size for rendering calculations
    pub fn sprite_size(&self) -> u32 {
        self.game_state.sprite_size()
    }

    /// Get the player entity ID
    pub fn player_id(&self) -> Option<toki_core::entity::EntityId> {
        self.game_state.player_id()
    }

    /// Get entities for camera system integration
    pub fn entities_for_camera(&self) -> Vec<Entity> {
        self.game_state.entities_owned()
    }

    /// Get entity collision boxes for debug rendering
    pub fn get_entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
        self.game_state.get_entity_collision_boxes()
    }

    /// Get solid tile positions for debug rendering
    pub fn get_solid_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)> {
        self.game_state.get_solid_tile_positions(tilemap, atlas)
    }

    /// Get trigger tile positions for debug rendering
    pub fn get_trigger_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)> {
        self.game_state.get_trigger_tile_positions(tilemap, atlas)
    }

    /// Check if debug collision rendering is enabled
    pub fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.game_state.is_debug_collision_rendering_enabled()
    }
}
