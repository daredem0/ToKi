
use toki_core::animation::AnimationState;
use toki_core::rules::{
    RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleTarget, RuleTrigger,
};

use super::{RuleGraph, RuleGraphEdge, RuleGraphError};

fn append_condition_to_chain(
    graph: &mut RuleGraph,
    trigger_node_id: super::RuleGraphNodeId,
    condition: RuleCondition,
) -> Result<(), super::RuleGraphEditError> {
    let mut rules = graph
        .to_rule_set()
        .map_err(super::RuleGraphEditError::GraphInvalid)?;
    let rule_id = graph
        .chains
        .iter()
        .find(|chain| chain.trigger_node_id == trigger_node_id)
        .map(|chain| chain.rule_id.clone())
        .ok_or(super::RuleGraphEditError::MissingChain { trigger_node_id })?;
    let rule = rules
        .rules
        .iter_mut()
        .find(|rule| rule.id == rule_id)
        .ok_or(super::RuleGraphEditError::MissingChain { trigger_node_id })?;
    rule.conditions.push(condition);
    *graph = RuleGraph::from_rule_set(&rules);
    Ok(())
}

fn trigger_node_for_node(
    graph: &RuleGraph,
    node_id: super::RuleGraphNodeId,
) -> Option<super::RuleGraphNodeId> {
    for chain in &graph.chains {
        let Ok(sequence) = graph.chain_node_sequence(chain.trigger_node_id) else {
            continue;
        };
        if sequence.contains(&node_id) {
            return Some(chain.trigger_node_id);
        }
    }
    None
}

fn sample_rules() -> RuleSet {
    RuleSet {
        rules: vec![
            toki_core::rules::Rule {
                id: "rule_spawn".to_string(),
                enabled: true,
                priority: 20,
                once: false,
                trigger: RuleTrigger::OnPlayerMove,
                conditions: vec![
                    RuleCondition::KeyHeld {
                        key: RuleKey::Right,
                    },
                    RuleCondition::TargetExists {
                        target: RuleTarget::Player,
                    },
                ],
                actions: vec![
                    RuleAction::PlaySound {
                        channel: RuleSoundChannel::Movement,
                        sound_id: "sfx_step".to_string(),
                    },
                    RuleAction::Spawn {
                        entity_type: toki_core::rules::RuleSpawnEntityType::Item,
                        position: [10, 20],
                    },
                ],
            },
            toki_core::rules::Rule {
                id: "rule_music".to_string(),
                enabled: false,
                priority: -2,
                once: true,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![
                    RuleAction::PlayMusic {
                        track_id: "bgm_1".to_string(),
                    },
                    RuleAction::PlayAnimation {
                        target: RuleTarget::Player,
                        state: AnimationState::Idle,
                    },
                ],
            },
        ],
    }
}

#[test]
fn rule_set_roundtrip_through_graph_is_lossless_and_deterministic() {
    let original = sample_rules();
    let graph = RuleGraph::from_rule_set(&original);
    let roundtrip = graph
        .to_rule_set()
        .expect("graph generated from rules should always be valid");

    assert_eq!(roundtrip, original);
}

#[test]
fn graph_to_rule_set_rejects_branching_chains() {
    let base = sample_rules();
    let mut graph = RuleGraph::from_rule_set(&base);
    let Some(first_chain) = graph.chains.first() else {
        panic!("expected at least one chain");
    };
    let trigger_id = first_chain.trigger_node_id;
    let Some(trigger_edge_target) = graph
        .edges
        .iter()
        .find(|edge| edge.from == trigger_id)
        .map(|edge| edge.to)
    else {
        panic!("expected trigger edge");
    };
    graph.edges.push(RuleGraphEdge {
        from: trigger_id,
        to: trigger_edge_target,
    });

    let error = graph
        .to_rule_set()
        .expect_err("branching trigger node must be rejected");
    assert!(matches!(error, RuleGraphError::NonLinearChain { .. }));
}

#[test]
fn add_trigger_chain_appends_trigger_only_rule() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let rule_id = graph
        .add_trigger_chain()
        .expect("adding trigger chain should work");
    let roundtrip = graph.to_rule_set().expect("graph must remain valid");
    let added_rule = roundtrip
        .rules
        .iter()
        .find(|rule| rule.id == rule_id)
        .expect("new rule should exist");
    assert!(added_rule.conditions.is_empty());
    assert!(added_rule.actions.is_empty());
}

