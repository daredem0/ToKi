use std::collections::HashMap;

use crate::dialog::{
    DialogCondition, DialogConditionTarget, DialogNodeKind, DialogRuntimeContext, DialogTree,
};
use crate::entity::HEALTH_STAT_ID;
use crate::menu::{MenuDialogView, MenuInput, MenuViewEntry};
use crate::GameState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogCompletion {
    pub dialog_id: String,
    pub outcome_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAdvanceResult {
    None,
    Closed(DialogCompletion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogStartError {
    MissingDialog(String),
    InvalidDialog(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveDialogState {
    dialog_id: String,
    current_node_id: String,
    selected_index: usize,
    context: DialogRuntimeContext,
}

#[derive(Debug, Default)]
pub struct DialogController {
    dialogs: HashMap<String, DialogTree>,
    active: Option<ActiveDialogState>,
}

impl DialogController {
    pub fn new(dialogs: Vec<DialogTree>) -> Self {
        let mut controller = Self::default();
        controller.set_dialogs(dialogs);
        controller
    }

    pub fn set_dialogs(&mut self, dialogs: Vec<DialogTree>) {
        self.dialogs = dialogs
            .into_iter()
            .map(|dialog| (dialog.id.clone(), dialog))
            .collect();
        if let Some(active) = &self.active {
            if !self.dialogs.contains_key(&active.dialog_id) {
                self.active = None;
            }
        }
    }

    pub fn is_open(&self) -> bool {
        self.active.is_some()
    }

    pub fn active_dialog_id(&self) -> Option<&str> {
        self.active.as_ref().map(|active| active.dialog_id.as_str())
    }

    pub fn active_dialog_gates_gameplay(&self) -> bool {
        self.active
            .as_ref()
            .and_then(|active| self.dialogs.get(&active.dialog_id))
            .is_some_and(|dialog| dialog.gate_gameplay)
    }

    pub fn start_dialog(
        &mut self,
        game_state: &GameState,
        dialog_id: &str,
        context: DialogRuntimeContext,
    ) -> Result<(), DialogStartError> {
        let Some(dialog) = self.dialogs.get(dialog_id) else {
            return Err(DialogStartError::MissingDialog(dialog_id.to_string()));
        };
        let report = dialog.validate();
        if !report.is_valid() {
            return Err(DialogStartError::InvalidDialog(report.errors.join("; ")));
        }

        self.active = Some(ActiveDialogState {
            dialog_id: dialog_id.to_string(),
            current_node_id: dialog.entry_node_id.clone(),
            selected_index: 0,
            context,
        });
        self.resolve_branches(game_state);
        Ok(())
    }

    pub fn close(&mut self) {
        self.active = None;
    }

    pub fn current_view(&self) -> Option<MenuDialogView> {
        let active = self.active.as_ref()?;
        let dialog = self.dialogs.get(&active.dialog_id)?;
        let node = dialog.node(&active.current_node_id)?;
        let body = match &node.kind {
            DialogNodeKind::Line { body, .. }
            | DialogNodeKind::Choice { body, .. }
            | DialogNodeKind::End { body, .. } => body.clone(),
            DialogNodeKind::Branch { .. } => String::new(),
        };
        let title = node
            .speaker_name
            .clone()
            .filter(|speaker| !speaker.trim().is_empty())
            .unwrap_or_else(|| dialog.title.clone());
        let entries = match &node.kind {
            DialogNodeKind::Line { next_node_id, .. } => line_entries(
                dialog.allow_cancel,
                next_node_id.is_some(),
                active.selected_index,
            ),
            DialogNodeKind::Choice { choices, .. } => {
                let choice_entries =
                    choices
                        .iter()
                        .enumerate()
                        .map(|(index, choice)| MenuViewEntry {
                            text: choice.label.clone(),
                            selected: index == active.selected_index,
                            selectable: true,
                            border_style_override: None,
                        });
                if dialog.allow_cancel {
                    choice_entries
                        .chain(std::iter::once(MenuViewEntry {
                            text: "Cancel".to_string(),
                            selected: active.selected_index == choices.len(),
                            selectable: true,
                            border_style_override: None,
                        }))
                        .collect()
                } else {
                    choice_entries.collect()
                }
            }
            DialogNodeKind::End { .. } => {
                line_entries(dialog.allow_cancel, false, active.selected_index)
            }
            DialogNodeKind::Branch { .. } => Vec::new(),
        };

        Some(MenuDialogView {
            dialog_id: dialog.id.clone(),
            title,
            body,
            entries,
            hide_main_menu: true,
        })
    }

    pub fn handle_input(
        &mut self,
        input: MenuInput,
        game_state: &GameState,
    ) -> DialogAdvanceResult {
        let Some(active) = self.active.as_mut() else {
            return DialogAdvanceResult::None;
        };
        let Some(dialog) = self.dialogs.get(&active.dialog_id) else {
            self.active = None;
            return DialogAdvanceResult::None;
        };
        let Some(node) = dialog.node(&active.current_node_id) else {
            self.active = None;
            return DialogAdvanceResult::None;
        };

        match &node.kind {
            DialogNodeKind::Choice { choices, .. } => match input {
                MenuInput::Up | MenuInput::Left => {
                    let total = choices.len() + usize::from(dialog.allow_cancel);
                    if total > 0 {
                        active.selected_index = (active.selected_index + total - 1) % total;
                    }
                    DialogAdvanceResult::None
                }
                MenuInput::Down | MenuInput::Right => {
                    let total = choices.len() + usize::from(dialog.allow_cancel);
                    if total > 0 {
                        active.selected_index = (active.selected_index + 1) % total;
                    }
                    DialogAdvanceResult::None
                }
                MenuInput::Confirm => {
                    if active.selected_index < choices.len() {
                        let next = choices[active.selected_index].next_node_id.clone();
                        active.current_node_id = next;
                        active.selected_index = 0;
                        self.resolve_branches(game_state);
                        DialogAdvanceResult::None
                    } else {
                        let completion = DialogCompletion {
                            dialog_id: active.dialog_id.clone(),
                            outcome_id: None,
                        };
                        self.active = None;
                        DialogAdvanceResult::Closed(completion)
                    }
                }
                MenuInput::Back => {
                    if dialog.allow_cancel {
                        let completion = DialogCompletion {
                            dialog_id: active.dialog_id.clone(),
                            outcome_id: None,
                        };
                        self.active = None;
                        DialogAdvanceResult::Closed(completion)
                    } else {
                        DialogAdvanceResult::None
                    }
                }
            },
            DialogNodeKind::Line { next_node_id, .. } => match input {
                MenuInput::Up | MenuInput::Left => {
                    cycle_binary_selection(active, dialog.allow_cancel, -1);
                    DialogAdvanceResult::None
                }
                MenuInput::Down | MenuInput::Right => {
                    cycle_binary_selection(active, dialog.allow_cancel, 1);
                    DialogAdvanceResult::None
                }
                MenuInput::Confirm => {
                    if dialog.allow_cancel && active.selected_index == 1 {
                        let completion = DialogCompletion {
                            dialog_id: active.dialog_id.clone(),
                            outcome_id: None,
                        };
                        self.active = None;
                        DialogAdvanceResult::Closed(completion)
                    } else if let Some(next) = next_node_id.clone() {
                        active.current_node_id = next;
                        active.selected_index = 0;
                        self.resolve_branches(game_state);
                        DialogAdvanceResult::None
                    } else {
                        let completion = DialogCompletion {
                            dialog_id: active.dialog_id.clone(),
                            outcome_id: None,
                        };
                        self.active = None;
                        DialogAdvanceResult::Closed(completion)
                    }
                }
                MenuInput::Back if dialog.allow_cancel => {
                    let completion = DialogCompletion {
                        dialog_id: active.dialog_id.clone(),
                        outcome_id: None,
                    };
                    self.active = None;
                    DialogAdvanceResult::Closed(completion)
                }
                _ => DialogAdvanceResult::None,
            },
            DialogNodeKind::End { outcome_id, .. } => match input {
                MenuInput::Up | MenuInput::Left => {
                    cycle_binary_selection(active, dialog.allow_cancel, -1);
                    DialogAdvanceResult::None
                }
                MenuInput::Down | MenuInput::Right => {
                    cycle_binary_selection(active, dialog.allow_cancel, 1);
                    DialogAdvanceResult::None
                }
                MenuInput::Confirm if dialog.allow_cancel && active.selected_index == 1 => {
                    let completion = DialogCompletion {
                        dialog_id: active.dialog_id.clone(),
                        outcome_id: None,
                    };
                    self.active = None;
                    DialogAdvanceResult::Closed(completion)
                }
                MenuInput::Confirm | MenuInput::Back
                    if dialog.allow_cancel || matches!(input, MenuInput::Confirm) =>
                {
                    let completion = DialogCompletion {
                        dialog_id: active.dialog_id.clone(),
                        outcome_id: outcome_id.clone(),
                    };
                    self.active = None;
                    DialogAdvanceResult::Closed(completion)
                }
                _ => DialogAdvanceResult::None,
            },
            DialogNodeKind::Branch { .. } => {
                self.resolve_branches(game_state);
                DialogAdvanceResult::None
            }
        }
    }

    pub fn activate_entry(
        &mut self,
        entry_index: usize,
        game_state: &GameState,
    ) -> DialogAdvanceResult {
        let Some(active) = self.active.as_mut() else {
            return DialogAdvanceResult::None;
        };
        active.selected_index = entry_index;
        self.handle_input(MenuInput::Confirm, game_state)
    }

    pub fn select_entry(&mut self, entry_index: usize) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        let Some(dialog) = self.dialogs.get(&active.dialog_id) else {
            return;
        };
        let Some(node) = dialog.node(&active.current_node_id) else {
            return;
        };
        let max_index = match &node.kind {
            DialogNodeKind::Choice { choices, .. } => {
                choices.len() + usize::from(dialog.allow_cancel)
            }
            DialogNodeKind::Line { .. } | DialogNodeKind::End { .. } => {
                1 + usize::from(dialog.allow_cancel)
            }
            DialogNodeKind::Branch { .. } => 0,
        };
        if max_index == 0 {
            return;
        }
        active.selected_index = entry_index.min(max_index - 1);
    }

    fn resolve_branches(&mut self, game_state: &GameState) {
        for _ in 0..32 {
            let Some(active) = self.active.as_mut() else {
                return;
            };
            let Some(dialog) = self.dialogs.get(&active.dialog_id) else {
                self.active = None;
                return;
            };
            let Some(node) = dialog.node(&active.current_node_id) else {
                self.active = None;
                return;
            };
            let DialogNodeKind::Branch {
                branches,
                default_next_node_id,
            } = &node.kind
            else {
                return;
            };
            let context = active.context;
            let next = branches
                .iter()
                .find(|branch| Self::conditions_match(game_state, &context, &branch.conditions))
                .map(|branch| branch.next_node_id.clone())
                .or_else(|| default_next_node_id.clone());
            let Some(next) = next else {
                self.active = None;
                return;
            };
            active.current_node_id = next;
            active.selected_index = 0;
        }
        self.active = None;
    }

    fn conditions_match(
        game_state: &GameState,
        context: &DialogRuntimeContext,
        conditions: &[DialogCondition],
    ) -> bool {
        conditions
            .iter()
            .all(|condition| Self::condition_matches(game_state, context, condition))
    }

    fn condition_matches(
        game_state: &GameState,
        context: &DialogRuntimeContext,
        condition: &DialogCondition,
    ) -> bool {
        match condition {
            DialogCondition::HealthBelow { target, threshold } => {
                resolve_dialog_target(game_state, context, *target)
                    .and_then(|entity_id| game_state.entity_manager().get_entity(entity_id))
                    .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                    .is_some_and(|health| health < *threshold)
            }
            DialogCondition::HealthAbove { target, threshold } => {
                resolve_dialog_target(game_state, context, *target)
                    .and_then(|entity_id| game_state.entity_manager().get_entity(entity_id))
                    .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                    .is_some_and(|health| health > *threshold)
            }
            DialogCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => resolve_dialog_target(game_state, context, *target)
                .and_then(|entity_id| game_state.entity_manager().get_entity(entity_id))
                .is_some_and(|entity| {
                    entity.attributes.inventory.item_count(item_id) >= *min_count
                }),
            DialogCondition::EntityHasTag { target, tag } => {
                resolve_dialog_target(game_state, context, *target)
                    .and_then(|entity_id| game_state.entity_manager().get_entity(entity_id))
                    .is_some_and(|entity| entity.tags.contains(tag))
            }
            DialogCondition::EntityIsKind {
                target,
                entity_kind,
            } => resolve_dialog_target(game_state, context, *target)
                .and_then(|entity_id| game_state.entity_manager().get_entity(entity_id))
                .is_some_and(|entity| entity.entity_kind == *entity_kind),
        }
    }
}

fn resolve_dialog_target(
    game_state: &GameState,
    context: &DialogRuntimeContext,
    target: DialogConditionTarget,
) -> Option<crate::entity::EntityId> {
    match target {
        DialogConditionTarget::Player => game_state.player_id(),
        DialogConditionTarget::Interactor => context.interactor,
        DialogConditionTarget::Speaker => context.speaker,
    }
}

fn line_entries(allow_cancel: bool, has_next: bool, selected_index: usize) -> Vec<MenuViewEntry> {
    let mut entries = vec![MenuViewEntry {
        text: if has_next {
            "Continue".to_string()
        } else {
            "Close".to_string()
        },
        selected: selected_index == 0,
        selectable: true,
        border_style_override: None,
    }];
    if allow_cancel {
        entries.push(MenuViewEntry {
            text: "Cancel".to_string(),
            selected: selected_index == 1,
            selectable: true,
            border_style_override: None,
        });
    }
    entries
}

fn cycle_binary_selection(active: &mut ActiveDialogState, allow_cancel: bool, direction: i32) {
    if !allow_cancel {
        active.selected_index = 0;
        return;
    }

    let total = 2usize;
    if direction < 0 {
        active.selected_index = (active.selected_index + total - 1) % total;
    } else {
        active.selected_index = (active.selected_index + 1) % total;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialog::{DialogChoice, DialogNode, DialogNodeKind, DialogTree};
    use crate::menu::MenuInput;
    use crate::GameState;

    fn simple_dialog() -> DialogTree {
        DialogTree {
            id: "intro".to_string(),
            title: "Intro".to_string(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: Some("Guide".to_string()),
                    kind: DialogNodeKind::Line {
                        body: "Hello".to_string(),
                        next_node_id: Some("end".to_string()),
                    },
                },
                DialogNode {
                    id: "end".to_string(),
                    speaker_name: None,
                    kind: DialogNodeKind::End {
                        body: "Bye".to_string(),
                        outcome_id: Some("done".to_string()),
                    },
                },
            ],
        }
    }

    #[test]
    fn dialog_controller_advances_and_emits_outcome() {
        let game_state = GameState::new_empty();
        let mut controller = DialogController::new(vec![simple_dialog()]);
        controller
            .start_dialog(&game_state, "intro", DialogRuntimeContext::default())
            .expect("dialog should start");

        let first = controller.current_view().expect("view");
        assert_eq!(first.title, "Guide");
        assert_eq!(first.body, "Hello");
        assert_eq!(first.entries.len(), 2);

        assert_eq!(
            controller.handle_input(MenuInput::Confirm, &game_state),
            DialogAdvanceResult::None
        );
        let second = controller.current_view().expect("view");
        assert_eq!(second.body, "Bye");

        assert_eq!(
            controller.handle_input(MenuInput::Confirm, &game_state),
            DialogAdvanceResult::Closed(DialogCompletion {
                dialog_id: "intro".to_string(),
                outcome_id: Some("done".to_string()),
            })
        );
        assert!(!controller.is_open());
    }

    #[test]
    fn dialog_controller_choice_navigation_selects_branch() {
        let game_state = GameState::new_empty();
        let dialog = DialogTree {
            id: "choices".to_string(),
            title: "Choices".to_string(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    kind: DialogNodeKind::Choice {
                        body: "Choose".to_string(),
                        choices: vec![
                            DialogChoice {
                                id: "a".to_string(),
                                label: "Alpha".to_string(),
                                next_node_id: "end_a".to_string(),
                                conditions: Vec::new(),
                            },
                            DialogChoice {
                                id: "b".to_string(),
                                label: "Beta".to_string(),
                                next_node_id: "end_b".to_string(),
                                conditions: Vec::new(),
                            },
                        ],
                    },
                },
                DialogNode {
                    id: "end_a".to_string(),
                    speaker_name: None,
                    kind: DialogNodeKind::End {
                        body: "A".to_string(),
                        outcome_id: Some("alpha".to_string()),
                    },
                },
                DialogNode {
                    id: "end_b".to_string(),
                    speaker_name: None,
                    kind: DialogNodeKind::End {
                        body: "B".to_string(),
                        outcome_id: Some("beta".to_string()),
                    },
                },
            ],
        };
        let mut controller = DialogController::new(vec![dialog]);
        controller
            .start_dialog(&game_state, "choices", DialogRuntimeContext::default())
            .expect("dialog should start");
        controller.handle_input(MenuInput::Down, &game_state);
        assert_eq!(
            controller.current_view().expect("view").entries[1].text,
            "Beta"
        );
        assert!(controller.current_view().expect("view").entries[1].selected);
        assert_eq!(
            controller.handle_input(MenuInput::Confirm, &game_state),
            DialogAdvanceResult::None
        );
        assert_eq!(
            controller.handle_input(MenuInput::Confirm, &game_state),
            DialogAdvanceResult::Closed(DialogCompletion {
                dialog_id: "choices".to_string(),
                outcome_id: Some("beta".to_string()),
            })
        );
    }

    #[test]
    fn line_dialog_navigation_can_select_cancel() {
        let game_state = GameState::new_empty();
        let mut controller = DialogController::new(vec![simple_dialog()]);
        controller
            .start_dialog(&game_state, "intro", DialogRuntimeContext::default())
            .expect("dialog should start");

        controller.handle_input(MenuInput::Right, &game_state);
        let view = controller.current_view().expect("view");
        assert!(view.entries[1].selected);

        assert_eq!(
            controller.handle_input(MenuInput::Confirm, &game_state),
            DialogAdvanceResult::Closed(DialogCompletion {
                dialog_id: "intro".to_string(),
                outcome_id: None,
            })
        );
    }

    #[test]
    fn dialog_controller_reports_active_gate_gameplay_flag() {
        let game_state = GameState::new_empty();
        let mut dialog = simple_dialog();
        dialog.gate_gameplay = false;
        let mut controller = DialogController::new(vec![dialog]);
        controller
            .start_dialog(&game_state, "intro", DialogRuntimeContext::default())
            .expect("dialog should start");

        assert!(!controller.active_dialog_gates_gameplay());
    }
}
