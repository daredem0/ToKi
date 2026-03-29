use crate::project::{DialogGraphLayout, ProjectAssets};
use crate::ui::graph_canvas::GraphCanvasState;
use std::collections::{BTreeMap, HashMap};

use super::EditorUI;
use toki_core::dialog::{DialogNode, DialogNodeKind, DialogTree};

#[derive(Debug, Clone, Default)]
pub(crate) struct DialogEditorState {
    pub selected_dialog_id: Option<String>,
    pub loaded_dialog_id: Option<String>,
    pub draft: Option<DialogTree>,
    pub selected_node_id: Option<String>,
    pub node_id_edit_target: Option<String>,
    pub node_id_edit_value: String,
    pub dirty: bool,
    pub status_message: Option<String>,
    pub graph_canvas: GraphCanvasState,
    pub layouts_by_dialog: HashMap<String, DialogGraphLayout>,
    pub layout_dirty: bool,
}

impl DialogEditorState {
    pub fn new_dialog(existing_dialog_ids: &[String]) -> Self {
        let dialog_id = unique_dialog_id(existing_dialog_ids);
        let start_id = "start".to_string();
        let end_id = "end".to_string();
        Self {
            selected_dialog_id: Some(dialog_id.clone()),
            loaded_dialog_id: None,
            draft: Some(DialogTree {
                id: dialog_id.into(),
                title: "New Dialog".to_string(),
                entry_node_id: start_id.clone(),
                allow_cancel: true,
                gate_gameplay: true,
                nodes: vec![
                    DialogNode {
                        id: start_id,
                        speaker_name: Some("NPC".to_string()),
                        conditions: Vec::new(),
                        kind: DialogNodeKind::Line {
                            body: "Hello.".to_string(),
                            next_node_id: Some(end_id.clone()),
                        },
                    },
                    DialogNode {
                        id: end_id.clone(),
                        speaker_name: None,
                        conditions: Vec::new(),
                        kind: DialogNodeKind::End {
                            body: String::new(),
                            outcome_id: Some("done".to_string()),
                        },
                    },
                ],
            }),
            selected_node_id: Some("start".to_string()),
            node_id_edit_target: Some("start".to_string()),
            node_id_edit_value: "start".to_string(),
            dirty: true,
            status_message: Some("Created new dialog draft".to_string()),
            graph_canvas: GraphCanvasState::default(),
            layouts_by_dialog: HashMap::new(),
            layout_dirty: false,
        }
    }

    pub fn load_dialog(&mut self, dialog: DialogTree) {
        let dialog_id = dialog.id.to_string();
        let selected_node_id = self
            .selected_node_id
            .clone()
            .filter(|node_id| dialog.nodes.iter().any(|node| node.id == *node_id))
            .or_else(|| dialog.nodes.first().map(|node| node.id.clone()));
        self.selected_dialog_id = Some(dialog_id.clone());
        self.loaded_dialog_id = Some(dialog_id.clone());
        self.draft = Some(dialog);
        self.selected_node_id = selected_node_id.clone();
        self.node_id_edit_target = selected_node_id.clone();
        self.node_id_edit_value = selected_node_id.unwrap_or_default();
        self.dirty = false;
        self.status_message = None;
        let layout = self
            .layouts_by_dialog
            .get(&dialog_id)
            .cloned()
            .unwrap_or_default();
        self.graph_canvas.zoom = layout.zoom;
        self.graph_canvas.pan = layout.pan;
        self.graph_canvas.connecting_from = None;
    }

    pub fn select_dialog_node(&mut self, node_id: String) {
        self.selected_node_id = Some(node_id.clone());
        self.node_id_edit_target = Some(node_id.clone());
        self.node_id_edit_value = node_id;
    }

    pub fn sync_node_id_editor(&mut self, selected_node_id: Option<&str>) {
        if self.node_id_edit_target.as_deref() != selected_node_id {
            self.node_id_edit_target = selected_node_id.map(str::to_string);
            self.node_id_edit_value = selected_node_id.unwrap_or_default().to_string();
        }
    }

