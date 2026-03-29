use glam::{IVec2, UVec2};
use toki_core::animation::{AnimationClip, AnimationState, LoopMode};
use toki_core::entity::{MovementProfile, PrimaryProjectileDef};
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::{
    game::{GameSimulation, InputAction, InputSystem, RenderQueryService},
    GameState,
};
use toki_test_fixtures::{test_atlas, test_entity_definition, test_tilemap};

fn create_test_sprite() -> SpriteInstance {
    let animation = Animation {
        name: "combat_test".into(),
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

fn sprite_render_requests(
    state: &GameState,
) -> Vec<toki_core::sprite_render::SpriteRenderRequest> {
    RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
    .sprite_render_requests()
}

#[test]
fn projectile_damage_resolves_and_despawns_on_hit() {
    let mut game_state = GameState::new(create_test_sprite());
    let player_id = game_state.world().player_id().expect("player should exist");
    game_state
        .world_mut()
        .entity_manager_mut()
        .storage_mut()
        .components_mut()
        .set_primary_projectile(
            player_id,
            Some(PrimaryProjectileDef {
                sheet: "fauna".to_string(),
                object_name: "rock".to_string(),
                size: [16, 16],
                speed: 4,
                damage: 8,
                lifetime_ticks: 10,
                spawn_offset: [0, 0],
            }),
        );
    let player = game_state
        .world_mut()
        .entity_manager_mut()
        .get_player_mut()
        .expect("player should exist");
    let controller = player
        .attributes
        .rendering
        .animation_controller
        .as_mut()
        .expect("player controller should exist");
    controller.add_clip(AnimationClip {
        state: AnimationState::IdleRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/walk_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: LoopMode::Loop,
    });
    controller.add_clip(AnimationClip {
        state: AnimationState::AttackRight,
        atlas_name: "players.json".to_string(),
        frame_tile_names: vec!["player/attack_right_a".to_string()],
        frame_positions: None,
        frame_duration_ms: 120.0,
        frame_durations_ms: None,
        loop_mode: LoopMode::Once,
    });
    controller.play(AnimationState::IdleRight);

    let mut target_definition = test_entity_definition("projectile_target", "creature");
    target_definition.attributes.health = Some(25);
    let target_id = game_state
        .world_mut()
        .entity_manager_mut()
        .spawn_from_definition(&target_definition, IVec2::new(90, 60))
        .expect("target should spawn");

    let world_bounds = UVec2::new(160, 128);
    let tilemap = test_tilemap();
    let atlas = test_atlas();

    InputSystem::handle_profile_action_press(
        game_state.runtime_mut(),
        MovementProfile::PlayerWasd,
        InputAction::Primary,
    );
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);
    GameSimulation::tick_fixed(&mut game_state, world_bounds, &tilemap, &atlas);

    let target = game_state
        .world()
        .entity_manager()
        .get_entity(target_id)
        .expect("target should survive non-lethal projectile damage");
    assert_eq!(target.attributes.current_stat("health"), Some(17));
    assert!(
        !sprite_render_requests(&game_state)
            .into_iter()
            .any(|request| matches!(
                request.origin,
                toki_core::sprite_render::SpriteRenderOrigin::Projectile(_)
            )),
        "projectile should despawn on hit"
    );
}