#[test]
fn append_condition_and_remove_node_roundtrip() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let trigger = graph.chains[0].trigger_node_id;
    append_condition_to_chain(&mut graph, trigger, RuleCondition::Always)
        .expect("append condition should work");

    let chain_nodes = graph
        .chain_node_sequence(trigger)
        .expect("chain should remain linear");
    let condition_node = chain_nodes[1];
    graph
        .remove_node(condition_node)
        .expect("removing condition node should work");

    graph.to_rule_set().expect("graph should remain valid");
}

#[test]
fn connect_nodes_allows_action_to_trigger_when_linear() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let first_trigger = graph.chains[0].trigger_node_id;
    let second_trigger = graph.chains[1].trigger_node_id;
    let nodes = graph
        .chain_node_sequence(first_trigger)
        .expect("chain should be valid");
    let action_node = *nodes
        .iter()
        .find(|node_id| {
            let node = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == **node_id)
                .expect("node must exist");
            matches!(node.kind, super::RuleGraphNodeKind::Action(_))
        })
        .expect("action node expected");
    let previous_targets = graph
        .edges
        .iter()
        .filter(|edge| edge.from == action_node)
        .map(|edge| edge.to)
        .collect::<Vec<_>>();
    for previous_target in previous_targets {
        graph.disconnect_nodes(action_node, previous_target);
    }
    graph
        .connect_nodes(action_node, second_trigger)
        .expect("action -> trigger should be accepted in free-connection mode");
    graph
        .to_rule_set()
        .expect("rewired linear chain should still serialize");
}

#[test]
fn connect_nodes_allows_joining_into_existing_trigger_chain() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let first_trigger = graph.chains[0].trigger_node_id;
    let second_trigger = graph.chains[1].trigger_node_id;

    let first_sequence = graph
        .chain_node_sequence(first_trigger)
        .expect("first chain should be valid");
    let second_sequence = graph
        .chain_node_sequence(second_trigger)
        .expect("second chain should be valid");

    let first_action = *first_sequence
        .iter()
        .find(|node_id| {
            let node = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == **node_id)
                .expect("node must exist");
            matches!(node.kind, super::RuleGraphNodeKind::Action(_))
        })
        .expect("first chain action should exist");
    let second_condition = *second_sequence
        .iter()
        .find(|node_id| {
            let node = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == **node_id)
                .expect("node must exist");
            matches!(node.kind, super::RuleGraphNodeKind::Condition(_))
        })
        .expect("second chain condition should exist");
    let previous_targets = graph
        .edges
        .iter()
        .filter(|edge| edge.from == first_action)
        .map(|edge| edge.to)
        .collect::<Vec<_>>();
    for previous_target in previous_targets {
        graph.disconnect_nodes(first_action, previous_target);
    }

    graph
        .connect_nodes(first_action, second_condition)
        .expect("joining into existing condition node should be accepted");

    let roundtrip = graph
        .to_rule_set()
        .expect("joined chain should still serialize");
    assert_eq!(roundtrip.rules.len(), 2);
}

#[test]
fn connect_nodes_preserves_existing_outgoing_edges() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let from = graph
        .add_action_node(RuleAction::PlayMusic {
            track_id: "music_a".to_string(),
        })
        .expect("detached source node should be created");
    let to_a = graph
        .add_condition_node(RuleCondition::Always)
        .expect("first detached target should be created");
    let to_b = graph
        .add_condition_node(RuleCondition::KeyHeld { key: RuleKey::Left })
        .expect("second detached target should be created");

    graph
        .connect_nodes(from, to_a)
        .expect("first connect should succeed");
    graph
        .connect_nodes(from, to_b)
        .expect("second connect should keep the first one");

    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.from == from && edge.to == to_a));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.from == from && edge.to == to_b));
}

#[test]
fn can_connect_nodes_rejects_cycle_candidates() {
    let graph = RuleGraph::from_rule_set(&sample_rules());
    let first_trigger = graph.chains[0].trigger_node_id;
    let sequence = graph
        .chain_node_sequence(first_trigger)
        .expect("chain should be valid");
    let last_action = *sequence
        .iter()
        .rev()
        .find(|node_id| {
            graph
                .nodes
                .iter()
                .find(|node| node.id == **node_id)
                .is_some_and(|node| matches!(node.kind, super::RuleGraphNodeKind::Action(_)))
        })
        .expect("expected action in chain");

    let error = graph
        .can_connect_nodes(last_action, first_trigger)
        .expect_err("action -> trigger in same chain must be rejected as cycle");
    assert!(matches!(
        error,
        super::RuleGraphEditError::InvalidConnection { .. }
    ));
}

