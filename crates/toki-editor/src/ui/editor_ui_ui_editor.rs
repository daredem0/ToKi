use crate::project::{ProjectAssets, UiEditorLayoutState};
use std::collections::HashMap;

use super::EditorUI;
use toki_core::ui_layout::{
    UiAnchor, UiLayoutAsset, UiLayoutSpec, UiSpacing, UiTextSegment, UiTextTemplate, UiWidgetKind,
    UiWidgetNode, UiWidgetStyle,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UiCanvasInteraction {
    Pan,
    MoveWidget {
        widget_id: String,
        press_origin: [f32; 2],
        start_offset: [f32; 2],
    },
    ResizeWidget {
        widget_id: String,
        press_origin: [f32; 2],
        start_size: [f32; 2],
    },
}

#[derive(Debug, Clone, Default)]
pub(crate) struct UiEditorState {
    pub selected_layout_id: Option<String>,
    pub loaded_layout_id: Option<String>,
    pub draft: Option<UiLayoutAsset>,
    pub selected_widget_id: Option<String>,
    pub dirty: bool,
    pub status_message: Option<String>,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub canvas_interaction: Option<UiCanvasInteraction>,
    pub views_by_layout: HashMap<String, UiEditorLayoutState>,
    pub view_dirty: bool,
}

impl UiEditorState {
    pub fn new_layout(existing_layout_ids: &[String]) -> Self {
        let layout_id = unique_ui_layout_id(existing_layout_ids);
        let root_id = "root".to_string();
        let label_id = "label_1".to_string();
        let viewport_size = [
            toki_core::project_runtime::default_resolution_width() as f32,
            toki_core::project_runtime::default_resolution_height() as f32,
        ];
        let root = UiWidgetNode {
            id: root_id.clone().into(),
            title: "Viewport Root".to_string(),
            layout: UiLayoutSpec {
                anchor: UiAnchor::Stretch,
                size: viewport_size,
                ..UiLayoutSpec::default()
            },
            style: UiWidgetStyle::default(),
            event_id: None,
            focusable: false,
            visible_if: None,
            enabled_if: None,
            kind: UiWidgetKind::GridContainer {
                columns: 1,
                spacing: UiSpacing::default(),
            },
            children: vec![UiWidgetNode {
                id: label_id.clone().into(),
                title: "Title".to_string(),
                layout: UiLayoutSpec {
                    anchor: UiAnchor::TopLeft,
                    offset: [8.0, 8.0],
                    size: [96.0, 18.0],
                    ..UiLayoutSpec::default()
                },
                style: UiWidgetStyle {
                    typography: toki_core::ui_layout::UiTypography {
                        font_size_px: Some(8),
                        ..Default::default()
                    },
                    ..UiWidgetStyle::default()
                },
                event_id: None,
                focusable: false,
                visible_if: None,
                enabled_if: None,
                kind: UiWidgetKind::Label {
                    content: UiTextTemplate {
                        segments: vec![UiTextSegment::Literal {
                            text: "HUD".to_string(),
                        }],
                    },
                },
                children: Vec::new(),
            }],
        };
        let mut state = Self {
            selected_layout_id: Some(layout_id.clone()),
            loaded_layout_id: None,
            draft: Some(UiLayoutAsset {
                id: layout_id.into(),
                title: "New UI Layout".to_string(),
                startup_visible: false,
                z_order: 0,
                root,
            }),
            selected_widget_id: Some(label_id),
            dirty: true,
            status_message: Some("Created new UI layout draft".to_string()),
            zoom: 1.0,
            pan: [12.0, 12.0],
            canvas_interaction: None,
            views_by_layout: HashMap::new(),
            view_dirty: false,
        };
        state.persist_active_view_into_layout();
        state
    }

    pub fn load_layout(&mut self, layout: UiLayoutAsset) {
        let layout_id = layout.id.to_string();
        let selected_widget_id = self
            .selected_widget_id
            .clone()
            .filter(|widget_id| layout_contains_widget(&layout.root, widget_id))
            .or_else(|| first_selectable_widget_id(&layout.root));
        self.selected_layout_id = Some(layout_id.clone());
        self.loaded_layout_id = Some(layout_id.clone());
        self.draft = Some(layout);
        self.selected_widget_id = selected_widget_id;
        self.dirty = false;
        self.status_message = None;
        let view = self
            .views_by_layout
            .get(&layout_id)
            .cloned()
            .unwrap_or_default();
        self.zoom = view.zoom;
        self.pan = view.pan;
        self.canvas_interaction = None;
    }

    pub fn select_widget(&mut self, widget_id: String) {
        self.selected_widget_id = Some(widget_id.clone());
        if let Some(layout_id) = self.selected_layout_id.clone() {
            self.views_by_layout
                .entry(layout_id)
                .or_default()
                .selected_widget_id = Some(widget_id);
            self.view_dirty = true;
        }
    }

    pub fn sync_active_view_from_layout(&mut self) {
        let Some(layout_id) = self.selected_layout_id.clone() else {
            self.zoom = 1.0;
            self.pan = [12.0, 12.0];
            return;
        };
        let view = self
            .views_by_layout
            .get(&layout_id)
            .cloned()
            .unwrap_or_default();
        self.zoom = view.zoom;
        self.pan = view.pan;
        if view.selected_widget_id.is_some() {
            self.selected_widget_id = view.selected_widget_id;
        }
    }

    pub fn persist_active_view_into_layout(&mut self) {
        let Some(layout_id) = self.selected_layout_id.clone() else {
            return;
        };
        let view = self.views_by_layout.entry(layout_id).or_default();
        let mut changed = false;
        if (view.zoom - self.zoom).abs() > f32::EPSILON {
            view.zoom = self.zoom;
            changed = true;
        }
        if view.pan != self.pan {
            view.pan = self.pan;
            changed = true;
        }
        if view.selected_widget_id != self.selected_widget_id {
            view.selected_widget_id = self.selected_widget_id.clone();
            changed = true;
        }
        if changed {
            self.view_dirty = true;
        }
    }

    #[cfg(test)]
    pub fn begin_move_widget(&mut self, widget_id: String) {
        self.select_widget(widget_id.clone());
        self.canvas_interaction = Some(UiCanvasInteraction::MoveWidget {
            widget_id,
            press_origin: [0.0, 0.0],
            start_offset: [0.0, 0.0],
        });
    }

    #[cfg(test)]
    pub fn begin_resize_widget(&mut self, widget_id: String) {
        self.select_widget(widget_id.clone());
        self.canvas_interaction = Some(UiCanvasInteraction::ResizeWidget {
            widget_id,
            press_origin: [0.0, 0.0],
            start_size: [0.0, 0.0],
        });
    }
}

pub(crate) fn sync_ui_layout_registry(ui_state: &mut EditorUI, project_assets: &mut ProjectAssets) {
    let layout_names = project_assets.get_ui_layout_names();
    let ui_state_ref = &mut crate::ui::editor_context::ui_editor_state_mut(ui_state).ui;
    if ui_state_ref.draft.is_none()
        && ui_state_ref.selected_layout_id.is_none()
        && !layout_names.is_empty()
    {
        if let Ok(Some(layout)) = project_assets.load_ui_layout(&layout_names[0]) {
            ui_state_ref.load_layout(layout);
        }
    }
}

pub(crate) fn load_ui_layout_views_from_project(
    ui_state: &mut EditorUI,
    views: &HashMap<String, UiEditorLayoutState>,
) {
    let editor_state = &mut crate::ui::editor_context::ui_editor_state_mut(ui_state).ui;
    editor_state.views_by_layout = views.clone();
    editor_state.view_dirty = false;
    editor_state.sync_active_view_from_layout();
}

pub(crate) fn export_ui_layout_views_for_project(
    ui_state: &EditorUI,
) -> HashMap<String, UiEditorLayoutState> {
    crate::ui::editor_context::ui_editor_state(ui_state)
        .ui
        .views_by_layout
        .clone()
}

pub(crate) fn is_ui_layout_view_dirty(ui_state: &EditorUI) -> bool {
    crate::ui::editor_context::ui_editor_state(ui_state)
        .ui
        .view_dirty
}

pub(crate) fn clear_ui_layout_view_dirty(ui_state: &mut EditorUI) {
    crate::ui::editor_context::ui_editor_state_mut(ui_state)
        .ui
        .view_dirty = false;
}

fn unique_ui_layout_id(existing_layout_ids: &[String]) -> String {
    let mut index = 1usize;
    loop {
        let candidate = format!("ui_{index}");
        if !existing_layout_ids.iter().any(|id| id == &candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn layout_contains_widget(widget: &UiWidgetNode, widget_id: &str) -> bool {
    widget.id.as_str() == widget_id
        || widget
            .children
            .iter()
            .any(|child| layout_contains_widget(child, widget_id))
}

fn first_selectable_widget_id(widget: &UiWidgetNode) -> Option<String> {
    if widget.id.as_str() != "root" {
        return Some(widget.id.to_string());
    }
    widget
        .children
        .iter()
        .find_map(first_selectable_widget_id)
        .or_else(|| Some(widget.id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_layout_creates_default_draft_and_selection() {
        let state = UiEditorState::new_layout(&[]);
        let layout = state.draft.expect("draft should exist");
        assert_eq!(layout.id.as_str(), "ui_1");
        assert_eq!(layout.root.id.as_str(), "root");
        assert_eq!(layout.root.title, "Viewport Root");
        assert_eq!(layout.root.children.len(), 1);
        assert_eq!(layout.root.children[0].layout.offset, [8.0, 8.0]);
        assert_eq!(layout.root.children[0].layout.size, [96.0, 18.0]);
        assert_eq!(state.selected_widget_id.as_deref(), Some("label_1"));
        assert!(state.dirty);
    }

    #[test]
    fn load_layout_applies_persisted_view_state() {
        let mut state = UiEditorState::default();
        state.views_by_layout.insert(
            "hud".to_string(),
            UiEditorLayoutState {
                zoom: 1.5,
                pan: [48.0, 24.0],
                selected_widget_id: Some("label_1".to_string()),
            },
        );
        state.load_layout(UiLayoutAsset {
            id: "hud".into(),
            title: "HUD".to_string(),
            startup_visible: true,
            z_order: 0,
            root: UiWidgetNode {
                children: vec![UiWidgetNode {
                    id: "label_1".into(),
                    ..UiWidgetNode::default()
                }],
                ..UiWidgetNode::default()
            },
        });

        assert_eq!(state.zoom, 1.5);
        assert_eq!(state.pan, [48.0, 24.0]);
        assert_eq!(state.selected_widget_id.as_deref(), Some("label_1"));
    }

    #[test]
    fn begin_move_widget_selects_it_for_inspector() {
        let mut state = UiEditorState::default();

        state.begin_move_widget("progress_1".to_string());

        assert_eq!(state.selected_widget_id.as_deref(), Some("progress_1"));
        assert!(matches!(
            state.canvas_interaction,
            Some(UiCanvasInteraction::MoveWidget { ref widget_id, .. })
                if widget_id == "progress_1"
        ));
    }

    #[test]
    fn begin_resize_widget_selects_it_for_inspector() {
        let mut state = UiEditorState::default();

        state.begin_resize_widget("label_2".to_string());

        assert_eq!(state.selected_widget_id.as_deref(), Some("label_2"));
        assert!(matches!(
            state.canvas_interaction,
            Some(UiCanvasInteraction::ResizeWidget { ref widget_id, .. })
                if widget_id == "label_2"
        ));
    }
}
