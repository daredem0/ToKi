use crate::project::ProjectAssets;
use crate::ui::editor_ui::{sync_dialog_registry, EditorUI};
use egui::Ui;
use toki_core::dialog::{
    DialogBranch, DialogChoice, DialogCondition, DialogConditionTarget, DialogNode, DialogNodeKind,
};
use toki_core::entity::EntityKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogNodeKindSelection {
    Line,
    Choice,
    Branch,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogConditionKind {
    HealthBelow,
    HealthAbove,
    HasInventoryItem,
    EntityHasTag,
    EntityIsKind,
}

pub(super) fn render_dialog_editor(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    project_assets: Option<&mut ProjectAssets>,
) {
    let Some(project_assets) = project_assets else {
        ui.label("Open a project to author dialog assets.");
        return;
    };

    sync_dialog_registry(ui_state, project_assets);
    render_dialog_main(ui, ui_state, project_assets);
}

fn render_dialog_main(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    project_assets: &mut ProjectAssets,
) {
    let Some(mut dialog) = ui_state.dialog.draft.take() else {
        ui.label("No dialog selected.");
        return;
    };

    ui.horizontal(|ui| {
        if ui
            .add_enabled(ui_state.dialog.dirty, egui::Button::new("Save Dialog"))
            .clicked()
        {
            let validation = dialog.validate();
            if !validation.is_valid() {
                ui_state.dialog.status_message = Some(format!(
                    "Cannot save dialog with {} validation error(s)",
                    validation.errors.len()
                ));
            } else if let Err(error) = project_assets.save_dialog(&dialog) {
                ui_state.dialog.status_message =
                    Some(format!("Failed to save dialog '{}': {error}", dialog.id));
            } else {
                ui_state.dialog.selected_dialog_id = Some(dialog.id.clone());
                ui_state.dialog.loaded_dialog_id = Some(dialog.id.clone());
                ui_state.dialog.dirty = false;
                ui_state.dialog.status_message = Some("Dialog saved".to_string());
                sync_dialog_registry(ui_state, project_assets);
            }
        }

        if let Some(status) = &ui_state.dialog.status_message {
            ui.label(status);
        }
    });

    let mut dirty = false;
    ui.separator();
    ui.heading("Dialog");
    ui.horizontal(|ui| {
        ui.label("Id:");
        dirty |= ui.text_edit_singleline(&mut dialog.id).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Title:");
        dirty |= ui.text_edit_singleline(&mut dialog.title).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Entry Node:");
        dirty |= ui.text_edit_singleline(&mut dialog.entry_node_id).changed();
    });
    dirty |= ui.checkbox(&mut dialog.allow_cancel, "Allow Cancel").changed();

    let validation = dialog.validate();
    if !validation.errors.is_empty() {
        for error in &validation.errors {
            ui.colored_label(egui::Color32::from_rgb(255, 120, 120), error);
        }
    }
    if !validation.warnings.is_empty() {
        for warning in &validation.warnings {
            ui.colored_label(egui::Color32::from_rgb(255, 210, 80), warning);
        }
    }

    ui.separator();
    ui.columns(2, |columns| {
        columns[0].heading("Nodes");
        render_node_list(&mut columns[0], ui_state, &mut dialog, &mut dirty);
        columns[1].heading("Node Editor");
        render_node_editor(&mut columns[1], ui_state, &mut dialog, &mut dirty);
    });

    if dirty {
        ui_state.dialog.dirty = true;
    }
    ui_state.dialog.draft = Some(dialog);
}

fn render_node_list(ui: &mut Ui, ui_state: &mut EditorUI, dialog: &mut toki_core::dialog::DialogTree, dirty: &mut bool) {
    ui.horizontal_wrapped(|ui| {
        for (label, kind) in [
            ("+ Line", DialogNodeKindSelection::Line),
            ("+ Choice", DialogNodeKindSelection::Choice),
            ("+ Branch", DialogNodeKindSelection::Branch),
            ("+ End", DialogNodeKindSelection::End),
        ] {
            if ui.small_button(label).clicked() {
                let new_id = unique_node_id(dialog, "node");
                dialog.nodes.push(DialogNode {
                    id: new_id.clone(),
                    speaker_name: None,
                    kind: default_node_kind(kind),
                });
                ui_state.dialog.selected_node_id = Some(new_id);
                *dirty = true;
            }
        }
    });

    egui::ScrollArea::vertical().show(ui, |ui| {
        for node in &dialog.nodes {
            let selected = ui_state.dialog.selected_node_id.as_deref() == Some(node.id.as_str());
            if ui.selectable_label(selected, &node.id).clicked() {
                ui_state.dialog.selected_node_id = Some(node.id.clone());
            }
        }
    });
}

fn render_node_editor(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    dialog: &mut toki_core::dialog::DialogTree,
    dirty: &mut bool,
) {
    let Some(selected_node_id) = ui_state.dialog.selected_node_id.clone() else {
        ui.label("Select a node.");
        return;
    };
    let Some(node_index) = dialog.nodes.iter().position(|node| node.id == selected_node_id) else {
        ui.label("Selected node no longer exists.");
        return;
    };
    let node = &mut dialog.nodes[node_index];

    ui.horizontal(|ui| {
        ui.label("Node Id:");
        *dirty |= ui.text_edit_singleline(&mut node.id).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Speaker:");
        let speaker = node.speaker_name.get_or_insert_with(String::new);
        *dirty |= ui.text_edit_singleline(speaker).changed();
        if speaker.trim().is_empty() {
            node.speaker_name = None;
        }
    });

    let current_kind = kind_selection(&node.kind);
    let mut selected_kind = current_kind;
    ui.horizontal(|ui| {
        ui.label("Kind:");
        egui::ComboBox::from_id_salt(("dialog_node_kind", node_index))
            .selected_text(kind_label(current_kind))
            .show_ui(ui, |ui| {
                for candidate in [
                    DialogNodeKindSelection::Line,
                    DialogNodeKindSelection::Choice,
                    DialogNodeKindSelection::Branch,
                    DialogNodeKindSelection::End,
                ] {
                    *dirty |= ui
                        .selectable_value(&mut selected_kind, candidate, kind_label(candidate))
                        .changed();
                }
            });
    });
    if selected_kind != current_kind {
        node.kind = default_node_kind(selected_kind);
        *dirty = true;
    }

    ui.separator();
    match &mut node.kind {
        DialogNodeKind::Line { body, next_node_id } => {
            ui.label("Body:");
            *dirty |= ui.text_edit_multiline(body).changed();
            optional_node_ref_editor(ui, "Next Node:", next_node_id, dirty);
        }
        DialogNodeKind::Choice { body, choices } => {
            ui.label("Body:");
            *dirty |= ui.text_edit_multiline(body).changed();
            if ui.small_button("+ Choice").clicked() {
                choices.push(DialogChoice {
                    id: format!("choice_{}", choices.len() + 1),
                    label: "Choice".to_string(),
                    next_node_id: String::new(),
                    conditions: Vec::new(),
                });
                *dirty = true;
            }
            for (index, choice) in choices.iter_mut().enumerate() {
                ui.separator();
                ui.label(format!("Choice {}", index + 1));
                ui.horizontal(|ui| {
                    ui.label("Id:");
                    *dirty |= ui.text_edit_singleline(&mut choice.id).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Label:");
                    *dirty |= ui.text_edit_singleline(&mut choice.label).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Next Node:");
                    *dirty |= ui.text_edit_singleline(&mut choice.next_node_id).changed();
                });
                render_conditions(ui, &mut choice.conditions, dirty, ("choice_conditions", index));
            }
        }
        DialogNodeKind::Branch {
            branches,
            default_next_node_id,
        } => {
            optional_node_ref_editor(ui, "Default Next:", default_next_node_id, dirty);
            if ui.small_button("+ Branch").clicked() {
                branches.push(DialogBranch {
                    conditions: Vec::new(),
                    next_node_id: String::new(),
                });
                *dirty = true;
            }
            for (index, branch) in branches.iter_mut().enumerate() {
                ui.separator();
                ui.label(format!("Branch {}", index + 1));
                ui.horizontal(|ui| {
                    ui.label("Next Node:");
                    *dirty |= ui.text_edit_singleline(&mut branch.next_node_id).changed();
                });
                render_conditions(ui, &mut branch.conditions, dirty, ("branch_conditions", index));
            }
        }
        DialogNodeKind::End { body, outcome_id } => {
            ui.label("Body:");
            *dirty |= ui.text_edit_multiline(body).changed();
            optional_node_ref_editor(ui, "Outcome Id:", outcome_id, dirty);
        }
    }
}

fn render_conditions(
    ui: &mut Ui,
    conditions: &mut Vec<DialogCondition>,
    dirty: &mut bool,
    id_salt: impl std::hash::Hash,
) {
    ui.horizontal(|ui| {
        if ui.small_button("+ Condition").clicked() {
            conditions.push(DialogCondition::EntityHasTag {
                target: DialogConditionTarget::Player,
                tag: String::new(),
            });
            *dirty = true;
        }
    });

    for (index, condition) in conditions.iter_mut().enumerate() {
        ui.group(|ui| {
            let current_kind = condition_kind(condition);
            let mut selected_kind = current_kind;
            egui::ComboBox::from_id_salt((&id_salt, "condition_kind", index))
                .selected_text(condition_kind_label(current_kind))
                .show_ui(ui, |ui| {
                    for candidate in [
                        DialogConditionKind::HealthBelow,
                        DialogConditionKind::HealthAbove,
                        DialogConditionKind::HasInventoryItem,
                        DialogConditionKind::EntityHasTag,
                        DialogConditionKind::EntityIsKind,
                    ] {
                        *dirty |= ui
                            .selectable_value(
                                &mut selected_kind,
                                candidate,
                                condition_kind_label(candidate),
                            )
                            .changed();
                    }
                });
            if selected_kind != current_kind {
                *condition = default_condition(selected_kind);
                *dirty = true;
            }
            match condition {
                DialogCondition::HealthBelow { target, threshold }
                | DialogCondition::HealthAbove { target, threshold } => {
                    *dirty |= render_condition_target(ui, target, (&id_salt, "target", index));
                    ui.horizontal(|ui| {
                        ui.label("Threshold:");
                        *dirty |= ui.add(egui::DragValue::new(threshold).speed(1.0)).changed();
                    });
                }
                DialogCondition::HasInventoryItem {
                    target,
                    item_id,
                    min_count,
                } => {
                    *dirty |= render_condition_target(ui, target, (&id_salt, "target", index));
                    ui.horizontal(|ui| {
                        ui.label("Item Id:");
                        *dirty |= ui.text_edit_singleline(item_id).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Min Count:");
                        let mut count = *min_count as i32;
                        if ui.add(egui::DragValue::new(&mut count).range(0..=9999)).changed() {
                            *min_count = count.max(0) as u32;
                            *dirty = true;
                        }
                    });
                }
                DialogCondition::EntityHasTag { target, tag } => {
                    *dirty |= render_condition_target(ui, target, (&id_salt, "target", index));
                    ui.horizontal(|ui| {
                        ui.label("Tag:");
                        *dirty |= ui.text_edit_singleline(tag).changed();
                    });
                }
                DialogCondition::EntityIsKind {
                    target,
                    entity_kind,
                } => {
                    *dirty |= render_condition_target(ui, target, (&id_salt, "target", index));
                    egui::ComboBox::from_id_salt((&id_salt, "entity_kind", index))
                        .selected_text(format!("{entity_kind:?}"))
                        .show_ui(ui, |ui| {
                            for candidate in [
                                EntityKind::Player,
                                EntityKind::Npc,
                                EntityKind::Item,
                                EntityKind::Decoration,
                                EntityKind::Trigger,
                                EntityKind::Projectile,
                            ] {
                                *dirty |= ui
                                    .selectable_value(
                                        entity_kind,
                                        candidate,
                                        format!("{candidate:?}"),
                                    )
                                    .changed();
                            }
                        });
                }
            }
        });
    }
}

fn render_condition_target(
    ui: &mut Ui,
    target: &mut DialogConditionTarget,
    id_salt: impl std::hash::Hash,
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(match target {
            DialogConditionTarget::Player => "Player",
            DialogConditionTarget::Interactor => "Interactor",
            DialogConditionTarget::Speaker => "Speaker",
        })
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(target, DialogConditionTarget::Player, "Player")
                .changed();
            changed |= ui
                .selectable_value(target, DialogConditionTarget::Interactor, "Interactor")
                .changed();
            changed |= ui
                .selectable_value(target, DialogConditionTarget::Speaker, "Speaker")
                .changed();
        });
    changed
}

