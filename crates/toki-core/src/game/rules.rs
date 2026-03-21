use std::collections::{HashMap, HashSet};

use tracing::{debug, warn};

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{Entity, EntityAttributes, EntityId, EntityKind, HEALTH_STAT_ID};
use crate::events::GameUpdateResult;
use crate::rules::{
    Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleSpawnEntityType, RuleTarget,
    RuleTrigger, TriggerContext,
};

use super::{combat::StatChangeRequest, AudioChannel, AudioEvent, GameState};

/// A collision event between an entity and another entity or the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionEvent {
    /// The entity that was moving/checking collision.
    pub entity_a: EntityId,
    /// The entity that was collided with, if entity-entity collision.
    /// `None` for tile/world collisions.
    pub entity_b: Option<EntityId>,
}

/// A damage event recording who was damaged and by whom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageEvent {
    /// The entity that received damage.
    pub victim: EntityId,
    /// The entity that caused the damage, if known.
    pub attacker: Option<EntityId>,
}

/// A death event recording who died and who caused it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeathEvent {
    /// The entity that died.
    pub victim: EntityId,
    /// The entity that caused the death, if known.
    pub attacker: Option<EntityId>,
}

/// The spatial relationship between player and interactable when interaction occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionSpatial {
    /// Player was overlapping the interactable (strict AABB intersection).
    Overlap,
    /// Player was adjacent to the interactable (within reach but not overlapping).
    Adjacent,
    /// Player was facing the interactable and within reach.
    InFront,
}

/// An interaction event recording when the player interacts with an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionEvent {
    /// The entity that initiated the interaction (player).
    pub interactor: EntityId,
    /// The entity being interacted with.
    pub interactable: EntityId,
    /// The spatial relationship when interaction occurred.
    pub spatial: InteractionSpatial,
}

/// A tile transition event recording when an entity enters or exits a specific tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileTransitionEvent {
    /// The entity that entered or exited the tile.
    pub entity_id: EntityId,
    /// The tile x-coordinate.
    pub tile_x: u32,
    /// The tile y-coordinate.
    pub tile_y: u32,
    /// Whether this is an enter (true) or exit (false) event.
    pub is_enter: bool,
}

#[derive(Debug, Default)]
pub(super) struct RuleRuntimeState {
    pub(super) started: bool,
    pub(super) fired_once_rules: HashSet<String>,
    pub(super) velocities: HashMap<EntityId, glam::IVec2>,
    /// Collision events that occurred this frame.
    pub(super) frame_collisions: Vec<CollisionEvent>,
    /// Damage events that occurred this frame.
    pub(super) frame_damage_events: Vec<DamageEvent>,
    /// Death events that occurred this frame.
    pub(super) frame_death_events: Vec<DeathEvent>,
    /// Interaction events that occurred this frame.
    pub(super) frame_interactions: Vec<InteractionEvent>,
    /// Previous tile positions for entities, used to detect tile transitions.
    /// Key: EntityId, Value: (tile_x, tile_y)
    pub(super) entity_tile_positions: HashMap<EntityId, (u32, u32)>,
    /// Tile transition events that occurred this frame.
    pub(super) frame_tile_transitions: Vec<TileTransitionEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RuleCommand {
    PlaySound {
        channel: AudioChannel,
        sound_id: String,
    },
    PlayMusic {
        track_id: String,
    },
    SetVelocity {
        entity_id: EntityId,
        velocity: glam::IVec2,
    },
    PlayAnimation {
        entity_id: EntityId,
        state: AnimationState,
    },
    Spawn {
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    },
    DestroySelf {
        entity_id: EntityId,
    },
    SwitchScene {
        scene_name: String,
        spawn_point_id: String,
    },
    DamageEntity {
        entity_id: EntityId,
        amount: i32,
    },
    HealEntity {
        entity_id: EntityId,
        amount: i32,
    },
    AddInventoryItem {
        entity_id: EntityId,
        item_id: String,
        count: u32,
    },
    RemoveInventoryItem {
        entity_id: EntityId,
        item_id: String,
        count: u32,
    },
    SetEntityActive {
        entity_id: EntityId,
        active: bool,
    },
    TeleportEntity {
        entity_id: EntityId,
        tile_x: u32,
        tile_y: u32,
    },
}

pub(super) type PendingSceneSwitch = (String, String);

impl GameState {
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    pub fn rules_mut(&mut self) -> &mut RuleSet {
        &mut self.rules
    }

