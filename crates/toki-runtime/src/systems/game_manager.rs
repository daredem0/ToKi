use toki_core::entity::MovementProfile;
use toki_core::game::AudioEvent;
use toki_core::menu::InventoryEntry;
use toki_core::sprite_render::SpriteRenderRequest;
use toki_core::{
    assets::atlas::AtlasMeta,
    assets::tilemap::TileMap,
    entity::Entity,
    game::{EntityHealthBar, InputAction},
    sprite::SpriteFrame,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyboardBinding {
    Direct(InputKey),
    Profile {
        profile: MovementProfile,
        input_key: InputKey,
    },
    ProfileAction {
        profile: MovementProfile,
        action: InputAction,
    },
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
        if let Some(binding) = self.translate_keycode(key) {
            match binding {
                KeyboardBinding::Direct(input_key) => {
                    if pressed {
                        self.game_state.handle_key_press(input_key);
                    } else {
                        self.game_state.handle_key_release(input_key);
                    }
                }
                KeyboardBinding::Profile { profile, input_key } => {
                    if pressed {
                        self.game_state.handle_profile_key_press(profile, input_key);
                    } else {
                        self.game_state
                            .handle_profile_key_release(profile, input_key);
                    }
                }
                KeyboardBinding::ProfileAction { profile, action } => {
                    if pressed {
                        self.game_state.handle_profile_action_press(profile, action);
                    } else {
                        self.game_state
                            .handle_profile_action_release(profile, action);
                    }
                }
            }
        }
    }

    pub fn clear_runtime_inputs(&mut self) {
        self.game_state.clear_runtime_inputs();
    }

    /// Translate winit KeyCode to core InputKey
    fn translate_keycode(&self, key: KeyCode) -> Option<KeyboardBinding> {
        match key {
            KeyCode::KeyW => Some(KeyboardBinding::Profile {
                profile: MovementProfile::PlayerWasd,
                input_key: InputKey::Up,
            }),
            KeyCode::KeyA => Some(KeyboardBinding::Profile {
                profile: MovementProfile::PlayerWasd,
                input_key: InputKey::Left,
            }),
            KeyCode::KeyS => Some(KeyboardBinding::Profile {
                profile: MovementProfile::PlayerWasd,
                input_key: InputKey::Down,
            }),
            KeyCode::KeyD => Some(KeyboardBinding::Profile {
                profile: MovementProfile::PlayerWasd,
                input_key: InputKey::Right,
            }),
            KeyCode::Space => Some(KeyboardBinding::ProfileAction {
                profile: MovementProfile::PlayerWasd,
                action: InputAction::Primary,
            }),
            KeyCode::F4 => Some(KeyboardBinding::Direct(InputKey::DebugToggle)),
            _ => None,
        }
    }

    pub fn get_sprite_render_requests(&self) -> Vec<SpriteRenderRequest> {
        self.game_state.get_sprite_render_requests()
    }

    pub fn get_entity_health_bars(&self) -> Vec<EntityHealthBar> {
        self.game_state.get_entity_health_bars()
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

    /// Get the player entity ID
    pub fn player_id(&self) -> Option<toki_core::entity::EntityId> {
        self.game_state.player_id()
    }

    pub fn player_inventory_entries(&self) -> Vec<InventoryEntry> {
        self.game_state.player_inventory_entries()
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
#[path = "game_manager_tests.rs"]
mod tests;