#[test]
fn stable_node_keys_remain_unique_after_cross_chain_connect() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let first_trigger = graph.chains[0].trigger_node_id;
    let second_trigger = graph.chains[1].trigger_node_id;
    let first_sequence = graph
        .chain_node_sequence(first_trigger)
        .expect("first chain should be valid");
    let first_action = *first_sequence
        .iter()
        .find(|node_id| {
            graph
                .nodes
                .iter()
                .find(|node| node.id == **node_id)
                .is_some_and(|node| matches!(node.kind, super::RuleGraphNodeKind::Action(_)))
        })
        .expect("expected action in first chain");

    graph
        .connect_nodes(first_action, second_trigger)
        .expect("cross-chain connect should succeed");

    let mut keys = graph
        .nodes
        .iter()
        .filter_map(|node| graph.stable_node_key(node.id))
        .collect::<Vec<_>>();
    let total = keys.len();
    keys.sort();
    keys.dedup();
    assert_eq!(keys.len(), total, "stable keys must remain unique");
}

#[test]
fn from_rule_set_merges_shared_action_nodes_for_multiple_triggers() {
    let shared_action = RuleAction::PlayMusic {
        track_id: "bgm_shared".to_string(),
    };
    let rules = RuleSet {
        rules: vec![
            toki_core::rules::Rule {
                id: "rule_a".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: Vec::new(),
                actions: vec![shared_action.clone()],
            },
            toki_core::rules::Rule {
                id: "rule_b".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: Vec::new(),
                actions: vec![shared_action.clone()],
            },
        ],
    };

    let graph = RuleGraph::from_rule_set(&rules);
    let shared_action_nodes = graph
            .nodes
            .iter()
            .filter(|node| matches!(&node.kind, super::RuleGraphNodeKind::Action(action) if *action == shared_action))
            .map(|node| node.id)
            .collect::<Vec<_>>();
    assert_eq!(
        shared_action_nodes.len(),
        1,
        "identical action suffix should be represented by one shared node"
    );

    let shared_action_id = shared_action_nodes[0];
    let trigger_ids = graph
        .chains
        .iter()
        .map(|chain| chain.trigger_node_id)
        .collect::<Vec<_>>();
    assert!(
        graph
            .edges
            .iter()
            .filter(|edge| trigger_ids.contains(&edge.from) && edge.to == shared_action_id)
            .count()
            >= 2
    );
}

#[test]
fn from_rule_set_merges_shared_condition_and_action_suffixes() {
    let shared_action = RuleAction::DestroySelf {
        target: RuleTarget::Player,
    };
    let rules = RuleSet {
        rules: vec![
            toki_core::rules::Rule {
                id: "rule_a".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![shared_action.clone()],
            },
            toki_core::rules::Rule {
                id: "rule_b".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![RuleCondition::Always],
                actions: vec![shared_action.clone()],
            },
        ],
    };

    let graph = RuleGraph::from_rule_set(&rules);
    let condition_nodes = graph
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.kind,
                super::RuleGraphNodeKind::Condition(RuleCondition::Always)
            )
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();
    let action_nodes = graph
            .nodes
            .iter()
            .filter(|node| matches!(&node.kind, super::RuleGraphNodeKind::Action(action) if *action == shared_action))
            .map(|node| node.id)
            .collect::<Vec<_>>();

    assert_eq!(condition_nodes.len(), 1);
    assert_eq!(action_nodes.len(), 1);
    assert!(graph
        .edges
        .iter()
        .any(|edge| { edge.from == condition_nodes[0] && edge.to == action_nodes[0] }));
}

#[test]
fn add_condition_node_creates_new_independent_chain() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let existing_trigger = graph.chains[0].trigger_node_id;
    let initial_chains = graph.chains.len();

    let new_condition = graph
        .add_condition_node(RuleCondition::Always)
        .expect("adding standalone condition node should succeed");
    assert_eq!(graph.chains.len(), initial_chains);

    assert!(
        !graph
            .edges
            .iter()
            .any(|edge| edge.from == existing_trigger && edge.to == new_condition),
        "new condition should not auto-connect to the existing chain"
    );
    assert!(
        graph.stable_node_key(new_condition).is_some(),
        "standalone condition should receive a stable key in the editor graph"
    );
}

#[test]
fn disconnect_node_removes_all_incident_edges() {
    let mut graph = RuleGraph::from_rule_set(&sample_rules());
    let first_trigger = graph.chains[0].trigger_node_id;
    let sequence = graph
        .chain_node_sequence(first_trigger)
        .expect("chain should be valid");
    let disconnect_target = *sequence
        .iter()
        .find(|node_id| {
            graph
                .nodes
                .iter()
                .find(|node| node.id == **node_id)
                .is_some_and(|node| matches!(node.kind, super::RuleGraphNodeKind::Action(_)))
        })
        .expect("chain should contain an action node");

    graph
        .disconnect_node(disconnect_target)
        .expect("disconnecting node should succeed");
    assert!(!graph
        .edges
        .iter()
        .any(|edge| edge.from == disconnect_target || edge.to == disconnect_target));
    graph
        .to_rule_set()
        .expect("graph should remain serializable after disconnect");
}

