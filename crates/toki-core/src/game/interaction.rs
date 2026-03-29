use super::GameState;
use crate::collision;
use crate::game::rules::{InteractionEvent, InteractionSpatial};

pub struct InteractionSystem;

impl InteractionSystem {
    pub fn collect_overlapping_pickups(state: &mut GameState) {
        state.collect_overlapping_pickups();
    }

    pub fn collect_interaction_events(state: &mut GameState) {
        state.collect_interaction_events();
    }
}

impl GameState {
    /// Collects interaction events when the player presses interact while overlapping
    /// or adjacent to interactable entities.
    pub(super) fn collect_interaction_events(&mut self) {
        // Check if interact key is held
        let held_keys = self.all_held_keys();
        if !held_keys.contains(&super::InputKey::Interact) {
            return;
        }

        // Get the player
        let Some(player_id) = self.world.player_id else {
            return;
        };
        let Some(player) = self.world.entity_manager.get_entity(player_id) else {
            return;
        };

        let player_pos = player.position;
        let player_size = player.size;
        let player_facing = player
            .attributes
            .rendering
            .animation_controller
            .as_ref()
            .map(|ac| Self::facing_from_animation_state(ac.current_clip_state))
            .unwrap_or(super::animation::FacingDirection::Down);

        // Find all interactable entities
        let mut interactable_ids = self
            .world
            .entity_manager
            .active_entities()
            .into_iter()
            .filter(|&entity_id| {
                if entity_id == player_id {
                    return false;
                }
                self.world.entity_manager
                    .get_entity(entity_id)
                    .is_some_and(|entity| {
                        entity.attributes.behavior.interactable
                            && entity.attributes.behavior.active
                    })
            })
            .collect::<Vec<_>>();
        interactable_ids.sort_unstable();

        // Check for overlaps and record interaction events with spatial relationship
        for interactable_id in interactable_ids {
            let Some(interactable) = self.world.entity_manager.get_entity(interactable_id) else {
                continue;
            };

            let interactable_pos = interactable.position;
            let interactable_size = interactable.size;
            let interaction_reach = interactable.attributes.behavior.interaction_reach as i32;

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
