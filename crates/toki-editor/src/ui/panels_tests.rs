use super::{GraphValidationFixCommand, PanelSystem};
use crate::ui::rule_graph::RuleGraph;
use std::collections::HashMap;
use toki_core::rules::{
    Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleTarget, RuleTrigger,
};

#[test]
fn trigger_summary_is_semantic() {
    assert_eq!(
        PanelSystem::trigger_summary(RuleTrigger::OnStart),
        "OnStart"
    );
    assert_eq!(
        PanelSystem::trigger_summary(RuleTrigger::OnDamaged),
        "OnDamaged"
    );
    assert_eq!(
        PanelSystem::trigger_summary(RuleTrigger::OnDeath),
        "OnDeath"
    );
    assert_eq!(
        PanelSystem::trigger_summary(RuleTrigger::OnKey { key: RuleKey::Left }),
        "OnKey(Left)"
    );
}

#[test]
fn condition_summary_is_semantic() {
    assert_eq!(
        PanelSystem::condition_summary(RuleCondition::Always),
        "Always"
    );
    assert_eq!(
        PanelSystem::condition_summary(RuleCondition::TargetExists {
            target: RuleTarget::Player
        }),
        "TargetExists(Player)"
    );
}

#[test]
fn action_summary_is_semantic() {
    assert_eq!(
        PanelSystem::action_summary(&RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "sfx_step".to_string(),
        }),
        "PlaySound(Movement, sfx_step)"
    );
    assert_eq!(
        PanelSystem::action_summary(&RuleAction::PlayMusic {
            track_id: "bgm_forest".to_string(),
        }),
        "PlayMusic(bgm_forest)"
    );
}

#[test]
fn sanitize_grid_size_axis_clamps_to_minimum_one() {
    assert_eq!(PanelSystem::sanitize_grid_size_axis(-32), 1);
    assert_eq!(PanelSystem::sanitize_grid_size_axis(0), 1);
    assert_eq!(PanelSystem::sanitize_grid_size_axis(24), 24);
}

#[test]
fn first_grid_line_at_or_before_handles_negative_coordinates() {
    assert_eq!(PanelSystem::first_grid_line_at_or_before(0, 16), 0);
    assert_eq!(PanelSystem::first_grid_line_at_or_before(15, 16), 0);
    assert_eq!(PanelSystem::first_grid_line_at_or_before(16, 16), 16);
    assert_eq!(PanelSystem::first_grid_line_at_or_before(-1, 16), -16);
    assert_eq!(PanelSystem::first_grid_line_at_or_before(-17, 16), -32);
}

#[test]
fn grid_world_lines_emits_step_aligned_lines_inside_range() {
    assert_eq!(PanelSystem::grid_world_lines(3, 40, 16), vec![16, 32]);
    assert_eq!(PanelSystem::grid_world_lines(-20, 20, 16), vec![-16, 0, 16]);
}

#[test]
fn compute_viewport_display_rect_keeps_aspect_and_centers() {
    let outer = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(320.0, 144.0));
    let display = PanelSystem::compute_viewport_display_rect(outer, (160, 144), false);
    assert_eq!(display.width(), 160.0);
    assert_eq!(display.height(), 144.0);
    assert_eq!(display.left(), 80.0);
    assert_eq!(display.right(), 240.0);
}

#[test]
fn compute_viewport_display_rect_uses_full_rect_for_responsive_viewports() {
    let outer = egui::Rect::from_min_size(egui::Pos2::new(10.0, 20.0), egui::vec2(640.0, 360.0));
    let display = PanelSystem::compute_viewport_display_rect(outer, (640, 360), true);
    assert_eq!(display, outer);
}

#[test]
fn map_editor_tile_screen_rect_maps_tile_bounds_through_camera_scale() {
    let display = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(160.0, 144.0));
    let tile_rect = PanelSystem::map_editor_tile_screen_rect(
        display,
        (160, 144),
        glam::IVec2::ZERO,
        1.0,
        glam::UVec2::new(16, 16),
        glam::UVec2::new(1, 2),
    )
    .expect("tile screen rect should be computed");

    assert_eq!(tile_rect.min, egui::pos2(16.0, 32.0));
    assert_eq!(tile_rect.max, egui::pos2(32.0, 48.0));
}