fn optional_node_ref_editor(
    ui: &mut Ui,
    label: &str,
    value: &mut Option<String>,
    dirty: &mut bool,
) {
    let text = value.get_or_insert_with(String::new);
    ui.horizontal(|ui| {
        ui.label(label);
        *dirty |= ui.text_edit_singleline(text).changed();
    });
    if text.trim().is_empty() {
        *value = None;
    }
}

fn kind_selection(kind: &DialogNodeKind) -> DialogNodeKindSelection {
    match kind {
        DialogNodeKind::Line { .. } => DialogNodeKindSelection::Line,
        DialogNodeKind::Choice { .. } => DialogNodeKindSelection::Choice,
        DialogNodeKind::Branch { .. } => DialogNodeKindSelection::Branch,
        DialogNodeKind::End { .. } => DialogNodeKindSelection::End,
    }
}

fn kind_label(kind: DialogNodeKindSelection) -> &'static str {
    match kind {
        DialogNodeKindSelection::Line => "Line",
        DialogNodeKindSelection::Choice => "Choice",
        DialogNodeKindSelection::Branch => "Branch",
        DialogNodeKindSelection::End => "End",
    }
}

fn default_node_kind(kind: DialogNodeKindSelection) -> DialogNodeKind {
    match kind {
        DialogNodeKindSelection::Line => DialogNodeKind::Line {
            body: String::new(),
            next_node_id: None,
        },
        DialogNodeKindSelection::Choice => DialogNodeKind::Choice {
            body: String::new(),
            choices: Vec::new(),
        },
        DialogNodeKindSelection::Branch => DialogNodeKind::Branch {
            branches: Vec::new(),
            default_next_node_id: None,
        },
        DialogNodeKindSelection::End => DialogNodeKind::End {
            body: String::new(),
            outcome_id: None,
        },
    }
}

