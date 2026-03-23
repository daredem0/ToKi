use crate::project::ProjectAssets;
use std::collections::BTreeMap;

use super::EditorUI;
use toki_core::dialog::{DialogNode, DialogNodeKind, DialogTree};

#[derive(Debug, Clone, Default)]
pub(crate) struct DialogEditorState {
    pub selected_dialog_id: Option<String>,
    pub loaded_dialog_id: Option<String>,
    pub draft: Option<DialogTree>,
    pub selected_node_id: Option<String>,
    pub dirty: bool,
    pub status_message: Option<String>,
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
                id: dialog_id,
                title: "New Dialog".to_string(),
                entry_node_id: start_id.clone(),
                allow_cancel: true,
                gate_gameplay: true,
                nodes: vec![
                    DialogNode {
                        id: start_id,
                        speaker_name: Some("NPC".to_string()),
                        kind: DialogNodeKind::Line {
                            body: "Hello.".to_string(),
                            next_node_id: Some(end_id.clone()),
                        },
                    },
                    DialogNode {
                        id: end_id.clone(),
                        speaker_name: None,
                        kind: DialogNodeKind::End {
                            body: String::new(),
                            outcome_id: Some("done".to_string()),
                        },
                    },
                ],
            }),
            selected_node_id: Some("start".to_string()),
            dirty: true,
            status_message: Some("Created new dialog draft".to_string()),
        }
    }

    pub fn load_dialog(&mut self, dialog: DialogTree) {
        let selected_node_id = self
            .selected_node_id
            .clone()
            .filter(|node_id| dialog.nodes.iter().any(|node| node.id == *node_id))
            .or_else(|| dialog.nodes.first().map(|node| node.id.clone()));
        self.selected_dialog_id = Some(dialog.id.clone());
        self.loaded_dialog_id = Some(dialog.id.clone());
        self.draft = Some(dialog);
        self.selected_node_id = selected_node_id;
        self.dirty = false;
        self.status_message = None;
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
                (dialog.id.clone(), outcomes)
            })
            .collect()
    }
}

pub(crate) fn sync_dialog_registry(ui_state: &mut EditorUI, project_assets: &mut ProjectAssets) {
    let dialog_names = project_assets.get_dialog_names();
    if ui_state.dialog.draft.is_none()
        && ui_state.dialog.selected_dialog_id.is_none()
        && !dialog_names.is_empty()
    {
        if let Ok(Some(dialog)) = project_assets.load_dialog(&dialog_names[0]) {
            ui_state.dialog.load_dialog(dialog);
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
    }

    #[test]
    fn collect_available_dialogs_extracts_unique_outcomes() {
        let dialogs = vec![DialogTree {
            id: "intro".to_string(),
            title: String::new(),
            entry_node_id: "end".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![DialogNode {
                id: "end".to_string(),
                speaker_name: None,
                kind: DialogNodeKind::End {
                    body: String::new(),
                    outcome_id: Some("accepted".to_string()),
                },
            }],
        }];
        let available = DialogEditorState::collect_available_dialogs(&dialogs);
        assert_eq!(available.get("intro"), Some(&vec!["accepted".to_string()]));
    }
}
