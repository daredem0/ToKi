use toki_core::game::AudioEvent;
use toki_core::{
    assets::atlas::AtlasMeta, assets::tilemap::TileMap, entity::Entity, sprite::SpriteFrame,
    GameState, GameUpdateResult, InputKey,
};
use winit::keyboard::KeyCode;

/// Game manager that wraps the core GameState and provides runtime integration.
///
/// Handles translation between platform-specific events (winit) and core game logic,
/// providing a clean interface for the App to coordinate game state with other systems.
#[derive(Debug)]
pub struct GameManager {
    pub game_state: GameState,
}

impl GameManager {
    /// Create a new GameManager with the given core GameState
    pub fn new(game_state: GameState) -> Self {
        Self { game_state }
    }

    /// Update the game state by one tick
    /// Returns GameUpdateResult with movement info and audio events
    pub fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
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

    /// Get all renderable entities with their sprite data
    pub fn get_renderable_entities(
        &self,
    ) -> Vec<(toki_core::entity::EntityId, glam::IVec2, glam::UVec2)> {
        self.game_state.get_renderable_entities()
    }

    /// Get sprite frame for a specific entity
    pub fn get_entity_sprite_frame(
        &self,
        entity_id: toki_core::entity::EntityId,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> Option<SpriteFrame> {
        self.game_state
            .get_entity_sprite_frame(entity_id, atlas, texture_size)
    }

    /// Spawn an NPC that looks like the player
    pub fn spawn_player_like_npc(&mut self, position: glam::IVec2) -> toki_core::entity::EntityId {
        self.game_state.spawn_player_like_npc(position)
    }

    /// Get the current sprite frame for rendering (legacy method for backwards compatibility)
    pub fn current_sprite_frame(
        &self,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> SpriteFrame {
        self.game_state.current_sprite_frame(atlas, texture_size)
    }

    /// Get player position for rendering (legacy method for backwards compatibility)
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
    pub fn get_solid_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        self.game_state.get_solid_tile_positions(tilemap, atlas)
    }

    /// Get trigger tile positions for debug rendering
    pub fn get_trigger_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        self.game_state.get_trigger_tile_positions(tilemap, atlas)
    }

    /// Check if debug collision rendering is enabled
    pub fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.game_state.is_debug_collision_rendering_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::GameManager;
    use toki_core::GameState;
    use winit::keyboard::KeyCode;

    #[test]
    fn debug_toggle_key_is_forwarded_to_core_input() {
        let mut game_state = GameState::new_empty();
        game_state.spawn_player_at(glam::IVec2::new(0, 0));
        let mut manager = GameManager::new(game_state);

        assert!(!manager.is_debug_collision_rendering_enabled());
        manager.handle_keyboard_input(KeyCode::F4, true);
        assert!(manager.is_debug_collision_rendering_enabled());
    }

    #[test]
    fn unsupported_key_does_not_change_debug_state() {
        let mut game_state = GameState::new_empty();
        game_state.spawn_player_at(glam::IVec2::new(0, 0));
        let mut manager = GameManager::new(game_state);

        manager.handle_keyboard_input(KeyCode::Space, true);
        assert!(!manager.is_debug_collision_rendering_enabled());
    }
}
