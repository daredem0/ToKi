use crate::animation::AnimationState;
use crate::collision;
use crate::collision::CollisionBox;
use crate::entity::{
    Entity, EntityId, EntityRendering, MovementComponent, OptionalEntityComponents,
    ATTACK_POWER_STAT_ID, HEALTH_STAT_ID,
};

use super::animation::FacingDirection;
use super::stat_effects::{StatChangeRequest, StatEffectService};
use super::{GameState, InputAction, RuntimeState, WorldContext, WorldState};

pub struct CombatSystem;

pub struct CombatService<'a> {
    world: &'a mut WorldState,
    runtime: &'a mut RuntimeState,
}

impl CombatSystem {
    pub fn process_profile_actions(state: &mut GameState) {
        state.combat_service().process_profile_actions();
    }

    pub(crate) fn update_projectiles(state: &mut GameState, world: WorldContext<'_>) {
        state
            .combat_service()
            .update_projectiles(world.tilemap, world.atlas);
    }
}

impl<'a> CombatService<'a> {
    pub(crate) fn new(world: &'a mut WorldState, runtime: &'a mut RuntimeState) -> Self {
        Self { world, runtime }
    }

    fn stat_effect_service(&mut self) -> StatEffectService<'_> {
        StatEffectService::new(
            &mut self.world.entity_manager,
            &mut self.runtime.rules,
            &mut self.runtime.effects.pending_stat_changes,
            &mut self.runtime.effects.pending_despawns,
        )
    }

    fn can_entity_move_to_position(
        &self,
        entity_id: EntityId,
        new_position: glam::IVec2,
        tilemap: &crate::assets::tilemap::TileMap,
        atlas: &crate::assets::atlas::AtlasMeta,
    ) -> bool {
        let Some(entity) = self.world.entity_manager.get_entity(entity_id) else {
            return false;
        };

        collision::can_entity_move_to_position(entity, new_position, tilemap, atlas)
            && !self
                .world
                .entity_manager
                .would_collide_with_solid_entity(entity_id, new_position)
    }

    fn facing_vector(facing: FacingDirection) -> glam::IVec2 {
        facing.to_ivec2()
    }

    fn primary_action_damage_for_entity(&self, entity: &Entity) -> i32 {
        self.world
            .entity_manager
            .combat(entity.id)
            .and_then(|combat| combat.current_stat(ATTACK_POWER_STAT_ID))
            .or_else(|| {
                self.world
                    .entity_manager
                    .combat(entity.id)
                    .and_then(|combat| combat.base_stat(ATTACK_POWER_STAT_ID))
            })
            .unwrap_or(10)
    }

    fn primary_action_hitbox(
        entity: &Entity,
        facing: FacingDirection,
    ) -> (glam::IVec2, glam::UVec2) {
        let (origin, size) = entity.interaction_bounds();
        facing.offset_bounds(origin, size, size.y.max(size.x) as i32)
    }

    fn collect_primary_action_stat_changes(
        &self,
        attacker_id: EntityId,
        facing: FacingDirection,
    ) -> Vec<StatChangeRequest> {
        let Some(attacker) = self.world.entity_manager.get_entity(attacker_id) else {
            return Vec::new();
        };

        let damage = self.primary_action_damage_for_entity(attacker);
        if damage <= 0 {
            return Vec::new();
        }

        let (hitbox_pos, hitbox_size) = Self::primary_action_hitbox(attacker, facing);
        let mut target_ids = self.world.entity_manager.active_entities();
        target_ids.sort_unstable();

        let changes = target_ids
            .into_iter()
            .filter(|&target_id| target_id != attacker_id)
            .filter_map(|target_id| {
                let target = self.world.entity_manager.get_entity(target_id)?;
                if !target.active
                    || self
                        .world
                        .entity_manager
                        .combat(target_id)
                        .and_then(|combat| combat.current_stat(HEALTH_STAT_ID))
                        .is_none()
                {
                    return None;
                }
                let (target_pos, target_size) = target.interaction_bounds();
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
        let Some(attacker) = self.world.entity_manager.get_entity(attacker_id) else {
            return;
        };
        let Some(spec) = self
            .world
            .entity_manager
            .storage()
            .components()
            .primary_projectile(attacker_id)
            .cloned()
        else {
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

        let projectile_id = self.world.entity_manager.spawn_entity(
            crate::entity::EntityKind::Projectile,
            spawn_position,
            size,
            EntityRendering {
                visible: true,
                ..EntityRendering::default()
            },
            false,
            true,
            OptionalEntityComponents {
                movement: Some(MovementComponent {
                    speed: 0.0,
                    can_move: false,
                    ..MovementComponent::default()
                }),
                ..OptionalEntityComponents::default()
            },
        );
        self.world
            .entity_manager
            .storage_mut()
            .components_mut()
            .set_projectile(
                projectile_id,
                Some(crate::entity::ProjectileState {
                    sheet: spec.sheet,
                    object_name: spec.object_name,
                    size: spec.size,
                    velocity: [velocity.x, velocity.y],
                    remaining_ticks: spec.lifetime_ticks,
                    damage: spec.damage.max(0),
                    owner_id: Some(attacker_id),
                }),
            );
        if let Some(projectile) = self.world.entity_manager.get_entity_mut(projectile_id) {
            projectile.category = "projectile".to_string();
            projectile.collision_box = Some(CollisionBox::solid_box(size));
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
        let projectile = self.world.entity_manager.get_entity(projectile_id)?;
        let projectile_state = self
            .world
            .entity_manager
            .storage()
            .components()
            .projectile(projectile_id)?;
        let (projectile_pos, projectile_size) = projectile.interaction_bounds();

        let mut target_ids = self.world.entity_manager.active_entities();
        target_ids.sort_unstable();
        for target_id in target_ids {
            if target_id == projectile_id || projectile_state.owner_id == Some(target_id) {
                continue;
            }

            let Some(target) = self.world.entity_manager.get_entity(target_id) else {
                continue;
            };
            if !target.active
                || self
                    .world
                    .entity_manager
                    .combat(target_id)
                    .and_then(|combat| combat.current_stat(HEALTH_STAT_ID))
                    .is_none()
                || self
                    .world
                    .entity_manager
                    .storage()
                    .components()
                    .projectile(target_id)
                    .is_some()
            {
                continue;
            }

            let (target_pos, target_size) = target.interaction_bounds();
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
            .world
            .entity_manager
            .storage()
            .components()
            .projectile_ids()
            .filter(|&entity_id| {
                self.world
                    .entity_manager
                    .get_entity(entity_id)
                    .is_some_and(|entity| entity.active)
            })
            .collect::<Vec<_>>();

        let mut despawn_ids = Vec::new();

        for projectile_id in projectile_ids {
            let Some((current_position, velocity, remaining_ticks, damage, owner_id)) = self
                .world
                .entity_manager
                .get_entity(projectile_id)
                .and_then(|entity| {
                    self.world
                        .entity_manager
                        .storage()
                        .components()
                        .projectile(entity.id)
                        .map(|projectile| {
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

            if let Some(projectile_entity) = self.world.entity_manager.get_entity_mut(projectile_id)
            {
                projectile_entity.position = new_position;
                if let Some(projectile) = self
                    .world
                    .entity_manager
                    .storage_mut()
                    .components_mut()
                    .projectile_mut(projectile_id)
                {
                    projectile.remaining_ticks = projectile.remaining_ticks.saturating_sub(1);
                    tracing::trace!(
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
                self.stat_effect_service()
                    .queue_damage(target_id, damage, owner_id);
                despawn_ids.push(projectile_id);
                continue;
            }

            let expired = self
                .world
                .entity_manager
                .get_entity(projectile_id)
                .and_then(|entity| {
                    self.world
                        .entity_manager
                        .storage()
                        .components()
                        .projectile(entity.id)
                })
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
            self.world.entity_manager.despawn_entity(entity_id);
        }
    }

    fn trigger_entity_primary_action(&mut self, entity_id: EntityId) -> bool {
        let triggered_facing = {
            let Some(animation_controller) = self
                .world
                .entity_manager
                .get_entity_mut(entity_id)
                .and_then(|entity| entity.rendering.animation_controller.as_mut())
            else {
                return false;
            };

            let facing = GameState::facing_from_animation_state(animation_controller.current_clip_state);
            let directional_attack = GameState::directional_attack_state(facing);
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
        self.runtime
            .effects
            .pending_stat_changes
            .extend(self.collect_primary_action_stat_changes(entity_id, facing));
        true
    }

    pub(super) fn process_profile_actions(&mut self) {
        let pending_actions = self.runtime.input.take_pending_profile_actions();
        if pending_actions.is_empty() {
            return;
        }

        let mut controlled_entity_ids = self
            .world
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                let entity = self.world.entity_manager.get_entity(entity_id)?;
                if matches!(
                    entity.effective_movement_profile(self.world.entity_manager.movement(entity_id)),
                    crate::entity::MovementProfile::PlayerWasd
                ) {
                    Some(entity_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        controlled_entity_ids.sort_unstable();
        if controlled_entity_ids.is_empty() {
            return;
        }

        for (profile, actions) in pending_actions {
            if !actions.contains(&InputAction::Primary) {
                continue;
            }
            for &entity_id in &controlled_entity_ids {
                let Some(entity) = self.world.entity_manager.get_entity(entity_id) else {
                    continue;
                };
                if entity.effective_movement_profile(self.world.entity_manager.movement(entity_id))
                    != profile
                {
                    continue;
                }
                self.trigger_entity_primary_action(entity_id);
            }
        }
    }

    /// Apply damage to an entity directly (for testing).
    /// This queues a stat change request that will be applied during the next update.
    pub fn deal_damage_to_entity(
        &mut self,
        target_id: EntityId,
        damage: i32,
        attacker_id: Option<EntityId>,
    ) {
        self.stat_effect_service()
            .queue_damage(target_id, damage, attacker_id);
    }
}

impl GameState {
    pub fn combat_service(&mut self) -> CombatService<'_> {
        CombatService::new(&mut self.world, &mut self.runtime)
    }

    pub fn deal_damage_to_entity(
        &mut self,
        target_id: EntityId,
        damage: i32,
        attacker_id: Option<EntityId>,
    ) {
        self.combat_service()
            .deal_damage_to_entity(target_id, damage, attacker_id);
    }
}