#[test]
fn restore_graph_layout_preserves_existing_nodes_after_add_chain() {
    let rules = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "bgm_1".to_string(),
            }],
        }],
    };
    let mut graph = RuleGraph::from_rule_set(&rules);
    let initial_nodes = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();

    for (index, node_id) in initial_nodes.iter().enumerate() {
        graph
            .set_node_position(*node_id, [400.0 + index as f32 * 37.0, 250.0])
            .expect("existing node should accept custom position");
    }

    let remembered_layout = PanelSystem::remember_graph_layout(&graph);
    graph
        .add_trigger_chain()
        .expect("adding trigger chain should succeed");
    PanelSystem::restore_graph_layout(&mut graph, &remembered_layout);

    for node_id in initial_nodes {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .expect("original node should still exist");
        let node_key = graph
            .stable_node_key(node_id)
            .expect("original node should still have stable key");
        assert_eq!(
            Some(node.position),
            remembered_layout.get(&node_key).copied(),
            "layout should be preserved for existing node key {node_key}"
        );
    }
}

#[test]
fn graph_zoom_scales_node_visuals() {
    let zoomed_out = PanelSystem::graph_node_max_size(0.5);
    let zoomed_in = PanelSystem::graph_node_max_size(2.0);
    assert!(
        zoomed_out.x < zoomed_in.x && zoomed_out.y < zoomed_in.y,
        "node size should increase with zoom"
    );

    let font_out = PanelSystem::graph_node_font_size(0.5);
    let font_in = PanelSystem::graph_node_font_size(2.0);
    assert!(font_out < font_in, "font size should increase with zoom");

    let edge_out = PanelSystem::graph_edge_stroke_width(0.5);
    let edge_in = PanelSystem::graph_edge_stroke_width(2.0);
    assert!(edge_out < edge_in, "edge stroke should increase with zoom");
}

#[test]
fn enforce_graph_border_gap_moves_pan_when_left_or_top_nodes_touch_border() {
    let mut graph = RuleGraph::from_rule_set(&RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "bgm_1".to_string(),
            }],
        }],
    });
    for node in &mut graph.nodes {
        node.position[0] = 0.0;
        node.position[1] = 0.0;
    }

    let mut pan = [0.0, 0.0];
    PanelSystem::enforce_graph_border_gap(&graph, 1.0, &mut pan);

    assert!(
        pan[0] > 0.0,
        "x pan should be increased to preserve border gap"
    );
    assert!(
        pan[1] > 0.0,
        "y pan should be increased to preserve border gap"
    );
}

#[test]
fn collect_graph_validation_issues_reports_non_linear_chain_error() {
    let rules = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "bgm_1".to_string(),
            }],
        }],
    };
    let mut graph = RuleGraph::from_rule_set(&rules);
    let trigger = graph.chains[0].trigger_node_id;
    let detached = graph
        .add_action_node(RuleAction::PlayMusic {
            track_id: "bgm_2".to_string(),
        })
        .expect("detached node should be added");
    graph
        .connect_nodes(trigger, detached)
        .expect("adding a second trigger outgoing edge should be allowed in free graph");

    let badges = PanelSystem::rule_graph_node_badges(&graph);
    let issues = PanelSystem::collect_graph_validation_issues(&graph, &badges);
    let issue = issues
        .iter()
        .find(|issue| {
            issue.severity == super::GraphValidationSeverity::Error
                && issue.message.contains("multiple outgoing edges")
        })
        .expect("non-linear chain issue should be reported");
    assert!(issue.fixes.iter().any(|fix| matches!(
        &fix.command,
        GraphValidationFixCommand::DisconnectEdges(edges)
            if edges.iter().any(|(from, _)| *from == trigger)
    )));
}