fn condition_kind(condition: &DialogCondition) -> DialogConditionKind {
    match condition {
        DialogCondition::HealthBelow { .. } => DialogConditionKind::HealthBelow,
        DialogCondition::HealthAbove { .. } => DialogConditionKind::HealthAbove,
        DialogCondition::HasInventoryItem { .. } => DialogConditionKind::HasInventoryItem,
        DialogCondition::EntityHasTag { .. } => DialogConditionKind::EntityHasTag,
        DialogCondition::EntityIsKind { .. } => DialogConditionKind::EntityIsKind,
    }
}

fn condition_kind_label(kind: DialogConditionKind) -> &'static str {
    match kind {
        DialogConditionKind::HealthBelow => "HealthBelow",
        DialogConditionKind::HealthAbove => "HealthAbove",
        DialogConditionKind::HasInventoryItem => "HasInventoryItem",
        DialogConditionKind::EntityHasTag => "EntityHasTag",
        DialogConditionKind::EntityIsKind => "EntityIsKind",
    }
}

fn default_condition(kind: DialogConditionKind) -> DialogCondition {
    match kind {
        DialogConditionKind::HealthBelow => DialogCondition::HealthBelow {
            target: DialogConditionTarget::Player,
            threshold: 50,
        },
        DialogConditionKind::HealthAbove => DialogCondition::HealthAbove {
            target: DialogConditionTarget::Player,
            threshold: 50,
        },
        DialogConditionKind::HasInventoryItem => DialogCondition::HasInventoryItem {
            target: DialogConditionTarget::Player,
            item_id: String::new(),
            min_count: 1,
        },
        DialogConditionKind::EntityHasTag => DialogCondition::EntityHasTag {
            target: DialogConditionTarget::Player,
            tag: String::new(),
        },
        DialogConditionKind::EntityIsKind => DialogCondition::EntityIsKind {
            target: DialogConditionTarget::Player,
            entity_kind: EntityKind::Npc,
        },
    }
}

fn unique_node_id(dialog: &toki_core::dialog::DialogTree, prefix: &str) -> String {
    let mut index = 1usize;
    loop {
        let candidate = format!("{prefix}_{index}");
        if !dialog.nodes.iter().any(|node| node.id == candidate) {
            return candidate;
        }
        index += 1;
    }
}