    pub fn set_rules(&mut self, rules: RuleSet) {
        self.rules = rules;
        self.rule_runtime = RuleRuntimeState::default();
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.rules.push(rule);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // UNIFIED RULE EVALUATION HELPERS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Collects matching rules after applying trigger-specific filtering.
    ///
    /// This extracts the common filter-sort pattern shared by collision, damage,
    /// death, and interaction rule evaluation. The caller provides a predicate
    /// that implements trigger-specific matching (e.g., OnCollision with entity filter).
    ///
    /// Returns rules sorted by priority (highest first), then by ID for stability.
    fn collect_filtered_rules<F>(&self, rule_filter: F) -> Vec<Rule>
    where
        F: Fn(&Rule) -> bool,
    {
        let mut matching_rules = self
            .rules
            .rules
            .iter()
            .filter(|rule| rule.enabled)
            .filter(|rule| rule_filter(rule))
            .filter(|rule| {
                !(rule.once
                    && self
                        .rule_runtime
                        .fired_once_rules
                        .contains(rule.id.as_str()))
            })
            .cloned()
            .collect::<Vec<_>>();

        matching_rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
        matching_rules
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RULE COMMAND COLLECTION METHODS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Collects rule commands for a trigger without context.
    /// Use `collect_rule_commands_for_trigger_with_context` for triggers that provide context.
    pub(super) fn collect_rule_commands_for_trigger(
        &mut self,
        trigger: RuleTrigger,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        self.collect_rule_commands_for_trigger_with_context(
            trigger,
            TriggerContext::empty(),
            command_buffer,
        );
    }

    /// Collects rule commands for a trigger with entity context.
    ///
    /// # Architecture Note (for Phase 1.5B+ implementers)
    ///
    /// This is the core rule evaluation entry point when context is available.
    /// The `context` parameter provides `trigger_self` and `trigger_other` entity IDs
    /// that can be referenced via `RuleTarget::TriggerSelf` and `RuleTarget::TriggerOther`.
    ///
    /// When adding new context-providing triggers:
    /// 1. Fire this method with appropriate `TriggerContext`
    /// 2. Ensure conditions/actions using `TriggerSelf`/`TriggerOther` work correctly
    pub(super) fn collect_rule_commands_for_trigger_with_context(
        &mut self,
        trigger: RuleTrigger,
        context: TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let mut matching_rules = self
            .rules
            .rules
            .iter()
            .filter(|rule| rule.enabled && rule.trigger == trigger)
            .filter(|rule| {
                !(rule.once
                    && self
                        .rule_runtime
                        .fired_once_rules
                        .contains(rule.id.as_str()))
            })
            .cloned()
            .collect::<Vec<_>>();

        matching_rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

        for rule in matching_rules {
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            debug!(
                rule_id = %rule.id,
                trigger = ?trigger,
                conditions_passed = conditions_result,
                "Rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            for action in &rule.actions {
                debug!(rule_id = %rule.id, action = ?action, "Executing action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule.once {
                self.rule_runtime.fired_once_rules.insert(rule.id);
            }
        }
    }

    /// Collects rule commands for OnInteract triggers, filtering by interaction mode and entity.
    ///
    /// The `event` contains the spatial relationship between player and interactable.
    /// Only rules whose interaction mode matches the spatial relationship will fire.
    /// If a rule specifies an entity filter, it only fires when that entity is the interactable.
    pub(super) fn collect_rule_commands_for_interaction(
        &mut self,
        event: &InteractionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = TriggerContext::with_pair(event.interactor, event.interactable);

        let matching_rules = self.collect_filtered_rules(|rule| {
            if !matches!(rule.trigger, RuleTrigger::OnInteract { .. }) {
                return false;
            }
            let mode = rule.trigger.interaction_mode().unwrap_or_default();
            Self::interaction_mode_matches(mode, event.spatial)
                && self.entity_filter_matches(
                    rule.trigger.interact_entity_filter(),
                    event.interactable,
                    &context,
                )
        });

        for rule in matching_rules {
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            debug!(
                rule_id = %rule.id,
                trigger = ?rule.trigger,
                interactor = ?event.interactor,
                interactable = ?event.interactable,
                spatial = ?event.spatial,
                conditions_passed = conditions_result,
                "Interaction rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            for action in &rule.actions {
                debug!(rule_id = %rule.id, action = ?action, "Executing action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule.once {
                self.rule_runtime.fired_once_rules.insert(rule.id);
            }
        }
    }

    /// Checks if an interaction mode matches a spatial relationship.
    ///
    /// - `Overlap`: Only matches `Overlap` spatial
    /// - `Adjacent`: Matches `Overlap`, `Adjacent`, or `InFront` (anything within reach)
    /// - `InFront`: Only matches `InFront` or `Overlap` (overlap implies you're "on" the entity)
    fn interaction_mode_matches(
        mode: crate::rules::InteractionMode,
        spatial: InteractionSpatial,
    ) -> bool {
        use crate::rules::InteractionMode;

        match mode {
            InteractionMode::Overlap => matches!(spatial, InteractionSpatial::Overlap),
            InteractionMode::Adjacent => true, // Adjacent mode accepts any proximity
            InteractionMode::InFront => matches!(
                spatial,
                InteractionSpatial::InFront | InteractionSpatial::Overlap
            ),
        }
    }

    /// Collects rule commands for OnCollision triggers, filtering by entity if specified.
    ///
    /// If a rule has `OnCollision { entity: Some(target) }`, it only fires when the
    /// resolved target matches the collision event's entity_a. If `entity: None`, it fires
    /// for all collision events.
    pub(super) fn collect_rule_commands_for_collision(
        &mut self,
        event: &CollisionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = if let Some(entity_b) = event.entity_b {
            TriggerContext::with_pair(event.entity_a, entity_b)
        } else {
            TriggerContext::with_self_only(event.entity_a)
        };

        let matching_rules = self.collect_filtered_rules(|rule| {
            matches!(rule.trigger, RuleTrigger::OnCollision { .. })
                && self.entity_filter_matches(
                    rule.trigger.collision_entity_filter(),
                    event.entity_a,
                    &context,
                )
        });

        for rule in matching_rules {
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);

            // Wall collisions (entity_b=None) are high-frequency, use trace level
            if event.entity_b.is_some() {
                debug!(
                    rule_id = %rule.id,
                    trigger = ?rule.trigger,
                    entity_a = ?event.entity_a,
                    entity_b = ?event.entity_b,
                    conditions_passed = conditions_result,
                    "Collision rule evaluated"
                );
            } else {
                tracing::trace!(
                    rule_id = %rule.id,
                    trigger = ?rule.trigger,
                    entity_a = ?event.entity_a,
                    conditions_passed = conditions_result,
                    "Wall collision rule evaluated"
                );
            }

            if !conditions_result {
                continue;
            }

            for action in &rule.actions {
                if event.entity_b.is_some() {
                    debug!(rule_id = %rule.id, action = ?action, "Executing action");
                } else {
                    tracing::trace!(rule_id = %rule.id, action = ?action, "Executing wall collision action");
                }
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule.once {
                self.rule_runtime.fired_once_rules.insert(rule.id);
            }
        }
    }

    /// Collects rule commands for OnDamaged triggers, filtering by entity if specified.
    ///
    /// If a rule has `OnDamaged { entity: Some(target) }`, it only fires when the
    /// resolved target matches the damage event's victim. If `entity: None`, it fires
    /// for all damage events.
    pub(super) fn collect_rule_commands_for_damage(
        &mut self,
        event: &DamageEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = if let Some(attacker) = event.attacker {
            TriggerContext::with_pair(event.victim, attacker)
        } else {
            TriggerContext::with_self_only(event.victim)
        };

        let matching_rules = self.collect_filtered_rules(|rule| {
            matches!(rule.trigger, RuleTrigger::OnDamaged { .. })
                && self.entity_filter_matches(
                    rule.trigger.damaged_entity_filter(),
                    event.victim,
                    &context,
                )
        });

        for rule in matching_rules {
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            debug!(
                rule_id = %rule.id,
                trigger = ?rule.trigger,
                victim = ?event.victim,
                attacker = ?event.attacker,
                conditions_passed = conditions_result,
                "Damage rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            for action in &rule.actions {
                debug!(rule_id = %rule.id, action = ?action, "Executing action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule.once {
                self.rule_runtime.fired_once_rules.insert(rule.id);
            }
        }
    }

    /// Checks if an entity filter matches a target entity.
    ///
    /// - `None`: No filter, matches any entity
    /// - `Some(target)`: Resolves target and checks if it equals the event entity
    fn entity_filter_matches(
        &self,
        filter: Option<RuleTarget>,
        event_entity: EntityId,
        context: &TriggerContext,
    ) -> bool {
        match filter {
            None => true, // No filter, match all
            Some(target) => self
                .resolve_rule_target(target, context)
                .is_some_and(|id| id == event_entity),
        }
    }

    /// Collects rule commands for OnDeath triggers, filtering by entity if specified.
    ///
    /// If a rule has `OnDeath { entity: Some(target) }`, it only fires when the
    /// resolved target matches the death event's victim. If `entity: None`, it fires
    /// for all death events.
    pub(super) fn collect_rule_commands_for_death(
        &mut self,
        event: &DeathEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = if let Some(attacker) = event.attacker {
            TriggerContext::with_pair(event.victim, attacker)
        } else {
            TriggerContext::with_self_only(event.victim)
        };

        let matching_rules = self.collect_filtered_rules(|rule| {
            matches!(rule.trigger, RuleTrigger::OnDeath { .. })
                && self.entity_filter_matches(
                    rule.trigger.death_entity_filter(),
                    event.victim,
                    &context,
                )
        });

        for rule in matching_rules {
            let conditions_result = self.rule_conditions_match(&rule.conditions, &context);
            tracing::info!(
                rule_id = %rule.id,
                trigger = ?rule.trigger,
                victim = ?event.victim,
                attacker = ?event.attacker,
                conditions_passed = conditions_result,
                "Death rule evaluated"
            );

            if !conditions_result {
                continue;
            }

            for action in &rule.actions {
                tracing::info!(rule_id = %rule.id, action = ?action, "Executing death action");
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule.once {
                self.rule_runtime.fired_once_rules.insert(rule.id);
            }
        }
    }

    pub(super) fn collect_rule_commands_for_key_triggers(
        &mut self,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let held_keys = self.all_held_keys();

        for input_key in held_keys {
            let trigger = RuleTrigger::OnKey {
                key: Self::to_rule_key(input_key),
            };
            self.collect_rule_commands_for_trigger(trigger, command_buffer);
        }
    }

    pub(super) fn any_entity_overlaps_trigger_tile(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> bool {
        for entity_id in self.entity_manager.active_entities() {
            let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                continue;
            };
            if Self::entity_overlaps_trigger_tile(entity, tilemap, atlas) {
                return true;
            }
        }
        false
    }

    fn entity_overlaps_trigger_tile(entity: &Entity, tilemap: &TileMap, atlas: &AtlasMeta) -> bool {
        if tilemap.tile_size.x == 0 || tilemap.tile_size.y == 0 {
            return false;
        }
        if tilemap.size.x == 0 || tilemap.size.y == 0 {
            return false;
        }

        let (box_pos, box_size) = if let Some(collision_box) = &entity.collision_box {
            collision_box.world_bounds(entity.position)
        } else {
            (entity.position, entity.size)
        };
        if box_size.x == 0 || box_size.y == 0 {
            return false;
        }

        let tile_w = tilemap.tile_size.x as i32;
        let tile_h = tilemap.tile_size.y as i32;

        let tile_min_x = (box_pos.x / tile_w).max(0) as u32;
        let tile_min_y = (box_pos.y / tile_h).max(0) as u32;
        let tile_max_x = ((box_pos.x + box_size.x as i32 - 1) / tile_w).max(0) as u32;
        let tile_max_y = ((box_pos.y + box_size.y as i32 - 1) / tile_h).max(0) as u32;

        let map_max_x = tilemap.size.x.saturating_sub(1);
        let map_max_y = tilemap.size.y.saturating_sub(1);
        let tile_min_x = tile_min_x.min(map_max_x);
        let tile_min_y = tile_min_y.min(map_max_y);
        let tile_max_x = tile_max_x.min(map_max_x);
        let tile_max_y = tile_max_y.min(map_max_y);

        for y in tile_min_y..=tile_max_y {
            for x in tile_min_x..=tile_max_x {
                let Ok(tile_name) = tilemap.get_tile_name(x, y) else {
                    continue;
                };
                if atlas.is_tile_trigger(tile_name) {
                    return true;
                }
            }
        }

        false
    }

    fn rule_conditions_match(
        &self,
        conditions: &[RuleCondition],
        context: &TriggerContext,
    ) -> bool {
        conditions.iter().all(|condition| {
            let result = self.evaluate_condition(condition, context);
            tracing::trace!(condition = ?condition, result, "Condition evaluated");
            result
        })
    }

    fn evaluate_condition(&self, condition: &RuleCondition, context: &TriggerContext) -> bool {
        match condition {
            RuleCondition::Always => true,
            RuleCondition::TargetExists { target } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some(),
            RuleCondition::KeyHeld { key } => {
                self.all_held_keys().contains(&Self::to_input_key(*key))
            }
            RuleCondition::EntityActive { target, is_active } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| entity.attributes.active == *is_active),
            RuleCondition::HealthBelow { target, threshold } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                .is_some_and(|health| health < *threshold),
            RuleCondition::HealthAbove { target, threshold } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                .is_some_and(|health| health > *threshold),
            RuleCondition::TriggerOtherIsPlayer => context
                .trigger_other
                .is_some_and(|other_id| self.player_id == Some(other_id)),
            RuleCondition::EntityIsKind { target, kind } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| entity.entity_kind == *kind),
            RuleCondition::TriggerOtherIsKind { kind } => context
                .trigger_other
                .and_then(|other_id| self.entity_manager.get_entity(other_id))
                .is_some_and(|entity| entity.entity_kind == *kind),
            RuleCondition::EntityHasTag { target, tag } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| entity.tags.contains(tag)),
            RuleCondition::TriggerOtherHasTag { tag } => context
                .trigger_other
                .and_then(|other_id| self.entity_manager.get_entity(other_id))
                .is_some_and(|entity| entity.tags.contains(tag)),
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => self
                .resolve_rule_target(*target, context)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| {
                    entity.attributes.inventory.item_count(item_id) >= *min_count
                }),
        }
    }

    fn buffer_rule_action(
        &self,
        action: &RuleAction,
        context: &TriggerContext,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                let sound_id = sound_id.trim();
                if sound_id.is_empty() {
                    return;
                }

                let channel = match channel {
                    RuleSoundChannel::Movement => AudioChannel::Movement,
                    RuleSoundChannel::Collision => AudioChannel::Collision,
                };

                command_buffer.push(RuleCommand::PlaySound {
                    channel,
                    sound_id: sound_id.to_string(),
                });
            }
            RuleAction::PlayMusic { track_id } => {
                let track_id = track_id.trim();
                if track_id.is_empty() {
                    return;
                }
                command_buffer.push(RuleCommand::PlayMusic {
                    track_id: track_id.to_string(),
                });
            }
            RuleAction::PlayAnimation { target, state } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::PlayAnimation {
                        entity_id,
                        state: *state,
                    });
                }
            }
            RuleAction::SetVelocity { target, velocity } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::SetVelocity {
                        entity_id,
                        velocity: glam::IVec2::new(velocity[0], velocity[1]),
                    });
                }
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                command_buffer.push(RuleCommand::Spawn {
                    entity_type: *entity_type,
                    position: glam::IVec2::new(position[0], position[1]),
                });
            }
            RuleAction::DestroySelf { target } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::DestroySelf { entity_id });
                }
            }
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => {
                tracing::info!(
                    scene_name = %scene_name,
                    spawn_point_id = %spawn_point_id,
                    "Scene switch triggered"
                );
                command_buffer.push(RuleCommand::SwitchScene {
                    scene_name: scene_name.clone(),
                    spawn_point_id: spawn_point_id.clone(),
                });
            }
            RuleAction::DamageEntity { target, amount } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::DamageEntity {
                        entity_id,
                        amount: *amount,
                    });
                }
            }
            RuleAction::HealEntity { target, amount } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::HealEntity {
                        entity_id,
                        amount: *amount,
                    });
                }
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::AddInventoryItem {
                        entity_id,
                        item_id: item_id.clone(),
                        count: *count,
                    });
                }
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::RemoveInventoryItem {
                        entity_id,
                        item_id: item_id.clone(),
                        count: *count,
                    });
                }
            }
            RuleAction::SetEntityActive { target, active } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::SetEntityActive {
                        entity_id,
                        active: *active,
                    });
                }
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => {
                if let Some(entity_id) = self.resolve_rule_target(*target, context) {
                    command_buffer.push(RuleCommand::TeleportEntity {
                        entity_id,
                        tile_x: *tile_x,
                        tile_y: *tile_y,
                    });
                }
            }
        }
    }

    pub(super) fn apply_rule_commands(
        &mut self,
        commands: Vec<RuleCommand>,
        result: &mut GameUpdateResult<AudioEvent>,
        tilemap: &TileMap,
    ) -> (Vec<(EntityId, AnimationState)>, Option<PendingSceneSwitch>) {
        let mut buffered_velocities = HashMap::new();
        let mut buffered_animations = HashMap::new();
        let mut pending_scene_switch = None;

        for command in commands {
            match command {
                RuleCommand::PlaySound { channel, sound_id } => {
                    result.add_event(AudioEvent::PlaySound {
                        channel,
                        sound_id,
                        source_position: None,
                        hearing_radius: None,
                    });
                }
                RuleCommand::PlayMusic { track_id } => {
                    result.add_event(AudioEvent::BackgroundMusic(track_id));
                }
                RuleCommand::SetVelocity {
                    entity_id,
                    velocity,
                } => {
                    buffered_velocities.entry(entity_id).or_insert(velocity);
                }
                RuleCommand::PlayAnimation { entity_id, state } => {
                    buffered_animations.entry(entity_id).or_insert(state);
                }
                RuleCommand::Spawn {
                    entity_type,
                    position,
                } => {
                    self.spawn_entity_from_rule(entity_type, position);
                }
                RuleCommand::DestroySelf { entity_id } => {
                    let removed = self.entity_manager.despawn_entity(entity_id);
                    if removed {
                        if self.player_id == Some(entity_id) {
                            self.player_id = None;
                        }
                        self.rule_runtime.velocities.remove(&entity_id);
                    }
                }
                RuleCommand::SwitchScene {
                    scene_name,
                    spawn_point_id,
                } => {
                    let target = scene_name.trim();
                    let spawn = spawn_point_id.trim();
                    if !target.is_empty() && !spawn.is_empty() && pending_scene_switch.is_none() {
                        pending_scene_switch = Some((target.to_string(), spawn.to_string()));
                    }
                }
                RuleCommand::DamageEntity { entity_id, amount } => {
                    self.pending_stat_changes.push(StatChangeRequest {
                        target_entity_id: entity_id,
                        stat_id: HEALTH_STAT_ID.to_string(),
                        delta: -amount,
                        source_entity_id: None,
                    });
                }
                RuleCommand::HealEntity { entity_id, amount } => {
                    if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                        let current = entity.attributes.current_stat(HEALTH_STAT_ID).unwrap_or(0);
                        let max = entity.attributes.base_stat(HEALTH_STAT_ID).unwrap_or(0);
                        let capped_heal = amount.min(max - current);
                        if capped_heal > 0 {
                            self.pending_stat_changes.push(StatChangeRequest {
                                target_entity_id: entity_id,
                                stat_id: HEALTH_STAT_ID.to_string(),
                                delta: capped_heal,
                                source_entity_id: None,
                            });
                        }
                    }
                }
                RuleCommand::AddInventoryItem {
                    entity_id,
                    item_id,
                    count,
                } => {
                    if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
                        entity.attributes.inventory.add_item(&item_id, count);
                    }
                }
                RuleCommand::RemoveInventoryItem {
                    entity_id,
                    item_id,
                    count,
                } => {
                    if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
                        let available = entity.attributes.inventory.item_count(&item_id);
                        let to_remove = count.min(available);
                        if to_remove > 0 {
                            let new_count = available.saturating_sub(to_remove);
                            if new_count == 0 {
                                entity.attributes.inventory.items.remove(&item_id);
                            } else if let Some(entry) =
                                entity.attributes.inventory.items.get_mut(&item_id)
                            {
                                *entry = new_count;
                            }
                        }
                    }
                }
                RuleCommand::SetEntityActive { entity_id, active } => {
                    if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
                        entity.attributes.active = active;
                    }
                }
                RuleCommand::TeleportEntity {
                    entity_id,
                    tile_x,
                    tile_y,
                } => {
                    if let Some(entity) = self.entity_manager.get_entity_mut(entity_id) {
                        // Convert tile coordinates to pixel coordinates (top-left of tile)
                        let pixel_x = (tile_x * tilemap.tile_size.x) as i32;
                        let pixel_y = (tile_y * tilemap.tile_size.y) as i32;
                        entity.position = glam::IVec2::new(pixel_x, pixel_y);
                    }
                }
            }
        }

        for (entity_id, velocity) in buffered_velocities {
            self.rule_runtime.velocities.insert(entity_id, velocity);
        }

        let mut pending_animations = buffered_animations.into_iter().collect::<Vec<_>>();
        pending_animations.sort_by_key(|(entity_id, _)| *entity_id);
        (pending_animations, pending_scene_switch)
    }

    /// Resolves a rule target to an entity ID using the current trigger context.
    ///
    /// # Architecture Note (for Phase 1.5B+ implementers)
    ///
    /// This method handles all `RuleTarget` variants:
    /// - `Player`: Returns the player entity ID
    /// - `Entity(id)`: Returns the specified entity ID directly
    /// - `TriggerSelf`: Returns `context.trigger_self` (the primary subject)
    /// - `TriggerOther`: Returns `context.trigger_other` (the secondary entity)
    /// - `RuleOwner`: Currently returns `None` for scene-owned rules.
    ///   When entity-owned rules are added, this should return the owning entity.
    ///
    /// Returns `None` if the target cannot be resolved (e.g., no player, context missing).
    fn resolve_rule_target(
        &self,
        target: RuleTarget,
        context: &TriggerContext,
    ) -> Option<EntityId> {
        match target {
            RuleTarget::Player => self.player_id,
            RuleTarget::Entity(entity_id) => Some(entity_id),
            RuleTarget::TriggerSelf => context.trigger_self,
            RuleTarget::TriggerOther => context.trigger_other,
            // RuleOwner is only valid for entity-owned rules.
            // Currently all rules are scene-owned, so this returns None.
            // When entity-owned rules are added, pass the owner ID through context.
            RuleTarget::RuleOwner => None,
        }
    }

    fn spawn_entity_from_rule(
        &mut self,
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    ) -> EntityId {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => self.spawn_player_like_npc(position),
            RuleSpawnEntityType::Npc => self.entity_manager.spawn_entity(
                EntityKind::Npc,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes::default(),
            ),
            RuleSpawnEntityType::Item => self.entity_manager.spawn_entity(
                EntityKind::Item,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes {
                    solid: false,
                    can_move: false,
                    ..EntityAttributes::default()
                },
            ),
            RuleSpawnEntityType::Decoration => self.entity_manager.spawn_entity(
                EntityKind::Decoration,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes {
                    solid: false,
                    can_move: false,
                    ..EntityAttributes::default()
                },
            ),
            RuleSpawnEntityType::Trigger => self.entity_manager.spawn_entity(
                EntityKind::Trigger,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes {
                    solid: false,
                    can_move: false,
                    visible: false,
                    ..EntityAttributes::default()
                },
            ),
        }
    }

    pub(super) fn apply_rule_animations(
        &mut self,
        pending_animations: Vec<(EntityId, AnimationState)>,
    ) {
        for (entity_id, state) in pending_animations {
            let Some(entity) = self.entity_manager.get_entity_mut(entity_id) else {
                continue;
            };
            let Some(animation_controller) = entity.attributes.animation_controller.as_mut() else {
                continue;
            };

            if animation_controller.current_clip_state != state {
                animation_controller.play(state);
            }
        }
    }

    /// Gets the rule-assigned velocity for an entity, if any.
    /// Used by tests to verify rule actions.
    pub fn get_rule_velocity(&self, entity_id: EntityId) -> Option<glam::IVec2> {
        self.rule_runtime.velocities.get(&entity_id).copied()
    }

    /// Sets the rule-assigned velocity for an entity directly.
    /// Used by tests to set up specific scenarios.
    pub fn set_rule_velocity(&mut self, entity_id: EntityId, velocity: glam::IVec2) {
        self.rule_runtime.velocities.insert(entity_id, velocity);
    }

    /// Detects tile transitions for all active entities and populates frame_tile_transitions.
    ///
    /// This should be called after all movement is complete but before reactive rules fire.
    /// It compares each entity's current tile position with their previous tile position
    /// and generates OnTileExit and OnTileEnter events for transitions.
    pub(super) fn detect_tile_transitions(&mut self, tilemap: &TileMap) {
        if tilemap.tile_size.x == 0 || tilemap.tile_size.y == 0 {
            return;
        }

        let tile_w = tilemap.tile_size.x;
        let tile_h = tilemap.tile_size.y;

        for entity_id in self.entity_manager.active_entities() {
            let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                continue;
            };

            // ═══════════════════════════════════════════════════════════════
            // CENTER-POINT TILE DETECTION
            // ═══════════════════════════════════════════════════════════════
            // This implementation uses the entity's CENTER POINT to determine
            // which single tile they occupy. This means:
            //
            // 1. Only ONE tile is tracked per entity at any time
            // 2. Entities may physically overlap MULTIPLE tiles, but only the
            //    tile containing their center point is considered "occupied"
            // 3. Example: A 16x16 player on 8x8 tiles physically touches 4 tiles,
            //    but is tracked as being on only the ONE tile containing their center
            //
            // DESIGN RATIONALE:
            // - Simplicity: One tile per entity, clear transition events
            // - Common pattern: Used in classic RPGs (Pokemon, Zelda, etc.)
            // - Predictability: Center-point gives consistent behavior
            //
            // ALTERNATIVE (not implemented):
            // - Full coverage detection would track ALL tiles an entity overlaps
            // - More physically accurate but significantly more complex
            // - Would generate multiple enter/exit events per movement
            //
            // If you need precise multi-tile detection for specific entities,
            // consider adding a separate collision query system rather than
            // modifying this core tile tracking logic.
            // ═══════════════════════════════════════════════════════════════
            let center_x = entity.position.x + (entity.size.x as i32 / 2);
            let center_y = entity.position.y + (entity.size.y as i32 / 2);
            let current_tile_x = (center_x.max(0) as u32) / tile_w;
            let current_tile_y = (center_y.max(0) as u32) / tile_h;

            // Clamp to map bounds
            let current_tile_x = current_tile_x.min(tilemap.size.x.saturating_sub(1));
            let current_tile_y = current_tile_y.min(tilemap.size.y.saturating_sub(1));

            // Check if we have a previous tile position for this entity
            if let Some(&(prev_tile_x, prev_tile_y)) =
                self.rule_runtime.entity_tile_positions.get(&entity_id)
            {
                // If tile position changed, generate exit and enter events
                if (prev_tile_x, prev_tile_y) != (current_tile_x, current_tile_y) {
                    // Log player movement specifically
                    if Some(entity_id) == self.player_id {
                        tracing::trace!(
                            "Player moved from_tile=({},{}) to_tile=({},{}) pixel_pos=({},{})",
                            prev_tile_x,
                            prev_tile_y,
                            current_tile_x,
                            current_tile_y,
                            entity.position.x,
                            entity.position.y
                        );
                    } else {
                        tracing::trace!(
                            entity = ?entity_id,
                            from_tile = ?(prev_tile_x, prev_tile_y),
                            to_tile = ?(current_tile_x, current_tile_y),
                            pixel_pos = ?(entity.position.x, entity.position.y),
                            "Tile transition detected"
                        );
                    }

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
    }

    /// Collects rule commands for tile transition events (OnTileEnter/OnTileExit).
    ///
    /// Validates tile coordinates against the active tilemap bounds.
    /// Rules with out-of-bounds coordinates are skipped with a warning.
    pub(super) fn collect_rule_commands_for_tile_transitions(
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

            let mut matching_rules = self
                .rules
                .rules
                .iter()
                .filter(|rule| rule.enabled && rule.trigger == trigger)
                .filter(|rule| {
                    !(rule.once
                        && self
                            .rule_runtime
                            .fired_once_rules
                            .contains(rule.id.as_str()))
                })
                .cloned()
                .collect::<Vec<_>>();

            matching_rules
                .sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

            for rule in matching_rules {
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

                for action in &rule.actions {
                    debug!(rule_id = %rule.id, action = ?action, "Executing tile transition action");
                    self.buffer_rule_action(action, &context, command_buffer);
                }

                if rule.once {
                    self.rule_runtime.fired_once_rules.insert(rule.id);
                }
            }
        }
    }
}
