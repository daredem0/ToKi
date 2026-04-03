use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GraphCanvasState {
    pub zoom: f32,
    pub pan: [f32; 2],
    pub connecting_from: Option<GraphCanvasConnectionOrigin>,
}

impl Default for GraphCanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: [16.0, 16.0],
            connecting_from: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphCanvasConnectionOrigin {
    pub node_id: String,
    pub port_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphCanvasBadge {
    pub label: String,
    pub color: Color32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GraphCanvasNodeStyle {
    pub fill: Color32,
    pub stroke: Color32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphCanvasPort {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GraphCanvasNode {
    pub id: String,
    pub kind_label: String,
    pub title: String,
    pub summary: String,
    pub position: [f32; 2],
    pub badges: Vec<GraphCanvasBadge>,
    pub style: GraphCanvasNodeStyle,
    pub output_ports: Vec<GraphCanvasPort>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GraphCanvasEdge {
    pub from_node_id: String,
    pub from_port_id: String,
    pub to_node_id: String,
    pub label: Option<String>,
    pub highlight: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GraphCanvasAction {
    SelectNode(Option<String>),
    MoveNode {
        node_id: String,
        position: [f32; 2],
    },
    CreateNodeAt([f32; 2]),
    Connect {
        from_node_id: String,
        from_port_id: String,
        to_node_id: String,
    },
}

const NODE_WIDTH: f32 = 240.0;
const BASE_NODE_HEIGHT: f32 = 86.0;
const PORT_ROW_HEIGHT: f32 = 20.0;
const PORT_HANDLE_RADIUS: f32 = 6.0;
const NODE_CORNER_RADIUS: f32 = 8.0;
const NODE_STROKE_WIDTH: f32 = 1.0;
const CANVAS_MIN_HEIGHT: f32 = 280.0;

fn graph_position_from_canvas(rect: Rect, pan: [f32; 2], scale: f32, position: Pos2) -> [f32; 2] {
    [
        (position.x - rect.left() - pan[0]) / scale,
        (position.y - rect.top() - pan[1]) / scale,
    ]
}

pub(crate) fn render_graph_canvas(
    ui: &mut egui::Ui,
    state: &mut GraphCanvasState,
    nodes: &[GraphCanvasNode],
    edges: &[GraphCanvasEdge],
    selected_node_id: Option<&str>,
) -> Vec<GraphCanvasAction> {
    let desired_size = egui::vec2(
        ui.available_width(),
        ui.available_height().max(CANVAS_MIN_HEIGHT),
    );
    let (rect, canvas_response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let mut actions = Vec::new();

    painter.rect_filled(rect, 8.0, Color32::from_rgb(20, 24, 30));
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, Color32::from_gray(65)),
        StrokeKind::Inside,
    );

    let scale = state.zoom.clamp(0.35, 3.0);
    let to_canvas = |position: [f32; 2]| -> Pos2 {
        Pos2::new(
            rect.left() + state.pan[0] + position[0] * scale,
            rect.top() + state.pan[1] + position[1] * scale,
        )
    };
    if canvas_response.hovered() {
        if !ui.ctx().wants_keyboard_input() {
            if ui.input(|input| {
                input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals)
            }) {
                state.zoom = (state.zoom * 1.1).clamp(0.35, 3.0);
            }
            if ui.input(|input| input.key_pressed(egui::Key::Minus)) {
                state.zoom = (state.zoom / 1.1).clamp(0.35, 3.0);
            }
        }
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
            state.zoom = (state.zoom * zoom_factor).clamp(0.35, 3.0);
        }
    }

    if nodes.is_empty() {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "No dialog nodes",
            egui::TextStyle::Body.resolve(ui.style()),
            Color32::from_gray(170),
        );
        if canvas_response.double_clicked() {
            if let Some(pointer_pos) = canvas_response.interact_pointer_pos() {
                actions.push(GraphCanvasAction::CreateNodeAt(graph_position_from_canvas(
                    rect,
                    state.pan,
                    scale,
                    pointer_pos,
                )));
            }
        }
        return actions;
    }

    let mut node_rects = HashMap::<String, Rect>::new();
    let mut input_handles = HashMap::<String, Rect>::new();
    let mut output_handles = HashMap::<(String, String), Rect>::new();
    let mut output_positions = HashMap::<(String, String), Pos2>::new();

    for node in nodes {
        let center = to_canvas(node.position);
        let node_size = node_canvas_size(&painter, node, scale);
        let node_rect = Rect::from_center_size(center, node_size);
        node_rects.insert(node.id.clone(), node_rect);
        input_handles.insert(
            node.id.clone(),
            Rect::from_center_size(
                Pos2::new(node_rect.left(), node_rect.center().y),
                Vec2::splat(PORT_HANDLE_RADIUS * 2.0),
            ),
        );
        for (index, port) in node.output_ports.iter().enumerate() {
            let y = node_rect.top() + ((66.0 + index as f32 * PORT_ROW_HEIGHT) * scale);
            let port_pos = Pos2::new(node_rect.right(), y);
            let handle_rect =
                Rect::from_center_size(port_pos, Vec2::splat(PORT_HANDLE_RADIUS * 2.0));
            output_handles.insert((node.id.clone(), port.id.clone()), handle_rect);
            output_positions.insert((node.id.clone(), port.id.clone()), port_pos);
        }
    }

    for edge in edges {
        let Some(start) = output_positions
            .get(&(edge.from_node_id.clone(), edge.from_port_id.clone()))
            .copied()
        else {
            continue;
        };
        let Some(end_handle) = input_handles.get(&edge.to_node_id).copied() else {
            continue;
        };
        let end = end_handle.center();
        draw_graph_edge(
            &painter,
            start,
            end,
            edge.label.as_deref(),
            edge.highlight,
            scale,
        );
    }

    if let Some(connecting_from) = &state.connecting_from {
        if let Some(start) = output_positions
            .get(&(
                connecting_from.node_id.clone(),
                connecting_from.port_id.clone(),
            ))
            .copied()
        {
            if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                draw_graph_edge(&painter, start, pointer_pos, None, true, scale);
            }
        }
    }

    let mut clicked_node = None::<String>;
    let mut moved_node = None::<(String, [f32; 2])>;
    let mut any_node_dragged = false;
    let mut any_port_dragged = false;

    for node in nodes {
        let Some(node_rect) = node_rects.get(&node.id).copied() else {
            continue;
        };
        let node_response = ui.interact(
            node_rect,
            ui.make_persistent_id(("dialog_graph_node", &node.id)),
            Sense::click_and_drag(),
        );
        if node_response.clicked() {
            clicked_node = Some(node.id.clone());
        }
        if node_response.dragged() {
            any_node_dragged = true;
            let delta = ui.ctx().input(|input| input.pointer.delta());
            if delta != Vec2::ZERO {
                moved_node = Some((
                    node.id.clone(),
                    [
                        node.position[0] + delta.x / scale,
                        node.position[1] + delta.y / scale,
                    ],
                ));
            }
        }

        let is_selected = selected_node_id == Some(node.id.as_str());
        draw_graph_node(
            &painter,
            node_rect,
            node,
            is_selected,
            node_response.dragged(),
            scale,
        );

        if let Some(input_handle) = input_handles.get(&node.id).copied() {
            painter.circle_filled(
                input_handle.center(),
                PORT_HANDLE_RADIUS,
                Color32::from_gray(210),
            );
        }

        for port in &node.output_ports {
            let Some(handle_rect) = output_handles
                .get(&(node.id.clone(), port.id.clone()))
                .copied()
            else {
                continue;
            };
            let port_response = ui.interact(
                handle_rect.expand(4.0),
                ui.make_persistent_id(("dialog_graph_port", &node.id, &port.id)),
                Sense::click_and_drag(),
            );
            if port_response.clicked() || port_response.drag_started() {
                state.connecting_from = Some(GraphCanvasConnectionOrigin {
                    node_id: node.id.clone(),
                    port_id: port.id.clone(),
                });
            }
            if port_response.dragged() {
                any_port_dragged = true;
            }
            painter.circle_filled(
                handle_rect.center(),
                PORT_HANDLE_RADIUS,
                Color32::from_rgb(230, 230, 230),
            );
            painter.text(
                Pos2::new(handle_rect.left() - 6.0, handle_rect.center().y),
                Align2::RIGHT_CENTER,
                &port.label,
                FontId::proportional((12.0 * scale).clamp(10.0, 16.0)),
                Color32::from_gray(215),
            );
        }
    }

    if let Some(connecting_from) = state.connecting_from.clone() {
        let pointer_released = ui.input(|input| input.pointer.any_released());
        if pointer_released {
            let pointer_pos = ui.ctx().pointer_latest_pos();
            let target_node_id = pointer_pos.and_then(|pointer_pos| {
                input_handles
                    .iter()
                    .find(|(node_id, handle_rect)| {
                        **node_id != connecting_from.node_id
                            && handle_rect.expand(8.0).contains(pointer_pos)
                    })
                    .map(|(node_id, _)| node_id.clone())
            });
            if let Some(target_node_id) = target_node_id {
                actions.push(GraphCanvasAction::Connect {
                    from_node_id: connecting_from.node_id,
                    from_port_id: connecting_from.port_id,
                    to_node_id: target_node_id,
                });
            }
            state.connecting_from = None;
        }
    }

    if !any_node_dragged && !any_port_dragged && canvas_response.dragged() {
        let delta = ui.ctx().input(|input| input.pointer.delta());
        if delta != Vec2::ZERO {
            state.pan[0] += delta.x;
            state.pan[1] += delta.y;
        }
    }

    if canvas_response.double_clicked() {
        if let Some(pointer_pos) = canvas_response.interact_pointer_pos() {
            let clicked_on_node = node_rects
                .values()
                .any(|node_rect| node_rect.contains(pointer_pos));
            if !clicked_on_node {
                actions.push(GraphCanvasAction::CreateNodeAt(graph_position_from_canvas(
                    rect,
                    state.pan,
                    scale,
                    pointer_pos,
                )));
            }
        }
    } else if canvas_response.clicked() && clicked_node.is_none() {
        actions.push(GraphCanvasAction::SelectNode(None));
    }

    if let Some(clicked_node) = clicked_node {
        actions.push(GraphCanvasAction::SelectNode(Some(clicked_node)));
    }
    if let Some((node_id, position)) = moved_node {
        actions.push(GraphCanvasAction::MoveNode { node_id, position });
    }

    actions
}

fn node_canvas_size(painter: &egui::Painter, node: &GraphCanvasNode, scale: f32) -> Vec2 {
    let height =
        BASE_NODE_HEIGHT + node.output_ports.len().saturating_sub(1) as f32 * PORT_ROW_HEIGHT;
    Vec2::new(
        measured_title_width(painter, &node.title, scale),
        height * scale,
    )
}

pub(crate) fn measured_title_width(painter: &egui::Painter, title: &str, scale: f32) -> f32 {
    let font_id = FontId::proportional((15.0 * scale).clamp(12.0, 19.0));
    let text_w = painter
        .layout_no_wrap(title.to_string(), font_id, Color32::WHITE)
        .size()
        .x;
    (NODE_WIDTH * scale).max(text_w + 24.0 * scale)
}

/// Returns the unscaled (scale=1) canvas size for a node with the given number of output ports.
pub(crate) fn graph_canvas_node_size(num_output_ports: usize, scale: f32) -> Vec2 {
    let height = BASE_NODE_HEIGHT + num_output_ports.saturating_sub(1) as f32 * PORT_ROW_HEIGHT;
    Vec2::new(NODE_WIDTH * scale, height * scale)
}

fn draw_graph_node(
    painter: &egui::Painter,
    node_rect: Rect,
    node: &GraphCanvasNode,
    is_selected: bool,
    is_dragged: bool,
    scale: f32,
) {
    let fill = if is_dragged {
        node.style.fill.gamma_multiply(1.12)
    } else {
        node.style.fill
    };
    let stroke = if is_selected {
        Stroke::new(
            (NODE_STROKE_WIDTH * scale).clamp(1.0, 2.5),
            Color32::from_rgb(255, 240, 180),
        )
    } else {
        Stroke::new(
            (NODE_STROKE_WIDTH * scale).clamp(1.0, 2.5),
            node.style.stroke,
        )
    };

    painter.rect_filled(node_rect, NODE_CORNER_RADIUS, fill);
    painter.rect_stroke(node_rect, NODE_CORNER_RADIUS, stroke, StrokeKind::Inside);

    let title_pos = Pos2::new(
        node_rect.left() + 12.0 * scale,
        node_rect.top() + 12.0 * scale,
    );
    painter.text(
        title_pos,
        Align2::LEFT_TOP,
        &node.title,
        FontId::proportional((15.0 * scale).clamp(12.0, 19.0)),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(
            node_rect.left() + 12.0 * scale,
            node_rect.top() + 32.0 * scale,
        ),
        Align2::LEFT_TOP,
        format!("[{}]", node.kind_label),
        FontId::proportional((11.0 * scale).clamp(10.0, 14.0)),
        Color32::from_rgb(206, 214, 224),
    );
    painter.text(
        Pos2::new(
            node_rect.left() + 12.0 * scale,
            node_rect.top() + 48.0 * scale,
        ),
        Align2::LEFT_TOP,
        &node.summary,
        FontId::proportional((12.0 * scale).clamp(10.0, 15.0)),
        Color32::from_gray(218),
    );

    let mut badge_x = node_rect.right() - 12.0 * scale;
    for badge in node.badges.iter().rev() {
        let badge_width = ((badge.label.len() as f32 * 7.0) + 18.0) * scale;
        let badge_rect = Rect::from_min_size(
            Pos2::new(badge_x - badge_width, node_rect.top() + 30.0 * scale),
            Vec2::new(badge_width, 16.0 * scale),
        );
        painter.rect_filled(badge_rect, 8.0, badge.color);
        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            &badge.label,
            FontId::proportional((10.0 * scale).clamp(9.0, 13.0)),
            Color32::from_rgb(15, 18, 24),
        );
        badge_x = badge_rect.left() - 6.0 * scale;
    }
}

