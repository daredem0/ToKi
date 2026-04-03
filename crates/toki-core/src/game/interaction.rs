use super::{GameState, RuntimeState, WorldState};
use crate::collision;
use crate::game::rules::{InteractionEvent, InteractionSpatial};

pub struct InteractionSystem;

pub struct InteractionService<'a> {
    world: &'a mut WorldState,
    runtime: &'a mut RuntimeState,
}

impl InteractionSystem {
    pub fn collect_overlapping_pickups(state: &mut GameState) {
        state.interaction_service().collect_overlapping_pickups();
    }

    pub fn collect_interaction_events(state: &mut GameState) {
        state.interaction_service().collect_interaction_events();
    }
}

impl<'a> InteractionService<'a> {
    pub(crate) fn new(world: &'a mut WorldState, runtime: &'a mut RuntimeState) -> Self {
        Self { world, runtime }
    }

    pub(super) fn collect_overlapping_pickups(&mut self) {
        let mut collector_ids = self
            .world
            .entity_manager
            .active_entities()
            .into_iter()
            .filter(|&entity_id| {
                self.world
                    .entity_manager
                    .storage()
                    .components()
                    .inventory(entity_id)
                    .is_some()
            })
            .collect::<Vec<_>>();
        collector_ids.sort_unstable();

        let mut pickup_ids = self
            .world
            .entity_manager
            .storage()
            .components()
            .pickup_ids()
            .filter(|&entity_id| {
                self.world
                    .entity_manager
                    .get_entity(entity_id)
                    .is_some_and(|entity| entity.active)
            })
            .collect::<Vec<_>>();
        pickup_ids.sort_unstable();

        let mut collected = Vec::new();
        for collector_id in collector_ids {
            let Some(collector) = self.world.entity_manager.get_entity(collector_id) else {
                continue;
            };
            let (collector_pos, collector_size) = collector.interaction_bounds();

            for &pickup_id in &pickup_ids {
                let Some(pickup_entity) = self.world.entity_manager.get_entity(pickup_id) else {
                    continue;
                };
                let Some(pickup) = self
                    .world
                    .entity_manager
                    .storage()
                    .components()
                    .pickup(pickup_id)
                else {
                    continue;
                };
                if pickup.count == 0 || pickup.item_id.is_empty() {
                    continue;
                }

                let (pickup_pos, pickup_size) = pickup_entity.interaction_bounds();
                if !collision::aabb_overlap(collector_pos, collector_size, pickup_pos, pickup_size)
                {
                    continue;
                }

                collected.push((
                    collector_id,
                    pickup_id,
                    pickup.item_id.clone(),
                    pickup.count,
                ));
            }
        }

        collected.sort_unstable_by_key(|(_, pickup_id, _, _)| *pickup_id);
        collected.dedup_by_key(|(_, pickup_id, _, _)| *pickup_id);

        for (collector_id, pickup_id, item_id, count) in collected {
            if self
                .world
                .entity_manager
                .storage()
                .components()
                .inventory(collector_id)
                .is_none()
            {
                continue;
            }

            self.world
                .entity_manager
                .storage_mut()
                .components_mut()
                .ensure_inventory(collector_id)
                .add_item(&item_id, count);
            tracing::debug!(
                "Entity {} collected pickup {} item_id={} count={} new_count={}",
                collector_id,
                pickup_id,
                item_id,
                count,
                self.world
                    .entity_manager
                    .storage()
                    .components()
                    .inventory(collector_id)
                    .map(|inventory| inventory.item_count(&item_id))
                    .unwrap_or(0)
            );

            self.world.entity_manager.despawn_entity(pickup_id);
        }
    }

