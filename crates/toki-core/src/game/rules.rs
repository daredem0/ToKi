use std::collections::{HashMap, HashSet};

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{AiBehavior, Entity, EntityAttributes, EntityId, EntityKind};
use crate::events::GameUpdateResult;
use crate::rules::{
    Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleSpawnEntityType, RuleTarget,
    RuleTrigger, TriggerContext,
};

use super::{AudioChannel, AudioEvent, GameState};

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
            if !self.rule_conditions_match(&rule.conditions, &context) {
                continue;
            }

            for action in &rule.actions {
                self.buffer_rule_action(action, &context, command_buffer);
            }

            if rule.once {
                self.rule_runtime.fired_once_rules.insert(rule.id);
            }
        }
    }

    /// Collects rule commands for OnInteract triggers, filtering by interaction mode.
    ///
    /// The `event` contains the spatial relationship between player and interactable.
    /// Only rules whose interaction mode matches the spatial relationship will fire.
    pub(super) fn collect_rule_commands_for_interaction(
        &mut self,
        event: &InteractionEvent,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let context = TriggerContext::with_pair(event.interactor, event.interactable);

        let mut matching_rules = self
            .rules
            .rules
            .iter()
            .filter(|rule| rule.enabled)
            .filter(|rule| matches!(rule.trigger, RuleTrigger::OnInteract { .. }))
            .filter(|rule| {
                !(rule.once
                    && self
                        .rule_runtime
                        .fired_once_rules
                        .contains(rule.id.as_str()))
            })
            .filter(|rule| {
                // Check if the rule's interaction mode matches the event's spatial
                let mode = rule.trigger.interaction_mode().unwrap_or_default();
                Self::interaction_mode_matches(mode, event.spatial)
            })
            .cloned()
            .collect::<Vec<_>>();

        matching_rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

        for rule in matching_rules {
            if !self.rule_conditions_match(&rule.conditions, &context) {
                continue;
            }

            for action in &rule.actions {
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
    fn interaction_mode_matches(mode: crate::rules::InteractionMode, spatial: InteractionSpatial) -> bool {
        use crate::rules::InteractionMode;

        match mode {
            InteractionMode::Overlap => matches!(spatial, InteractionSpatial::Overlap),
            InteractionMode::Adjacent => true, // Adjacent mode accepts any proximity
            InteractionMode::InFront => matches!(spatial, InteractionSpatial::InFront | InteractionSpatial::Overlap),
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
        conditions.iter().all(|condition| match condition {
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
        })
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
                command_buffer.push(RuleCommand::SwitchScene {
                    scene_name: scene_name.clone(),
                    spawn_point_id: spawn_point_id.clone(),
                });
            }
        }
    }

    pub(super) fn apply_rule_commands(
        &mut self,
        commands: Vec<RuleCommand>,
        result: &mut GameUpdateResult<AudioEvent>,
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
    fn resolve_rule_target(&self, target: RuleTarget, context: &TriggerContext) -> Option<EntityId> {
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
                    ai_behavior: AiBehavior::None,
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
                    ai_behavior: AiBehavior::None,
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
                    ai_behavior: AiBehavior::None,
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
}