fn draw_graph_edge(
    painter: &egui::Painter,
    start: Pos2,
    end: Pos2,
    label: Option<&str>,
    highlight: bool,
    scale: f32,
) {
    let stroke = Stroke::new(
        (1.6 * scale).clamp(1.0, 2.8),
        if highlight {
            Color32::from_rgb(255, 211, 94)
        } else {
            Color32::from_rgb(126, 156, 210)
        },
    );
    let control_offset = ((end.x - start.x).abs() * 0.35).max(40.0 * scale);
    let cp1 = Pos2::new(start.x + control_offset, start.y);
    let cp2 = Pos2::new(end.x - control_offset, end.y);
    let points = cubic_bezier_points(start, cp1, cp2, end, 18);
    painter.add(egui::Shape::line(points.clone(), stroke));
    if let Some(label) = label.filter(|label| !label.trim().is_empty()) {
        let mid = points[points.len() / 2];
        painter.text(
            Pos2::new(mid.x, mid.y - 8.0 * scale),
            Align2::CENTER_BOTTOM,
            label,
            FontId::proportional((11.0 * scale).clamp(9.0, 14.0)),
            Color32::from_gray(220),
        );
    }
}

fn cubic_bezier_points(start: Pos2, cp1: Pos2, cp2: Pos2, end: Pos2, steps: usize) -> Vec<Pos2> {
    (0..=steps)
        .map(|index| {
            let t = index as f32 / steps as f32;
            let inv = 1.0 - t;
            let x = inv.powi(3) * start.x
                + 3.0 * inv.powi(2) * t * cp1.x
                + 3.0 * inv * t.powi(2) * cp2.x
                + t.powi(3) * end.x;
            let y = inv.powi(3) * start.y
                + 3.0 * inv.powi(2) * t * cp1.y
                + 3.0 * inv * t.powi(2) * cp2.y
                + t.powi(3) * end.y;
            Pos2::new(x, y)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubic_bezier_points_include_start_and_end() {
        let points = cubic_bezier_points(
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
            Pos2::new(20.0, 10.0),
            Pos2::new(30.0, 10.0),
            8,
        );
        assert_eq!(points.first().copied(), Some(Pos2::new(0.0, 0.0)));
        assert_eq!(points.last().copied(), Some(Pos2::new(30.0, 10.0)));
    }
}
