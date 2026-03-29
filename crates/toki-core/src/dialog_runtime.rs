use std::collections::HashMap;
use thiserror::Error;
use tracing::info;

use crate::dialog::{
    DialogCondition, DialogConditionTarget, DialogNodeKind, DialogRuntimeContext, DialogTree,
    DialogValidationReport,
};
use crate::entity::HEALTH_STAT_ID;
use crate::ids::DialogId;
use crate::menu::{MenuDialogView, MenuInput, MenuViewEntry};
use crate::GameState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogCompletion {
    pub dialog_id: DialogId,
    pub outcome_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAdvanceResult {
    None,
    Closed(DialogCompletion),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DialogStartError {
    #[error("missing dialog '{0}'")]
    MissingDialog(DialogId),
    #[error("invalid dialog: {0}")]
    InvalidDialog(DialogValidationReport),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveDialogState {
    dialog_id: DialogId,
    current_node_id: String,
    selected_index: usize,
    context: DialogRuntimeContext,
}

#[derive(Debug, Default)]
pub struct DialogController {
    dialogs: HashMap<DialogId, DialogTree>,
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

    pub fn active_dialog_id(&self) -> Option<&DialogId> {
        self.active.as_ref().map(|active| &active.dialog_id)
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
        dialog_id: &DialogId,
        context: DialogRuntimeContext,
    ) -> Result<(), DialogStartError> {
        let Some(dialog) = self.dialogs.get(dialog_id) else {
            return Err(DialogStartError::MissingDialog(dialog_id.clone()));
        };
        let report = dialog.validate();
        if !report.is_valid() {
            return Err(DialogStartError::InvalidDialog(report));
        }

        self.active = Some(ActiveDialogState {
            dialog_id: dialog_id.clone(),
            current_node_id: dialog.entry_node_id.clone(),
            selected_index: 0,
            context,
        });
        self.resolve_current_node(game_state);
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
            dialog_id: dialog.id.to_string(),
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
                        self.resolve_current_node(game_state);
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
                        self.resolve_current_node(game_state);
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
                self.resolve_current_node(game_state);
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

    fn resolve_current_node(&mut self, game_state: &GameState) {
        for _ in 0..64 {
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
            if !Self::conditions_match(
                game_state,
                &active.context,
                &active.dialog_id,
                &node.id,
                "node",
                &node.conditions,
            ) {
                match &node.kind {
                    DialogNodeKind::Line { next_node_id, .. } => {
                        info!(
                            dialog_id = %active.dialog_id,
                            node_id = %node.id,
                            next_node_id = ?next_node_id,
                            "Dialog node conditions failed; skipping line node"
                        );
                        let Some(next) = next_node_id.clone() else {
                            info!(
                                dialog_id = %active.dialog_id,
                                node_id = %node.id,
                                "Dialog node conditions failed and no fallback exists; closing dialog"
                            );
                            self.active = None;
                            return;
                        };
                        active.current_node_id = next;
                        active.selected_index = 0;
                        continue;
                    }
                    DialogNodeKind::End { .. } => {
                        info!(
                            dialog_id = %active.dialog_id,
                            node_id = %node.id,
                            "Dialog end node conditions failed; closing dialog"
                        );
                        self.active = None;
                        return;
                    }
                    DialogNodeKind::Choice { .. } | DialogNodeKind::Branch { .. } => {}
                }
            }
            let DialogNodeKind::Branch {
                branches,
                default_next_node_id,
            } = &node.kind
            else {
                return;
            };
            let context = active.context;
            let mut matched_branch = None;
            for (branch_index, branch) in branches.iter().enumerate() {
                if Self::conditions_match(
                    game_state,
                    &context,
                    &active.dialog_id,
                    &node.id,
                    &format!("branch_{}", branch_index + 1),
                    &branch.conditions,
                ) {
                    matched_branch = Some((branch_index, branch.next_node_id.clone()));
                    break;
                }
            }
            let next = matched_branch
                .as_ref()
                .map(|(_, next_node_id)| next_node_id.clone())
                .or_else(|| default_next_node_id.clone());
            let Some(next) = next else {
                info!(
                    dialog_id = %active.dialog_id,
                    node_id = %node.id,
                    "No dialog branch conditions matched and no default branch exists; closing dialog"
                );
                self.active = None;
                return;
            };
            if let Some((branch_index, _)) = matched_branch {
                info!(
                    dialog_id = %active.dialog_id,
                    node_id = %node.id,
                    branch_index = branch_index + 1,
                    next_node_id = %next,
                    "Dialog branch conditions matched; selecting branch"
                );
            } else {
                info!(
                    dialog_id = %active.dialog_id,
                    node_id = %node.id,
                    next_node_id = %next,
                    "No dialog branch conditions matched; using default branch"
                );
            }
            active.current_node_id = next;
            active.selected_index = 0;
        }
        self.active = None;
    }

    fn conditions_match(
        game_state: &GameState,
        context: &DialogRuntimeContext,
        dialog_id: &str,
        node_id: &str,
        scope: &str,
        conditions: &[DialogCondition],
    ) -> bool {
        conditions.iter().all(|condition| {
            let result = Self::condition_matches(game_state, context, condition);
            info!(
                dialog_id = %dialog_id,
                node_id = %node_id,
                scope = %scope,
                condition = ?condition,
                result,
                "Dialog condition evaluated"
            );
            result
        })
    }

    fn condition_matches(
        game_state: &GameState,
        context: &DialogRuntimeContext,
        condition: &DialogCondition,
    ) -> bool {
        match condition {
            DialogCondition::HealthBelow { target, threshold } => {
                resolve_dialog_target(game_state, context, *target)
                    .and_then(|entity_id| game_state.world().entity_manager().get_entity(entity_id))
                    .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                    .is_some_and(|health| health < *threshold)
            }
            DialogCondition::HealthAbove { target, threshold } => {
                resolve_dialog_target(game_state, context, *target)
                    .and_then(|entity_id| game_state.world().entity_manager().get_entity(entity_id))
                    .and_then(|entity| entity.attributes.stats.current(HEALTH_STAT_ID))
                    .is_some_and(|health| health > *threshold)
            }
            DialogCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => resolve_dialog_target(game_state, context, *target)
                .and_then(|entity_id| game_state.world().entity_manager().get_entity(entity_id))
                .is_some_and(|entity| {
                    entity.attributes.inventory.item_count(item_id) >= *min_count
                }),
            DialogCondition::EntityHasTag { target, tag } => {
                resolve_dialog_target(game_state, context, *target)
                    .and_then(|entity_id| game_state.world().entity_manager().get_entity(entity_id))
                    .is_some_and(|entity| entity.tags.contains(tag))
            }
            DialogCondition::EntityIsKind {
                target,
                entity_kind,
            } => resolve_dialog_target(game_state, context, *target)
                .and_then(|entity_id| game_state.world().entity_manager().get_entity(entity_id))
                .is_some_and(|entity| entity.entity_kind == *entity_kind),
            DialogCondition::FlagEquals { flag, value } => game_state.flag(flag) == Some(value),
            DialogCondition::FlagSet { flag } => game_state.game_flags().is_set(flag),
            DialogCondition::FlagGreaterThan { flag, value } => game_state
                .flag(flag)
                .and_then(|flag_value| flag_value.as_int())
                .is_some_and(|flag_value| flag_value > *value),
        }
    }
}

fn resolve_dialog_target(
    game_state: &GameState,
    context: &DialogRuntimeContext,
    target: DialogConditionTarget,
) -> Option<crate::entity::EntityId> {
    match target {
        DialogConditionTarget::Player => game_state.world().player_id(),
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
    use crate::dialog::{DialogBranch, DialogChoice, DialogCondition, DialogNode, DialogNodeKind, DialogTree};
    use crate::flags::FlagValue;
    use crate::menu::MenuInput;
    use crate::GameState;

    fn simple_dialog() -> DialogTree {
        DialogTree {
            id: "intro".to_string().into(),
            title: "Intro".to_string(),
            entry_node_id: "start".to_string().into(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string().into(),
                    speaker_name: Some("Guide".to_string()),
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: "Hello".to_string(),
                        next_node_id: Some("end".to_string()),
                    },
                },
                DialogNode {
                    id: "end".to_string().into(),
                    speaker_name: None,
                    conditions: Vec::new(),
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
            .start_dialog(&game_state, &"intro".into(), DialogRuntimeContext::default())
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
                dialog_id: "intro".to_string().into(),
                outcome_id: Some("done".to_string()),
            })
        );
        assert!(!controller.is_open());
    }

    #[test]
    fn dialog_controller_choice_navigation_selects_branch() {
        let game_state = GameState::new_empty();
        let dialog = DialogTree {
            id: "choices".to_string().into(),
            title: "Choices".to_string(),
            entry_node_id: "start".to_string().into(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string().into(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Choice {
                        body: "Choose".to_string(),
                        choices: vec![
                            DialogChoice {
                                id: "a".to_string().into(),
                                label: "Alpha".to_string(),
                                next_node_id: "end_a".to_string().into(),
                                conditions: Vec::new(),
                            },
                            DialogChoice {
                                id: "b".to_string().into(),
                                label: "Beta".to_string(),
                                next_node_id: "end_b".to_string().into(),
                                conditions: Vec::new(),
                            },
                        ],
                    },
                },
                DialogNode {
                    id: "end_a".to_string().into(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: "A".to_string(),
                        outcome_id: Some("alpha".to_string()),
                    },
                },
                DialogNode {
                    id: "end_b".to_string().into(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: "B".to_string(),
                        outcome_id: Some("beta".to_string()),
                    },
                },
            ],
        };
        let mut controller = DialogController::new(vec![dialog]);
        controller
            .start_dialog(&game_state, &"choices".into(), DialogRuntimeContext::default())
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
                dialog_id: "choices".to_string().into(),
                outcome_id: Some("beta".to_string()),
            })
        );
    }

    #[test]
    fn line_dialog_navigation_can_select_cancel() {
        let game_state = GameState::new_empty();
        let mut controller = DialogController::new(vec![simple_dialog()]);
        controller
            .start_dialog(&game_state, &"intro".into(), DialogRuntimeContext::default())
            .expect("dialog should start");

        controller.handle_input(MenuInput::Right, &game_state);
        let view = controller.current_view().expect("view");
        assert!(view.entries[1].selected);

        assert_eq!(
            controller.handle_input(MenuInput::Confirm, &game_state),
            DialogAdvanceResult::Closed(DialogCompletion {
                dialog_id: "intro".to_string().into(),
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
            .start_dialog(&game_state, &"intro".into(), DialogRuntimeContext::default())
            .expect("dialog should start");

        assert!(!controller.active_dialog_gates_gameplay());
    }

    #[test]
    fn dialog_controller_resolves_branch_nodes_from_game_flags() {
        let mut game_state = GameState::new_empty();
        game_state.set_flag("met_npc", FlagValue::Bool(true));
        let dialog = DialogTree {
            id: "flag_branch".to_string().into(),
            title: "Flags".to_string(),
            entry_node_id: "branch".to_string().into(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "branch".to_string().into(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Branch {
                        branches: vec![DialogBranch {
                            conditions: vec![DialogCondition::FlagSet {
                                flag: "met_npc".to_string(),
                            }],
                            next_node_id: "met".to_string().into(),
                        }],
                        default_next_node_id: Some("new".to_string()),
                    },
                },
                DialogNode {
                    id: "met".to_string().into(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: "welcome back".to_string(),
                        outcome_id: Some("met".to_string()),
                    },
                },
                DialogNode {
                    id: "new".to_string().into(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: "nice to meet you".to_string(),
                        outcome_id: Some("new".to_string()),
                    },
                },
            ],
        };

        let mut controller = DialogController::new(vec![dialog]);
        controller
            .start_dialog(&game_state, &"flag_branch".into(), DialogRuntimeContext::default())
            .expect("dialog should start");

        let view = controller.current_view().expect("view");
        assert_eq!(view.body, "welcome back");
    }

    #[test]
    fn dialog_controller_skips_conditioned_line_nodes_until_it_finds_a_match() {
        let mut game_state = GameState::new_empty();
        game_state.set_flag("seen", FlagValue::Bool(true));
        let dialog = DialogTree {
            id: "line_skip".to_string().into(),
            title: "Skip".to_string(),
            entry_node_id: "start".to_string().into(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string().into(),
                    speaker_name: None,
                    conditions: vec![DialogCondition::FlagSet {
                        flag: "missing".to_string(),
                    }],
                    kind: DialogNodeKind::Line {
                        body: "skip me".to_string(),
                        next_node_id: Some("next".to_string()),
                    },
                },
                DialogNode {
                    id: "next".to_string().into(),
                    speaker_name: None,
                    conditions: vec![DialogCondition::FlagSet {
                        flag: "seen".to_string(),
                    }],
                    kind: DialogNodeKind::End {
                        body: "visible".to_string(),
                        outcome_id: Some("ok".to_string()),
                    },
                },
            ],
        };

        let mut controller = DialogController::new(vec![dialog]);
        controller
            .start_dialog(&game_state, &"line_skip".into(), DialogRuntimeContext::default())
            .expect("dialog should start");

        assert_eq!(controller.current_view().expect("view").body, "visible");
    }

    #[test]
    fn dialog_controller_closes_when_conditioned_end_node_does_not_match() {
        let game_state = GameState::new_empty();
        let dialog = DialogTree {
            id: "end_skip".to_string().into(),
            title: "End".to_string(),
            entry_node_id: "end".to_string().into(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![DialogNode {
                id: "end".to_string().into(),
                speaker_name: None,
                conditions: vec![DialogCondition::FlagSet {
                    flag: "missing".to_string(),
                }],
                kind: DialogNodeKind::End {
                    body: "hidden".to_string(),
                    outcome_id: Some("hidden".to_string()),
                },
            }],
        };

        let mut controller = DialogController::new(vec![dialog]);
        controller
            .start_dialog(&game_state, &"end_skip".into(), DialogRuntimeContext::default())
            .expect("dialog should start");

        assert!(!controller.is_open());
    }

    #[test]
    fn dialog_controller_returns_structured_validation_report_for_invalid_dialog() {
        let game_state = GameState::new_empty();
        let invalid = DialogTree {
            id: "broken".to_string().into(),
            title: String::new(),
            entry_node_id: "missing".to_string().into(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![DialogNode {
                id: "start".to_string().into(),
                speaker_name: None,
                conditions: Vec::new(),
                kind: DialogNodeKind::End {
                    body: String::new(),
                    outcome_id: None,
                },
            }],
        };

        let mut controller = DialogController::new(vec![invalid]);
        let result =
            controller.start_dialog(&game_state, &"broken".into(), DialogRuntimeContext::default());

        match result {
            Err(DialogStartError::InvalidDialog(report)) => {
                assert!(!report.is_valid());
                assert!(report
                    .errors
                    .iter()
                    .any(|error| error.contains("entry node 'missing' does not exist")));
            }
            other => panic!("expected structured invalid dialog error, got {other:?}"),
        }
    }
}
