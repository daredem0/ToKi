use super::{
    AudioEvent, CombatSystem, GameSimulation, GameState, GameUpdateResult, InteractionSystem,
    MovementService, MovementSystem, RuleSystem, UpdateContext, WorldContext, DEFAULT_TIMESTEP_MS,
};

struct TickPhaseState {
    animation_delta_ms: f32,
    result: GameUpdateResult<AudioEvent>,
    pending_rule_animations: Vec<(crate::entity::EntityId, crate::animation::AnimationState)>,
    pending_scene_switch: Option<super::rules::PendingSceneSwitch>,
    pending_persistence: Option<crate::events::PersistenceRequest>,
    pending_ui_requests: Vec<crate::ui_layout::UiRequest>,
    initial_player_position: glam::IVec2,
}

impl TickPhaseState {
    fn new(state: &mut GameState, time_scale: f32) -> Self {
        let animation_delta_ms = (DEFAULT_TIMESTEP_MS * time_scale).max(0.0);
        state.progress.play_time_remainder_ms += animation_delta_ms;
        let play_time_increment = state.progress.play_time_remainder_ms.floor() as u64;
        state.progress.play_time_ms = state
            .progress
            .play_time_ms
            .saturating_add(play_time_increment);
        state.progress.play_time_remainder_ms -= play_time_increment as f32;

        let initial_player_position = state
            .world
            .player_id()
            .and_then(|player_id| state.world.entity_manager.get_entity(player_id))
            .map(|entity| entity.position)
            .unwrap_or(glam::IVec2::ZERO);

        Self {
            animation_delta_ms,
            result: GameUpdateResult::new(),
            pending_rule_animations: Vec::new(),
            pending_scene_switch: None,
            pending_persistence: None,
            pending_ui_requests: Vec::new(),
            initial_player_position,
        }
    }
}

impl GameSimulation {
    pub fn tick(state: &mut GameState, ctx: UpdateContext<'_>) -> GameUpdateResult<AudioEvent> {
        let world = WorldContext::from(ctx);
        let mut phases = Self::begin_tick(state, ctx.time_scale, world);

        Self::tick_input_and_movement(state, world, ctx.time_scale, &mut phases);
        Self::tick_combat_and_interactions(state, world, ctx.time_scale, &mut phases);
        Self::tick_ai_and_tile_transitions(state, world, &mut phases);
        Self::tick_rules_and_reactions(state, world, &mut phases);

        Self::finalize_tick(state, &mut phases)
    }

    pub fn tick_fixed(
        state: &mut GameState,
        world_bounds: glam::UVec2,
        tilemap: &crate::assets::tilemap::TileMap,
        tileset: &crate::assets::tileset::TileSetResolver<'_>,
    ) -> GameUpdateResult<AudioEvent> {
        Self::tick(
            state,
            UpdateContext {
                time_scale: 1.0,
                world_bounds,
                tilemap,
                tileset,
            },
        )
    }

    pub fn tick_with_delta(
        state: &mut GameState,
        delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &crate::assets::tilemap::TileMap,
        tileset: &crate::assets::tileset::TileSetResolver<'_>,
    ) -> GameUpdateResult<AudioEvent> {
        Self::tick(
            state,
            UpdateContext {
                time_scale: delta_ms / DEFAULT_TIMESTEP_MS,
                world_bounds,
                tilemap,
                tileset,
            },
        )
    }

    fn begin_tick(
        state: &mut GameState,
        time_scale: f32,
        world: WorldContext<'_>,
    ) -> TickPhaseState {
        let mut phases = TickPhaseState::new(state, time_scale);
        let mut rule_commands = Vec::new();
        RuleSystem::begin_frame(state);
        RuleSystem::collect_frame_commands(state, &mut rule_commands);
        let command_result =
            RuleSystem::apply_commands(state, rule_commands, &mut phases.result, world.tilemap);
        phases.pending_rule_animations = command_result.pending_animations;
        phases.pending_scene_switch = command_result.pending_scene_switch;
        phases.pending_persistence = command_result.pending_persistence;
        phases.pending_ui_requests = command_result.pending_ui_requests;
        phases
    }

