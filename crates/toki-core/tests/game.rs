use glam::{IVec2, UVec2};
mod support;
use support::{test_atlas, test_tilemap};
use toki_core::entity::{
    build_decoration_entity, CombatComponent, DecorationSpec, Entity, EntityKind, EntityStats,
    OptionalEntityComponents, StoredEntity,
};
use toki_core::game::{GameSimulation, InputSystem, RenderQueryService, SceneSystem};
use toki_core::scene::Scene;
use toki_core::sprite::{Animation, Frame, SpriteInstance, SpriteSheetMeta};
use toki_core::{GameState, InputKey};

fn create_test_sprite() -> SpriteInstance {
    let animation = Animation {
        name: "game_test".into(),
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
    SpriteInstance::new(IVec2::new(0, 0), animation, sprite_sheet)
}

fn render_queries(state: &GameState) -> RenderQueryService<'_> {
    RenderQueryService::new(
        state.world().entity_manager(),
        state.world().player_id(),
        state.runtime().debug_collision_rendering(),
    )
}

fn player_position(state: &GameState) -> IVec2 {
    render_queries(state).player_position()
}

#[test]
fn fixed_tick_moves_player_right() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(1000, 1000),
        &test_tilemap(),
        &test_atlas(),
    );

    let after = player_position(&state);
    assert!(after.x > before.x);
    assert_eq!(after.y, before.y);
}

#[test]
fn fixed_tick_allows_diagonal_player_movement() {
    let mut state = GameState::new(create_test_sprite());
    let before = player_position(&state);

    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Right);
    InputSystem::handle_key_press(state.runtime_mut(), InputKey::Up);
    GameSimulation::tick_fixed(
        &mut state,
        UVec2::new(1000, 1000),
        &test_tilemap(),
        &test_atlas(),
    );

    let after = player_position(&state);
    assert!(after.x > before.x);
    assert_ne!(after, before);
}

#[test]
fn render_queries_include_visible_decoration_entities() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    scene.add_entity(build_decoration_entity(
        7,
        DecorationSpec::new(IVec2::new(32, 48), UVec2::new(16, 16), "items", "coin"),
    ));
    SceneSystem::add_scene(&mut state, scene);
    SceneSystem::load(&mut state, "main").expect("scene should load");

    let renderable_ids = render_queries(&state)
        .renderable_entities()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect::<Vec<_>>();

    assert!(renderable_ids.contains(&7));
}

#[test]
fn render_queries_skip_hidden_decoration_entities() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    let mut decoration = build_decoration_entity(
        8,
        DecorationSpec::new(IVec2::new(16, 16), UVec2::new(16, 16), "items", "gem"),
    );
    decoration.rendering.visible = false;
    scene.add_entity(decoration);
    SceneSystem::add_scene(&mut state, scene);
    SceneSystem::load(&mut state, "main").expect("scene should load");

    let renderable_ids = render_queries(&state)
        .renderable_entities()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect::<Vec<_>>();

    assert!(!renderable_ids.contains(&8));
}

#[test]
fn entity_health_bars_only_include_active_visible_combat_entities() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());

    let visible_fighter = Entity {
        id: 1,
        position: IVec2::new(20, 30),
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: None,
        persistent_across_saves: false,
        control_role: toki_core::entity::ControlRole::None,
        audio: toki_core::entity::EntityAudioSettings::default(),
        rendering: toki_core::entity::EntityRendering::default(),
        collision_box: None,
        solid: true,
        active: true,
        movement_accumulator: glam::Vec2::ZERO,
        tags: vec![],
    };
    let mut hidden_fighter = visible_fighter.clone();
    hidden_fighter.id = 2;
    hidden_fighter.rendering.visible = false;
    let mut inactive_fighter = visible_fighter.clone();
    inactive_fighter.id = 3;
    inactive_fighter.active = false;

    let mut stats = EntityStats::from_legacy_health(Some(20));
    stats.current.insert("health".to_string(), 12);
    let combat = CombatComponent {
        health: Some(12),
        stats,
    };

    scene.add_stored_entity(StoredEntity::new(
        visible_fighter,
        OptionalEntityComponents {
            combat: Some(combat.clone()),
            ..Default::default()
        },
    ));
    scene.add_stored_entity(StoredEntity::new(
        hidden_fighter,
        OptionalEntityComponents {
            combat: Some(combat.clone()),
            ..Default::default()
        },
    ));
    scene.add_stored_entity(StoredEntity::new(
        inactive_fighter,
        OptionalEntityComponents {
            combat: Some(combat),
            ..Default::default()
        },
    ));

    SceneSystem::add_scene(&mut state, scene);
    SceneSystem::load(&mut state, "main").expect("scene should load");

    let bars = render_queries(&state).entity_health_bars();
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].entity_id, 1);
    assert_eq!(bars[0].current, 12);
    assert_eq!(bars[0].max, 20);
}

#[test]
fn build_decoration_entity_is_inactive_but_still_visible_to_render_queries() {
    let decoration = build_decoration_entity(
        99,
        DecorationSpec::new(IVec2::new(64, 80), UVec2::new(16, 16), "items", "coin"),
    );

    assert_eq!(decoration.entity_kind, EntityKind::Decoration);
    assert!(!decoration.active);
    assert!(decoration.rendering.static_object_render.is_some());
}
