use crate::animation::AnimationState;
use crate::collision;
use crate::entity::{Entity, EntityId, ATTACK_POWER_STAT_ID, HEALTH_STAT_ID};

use super::game_animation::FacingDirection;
use super::{GameState, InputAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StatChangeRequest {
    pub(super) target_entity_id: EntityId,
    pub(super) stat_id: String,
    pub(super) delta: i32,
    pub(super) source_entity_id: Option<EntityId>,
}

impl GameState {
    fn primary_action_damage_for_entity(entity: &Entity) -> i32 {
        entity
            .attributes
            .current_stat(ATTACK_POWER_STAT_ID)
            .or_else(|| entity.attributes.base_stat(ATTACK_POWER_STAT_ID))
            .unwrap_or(10)
    }

    fn entity_bounds_for_stat_interaction(entity: &Entity) -> (glam::IVec2, glam::UVec2) {
        if let Some(collision_box) = &entity.collision_box {
            collision_box.world_bounds(entity.position)
        } else {
            (entity.position, entity.size)
        }
    }

    fn primary_action_hitbox(
        entity: &Entity,
        facing: FacingDirection,
    ) -> (glam::IVec2, glam::UVec2) {
        let (origin, size) = Self::entity_bounds_for_stat_interaction(entity);
        match facing {
            FacingDirection::Down => (glam::IVec2::new(origin.x, origin.y + size.y as i32), size),
            FacingDirection::Up => (glam::IVec2::new(origin.x, origin.y - size.y as i32), size),
            FacingDirection::Left => (glam::IVec2::new(origin.x - size.x as i32, origin.y), size),
            FacingDirection::Right => (glam::IVec2::new(origin.x + size.x as i32, origin.y), size),
        }
    }

    fn collect_primary_action_stat_changes(
        &self,
        attacker_id: EntityId,
        facing: FacingDirection,
    ) -> Vec<StatChangeRequest> {
        let Some(attacker) = self.entity_manager.get_entity(attacker_id) else {
            return Vec::new();
        };

        let damage = Self::primary_action_damage_for_entity(attacker);
        if damage <= 0 {
            return Vec::new();
        }

        let (hitbox_pos, hitbox_size) = Self::primary_action_hitbox(attacker, facing);
        let mut target_ids = self.entity_manager.active_entities();
        target_ids.sort_unstable();

        let changes = target_ids
            .into_iter()
            .filter(|&target_id| target_id != attacker_id)
            .filter_map(|target_id| {
                let target = self.entity_manager.get_entity(target_id)?;
                if !target.attributes.active
                    || target.attributes.current_stat(HEALTH_STAT_ID).is_none()
                {
                    return None;
                }
                let (target_pos, target_size) = Self::entity_bounds_for_stat_interaction(target);
                if !collision::aabb_overlap(hitbox_pos, hitbox_size, target_pos, target_size) {
                    return None;
                }
                Some(StatChangeRequest {
                    target_entity_id: target_id,
                    stat_id: HEALTH_STAT_ID.to_string(),
                    delta: -damage,
                    source_entity_id: Some(attacker_id),
                })
            })
            .collect::<Vec<_>>();

        if changes.is_empty() {
            tracing::debug!(
                "Primary action from entity {} facing {:?} produced no damage targets",
                attacker_id,
                facing
            );
        } else {
            for change in &changes {
                tracing::debug!(
                    "Primary action from entity {} queued {} change {} for target {}",
                    attacker_id,
                    change.stat_id,
                    change.delta,
                    change.target_entity_id
                );
            }
        }

        changes
    }

    pub(super) fn resolve_pending_stat_changes(&mut self) {
        let pending_stat_changes = std::mem::take(&mut self.pending_stat_changes);
        if pending_stat_changes.is_empty() {
            return;
        }

        let mut despawn_ids = Vec::new();
        for change in pending_stat_changes {
            let Some(entity) = self.entity_manager.get_entity_mut(change.target_entity_id) else {
                continue;
            };
            let previous_value = entity.attributes.current_stat(&change.stat_id);
            let Some(new_value) = entity
                .attributes
                .apply_stat_delta(&change.stat_id, change.delta)
            else {
                continue;
            };

            tracing::debug!(
                "Applied stat change: source={:?} target={} stat={} delta={} previous={:?} new={}",
                change.source_entity_id,
                change.target_entity_id,
                change.stat_id,
                change.delta,
                previous_value,
                new_value
            );

            if change.stat_id == HEALTH_STAT_ID && change.delta < 0 {
                self.rule_runtime.frame_damage_detected = true;
            }

            if change.stat_id == HEALTH_STAT_ID && new_value <= 0 {
                self.rule_runtime.frame_death_detected = true;
                tracing::info!(
                    "Entity {} reached zero {} and will be despawned",
                    change.target_entity_id,
                    change.stat_id
                );
                despawn_ids.push(change.target_entity_id);
            }
        }

        despawn_ids.sort_unstable();
        despawn_ids.dedup();
        for entity_id in despawn_ids {
            self.entity_manager.despawn_entity(entity_id);
        }
    }

    fn trigger_entity_primary_action(&mut self, entity_id: EntityId) -> bool {
        let triggered_facing = {
            let Some(animation_controller) = self
                .entity_manager
                .get_entity_mut(entity_id)
                .and_then(|entity| entity.attributes.animation_controller.as_mut())
            else {
                return false;
            };

            let facing = Self::facing_from_animation_state(animation_controller.current_clip_state);
            let directional_attack = Self::directional_attack_state(facing);
            let next_state = if animation_controller.has_clip(directional_attack) {
                directional_attack
            } else if animation_controller.has_clip(AnimationState::Attack) {
                AnimationState::Attack
            } else {
                return false;
            };

            if animation_controller.play(next_state) {
                Some(facing)
            } else {
                None
            }
        };

        let Some(facing) = triggered_facing else {
            return false;
        };

        tracing::debug!(
            "Entity {} triggered primary action facing {:?}",
            entity_id,
            facing
        );

        self.pending_stat_changes
            .extend(self.collect_primary_action_stat_changes(entity_id, facing));
        true
    }

    pub(super) fn process_profile_actions(&mut self) {
        let pending_actions = std::mem::take(&mut self.pending_profile_actions);
        if pending_actions.is_empty() {
            return;
        }

        let controlled_entity_ids = self.controlled_input_entity_ids();
        if controlled_entity_ids.is_empty() {
            return;
        }

        for (profile, actions) in pending_actions {
            if !actions.contains(&InputAction::Primary) {
                continue;
            }
            for &entity_id in &controlled_entity_ids {
                let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                    continue;
                };
                if Self::effective_movement_profile(entity) != profile {
                    continue;
                }
                self.trigger_entity_primary_action(entity_id);
            }
        }
    }
}
