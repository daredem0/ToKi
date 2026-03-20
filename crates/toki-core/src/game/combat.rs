use crate::animation::AnimationState;
use crate::collision;
use crate::collision::CollisionBox;
use crate::entity::{Entity, EntityId, ATTACK_POWER_STAT_ID, HEALTH_STAT_ID};

use super::animation::FacingDirection;
use super::rules::{DamageEvent, DeathEvent};
use super::{GameState, InputAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StatChangeRequest {
    pub(super) target_entity_id: EntityId,
    pub(super) stat_id: String,
    pub(super) delta: i32,
    pub(super) source_entity_id: Option<EntityId>,
}

impl GameState {
    fn facing_vector(facing: FacingDirection) -> glam::IVec2 {
        match facing {
            FacingDirection::Down => glam::IVec2::new(0, 1),
            FacingDirection::Up => glam::IVec2::new(0, -1),
            FacingDirection::Left => glam::IVec2::new(-1, 0),
            FacingDirection::Right => glam::IVec2::new(1, 0),
        }
    }

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

    fn spawn_primary_projectile(&mut self, attacker_id: EntityId, facing: FacingDirection) {
        let Some(attacker) = self.entity_manager.get_entity(attacker_id) else {
            return;
        };
        let Some(spec) = attacker.attributes.primary_projectile.clone() else {
            return;
        };
        if spec.size[0] == 0 || spec.size[1] == 0 || spec.lifetime_ticks == 0 {
            return;
        }

        let facing_vector = Self::facing_vector(facing);
        let spawn_position = attacker.position
            + glam::IVec2::new(spec.spawn_offset[0], spec.spawn_offset[1])
            + glam::IVec2::new(
                facing_vector.x * attacker.size.x as i32,
                facing_vector.y * attacker.size.y as i32,
            );
        let size = glam::UVec2::new(spec.size[0], spec.size[1]);
        let velocity = glam::IVec2::new(
            facing_vector.x * spec.speed as i32,
            facing_vector.y * spec.speed as i32,
        );
        let debug_sheet = spec.sheet.clone();
        let debug_object_name = spec.object_name.clone();
        let debug_damage = spec.damage.max(0);
        let debug_lifetime_ticks = spec.lifetime_ticks;

        let attributes = crate::entity::EntityAttributes {
            speed: 0.0,
            solid: false,
            visible: true,
            can_move: false,
            ai_behavior: crate::entity::AiBehavior::None,
            movement_profile: crate::entity::MovementProfile::None,
            primary_projectile: None,
            projectile: Some(crate::entity::ProjectileState {
                sheet: spec.sheet,
                object_name: spec.object_name,
                size: spec.size,
                velocity: [velocity.x, velocity.y],
                remaining_ticks: spec.lifetime_ticks,
                damage: spec.damage.max(0),
                owner_id: Some(attacker_id),
            }),
            ..crate::entity::EntityAttributes::default()
        };
        let projectile_id = self.entity_manager.spawn_entity(
            crate::entity::EntityKind::Projectile,
            spawn_position,
            size,
            attributes.clone(),
        );
        if let Some(projectile) = self.entity_manager.get_entity_mut(projectile_id) {
            projectile.category = "projectile".to_string();
            projectile.collision_box = Some(CollisionBox::solid_box(size));
            projectile.attributes = attributes;
        }

        tracing::debug!(
            "Entity {} spawned projectile {} using {}/{} at {:?} velocity {:?} damage={} lifetime_ticks={}",
            attacker_id,
            projectile_id,
            debug_sheet,
            debug_object_name,
            spawn_position,
            velocity,
            debug_damage,
            debug_lifetime_ticks
        );
    }

    fn projectile_hit_target(&self, projectile_id: EntityId) -> Option<EntityId> {
        let projectile = self.entity_manager.get_entity(projectile_id)?;
        let projectile_state = projectile.attributes.projectile.as_ref()?;
        let (projectile_pos, projectile_size) =
            Self::entity_bounds_for_stat_interaction(projectile);

        let mut target_ids = self.entity_manager.active_entities();
        target_ids.sort_unstable();
        for target_id in target_ids {
            if target_id == projectile_id || projectile_state.owner_id == Some(target_id) {
                continue;
            }

            let Some(target) = self.entity_manager.get_entity(target_id) else {
                continue;
            };
            if !target.attributes.active
                || target.attributes.current_stat(HEALTH_STAT_ID).is_none()
                || target.attributes.projectile.is_some()
            {
                continue;
            }

            let (target_pos, target_size) = Self::entity_bounds_for_stat_interaction(target);
            if collision::aabb_overlap(projectile_pos, projectile_size, target_pos, target_size) {
                return Some(target_id);
            }
        }

        None
    }

    pub(super) fn update_projectiles(
        &mut self,
        tilemap: &crate::assets::tilemap::TileMap,
        atlas: &crate::assets::atlas::AtlasMeta,
    ) {
        let projectile_ids = self
            .entity_manager
            .active_entities()
            .into_iter()
            .filter(|&entity_id| {
                self.entity_manager
                    .get_entity(entity_id)
                    .and_then(|entity| entity.attributes.projectile.as_ref())
                    .is_some()
            })
            .collect::<Vec<_>>();

        let mut despawn_ids = Vec::new();

        for projectile_id in projectile_ids {
            let Some((current_position, velocity, remaining_ticks, damage, owner_id)) = self
                .entity_manager
                .get_entity(projectile_id)
                .and_then(|entity| {
                    entity.attributes.projectile.as_ref().map(|projectile| {
                        (
                            entity.position,
                            glam::IVec2::new(projectile.velocity[0], projectile.velocity[1]),
                            projectile.remaining_ticks,
                            projectile.damage.max(0),
                            projectile.owner_id,
                        )
                    })
                })
            else {
                continue;
            };

            if remaining_ticks == 0 {
                tracing::debug!(
                    "Projectile {} expired before movement at {:?}",
                    projectile_id,
                    current_position
                );
                despawn_ids.push(projectile_id);
                continue;
            }

            let new_position = current_position + velocity;
            if !self.can_entity_move_to_position(projectile_id, new_position, tilemap, atlas) {
                tracing::debug!(
                    "Projectile {} blocked moving from {:?} to {:?} and will despawn",
                    projectile_id,
                    current_position,
                    new_position
                );
                despawn_ids.push(projectile_id);
                continue;
            }

            if let Some(projectile_entity) = self.entity_manager.get_entity_mut(projectile_id) {
                projectile_entity.position = new_position;
                if let Some(projectile) = projectile_entity.attributes.projectile.as_mut() {
                    projectile.remaining_ticks = projectile.remaining_ticks.saturating_sub(1);
                    tracing::debug!(
                        "Projectile {} moved from {:?} to {:?} remaining_ticks={}",
                        projectile_id,
                        current_position,
                        new_position,
                        projectile.remaining_ticks
                    );
                }
            }

            if let Some(target_id) = self.projectile_hit_target(projectile_id) {
                tracing::debug!(
                    "Projectile {} hit entity {} for {} {} damage",
                    projectile_id,
                    target_id,
                    damage,
                    HEALTH_STAT_ID
                );
                self.pending_stat_changes.push(StatChangeRequest {
                    target_entity_id: target_id,
                    stat_id: HEALTH_STAT_ID.to_string(),
                    delta: -damage,
                    source_entity_id: owner_id,
                });
                despawn_ids.push(projectile_id);
                continue;
            }

            let expired = self
                .entity_manager
                .get_entity(projectile_id)
                .and_then(|entity| entity.attributes.projectile.as_ref())
                .is_some_and(|projectile| projectile.remaining_ticks == 0);
            if expired {
                tracing::debug!(
                    "Projectile {} reached zero lifetime at {:?} and will despawn",
                    projectile_id,
                    new_position
                );
                despawn_ids.push(projectile_id);
            }
        }

        despawn_ids.sort_unstable();
        despawn_ids.dedup();
        for entity_id in despawn_ids {
            self.entity_manager.despawn_entity(entity_id);
        }
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
                // Record damage event with victim/attacker context
                self.rule_runtime.frame_damage_events.push(DamageEvent {
                    victim: change.target_entity_id,
                    attacker: change.source_entity_id,
                });
            }

            if change.stat_id == HEALTH_STAT_ID && new_value <= 0 {
                // Record death event with victim/attacker context
                self.rule_runtime.frame_death_events.push(DeathEvent {
                    victim: change.target_entity_id,
                    attacker: change.source_entity_id,
                });
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

        self.spawn_primary_projectile(entity_id, facing);
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