    pub fn collect_available_dialogs(dialogs: &[DialogTree]) -> BTreeMap<String, Vec<String>> {
        dialogs
            .iter()
            .map(|dialog| {
                let mut outcomes = dialog
                    .nodes
                    .iter()
                    .filter_map(|node| match &node.kind {
                        DialogNodeKind::End { outcome_id, .. } => outcome_id.clone(),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                outcomes.sort();
                outcomes.dedup();
                (dialog.id.to_string(), outcomes)
            })
            .collect()
    }

    pub fn ensure_layout_for_dialog(&mut self, dialog_id: &str) -> &mut DialogGraphLayout {
        self.layouts_by_dialog
            .entry(dialog_id.to_string())
            .or_default()
    }

    pub fn sync_active_graph_view_from_layout(&mut self) {
        let Some(dialog_id) = self.selected_dialog_id.clone() else {
            self.graph_canvas = GraphCanvasState::default();
            return;
        };
        let layout = self
            .layouts_by_dialog
            .get(&dialog_id)
            .cloned()
            .unwrap_or_default();
        self.graph_canvas.zoom = layout.zoom;
        self.graph_canvas.pan = layout.pan;
        self.graph_canvas.connecting_from = None;
    }

    pub fn persist_active_graph_view_into_layout(&mut self) {
        let Some(dialog_id) = self.selected_dialog_id.clone() else {
            return;
        };
        let layout = self.layouts_by_dialog.entry(dialog_id).or_default();
        let mut changed = false;
        if (layout.zoom - self.graph_canvas.zoom).abs() > f32::EPSILON {
            layout.zoom = self.graph_canvas.zoom;
            changed = true;
        }
        if layout.pan != self.graph_canvas.pan {
            layout.pan = self.graph_canvas.pan;
            changed = true;
        }
        if changed {
            self.layout_dirty = true;
        }
    }
}

pub(crate) fn sync_dialog_registry(ui_state: &mut EditorUI, project_assets: &mut ProjectAssets) {
    let dialog_names = project_assets.get_dialog_names();
    if crate::ui::editor_context::dialog_state(ui_state)
        .draft
        .is_none()
        && crate::ui::editor_context::dialog_state(ui_state)
            .selected_dialog_id
            .is_none()
        && !dialog_names.is_empty()
    {
        if let Ok(Some(dialog)) = project_assets.load_dialog(&dialog_names[0]) {
            crate::ui::editor_context::dialog_state_mut(ui_state).load_dialog(dialog);
        }
    }

    let dialogs = dialog_names
        .iter()
        .filter_map(|dialog_id| project_assets.load_dialog(dialog_id).ok().flatten())
        .collect::<Vec<_>>();
    ui_state
        .project
        .set_available_dialogs(&DialogEditorState::collect_available_dialogs(&dialogs));
}

pub(crate) fn load_dialog_graph_layouts_from_project(
    ui_state: &mut EditorUI,
    layouts: &HashMap<String, DialogGraphLayout>,
) {
    let dialog_state = crate::ui::editor_context::dialog_state_mut(ui_state);
    dialog_state.layouts_by_dialog = layouts.clone();
    dialog_state.layout_dirty = false;
    dialog_state.sync_active_graph_view_from_layout();
}

pub(crate) fn export_dialog_graph_layouts_for_project(
    ui_state: &EditorUI,
) -> HashMap<String, DialogGraphLayout> {
    crate::ui::editor_context::dialog_state(ui_state)
        .layouts_by_dialog
        .clone()
}

pub(crate) fn is_dialog_graph_layout_dirty(ui_state: &EditorUI) -> bool {
    crate::ui::editor_context::dialog_state(ui_state).layout_dirty
}

pub(crate) fn clear_dialog_graph_layout_dirty(ui_state: &mut EditorUI) {
    crate::ui::editor_context::dialog_state_mut(ui_state).layout_dirty = false;
}

fn unique_dialog_id(existing_dialog_ids: &[String]) -> String {
    let mut index = 1usize;
    loop {
        let candidate = format!("dialog_{index}");
        if !existing_dialog_ids
            .iter()
            .any(|existing| existing == &candidate)
        {
            return candidate;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_dialog_creates_valid_default_tree() {
        let state = DialogEditorState::new_dialog(&[]);
        let dialog = state.draft.as_ref().expect("draft");
        assert!(dialog.validate().is_valid());
        assert_eq!(state.selected_node_id.as_deref(), Some("start"));
        assert_eq!(state.node_id_edit_target.as_deref(), Some("start"));
        assert_eq!(state.node_id_edit_value, "start");
    }

    #[test]
    fn collect_available_dialogs_extracts_unique_outcomes() {
        let dialogs = vec![DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "end".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![DialogNode {
                id: "end".to_string(),
                speaker_name: None,
                conditions: Vec::new(),
                kind: DialogNodeKind::End {
                    body: String::new(),
                    outcome_id: Some("accepted".to_string()),
                },
            }],
        }];
        let available = DialogEditorState::collect_available_dialogs(&dialogs);
        assert_eq!(available.get("intro"), Some(&vec!["accepted".to_string()]));
    }

    #[test]
    fn sync_node_id_editor_tracks_selected_node() {
        let mut state = DialogEditorState::default();

        state.sync_node_id_editor(Some("start"));
        assert_eq!(state.node_id_edit_target.as_deref(), Some("start"));
        assert_eq!(state.node_id_edit_value, "start");

        state.node_id_edit_value = "draft_value".to_string();
        state.sync_node_id_editor(Some("start"));
        assert_eq!(state.node_id_edit_value, "draft_value");

        state.sync_node_id_editor(Some("end"));
        assert_eq!(state.node_id_edit_target.as_deref(), Some("end"));
        assert_eq!(state.node_id_edit_value, "end");
    }
}