#[test]
fn trigger_node_for_node_resolves_owner_chain_trigger() {
    let graph = RuleGraph::from_rule_set(&sample_rules());
    let trigger = graph.chains[0].trigger_node_id;
    let sequence = graph
        .chain_node_sequence(trigger)
        .expect("chain should be valid");
    let action_node = *sequence
        .iter()
        .find(|node_id| {
            let node = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == **node_id)
                .expect("node must exist");
            matches!(node.kind, super::RuleGraphNodeKind::Action(_))
        })
        .expect("action node expected");
    assert_eq!(trigger_node_for_node(&graph, action_node), Some(trigger));
}

#[test]
fn stable_node_keys_are_generated_for_chain_nodes() {
    let graph = RuleGraph::from_rule_set(&sample_rules());
    let trigger = graph.chains[0].trigger_node_id;
    let sequence = graph
        .chain_node_sequence(trigger)
        .expect("chain should be valid");

    let keys = sequence
        .iter()
        .filter_map(|node_id| graph.stable_node_key(*node_id))
        .collect::<Vec<_>>();

    assert_eq!(keys.len(), sequence.len());
    assert!(keys.iter().all(|key| key.starts_with("node:")));
    let mut unique = keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys.len());
}

#[test]
fn node_id_for_stable_key_resolves_existing_node() {
    let graph = RuleGraph::from_rule_set(&sample_rules());
    let trigger_key = graph
        .stable_node_key(graph.chains[0].trigger_node_id)
        .expect("expected trigger to have a stable key");
    let Some(trigger_id) = graph.node_id_for_stable_key(&trigger_key) else {
        panic!("expected stable key to resolve to a node id");
    };
    let trigger_node = graph
        .nodes
        .iter()
        .find(|node| node.id == trigger_id)
        .expect("resolved node id should exist");
    assert!(matches!(
        trigger_node.kind,
        super::RuleGraphNodeKind::Trigger(_)
    ));
}

#[test]
fn node_id_for_stable_key_supports_legacy_chain_keys() {
    let graph = RuleGraph::from_rule_set(&sample_rules());
    let Some(trigger_id) = graph.node_id_for_stable_key("rule_spawn:trigger") else {
        panic!("expected legacy trigger key to resolve");
    };
    let trigger_node = graph
        .nodes
        .iter()
        .find(|node| node.id == trigger_id)
        .expect("resolved node id should exist");
    assert!(matches!(
        trigger_node.kind,
        super::RuleGraphNodeKind::Trigger(_)
    ));
}

#[test]
fn auto_layout_uses_minimum_spacing_between_nodes_and_chains() {
    let graph = RuleGraph::from_rule_set(&sample_rules());

    let first_trigger = graph.chains[0].trigger_node_id;
    let second_trigger = graph.chains[1].trigger_node_id;

    let first_trigger_node = graph
        .nodes
        .iter()
        .find(|node| node.id == first_trigger)
        .expect("first trigger node should exist");
    let second_trigger_node = graph
        .nodes
        .iter()
        .find(|node| node.id == second_trigger)
        .expect("second trigger node should exist");

    let center_spacing = second_trigger_node.position[1] - first_trigger_node.position[1];
    let edge_spacing = center_spacing - super::RuleGraph::auto_layout_node_height();
    let spacing_epsilon = 0.01_f32;
    assert!(
        edge_spacing + spacing_epsilon >= super::RuleGraph::auto_layout_vertical_edge_spacing(),
        "vertical edge spacing should keep chains visually separated"
    );

    let first_sequence = graph
        .chain_node_sequence(first_trigger)
        .expect("first chain should be valid");
    for pair in first_sequence.windows(2) {
        let left = graph
            .nodes
            .iter()
            .find(|node| node.id == pair[0])
            .expect("left node should exist");
        let right = graph
            .nodes
            .iter()
            .find(|node| node.id == pair[1])
            .expect("right node should exist");
        let center_spacing = right.position[0] - left.position[0];
        let edge_spacing = center_spacing - super::RuleGraph::auto_layout_node_width();
        assert!(
            edge_spacing + spacing_epsilon
                >= super::RuleGraph::auto_layout_horizontal_edge_spacing(),
            "horizontal edge spacing should keep consecutive nodes visually separated"
        );
    }
}