#[test]
fn collect_graph_validation_issues_warns_for_detached_action_nodes() {
    let rules = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "bgm_1".to_string(),
            }],
        }],
    };
    let mut graph = RuleGraph::from_rule_set(&rules);
    let detached = graph
        .add_action_node(RuleAction::PlaySound {
            channel: RuleSoundChannel::Movement,
            sound_id: "sfx_step".to_string(),
        })
        .expect("detached action should be added");

    let badges = PanelSystem::rule_graph_node_badges(&graph);
    let detached_label = PanelSystem::rule_graph_node_label(&graph, &badges, detached)
        .expect("detached node should have a display label");
    let issues = PanelSystem::collect_graph_validation_issues(&graph, &badges);
    let issue = issues
        .iter()
        .find(|issue| {
            issue.severity == super::GraphValidationSeverity::Warning
                && issue.message.contains(&detached_label)
                && issue.message.contains("detached")
        })
        .expect("detached node warning should be reported");
    assert!(issue.fixes.iter().any(|fix| {
        matches!(fix.command, GraphValidationFixCommand::RemoveNode(node_id) if node_id == detached)
    }));
}

#[test]
fn auto_layout_uses_edge_direction_and_positions_all_nodes() {
    let rules = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "bgm_1".to_string(),
            }],
        }],
    };
    let mut graph = RuleGraph::from_rule_set(&rules);
    let detached_action = graph
        .add_action_node(RuleAction::PlayMusic {
            track_id: "bgm_2".to_string(),
        })
        .expect("detached node should be addable");
    let trigger = graph.chains[0].trigger_node_id;
    graph
        .connect_nodes(detached_action, trigger)
        .expect("detached action should be connectable to trigger");

    let node_sizes = graph
        .nodes
        .iter()
        .map(|node| (node.id, egui::vec2(180.0, 36.0)))
        .collect::<HashMap<_, _>>();
    let positions = PanelSystem::compute_auto_layout_positions_from_sizes(&graph, &node_sizes);
    assert_eq!(
        positions.len(),
        graph.nodes.len(),
        "auto-layout should position all nodes, including detached/editor-only nodes"
    );

    for edge in &graph.edges {
        let from = positions
            .get(&edge.from)
            .expect("edge source must have a position");
        let to = positions
            .get(&edge.to)
            .expect("edge target must have a position");
        assert!(
            from[0] < to[0],
            "edge direction should move left-to-right in auto-layout ({} -> {})",
            edge.from,
            edge.to
        );
    }
}

#[test]
fn auto_layout_prevents_node_overlap() {
    let rules = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 0,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "bgm_1".to_string(),
            }],
        }],
    };
    let mut graph = RuleGraph::from_rule_set(&rules);
    for i in 0..6 {
        graph
            .add_action_node(RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: format!("sfx_{i}"),
            })
            .expect("standalone action should be addable");
    }

    let mut node_sizes = HashMap::<u64, egui::Vec2>::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        let width = 140.0 + (index as f32 * 11.0);
        let height = 28.0 + ((index % 3) as f32 * 6.0);
        node_sizes.insert(node.id, egui::vec2(width, height));
    }

    let positions = PanelSystem::compute_auto_layout_positions_from_sizes(&graph, &node_sizes);
    let node_ids = graph.nodes.iter().map(|node| node.id).collect::<Vec<_>>();
    for left_index in 0..node_ids.len() {
        for right_index in (left_index + 1)..node_ids.len() {
            let left_id = node_ids[left_index];
            let right_id = node_ids[right_index];
            let left_pos = positions
                .get(&left_id)
                .expect("left node should have a computed position");
            let right_pos = positions
                .get(&right_id)
                .expect("right node should have a computed position");
            let left_size = node_sizes
                .get(&left_id)
                .expect("left node should have a known size");
            let right_size = node_sizes
                .get(&right_id)
                .expect("right node should have a known size");

            let overlaps_x =
                (left_pos[0] - right_pos[0]).abs() < (left_size.x * 0.5 + right_size.x * 0.5);
            let overlaps_y =
                (left_pos[1] - right_pos[1]).abs() < (left_size.y * 0.5 + right_size.y * 0.5);

            assert!(
                !(overlaps_x && overlaps_y),
                "auto-layout should never overlap nodes ({} and {})",
                left_id,
                right_id
            );
        }
    }
}
