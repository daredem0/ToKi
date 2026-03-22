//! RunAndMultiply AI behavior implementation.

use crate::animation::AnimationState;
use crate::entity::{Entity, EntityId, EntityManager};
use glam::IVec2;

use super::constants::TILE_SIZE_PX;
use super::context::AiContext;
use super::movement::{
    compute_directions_away, compute_directions_toward, distance_between,
    try_movement_with_fallback,
};
use super::system::AiSystem;
use super::types::{AiSpawnRequest, AiUpdateResult, SpawnMode};

impl AiSystem {
    /// Update RunAndMultiply entity behavior.
    /// Priority: separation > flee from player > seek mate > idle wander
    pub(super) fn update_run_and_multiply_entity(
        &mut self,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let entity = ctx.entity_manager.get_entity(entity_id)?;
        let current_position = entity.position;
        let detection_radius = entity.attributes.ai_config.detection_radius;
        let definition_name = entity.definition_name.clone();

        // Handle separation state first
        if let Some(result) = self.handle_separation(entity, entity_id, ctx) {
            return Some(result);
        }

        let player_in_range =
            self.is_player_in_range(player_position, current_position, detection_radius);
        let mate = self.find_compatible_entity(
            entity_id,
            &definition_name,
            ctx.entity_manager,
            detection_radius,
        );

        // Check for mating collision
        if let Some(mate_id) = mate {
            if let Some(result) = self.handle_mating_collision(
                entity,
                entity_id,
                mate_id,
                ctx.entity_manager,
                detection_radius,
            ) {
                return Some(result);
            }
        }

        // Flee from player if in range
        if player_in_range {
            return self.flee_and_seek(entity, entity_id, player_position, mate, ctx);
        }

        // Seek mate if one exists
        if let Some(mate_id) = mate {
            return self.seek_entity(entity, entity_id, mate_id, ctx);
        }

        // No threats or mates - idle wander
        self.idle_wander(entity, entity_id, ctx)
    }

    fn is_player_in_range(
        &self,
        player_pos: Option<IVec2>,
        entity_pos: IVec2,
        radius: u32,
    ) -> bool {
        player_pos.is_some_and(|pos| distance_between(entity_pos, pos) <= radius as f32)
    }

    fn find_compatible_entity(
        &self,
        entity_id: EntityId,
        definition_name: &Option<String>,
        entity_manager: &EntityManager,
        detection_radius: u32,
    ) -> Option<EntityId> {
        let def_name = definition_name.as_ref()?;
        let entity = entity_manager.get_entity(entity_id)?;
        let current_pos = entity.position;
        let entity_kind = entity.entity_kind;

        entity_manager
            .active_entities_iter()
            .filter(|&other_id| other_id != entity_id)
            .filter_map(|other_id| {
                let other = entity_manager.get_entity(other_id)?;
                let other_def = other.definition_name.as_ref()?;
                if other_def == def_name
                    && other.entity_kind == entity_kind
                    && !self.is_entity_separating(other_id)
                {
                    let dist = distance_between(current_pos, other.position);
                    if dist <= detection_radius as f32 {
                        return Some(other_id);
                    }
                }
                None
            })
            .next()
    }

    fn handle_separation(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let state = self.entity_states.get(&entity_id)?;
        let separation = state.separation_state.as_ref()?;
        let other_ids = separation.other_entity_ids.clone();
        let required_distance = separation.required_distance;

        let closest = other_ids
            .iter()
            .filter_map(|&id| {
                let other = ctx.entity_manager.get_entity(id)?;
                let dist = distance_between(entity.position, other.position);
                Some((id, other.position, dist))
            })
            .min_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        let Some((_, closest_pos, _)) = closest else {
            let state = self.entity_states.get_mut(&entity_id)?;
            state.separation_state = None;
            return None;
        };

        let all_separated = other_ids.iter().all(|&id| {
            ctx.entity_manager
                .get_entity(id)
                .map(|other| distance_between(entity.position, other.position) >= required_distance)
                .unwrap_or(true)
        });

        if all_separated {
            let state = self.entity_states.get_mut(&entity_id)?;
            state.separation_state = None;
            return None;
        }

        let movement_step = entity.attributes.speed.round() as i32;
        let directions = compute_directions_away(entity.position, closest_pos, movement_step);

        try_movement_with_fallback(entity, entity_id, entity.position, &directions, ctx)
    }

