use toki_core::entity::MovementProfile;
use toki_core::game::AudioEvent;
use toki_core::menu::InventoryEntry;
use toki_core::sprite_render::SpriteRenderRequest;
use toki_core::{
    assets::atlas::AtlasMeta,
    assets::tilemap::TileMap,
    entity::Entity,
    game::{
        EntityHealthBar, GameSimulation, GroundShadow, InputAction, InputSystem,
        RenderQueryService, RuleSystem, SceneSystem,
    },
    scene::Scene,
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

    /// Update the game state by one fixed tick (16.67ms assumed)
    /// Returns GameUpdateResult with movement info and audio events
    pub fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        GameSimulation::tick_fixed(&mut self.game_state, world_bounds, tilemap, atlas)
    }

    /// Update the game state with delta time scaling
    /// Movement and animation are scaled proportionally to elapsed time
    pub fn update_with_delta(
        &mut self,
        delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        GameSimulation::tick_with_delta(
            &mut self.game_state,
            delta_ms,
            world_bounds,
            tilemap,
            atlas,
        )
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

    pub fn get_sprite_render_requests(&self) -> Vec<SpriteRenderRequest> {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .sprite_render_requests()
    }

    pub fn get_entity_health_bars(&self) -> Vec<EntityHealthBar> {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .entity_health_bars()
    }

    pub fn get_entity_ground_shadows(&self) -> Vec<GroundShadow> {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .entity_ground_shadows()
    }

    /// Spawn an NPC that looks like the player
    pub fn spawn_player_like_npc(&mut self, position: glam::IVec2) -> toki_core::entity::EntityId {
        self.game_state.spawn_player_like_npc(position)
    }

    /// Get the current player sprite frame for rendering.
    pub fn current_sprite_frame(
        &self,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> SpriteFrame {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .current_sprite_frame(atlas, texture_size)
    }

    /// Get player position for rendering.
    pub fn player_position(&self) -> glam::IVec2 {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .player_position()
    }

    /// Get the player entity ID
    pub fn player_id(&self) -> Option<toki_core::entity::EntityId> {
        self.game_state.world().player_id()
    }

    pub fn player_inventory_entries(&self) -> Vec<InventoryEntry> {
        self.game_state.player_inventory_entries()
    }

    pub fn active_scene_name(&self) -> Option<&str> {
        self.game_state.scene().scene_manager().active_scene_name()
    }

    pub fn active_scene(&self) -> Option<&Scene> {
        SceneSystem::active_scene(&self.game_state)
    }

    pub fn scene_named(&self, scene_name: &str) -> Option<&Scene> {
        self.game_state.scene().scene_manager().get_scene(scene_name)
    }

    pub fn transition_to_scene(
        &mut self,
        scene_name: &str,
        spawn_point_id: &str,
    ) -> Result<(), String> {
        SceneSystem::transition(&mut self.game_state, scene_name, spawn_point_id)
    }

    pub fn sync_entities_to_active_scene(&mut self) {
        SceneSystem::sync_entities_to_active_scene(&mut self.game_state);
    }

    pub fn record_dialog_completion(&mut self, dialog_id: &str, outcome_id: &str) {
        RuleSystem::record_dialog_completion(&mut self.game_state, dialog_id, outcome_id);
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

    /// Get entity collision boxes for debug rendering
    pub fn get_entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .entity_collision_boxes()
    }

    /// Get solid tile positions for debug rendering
    pub fn get_solid_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .solid_tile_positions(tilemap, atlas)
    }

    /// Get trigger tile positions for debug rendering
    pub fn get_trigger_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        RenderQueryService::new(
            self.game_state.world().entity_manager(),
            self.game_state.world().player_id(),
            self.game_state.runtime().debug_collision_rendering(),
        )
        .trigger_tile_positions(tilemap, atlas)
    }

    /// Check if debug collision rendering is enabled
    pub fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.game_state.runtime().debug_collision_rendering()
    }
}

#[cfg(test)]
#[path = "game_manager_tests.rs"]
mod tests;
