//! Tile transition detection and handling.
//!
//! Contains logic for detecting when entities move between tiles
//! and collecting rule commands for tile transition triggers.

use tracing::{debug, warn};

use crate::assets::tilemap::TileMap;
use crate::entity::EntityId;
use crate::rules::{RuleTrigger, TriggerContext};

use super::events::TileTransitionEvent;
use super::{GameState, RuleCommand};

/// Entity tile position data collected for transition processing.
struct EntityTileData {
    entity_id: EntityId,
    current_tile_x: u32,
    current_tile_y: u32,
    pixel_pos: glam::IVec2,
}

impl GameState {
    /// Detects tile transitions for all active entities and populates frame_tile_transitions.
    ///
    /// This should be called after all movement is complete but before reactive rules fire.
    /// It compares each entity's current tile position with their previous tile position
    /// and generates OnTileExit and OnTileEnter events for transitions.
    pub(in crate::game) fn detect_tile_transitions(&mut self, tilemap: &TileMap) {
        if tilemap.tile_size.x == 0 || tilemap.tile_size.y == 0 {
            return;
        }

        let tile_w = tilemap.tile_size.x;
        let tile_h = tilemap.tile_size.y;

        // Collect entity tile data first to avoid borrow conflicts
        let entity_data: Vec<EntityTileData> = self
            .entity_manager
            .active_entities_iter()
            .filter_map(|entity_id| {
                let entity = self.entity_manager.get_entity(entity_id)?;

                let center_x = entity.position.x + (entity.size.x as i32 / 2);
                let center_y = entity.position.y + (entity.size.y as i32 / 2);
                let current_tile_x = (center_x.max(0) as u32) / tile_w;
                let current_tile_y = (center_y.max(0) as u32) / tile_h;

                // Clamp to map bounds
                let current_tile_x = current_tile_x.min(tilemap.size.x.saturating_sub(1));
                let current_tile_y = current_tile_y.min(tilemap.size.y.saturating_sub(1));

                Some(EntityTileData {
                    entity_id,
                    current_tile_x,
                    current_tile_y,
                    pixel_pos: entity.position,
                })
            })
            .collect();

        // Now process the collected data
        for data in entity_data {
            self.process_entity_tile_transition(
                data.entity_id,
                data.current_tile_x,
                data.current_tile_y,
                data.pixel_pos,
            );
        }
    }

    fn process_entity_tile_transition(
        &mut self,
        entity_id: EntityId,
        current_tile_x: u32,
        current_tile_y: u32,
        pixel_pos: glam::IVec2,
    ) {
        // Check if we have a previous tile position for this entity
        if let Some(&(prev_tile_x, prev_tile_y)) =
            self.rule_runtime.entity_tile_positions.get(&entity_id)
        {
            // If tile position changed, generate exit and enter events
            if (prev_tile_x, prev_tile_y) != (current_tile_x, current_tile_y) {
                self.log_tile_transition(
                    entity_id,
                    prev_tile_x,
                    prev_tile_y,
                    current_tile_x,
                    current_tile_y,
                    pixel_pos,
                );

                // Exit previous tile
                self.rule_runtime
                    .frame_tile_transitions
                    .push(TileTransitionEvent {
                        entity_id,
                        tile_x: prev_tile_x,
                        tile_y: prev_tile_y,
                        is_enter: false,
                    });

                // Enter new tile
                self.rule_runtime
                    .frame_tile_transitions
                    .push(TileTransitionEvent {
                        entity_id,
                        tile_x: current_tile_x,
                        tile_y: current_tile_y,
                        is_enter: true,
                    });
            }
        }

        // Update stored tile position
        self.rule_runtime
            .entity_tile_positions
            .insert(entity_id, (current_tile_x, current_tile_y));
    }

