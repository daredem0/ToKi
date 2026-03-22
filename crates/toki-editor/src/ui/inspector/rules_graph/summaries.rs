//! Summary generation for rule graph nodes.

use super::*;

impl InspectorSystem {
    pub(in super::super) fn rule_graph_node_badges(graph: &RuleGraph) -> HashMap<u64, String> {
        let mut node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
        node_ids.sort_unstable();

        let mut trigger_index = 0usize;
        let mut condition_index = 0usize;
        let mut action_index = 0usize;
        let mut badges = HashMap::new();
        for node_id in node_ids {
            let Some(node) = graph.nodes.iter().find(|candidate| candidate.id == node_id) else {
                continue;
            };
            let badge = match node.kind {
                RuleGraphNodeKind::Trigger(_) => {
                    trigger_index += 1;
                    format!("T{}", trigger_index)
                }
                RuleGraphNodeKind::Condition(_) => {
                    condition_index += 1;
                    format!("C{}", condition_index)
                }
                RuleGraphNodeKind::Action(_) => {
                    action_index += 1;
                    format!("A{}", action_index)
                }
            };
            badges.insert(node_id, badge);
        }
        badges
    }

    pub(in super::super) fn rule_graph_node_label_for_inspector(
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        node_id: u64,
    ) -> Option<String> {
        let node = graph.nodes.iter().find(|node| node.id == node_id)?;
        let badge = node_badges
            .get(&node_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let details = match &node.kind {
            RuleGraphNodeKind::Trigger(trigger) => {
                format!("Trigger {}", Self::rule_graph_trigger_summary(*trigger))
            }
            RuleGraphNodeKind::Condition(condition) => {
                format!(
                    "Condition {}",
                    Self::rule_graph_condition_summary(condition)
                )
            }
            RuleGraphNodeKind::Action(action) => {
                format!("Action {}", Self::rule_graph_action_summary(action))
            }
        };
        Some(format!("{}: {}", badge, details))
    }

    pub(in super::super) fn rule_graph_trigger_summary(trigger: RuleTrigger) -> String {
        match trigger {
            RuleTrigger::OnStart => "OnStart".to_string(),
            RuleTrigger::OnUpdate => "OnUpdate".to_string(),
            RuleTrigger::OnPlayerMove => "OnPlayerMove".to_string(),
            RuleTrigger::OnKey { key } => format!("OnKey({})", Self::rule_key_label(key)),
            RuleTrigger::OnCollision { entity: None } => "OnCollision".to_string(),
            RuleTrigger::OnCollision {
                entity: Some(target),
            } => {
                format!("OnCollision({})", Self::rule_graph_target_summary(target))
            }
            RuleTrigger::OnDamaged { entity: None } => "OnDamaged".to_string(),
            RuleTrigger::OnDamaged {
                entity: Some(target),
            } => {
                format!("OnDamaged({})", Self::rule_graph_target_summary(target))
            }
            RuleTrigger::OnDeath { entity: None } => "OnDeath".to_string(),
            RuleTrigger::OnDeath {
                entity: Some(target),
            } => {
                format!("OnDeath({})", Self::rule_graph_target_summary(target))
            }
            RuleTrigger::OnTrigger => "OnTrigger".to_string(),
            RuleTrigger::OnInteract { .. } => "OnInteract".to_string(),
            RuleTrigger::OnTileEnter { x, y } => format!("OnTileEnter({}, {})", x, y),
            RuleTrigger::OnTileExit { x, y } => format!("OnTileExit({}, {})", x, y),
        }
    }

    pub(in super::super) fn rule_graph_condition_summary(condition: &RuleCondition) -> String {
        match condition {
            RuleCondition::Always => "Always".to_string(),
            RuleCondition::TargetExists { target } => {
                format!("TargetExists({})", Self::rule_graph_target_summary(*target))
            }
            RuleCondition::KeyHeld { key } => format!("KeyHeld({})", Self::rule_key_label(*key)),
            RuleCondition::EntityActive { target, is_active } => format!(
                "EntityActive({}, {})",
                Self::rule_graph_target_summary(*target),
                if *is_active { "true" } else { "false" }
            ),
            RuleCondition::HealthBelow { target, threshold } => format!(
                "HealthBelow({}, {})",
                Self::rule_graph_target_summary(*target),
                threshold
            ),
            RuleCondition::HealthAbove { target, threshold } => format!(
                "HealthAbove({}, {})",
                Self::rule_graph_target_summary(*target),
                threshold
            ),
            RuleCondition::TriggerOtherIsPlayer => "TriggerOtherIsPlayer".to_string(),
            RuleCondition::EntityIsKind { target, kind } => format!(
                "EntityIsKind({}, {:?})",
                Self::rule_graph_target_summary(*target),
                kind
            ),
            RuleCondition::TriggerOtherIsKind { kind } => format!("TriggerOtherIsKind({:?})", kind),
            RuleCondition::EntityHasTag { target, tag } => format!(
                "EntityHasTag({}, {})",
                Self::rule_graph_target_summary(*target),
                tag
            ),
            RuleCondition::TriggerOtherHasTag { tag } => format!("TriggerOtherHasTag({})", tag),
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => format!(
                "HasInventoryItem({}, {}, {})",
                Self::rule_graph_target_summary(*target),
                item_id,
                min_count
            ),
        }
    }

    pub(in super::super) fn rule_graph_action_summary(action: &RuleAction) -> String {
        match action {
            RuleAction::PlaySound { channel, sound_id } => format!(
                "PlaySound({:?}, {})",
                channel,
                if sound_id.is_empty() {
                    "<empty>"
                } else {
                    sound_id
                }
            ),
            RuleAction::PlayMusic { track_id } => format!(
                "PlayMusic({})",
                if track_id.is_empty() {
                    "<empty>"
                } else {
                    track_id
                }
            ),
            RuleAction::PlayAnimation { target, state } => {
                format!(
                    "PlayAnimation({}, {:?})",
                    Self::rule_graph_target_summary(*target),
                    state
                )
            }
            RuleAction::SetVelocity { target, velocity } => format!(
                "SetVelocity({}, [{}, {}])",
                Self::rule_graph_target_summary(*target),
                velocity[0],
                velocity[1]
            ),
            RuleAction::Spawn {
                entity_type,
                position,
            } => format!(
                "Spawn({:?}, [{}, {}])",
                entity_type, position[0], position[1]
            ),
            RuleAction::DestroySelf { target } => {
                format!("DestroySelf({})", Self::rule_graph_target_summary(*target))
            }
            RuleAction::SwitchScene {
                scene_name,
                spawn_point_id,
            } => {
                let scene = if scene_name.is_empty() {
                    "<empty>"
                } else {
                    scene_name
                };
                let spawn = if spawn_point_id.is_empty() {
                    "<empty>"
                } else {
                    spawn_point_id
                };
                format!("SwitchScene({scene} -> {spawn})")
            }
            RuleAction::DamageEntity { target, amount } => {
                format!(
                    "DamageEntity({}, {})",
                    Self::rule_graph_target_summary(*target),
                    amount
                )
            }
            RuleAction::HealEntity { target, amount } => {
                format!(
                    "HealEntity({}, {})",
                    Self::rule_graph_target_summary(*target),
                    amount
                )
            }
            RuleAction::AddInventoryItem {
                target,
                item_id,
                count,
            } => {
                let item = if item_id.is_empty() {
                    "<empty>"
                } else {
                    item_id
                };
                format!(
                    "AddItem({}, {}, {})",
                    Self::rule_graph_target_summary(*target),
                    item,
                    count
                )
            }
            RuleAction::RemoveInventoryItem {
                target,
                item_id,
                count,
            } => {
                let item = if item_id.is_empty() {
                    "<empty>"
                } else {
                    item_id
                };
                format!(
                    "RemoveItem({}, {}, {})",
                    Self::rule_graph_target_summary(*target),
                    item,
                    count
                )
            }
            RuleAction::SetEntityActive { target, active } => {
                format!(
                    "SetActive({}, {})",
                    Self::rule_graph_target_summary(*target),
                    active
                )
            }
            RuleAction::TeleportEntity {
                target,
                tile_x,
                tile_y,
            } => {
                format!(
                    "Teleport({}, tile[{}, {}])",
                    Self::rule_graph_target_summary(*target),
                    tile_x,
                    tile_y
                )
            }
        }
    }

    pub(super) fn rule_graph_target_summary(target: RuleTarget) -> String {
        match target {
            RuleTarget::Player => "Player".to_string(),
            RuleTarget::Entity(id) => format!("Entity({})", id),
            RuleTarget::RuleOwner => "RuleOwner".to_string(),
            RuleTarget::TriggerSelf => "TriggerSelf".to_string(),
            RuleTarget::TriggerOther => "TriggerOther".to_string(),
        }
    }
}