    fn tick_input_and_movement(
        state: &mut GameState,
        world: WorldContext<'_>,
        time_scale: f32,
        phases: &mut TickPhaseState,
    ) {
        let input_result = MovementSystem::process_input_scaled(state, world, time_scale);
        phases.result.player_moved = input_result.player_moved;
        phases.result.add_events(input_result.events);

        if MovementSystem::apply_rule_velocities(state, world, &mut phases.result) {
            phases.result.player_moved = true;
        }

        let intended_player_delta = state
            .world
            .player_id()
            .and_then(|player_id| {
                state
                    .world
                    .entity_manager
                    .get_entity(player_id)
                    .map(|entity| {
                        state.held_keys_for_profile(entity.effective_movement_profile(
                            state.world.entity_manager.movement(player_id),
                        ))
                    })
            })
            .map(|keys| MovementService::movement_delta_from_keys(&keys))
            .unwrap_or(glam::IVec2::ZERO);

        MovementSystem::update_player_animation(
            state,
            phases.initial_player_position,
            intended_player_delta,
        );
    }

    fn tick_combat_and_interactions(
        state: &mut GameState,
        world: WorldContext<'_>,
        time_scale: f32,
        _phases: &mut TickPhaseState,
    ) {
        CombatSystem::process_profile_actions(state);
        CombatSystem::update_projectiles(state, world, time_scale);
        InteractionSystem::collect_overlapping_pickups(state);
        InteractionSystem::collect_interaction_events(state);
        state.resolve_pending_stat_changes();
    }

    fn tick_ai_and_tile_transitions(
        state: &mut GameState,
        world: WorldContext<'_>,
        phases: &mut TickPhaseState,
    ) {
        state.update_npc_ai_with_delta(phases.animation_delta_ms, world, &mut phases.result);
        state.detect_tile_transitions(world.tilemap);
    }

    fn tick_rules_and_reactions(
        state: &mut GameState,
        world: WorldContext<'_>,
        phases: &mut TickPhaseState,
    ) {
        let reactive_rule_commands = RuleSystem::collect_reactive_commands(
            state,
            phases.result.player_moved,
            world.tilemap,
            world.tileset,
        );
        let mut command_result = RuleSystem::apply_commands(
            state,
            reactive_rule_commands,
            &mut phases.result,
            world.tilemap,
        );
        if phases.pending_scene_switch.is_none() {
            phases.pending_scene_switch = command_result.pending_scene_switch;
        }
        if phases.pending_persistence.is_none() {
            phases.pending_persistence = command_result.pending_persistence;
        }
        phases
            .pending_rule_animations
            .append(&mut command_result.pending_animations);
        phases
            .pending_ui_requests
            .append(&mut command_result.pending_ui_requests);
    }

    fn finalize_tick(
        state: &mut GameState,
        phases: &mut TickPhaseState,
    ) -> GameUpdateResult<AudioEvent> {
        state.apply_rule_animations(std::mem::take(&mut phases.pending_rule_animations));
        state.flush_pending_despawns();

        let completed_animation_loops = state
            .world
            .entity_manager
            .update_animations(phases.animation_delta_ms);
        for (entity_id, completed_loops) in completed_animation_loops {
            MovementSystem::emit_animation_loop_audio(
                state,
                entity_id,
                completed_loops,
                &mut phases.result,
            );
        }

        if let Some(request) = phases.pending_scene_switch.take() {
            phases.result.request_scene_switch(
                request.scene_name,
                request.spawn_point_id,
                request.transition,
                request.duration_ms,
            );
        }
        match phases.pending_persistence.take() {
            Some(crate::events::PersistenceRequest::SaveSlot { slot }) => {
                phases.result.request_save_slot(slot);
            }
            Some(crate::events::PersistenceRequest::LoadSlot { slot }) => {
                phases.result.request_load_slot(slot);
            }
            None => {}
        }
        phases
            .result
            .ui_requests
            .append(&mut phases.pending_ui_requests);

        std::mem::replace(&mut phases.result, GameUpdateResult::new())
    }
}