    /// Collects interaction events when the player presses interact while overlapping
    /// or adjacent to interactable entities.
    pub(super) fn collect_interaction_events(&mut self) {
        // Check if interact key is held
        let held_keys = self.runtime.input.all_held_keys();
        if !held_keys.contains(&super::InputKey::Interact) {
            return;
        }

        // Get the player
        let Some(player_id) = self.world.player_id() else {
            return;
        };
        let Some(player) = self.world.entity_manager.get_entity(player_id) else {
            return;
        };

        let player_pos = player.position;
        let player_size = player.size;
        let player_facing = player
            .rendering
            .animation_controller
            .as_ref()
            .map(|ac| GameState::facing_from_animation_state(ac.current_clip_state))
            .unwrap_or(super::animation::FacingDirection::Down);

        // Find all interactable entities
        let mut interactable_ids = self
            .world
            .entity_manager
            .storage()
            .components()
            .interaction_ids()
            .filter(|&entity_id| {
                if entity_id == player_id {
                    return false;
                }
                self.world.entity_manager.get_entity(entity_id).is_some()
            })
            .collect::<Vec<_>>();
        interactable_ids.sort_unstable();

        // Check for overlaps and record interaction events with spatial relationship
        for interactable_id in interactable_ids {
            let Some(interactable) = self.world.entity_manager.get_entity(interactable_id) else {
                continue;
            };
            let Some(interaction) = self.world.entity_manager.interaction(interactable_id) else {
                continue;
            };

            let interactable_pos = interactable.position;
            let interactable_size = interactable.size;
            let interaction_reach = interaction.interaction_reach as i32;

            // Determine spatial relationship
            let spatial = Self::determine_interaction_spatial(
                player_pos,
                player_size,
                player_facing,
                interactable_pos,
                interactable_size,
                interaction_reach,
            );

            if let Some(spatial) = spatial {
                self.runtime.rules.frame_interactions.push(InteractionEvent {
                    interactor: player_id,
                    interactable: interactable_id,
                    spatial,
                });

                tracing::debug!(
                    "Player {} interacting with entity {} (spatial: {:?})",
                    player_id,
                    interactable_id,
                    spatial
                );
            }
        }
    }

    /// Determines the spatial relationship between player and interactable.
    /// Returns None if the player is too far to interact.
    fn determine_interaction_spatial(
        player_pos: glam::IVec2,
        player_size: glam::UVec2,
        player_facing: super::animation::FacingDirection,
        interactable_pos: glam::IVec2,
        interactable_size: glam::UVec2,
        interaction_reach: i32,
    ) -> Option<InteractionSpatial> {
        // Check strict overlap first
        let overlaps =
            collision::aabb_overlap(player_pos, player_size, interactable_pos, interactable_size);

        if overlaps {
            return Some(InteractionSpatial::Overlap);
        }

        // Check if player is facing the interactable and within reach
        let is_in_front = Self::is_facing_entity(
            player_pos,
            player_size,
            player_facing,
            interactable_pos,
            interactable_size,
            interaction_reach,
        );

        if is_in_front {
            return Some(InteractionSpatial::InFront);
        }

        // Check if adjacent (within reach in any direction)
        let reach = interaction_reach.max(1); // At least 1 pixel reach for adjacent
        let expanded_pos = glam::IVec2::new(player_pos.x - reach, player_pos.y - reach);
        let expanded_size = glam::UVec2::new(
            player_size.x + (reach * 2) as u32,
            player_size.y + (reach * 2) as u32,
        );

        let adjacent = collision::aabb_overlap(
            expanded_pos,
            expanded_size,
            interactable_pos,
            interactable_size,
        );

        if adjacent {
            return Some(InteractionSpatial::Adjacent);
        }

        None
    }

    /// Checks if the player is facing an entity and within reach in that direction.
    fn is_facing_entity(
        player_pos: glam::IVec2,
        player_size: glam::UVec2,
        player_facing: super::animation::FacingDirection,
        interactable_pos: glam::IVec2,
        interactable_size: glam::UVec2,
        interaction_reach: i32,
    ) -> bool {
        let (reach_pos, reach_size) =
            player_facing.reach_bounds(player_pos, player_size, interaction_reach);

        collision::aabb_overlap(reach_pos, reach_size, interactable_pos, interactable_size)
    }
}

impl GameState {
    pub fn interaction_service(&mut self) -> InteractionService<'_> {
        InteractionService::new(&mut self.world, &mut self.runtime)
    }
}