    fn log_tile_transition(
        &self,
        entity_id: EntityId,
        prev_tile_x: u32,
        prev_tile_y: u32,
        current_tile_x: u32,
        current_tile_y: u32,
        pixel_pos: glam::IVec2,
    ) {
        if Some(entity_id) == self.player_id {
            tracing::trace!(
                "Player moved from_tile=({},{}) to_tile=({},{}) pixel_pos=({},{})",
                prev_tile_x,
                prev_tile_y,
                current_tile_x,
                current_tile_y,
                pixel_pos.x,
                pixel_pos.y
            );
        } else {
            tracing::trace!(
                entity = ?entity_id,
                from_tile = ?(prev_tile_x, prev_tile_y),
                to_tile = ?(current_tile_x, current_tile_y),
                pixel_pos = ?(pixel_pos.x, pixel_pos.y),
                "Tile transition detected"
            );
        }
    }

    /// Collects rule commands for tile transition events (OnTileEnter/OnTileExit).
    ///
    /// Validates tile coordinates against the active tilemap bounds.
    /// Rules with out-of-bounds coordinates are skipped with a warning.
    pub(in crate::game) fn collect_rule_commands_for_tile_transitions(
        &mut self,
        tilemap: &TileMap,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let tile_events = std::mem::take(&mut self.rule_runtime.frame_tile_transitions);
        let map_width = tilemap.size.x;
        let map_height = tilemap.size.y;

        for event in tile_events {
            let trigger = if event.is_enter {
                RuleTrigger::OnTileEnter {
                    x: event.tile_x,
                    y: event.tile_y,
                }
            } else {
                RuleTrigger::OnTileExit {
                    x: event.tile_x,
                    y: event.tile_y,
                }
            };

            let context = TriggerContext::with_self_only(event.entity_id);

            // Collect matching rule indices to avoid borrow conflicts
            let matching_indices: Vec<usize> = self
                .rules
                .rules
                .iter()
                .enumerate()
                .filter(|(_, rule)| rule.enabled && rule.trigger == trigger)
                .filter(|(_, rule)| {
                    !(rule.once
                        && self
                            .rule_runtime
                            .fired_once_rules
                            .contains(rule.id.as_str()))
                })
                .map(|(i, _)| i)
                .collect();

            let mut sorted_indices = matching_indices;
            sorted_indices.sort_by(|&a, &b| {
                let rule_a = &self.rules.rules[a];
                let rule_b = &self.rules.rules[b];
                rule_b
                    .priority
                    .cmp(&rule_a.priority)
                    .then_with(|| rule_a.id.cmp(&rule_b.id))
            });

            let mut fired_once_ids = Vec::new();
            for idx in sorted_indices {
                let rule = &self.rules.rules[idx];

                // Validate tile coordinates are within map bounds
                if let Some((tile_x, tile_y)) = rule.trigger.tile_coordinates() {
                    if tile_x >= map_width || tile_y >= map_height {
                        warn!(
                            rule_id = %rule.id,
                            tile_x = tile_x,
                            tile_y = tile_y,
                            map_width = map_width,
                            map_height = map_height,
                            "Skipping tile trigger rule with out-of-bounds coordinates"
                        );
                        continue;
                    }
                }

                let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
                debug!(
                    rule_id = %rule.id,
                    trigger = ?trigger,
                    entity = ?event.entity_id,
                    tile_x = event.tile_x,
                    tile_y = event.tile_y,
                    is_enter = event.is_enter,
                    conditions_passed = conditions_result,
                    "Tile transition rule evaluated"
                );

                if !conditions_result {
                    continue;
                }

                let actions = rule.actions.clone();
                let rule_id = rule.id.clone();
                let rule_once = rule.once;

                for action in &actions {
                    debug!(rule_id = %rule_id, action = ?action, "Executing tile transition action");
                    self.buffer_rule_action(action, &context, command_buffer);
                }

                if rule_once {
                    fired_once_ids.push(rule_id);
                }
            }
            self.rule_runtime.fired_once_rules.extend(fired_once_ids);
        }
    }
}