    fn handle_mating_collision(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        mate_id: EntityId,
        entity_manager: &EntityManager,
        detection_radius: u32,
    ) -> Option<AiUpdateResult> {
        let mate = entity_manager.get_entity(mate_id)?;
        if !Self::entities_adjacent(entity, mate) {
            return None;
        }

        let spawn_pos = Self::find_free_spawn_position(entity, mate, entity_manager)?;
        let required_distance = (detection_radius * 2) as f32;

        self.enter_separation_state(entity_id, vec![mate_id], required_distance);
        self.enter_separation_state(mate_id, vec![entity_id], required_distance);

        Some(AiUpdateResult {
            entity_id,
            new_position: None,
            new_animation: Some(AnimationState::Idle),
            movement_distance: 0.0,
            spawn_request: Some(AiSpawnRequest {
                position: spawn_pos,
                parent_entity_ids: vec![entity_id, mate_id],
                separation_distance: required_distance,
                mode: SpawnMode::Clone {
                    source_entity_id: entity_id,
                },
            }),
        })
    }

    fn entities_adjacent(a: &Entity, b: &Entity) -> bool {
        let a_min = a.position;
        let a_max = a.position + a.size.as_ivec2();
        let b_min = b.position;
        let b_max = b.position + b.size.as_ivec2();

        let y_overlap = a_min.y < b_max.y && a_max.y > b_min.y;
        let x_overlap = a_min.x < b_max.x && a_max.x > b_min.x;

        let h_adjacent =
            y_overlap && ((a_max.x - b_min.x).abs() <= 2 || (b_max.x - a_min.x).abs() <= 2);
        let v_adjacent =
            x_overlap && ((a_max.y - b_min.y).abs() <= 2 || (b_max.y - a_min.y).abs() <= 2);

        h_adjacent || v_adjacent
    }

    fn find_free_spawn_position(
        entity: &Entity,
        mate: &Entity,
        entity_manager: &EntityManager,
    ) -> Option<IVec2> {
        let tile_size = TILE_SIZE_PX;
        let size = entity.size;
        let offsets = [
            IVec2::new(tile_size, 0),
            IVec2::new(-tile_size, 0),
            IVec2::new(0, tile_size),
            IVec2::new(0, -tile_size),
        ];

        for parent_pos in [entity.position, mate.position] {
            for offset in &offsets {
                let candidate = parent_pos + *offset;
                if candidate.x >= 0
                    && candidate.y >= 0
                    && entity_manager.is_spawn_position_free(candidate, size)
                {
                    return Some(candidate);
                }
            }
        }
        None
    }

    fn flee_and_seek(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        player_position: Option<IVec2>,
        mate: Option<EntityId>,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let player_pos = player_position?;
        let movement_step = entity.attributes.speed.round() as i32;

        if let Some(mate_id) = mate {
            if let Some(mate_entity) = ctx.entity_manager.get_entity(mate_id) {
                let directions =
                    compute_directions_toward(entity.position, mate_entity.position, movement_step);
                let result =
                    try_movement_with_fallback(entity, entity_id, entity.position, &directions, ctx);
                if result.as_ref().is_some_and(|r| r.new_position.is_some()) {
                    return result;
                }
            }
        }

        let directions = compute_directions_away(entity.position, player_pos, movement_step);
        try_movement_with_fallback(entity, entity_id, entity.position, &directions, ctx)
    }

    fn seek_entity(
        &mut self,
        entity: &Entity,
        entity_id: EntityId,
        target_id: EntityId,
        ctx: &AiContext,
    ) -> Option<AiUpdateResult> {
        let target = ctx.entity_manager.get_entity(target_id)?;
        let movement_step = entity.attributes.speed.round() as i32;
        let directions =
            compute_directions_toward(entity.position, target.position, movement_step);

        try_movement_with_fallback(entity, entity_id, entity.position, &directions, ctx)
    }
}
