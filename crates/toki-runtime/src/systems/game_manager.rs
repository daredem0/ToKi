use toki_core::entity::MovementProfile;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyboardBinding {
    Direct(InputKey),
    Profile {
        profile: MovementProfile,
        input_key: InputKey,
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
            }
        }
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
            KeyCode::F4 => Some(KeyboardBinding::Direct(InputKey::DebugToggle)),
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

    pub fn get_entity_current_atlas_name(
        &self,
        entity_id: toki_core::entity::EntityId,
    ) -> Option<String> {
        self.game_state.get_entity_current_atlas_name(entity_id)
    }

    pub fn get_entity_sprite_flip_x(&self, entity_id: toki_core::entity::EntityId) -> bool {
        self.game_state.get_entity_sprite_flip_x(entity_id)
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
    use std::collections::HashMap;
    use std::path::PathBuf;
    use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
    use toki_core::assets::tilemap::TileMap;
    use toki_core::GameState;
    use winit::keyboard::KeyCode;

    fn sample_atlas() -> AtlasMeta {
        let mut tiles = HashMap::new();
        tiles.insert(
            "slime/idle_0".to_string(),
            TileInfo {
                position: glam::UVec2::new(0, 0),
                properties: TileProperties::default(),
            },
        );
        tiles.insert(
            "slime/idle_1".to_string(),
            TileInfo {
                position: glam::UVec2::new(1, 0),
                properties: TileProperties::default(),
            },
        );
        tiles.insert(
            "solid".to_string(),
            TileInfo {
                position: glam::UVec2::new(0, 1),
                properties: TileProperties {
                    solid: true,
                    trigger: false,
                },
            },
        );
        tiles.insert(
            "trigger".to_string(),
            TileInfo {
                position: glam::UVec2::new(1, 1),
                properties: TileProperties {
                    solid: false,
                    trigger: true,
                },
            },
        );

        AtlasMeta {
            image: PathBuf::from("creatures.png"),
            tile_size: glam::UVec2::new(16, 16),
            tiles,
        }
    }

    fn sample_tilemap() -> TileMap {
        TileMap {
            size: glam::UVec2::new(2, 1),
            tile_size: glam::UVec2::new(16, 16),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec!["solid".to_string(), "trigger".to_string()],
        }
    }

    fn walkable_tilemap() -> TileMap {
        TileMap {
            size: glam::UVec2::new(2, 1),
            tile_size: glam::UVec2::new(16, 16),
            atlas: PathBuf::from("terrain.json"),
            tiles: vec!["trigger".to_string(), "trigger".to_string()],
        }
    }

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

    #[test]
    fn wrapper_methods_expose_core_entity_state() {
        let mut game_state = GameState::new_empty();
        let player_id = game_state.spawn_player_at(glam::IVec2::new(10, 12));
        let mut manager = GameManager::new(game_state);

        let npc_id = manager.spawn_player_like_npc(glam::IVec2::new(20, 12));
        let renderable = manager.get_renderable_entities();
        let entities_for_camera = manager.entities_for_camera();

        assert_eq!(manager.player_id(), Some(player_id));
        assert_eq!(manager.player_position(), glam::IVec2::new(10, 12));
        assert_eq!(manager.sprite_size(), 16);
        assert_eq!(renderable.len(), 2);
        assert_eq!(entities_for_camera.len(), 2);
        assert!(entities_for_camera
            .iter()
            .any(|entity| entity.id == player_id));
        assert!(entities_for_camera.iter().any(|entity| entity.id == npc_id));
    }

    #[test]
    fn sprite_frame_wrappers_resolve_from_atlas() {
        let mut game_state = GameState::new_empty();
        let player_id = game_state.spawn_player_at(glam::IVec2::new(0, 0));
        let manager = GameManager::new(game_state);
        let atlas = sample_atlas();
        let texture_size = atlas.image_size().expect("atlas image size should exist");

        let entity_frame = manager.get_entity_sprite_frame(player_id, &atlas, texture_size);
        let entity_frame = entity_frame.expect("player frame should resolve from atlas");

        let current_frame = manager.current_sprite_frame(&atlas, texture_size);
        assert_eq!(current_frame.u0, entity_frame.u0);
        assert_eq!(current_frame.v0, entity_frame.v0);
        assert_eq!(current_frame.u1, entity_frame.u1);
        assert_eq!(current_frame.v1, entity_frame.v1);
    }

    #[test]
    fn debug_collision_wrappers_return_tiles_and_boxes_when_enabled() {
        let mut game_state = GameState::new_empty();
        game_state.spawn_player_at(glam::IVec2::new(0, 0));
        let mut manager = GameManager::new(game_state);
        let atlas = sample_atlas();
        let tilemap = sample_tilemap();

        assert!(manager.get_entity_collision_boxes().is_empty());
        assert!(manager
            .get_solid_tile_positions(&tilemap, &atlas)
            .is_empty());
        assert!(manager
            .get_trigger_tile_positions(&tilemap, &atlas)
            .is_empty());

        manager.handle_keyboard_input(KeyCode::F4, true);

        assert!(!manager.get_entity_collision_boxes().is_empty());
        assert_eq!(
            manager.get_solid_tile_positions(&tilemap, &atlas),
            vec![(0, 0)]
        );
        assert_eq!(
            manager.get_trigger_tile_positions(&tilemap, &atlas),
            vec![(1, 0)]
        );
    }

    #[test]
    fn player_wasd_profile_ignores_arrow_keys_for_movement() {
        let mut game_state = GameState::new_empty();
        let player_id = game_state.spawn_player_at(glam::IVec2::new(0, 0));
        let mut manager = GameManager::new(game_state);
        let atlas = sample_atlas();
        let tilemap = walkable_tilemap();

        manager.handle_keyboard_input(KeyCode::ArrowRight, true);
        let result = manager.update(glam::UVec2::new(128, 128), &tilemap, &atlas);
        manager.handle_keyboard_input(KeyCode::ArrowRight, false);

        assert!(!result.player_moved);
        assert_eq!(
            manager
                .game_state
                .entity_manager()
                .get_entity(player_id)
                .expect("player should exist")
                .position,
            glam::IVec2::new(0, 0)
        );
    }

    #[test]
    fn player_wasd_profile_moves_from_wasd_keys() {
        let mut game_state = GameState::new_empty();
        let player_id = game_state.spawn_player_at(glam::IVec2::new(0, 0));
        let mut manager = GameManager::new(game_state);
        let atlas = sample_atlas();
        let tilemap = walkable_tilemap();

        manager.handle_keyboard_input(KeyCode::KeyD, true);
        let result = manager.update(glam::UVec2::new(128, 128), &tilemap, &atlas);
        manager.handle_keyboard_input(KeyCode::KeyD, false);

        assert!(result.player_moved);
        assert_eq!(
            manager
                .game_state
                .entity_manager()
                .get_entity(player_id)
                .expect("player should exist")
                .position,
            glam::IVec2::new(1, 0)
        );
    }
}
