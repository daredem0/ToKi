use glam::{IVec2, UVec2};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::{
    game::{GameSimulation, InputSystem, RenderQueryService},
    GameState, InputKey, DEFAULT_TIMESTEP_MS,
};
use toki_test_fixtures::{test_atlas, test_tilemap};

fn create_test_sprite() -> SpriteInstance {
    let animation = Animation {
        name: "tick_test".into(),
        looped: true,
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 100,
            },
            Frame {
                index: 1,
                duration_ms: 100,
            },
        ],
    };
    let sprite_sheet = SpriteSheetMeta {
        frame_size: (16, 16),
        frame_count: 2,
        sheet_size: (32, 16),
    };
    SpriteInstance::new(IVec2::new(50, 60), animation, sprite_sheet)
}

fn player_position(state: &GameState) -> IVec2 {
    RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
    .player_position()
}

#[test]
fn default_delta_tick_matches_fixed_tick_movement() {
    let mut fixed_state = GameState::new(create_test_sprite());
    let mut delta_state = GameState::new(create_test_sprite());

    let world_bounds = UVec2::new(1000, 1000);
    let tilemap = test_tilemap();
    let atlas = test_atlas();

    InputSystem::handle_key_press(fixed_state.runtime_mut(), InputKey::Right);
    InputSystem::handle_key_press(delta_state.runtime_mut(), InputKey::Right);

    GameSimulation::tick_fixed(&mut fixed_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_with_delta(
        &mut delta_state,
        DEFAULT_TIMESTEP_MS,
        world_bounds,
        &tilemap,
        &atlas,
    );

    assert_eq!(player_position(&fixed_state), player_position(&delta_state));
}
