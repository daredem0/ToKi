use super::GameState;
use crate::collision;
use crate::game::rules::{InteractionEvent, InteractionSpatial};

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
        let Some(player_id) = self.player_id else {
            return;
        };
        let Some(player) = self.entity_manager.get_entity(player_id) else {
            return;
        };

        let player_pos = player.position;
        let player_size = player.size;
        let player_facing = player
            .attributes
            .animation_controller
            .as_ref()
            .map(|ac| Self::facing_from_animation_state(ac.current_clip_state))
            .unwrap_or(super::animation::FacingDirection::Down);

        // Find all interactable entities
        let mut interactable_ids = self
            .entity_manager
            .active_entities()
            .into_iter()
            .filter(|&entity_id| {
                if entity_id == player_id {
                    return false;
                }
                self.entity_manager
                    .get_entity(entity_id)
                    .is_some_and(|entity| entity.attributes.interactable && entity.attributes.active)
            })
            .collect::<Vec<_>>();
        interactable_ids.sort_unstable();

        // Check for overlaps and record interaction events with spatial relationship
        for interactable_id in interactable_ids {
            let Some(interactable) = self.entity_manager.get_entity(interactable_id) else {
                continue;
            };

            let interactable_pos = interactable.position;
            let interactable_size = interactable.size;
            let interaction_reach = interactable.attributes.interaction_reach as i32;

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
                self.rule_runtime
                    .frame_interactions
                    .push(InteractionEvent {
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
        let overlaps = collision::aabb_overlap(
            player_pos,
            player_size,
            interactable_pos,
            interactable_size,
        );

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
        let expanded_pos = glam::IVec2::new(
            player_pos.x - reach,
            player_pos.y - reach,
        );
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
        use super::animation::FacingDirection;

        let reach = interaction_reach.max(1);

        // Create a reach box in the direction the player is facing
        let (reach_pos, reach_size) = match player_facing {
            FacingDirection::Up => {
                let pos = glam::IVec2::new(player_pos.x, player_pos.y - reach);
                let size = glam::UVec2::new(player_size.x, reach as u32);
                (pos, size)
            }
            FacingDirection::Down => {
                let pos = glam::IVec2::new(player_pos.x, player_pos.y + player_size.y as i32);
                let size = glam::UVec2::new(player_size.x, reach as u32);
                (pos, size)
            }
            FacingDirection::Left => {
                let pos = glam::IVec2::new(player_pos.x - reach, player_pos.y);
                let size = glam::UVec2::new(reach as u32, player_size.y);
                (pos, size)
            }
            FacingDirection::Right => {
                let pos = glam::IVec2::new(player_pos.x + player_size.x as i32, player_pos.y);
                let size = glam::UVec2::new(reach as u32, player_size.y);
                (pos, size)
            }
        };

        collision::aabb_overlap(reach_pos, reach_size, interactable_pos, interactable_size)
    }
}
