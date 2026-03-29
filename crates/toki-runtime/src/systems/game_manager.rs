use toki_core::entity::MovementProfile;
use toki_core::{
    entity::Entity,
    game::{InputAction, InputSystem},
    GameState, InputKey,
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

    /// Handle winit keyboard input events, translating to core InputKey events
    pub fn handle_keyboard_input(&mut self, key: KeyCode, pressed: bool) {
        if let Some(binding) = self.translate_keycode(key) {
            match binding {
                KeyboardBinding::Direct(input_key) => {
                    if pressed {
                        InputSystem::handle_key_press(self.game_state.runtime_mut(), input_key);
                    } else {
                        InputSystem::handle_key_release(self.game_state.runtime_mut(), input_key);
                    }
                }
                KeyboardBinding::Profile { profile, input_key } => {
                    if pressed {
                        InputSystem::handle_profile_key_press(
                            self.game_state.runtime_mut(),
                            profile,
                            input_key,
                        );
                    } else {
                        InputSystem::handle_profile_key_release(
                            self.game_state.runtime_mut(),
                            profile,
                            input_key,
                        );
                    }
                }
                KeyboardBinding::ProfileAction { profile, action } => {
                    if pressed {
                        InputSystem::handle_profile_action_press(
                            self.game_state.runtime_mut(),
                            profile,
                            action,
                        );
                    } else {
                        InputSystem::handle_profile_action_release(
                            self.game_state.runtime_mut(),
                            profile,
                            action,
                        );
                    }
                }
            }
        }
    }

    pub fn clear_runtime_inputs(&mut self) {
        InputSystem::clear(self.game_state.runtime_mut());
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
            KeyCode::KeyE => Some(KeyboardBinding::Direct(InputKey::Interact)),
            _ => None,
        }
    }

    /// Get the player entity ID
    pub fn player_id(&self) -> Option<toki_core::entity::EntityId> {
        self.game_state.world().player_id()
    }

    pub fn active_scene_name(&self) -> Option<&str> {
        self.game_state.scene().scene_manager().active_scene_name()
    }

    /// Get entities for camera system integration
    pub fn entities_for_camera(&self) -> Vec<Entity> {
        self.game_state
            .world()
            .entity_manager()
            .active_entities()
            .iter()
            .filter_map(|&id| self.game_state.world().entity_manager().get_entity(id))
            .cloned()
            .collect()
    }

    /// Check if debug collision rendering is enabled
    pub fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.game_state.runtime().debug_collision_rendering()
    }
}

#[cfg(test)]
#[path = "game_manager_tests.rs"]
mod tests;
