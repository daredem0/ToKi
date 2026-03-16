use super::*;

impl PanelSystem {
    pub(super) fn render_graph_canvas(
        ui: &mut egui::Ui,
        graph: &RuleGraph,
        node_badges: &HashMap<u64, String>,
        graph_zoom: f32,
        graph_pan: &mut [f32; 2],
    ) -> (Option<(u64, [f32; 2])>, Option<u64>) {
        let desired_size = egui::vec2(ui.available_width(), ui.available_height().max(220.0));
        let (rect, canvas_response) =
            ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 6.0, egui::Color32::from_rgb(20, 24, 30));
        painter.rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
            egui::StrokeKind::Inside,
        );

        if graph.nodes.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No Graph Nodes",
                egui::TextStyle::Body.resolve(ui.style()),
                egui::Color32::from_gray(170),
            );
            return (None, None);
        }

        let scale = graph_zoom.max(0.01);

        let to_canvas = |position: [f32; 2]| -> egui::Pos2 {
            egui::pos2(
                rect.left() + graph_pan[0] + position[0] * scale,
                rect.top() + graph_pan[1] + position[1] * scale,
            )
        };

        let mut node_positions = HashMap::<u64, egui::Pos2>::new();
        let mut node_labels = HashMap::<u64, String>::new();
        let mut node_rects = HashMap::<u64, egui::Rect>::new();
        for node in &graph.nodes {
            let center = to_canvas(node.position);
            let badge = node_badges
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| "?".to_string());
            let label = format!(
                "{}: {}",
                badge,
                Self::rule_graph_node_kind_compact_label(&node.kind)
            );
            let node_size = Self::graph_node_size_for_label(ui, &label, scale);
            let node_rect = egui::Rect::from_center_size(center, node_size);
            node_positions.insert(node.id, center);
            node_labels.insert(node.id, label);
            node_rects.insert(node.id, node_rect);
        }

        for edge in &graph.edges {
            let Some(from_rect) = node_rects.get(&edge.from).copied() else {
                continue;
            };
            let Some(to_rect) = node_rects.get(&edge.to).copied() else {
                continue;
            };
            let Some((start, end, direction)) = Self::graph_edge_points(from_rect, to_rect) else {
                continue;
            };
            let stroke = egui::Stroke::new(
                Self::graph_edge_stroke_width(scale),
                egui::Color32::from_rgb(130, 150, 185),
            );
            let arrow_length = (10.0 * scale).clamp(6.0, 18.0);
            let arrow_width = (5.0 * scale).clamp(3.0, 12.0);
            let body_end = end - direction * (arrow_length * 0.85);
            painter.line_segment([start, body_end], stroke);
            let perp = egui::vec2(-direction.y, direction.x);
            let arrow_left = end - direction * arrow_length + perp * arrow_width;
            let arrow_right = end - direction * arrow_length - perp * arrow_width;
            painter.line_segment([end, arrow_left], stroke);
            painter.line_segment([end, arrow_right], stroke);
        }

        let mut moved_node = None;
        let mut clicked_node = None;
        let mut any_node_dragged = false;
        let node_corner_radius = (6.0 * scale).clamp(2.0, 18.0);
        let node_stroke_width = (1.0 * scale).clamp(0.7, 2.5);
        let node_font_size = Self::graph_node_font_size(scale);
        for node in &graph.nodes {
            let Some(center) = node_positions.get(&node.id).copied() else {
                continue;
            };
            let Some(label) = node_labels.get(&node.id).cloned() else {
                continue;
            };
            let node_size = Self::graph_node_size_for_label(ui, &label, scale);
            let (fill, stroke) = match node.kind {
                RuleGraphNodeKind::Trigger(_) => (
                    egui::Color32::from_rgb(45, 122, 199),
                    egui::Color32::from_rgb(140, 190, 245),
                ),
                RuleGraphNodeKind::Condition(_) => (
                    egui::Color32::from_rgb(139, 92, 46),
                    egui::Color32::from_rgb(214, 158, 106),
                ),
                RuleGraphNodeKind::Action(_) => (
                    egui::Color32::from_rgb(58, 140, 82),
                    egui::Color32::from_rgb(133, 208, 154),
                ),
            };
            let node_rect = egui::Rect::from_center_size(center, node_size);
            let response = ui.interact(
                node_rect,
                ui.make_persistent_id(("graph_canvas_node", node.id)),
                egui::Sense::click_and_drag(),
            );
            if response.clicked() {
                clicked_node = Some(node.id);
            }
            if response.dragged() {
                any_node_dragged = true;
                let delta = ui.ctx().input(|input| input.pointer.delta());
                if delta != egui::Vec2::ZERO && scale > 0.0 {
                    moved_node = Some((
                        node.id,
                        [
                            node.position[0] + (delta.x / scale),
                            node.position[1] + (delta.y / scale),
                        ],
                    ));
                }
            }
            let draw_fill = if response.dragged() {
                fill.gamma_multiply(1.2)
            } else {
                fill
            };
            painter.rect_filled(node_rect, node_corner_radius, draw_fill);
            painter.rect_stroke(
                node_rect,
                node_corner_radius,
                egui::Stroke::new(node_stroke_width, stroke),
                egui::StrokeKind::Inside,
            );
            painter.text(
                node_rect.center(),
                egui::Align2::CENTER_CENTER,
                &label,
                egui::FontId::proportional(node_font_size),
                egui::Color32::WHITE,
            );
        }

        if !any_node_dragged && canvas_response.dragged() {
            let delta = ui.ctx().input(|input| input.pointer.delta());
            if delta != egui::Vec2::ZERO {
                graph_pan[0] += delta.x;
                graph_pan[1] += delta.y;
            }
        }

        (moved_node, clicked_node)
    }
}
