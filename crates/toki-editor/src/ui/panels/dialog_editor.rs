use crate::project::{DialogGraphLayout, Project, ProjectAssets};
use crate::ui::dialog_graph::{
    auto_layout_positions, connect_edge, create_line_node_at, disconnect_all_outgoing,
    duplicate_node, normalize_dialog_graph_layout, remove_layout_node_key, rename_layout_node_key,
    set_layout_node_position, unique_node_id, DialogGraphDocument, DialogGraphEdgeKind,
};
use crate::ui::editor_ui::{sync_dialog_registry, EditorUI};
use crate::ui::graph_canvas::{render_graph_canvas, GraphCanvasAction};
use egui::{Key, Ui};
use toki_core::dialog::{
    DialogBranch, DialogChoice, DialogCondition, DialogConditionTarget, DialogNode, DialogNodeKind,
};
use toki_core::entity::EntityKind;
use toki_core::FlagValue;

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
    FlagEquals,
    FlagSet,
    FlagGreaterThan,
}

pub(crate) fn render_dialog_editor(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    project_assets: Option<&mut ProjectAssets>,
    project: Option<&mut Project>,
) {
    let Some(project_assets) = project_assets else {
        ui.label("Open a project to author dialog assets.");
        return;
    };

    sync_dialog_registry(ui_state, project_assets);
    let declared_flags = project
        .as_ref()
        .map(|project| project.metadata.runtime.flags.declarations.as_slice())
        .unwrap_or(&[]);
    render_dialog_main(ui, ui_state, project_assets, declared_flags);
}

fn render_dialog_main(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    project_assets: &mut ProjectAssets,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) {
    let Some(mut dialog) = ui_state.dialog_editor_context_mut().dialog.draft.take() else {
        ui.label("No dialog selected.");
        return;
    };
    {
        let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
        let normalized_layout = normalize_dialog_graph_layout(
            &dialog,
            dialog_state
                .layouts_by_dialog
                .get(dialog.id.as_ref())
                .cloned(),
        );
        dialog_state
            .layouts_by_dialog
            .insert(dialog.id.to_string(), normalized_layout);
        dialog_state.persist_active_graph_view_into_layout();
    }

    handle_dialog_shortcuts(ui, ui_state, &mut dialog);

    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                ui_state.dialog_editor_context().dialog.dirty,
                egui::Button::new("Save Dialog"),
            )
            .clicked()
        {
            ui_state
                .dialog_editor_context_mut()
                .dialog
                .persist_active_graph_view_into_layout();
            let validation = dialog.validate();
            if !validation.is_valid() {
                ui_state.dialog_editor_context_mut().dialog.status_message = Some(format!(
                    "Cannot save dialog with {} validation error(s)",
                    validation.errors.len()
                ));
            } else if let Err(error) = project_assets.save_dialog(&dialog) {
                ui_state.dialog_editor_context_mut().dialog.status_message =
                    Some(format!("Failed to save dialog '{}': {error}", dialog.id));
            } else {
                ui_state
                    .dialog_editor_context_mut()
                    .dialog
                    .selected_dialog_id = Some(dialog.id.to_string());
                ui_state.dialog_editor_context_mut().dialog.loaded_dialog_id =
                    Some(dialog.id.to_string());
                ui_state.dialog_editor_context_mut().dialog.dirty = false;
                ui_state.dialog_editor_context_mut().dialog.status_message =
                    Some("Dialog saved".to_string());
                sync_dialog_registry(ui_state, project_assets);
            }
        }

        if let Some(status) = &ui_state.dialog_editor_context().dialog.status_message {
            ui.label(status);
        }
    });

    let mut dirty = false;
    ui.separator();
    render_dialog_settings(ui, ui_state, &mut dialog, &mut dirty);
    ui.separator();
    render_dialog_validation_summary(ui, &dialog, declared_flags);
    ui.separator();
    render_dialog_graph_workspace(ui, ui_state, &mut dialog, &mut dirty);

    if dirty {
        ui_state.dialog_editor_context_mut().dialog.dirty = true;
    }
    ui_state
        .dialog_editor_context_mut()
        .dialog
        .persist_active_graph_view_into_layout();
    ui_state.dialog_editor_context_mut().dialog.draft = Some(dialog);
}

fn handle_dialog_shortcuts(
    ui: &Ui,
    ui_state: &mut EditorUI,
    dialog: &mut toki_core::dialog::DialogTree,
) {
    if ui.ctx().wants_keyboard_input() {
        return;
    }
    let command_pressed = ui.input(|input| input.modifiers.command);
    let delete_pressed = ui.input(|input| input.key_pressed(Key::Delete));
    let duplicate_pressed = command_pressed && ui.input(|input| input.key_pressed(Key::D));
    let auto_layout_pressed = ui.input(|input| input.key_pressed(Key::A));
    let reset_view_pressed = ui.input(|input| input.key_pressed(Key::Num0));
    let focus_pressed =
        ui.input(|input| input.key_pressed(Key::F) || input.key_pressed(Key::Space));
    let escape_pressed = ui.input(|input| input.key_pressed(Key::Escape));

    if delete_pressed {
        let result = {
            let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
            let layout = dialog_state.layouts_by_dialog.get_mut(dialog.id.as_ref());
            delete_selected_node(dialog, &mut dialog_state.selected_node_id, layout)
        };
        match result {
            Ok(status) => {
                let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
                let selected_after_delete = dialog_state.selected_node_id.clone();
                dialog_state.sync_node_id_editor(selected_after_delete.as_deref());
                dialog_state.status_message = Some(status);
                dialog_state.dirty = true;
            }
            Err(error) => {
                ui_state.dialog_editor_context_mut().dialog.status_message = Some(error);
            }
        }
    }
    if duplicate_pressed {
        if let Some(selected_node_id) = ui_state
            .dialog_editor_context()
            .dialog
            .selected_node_id
            .clone()
        {
            let result = {
                let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
                let layout = dialog_state.ensure_layout_for_dialog(dialog.id.as_ref());
                duplicate_node(dialog, layout, &selected_node_id)
            };
            match result {
                Ok(duplicate_id) => {
                    let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
                    dialog_state.select_dialog_node(duplicate_id.clone());
                    dialog_state.status_message = Some(format!(
                        "Duplicated node '{selected_node_id}' to '{duplicate_id}'."
                    ));
                    dialog_state.dirty = true;
                }
                Err(error) => {
                    ui_state.dialog_editor_context_mut().dialog.status_message = Some(error);
                }
            }
        }
    }
    if auto_layout_pressed {
        let view = {
            let dialog_state = &ui_state.dialog_editor_context().dialog;
            (
                dialog_state.graph_canvas.zoom,
                dialog_state.graph_canvas.pan,
            )
        };
        let new_layout = {
            let mut layout = auto_layout_positions(dialog);
            layout.zoom = view.0;
            layout.pan = view.1;
            layout
        };
        let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
        dialog_state
            .layouts_by_dialog
            .insert(dialog.id.to_string(), new_layout);
        dialog_state.layout_dirty = true;
    }
    if reset_view_pressed {
        let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
        dialog_state.graph_canvas.zoom = 1.0;
        dialog_state.graph_canvas.pan = [16.0, 16.0];
        dialog_state.persist_active_graph_view_into_layout();
    }
    if focus_pressed {
        let selected_node_id = ui_state
            .dialog_editor_context()
            .dialog
            .selected_node_id
            .clone();
        let position = selected_node_id.as_ref().and_then(|selected_node_id| {
            ui_state
                .dialog_editor_context()
                .dialog
                .layouts_by_dialog
                .get(dialog.id.as_ref())
                .and_then(|layout| layout.node_positions.get(selected_node_id).copied())
        });
        if let Some(position) = position {
            let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
            let scale = dialog_state.graph_canvas.zoom.clamp(0.35, 3.0);
            dialog_state.graph_canvas.pan =
                [220.0 - position[0] * scale, 180.0 - position[1] * scale];
            dialog_state.persist_active_graph_view_into_layout();
        }
    }
    if escape_pressed {
        let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
        if dialog_state.graph_canvas.connecting_from.take().is_none() {
            dialog_state.selected_node_id = None;
            dialog_state.sync_node_id_editor(None);
        }
    }
}

pub(crate) fn render_dialog_inspector_panel(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    _project_assets: &mut ProjectAssets,
    project: Option<&Project>,
) {
    let Some(mut dialog) = ui_state.dialog_editor_context_mut().dialog.draft.take() else {
        ui.heading("Node Inspector");
        ui.label("No dialog selected.");
        return;
    };
    let mut dirty = false;
    ui.heading("Node Inspector");
    if let Some(status) = &ui_state.dialog_editor_context().dialog.status_message {
        ui.label(status);
    }
    ui.separator();
    render_node_editor(ui, ui_state, &mut dialog, &mut dirty);
    if let Some(project) = project {
        ui.separator();
        render_dialog_validation_summary(ui, &dialog, &project.metadata.runtime.flags.declarations);
    }
    if dirty {
        ui_state.dialog_editor_context_mut().dialog.dirty = true;
    }
    ui_state
        .dialog_editor_context_mut()
        .dialog
        .persist_active_graph_view_into_layout();
    ui_state.dialog_editor_context_mut().dialog.draft = Some(dialog);
}

fn render_dialog_settings(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    dialog: &mut toki_core::dialog::DialogTree,
    dirty: &mut bool,
) {
    let available_node_ids = dialog
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Id:");
                let old_id = dialog.id.to_string();
                let mut dialog_id = old_id.clone();
                if ui
                    .add_sized(
                        [220.0, ui.spacing().interact_size.y],
                        egui::TextEdit::singleline(&mut dialog_id),
                    )
                    .changed()
                {
                    dialog.id = dialog_id.clone().into();
                    let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
                    if let Some(existing_layout) = dialog_state.layouts_by_dialog.remove(&old_id) {
                        dialog_state
                            .layouts_by_dialog
                            .insert(dialog_id.clone(), existing_layout);
                    }
                    if dialog_state.selected_dialog_id.as_deref() == Some(old_id.as_str()) {
                        dialog_state.selected_dialog_id = Some(dialog_id.clone());
                    }
                    if dialog_state.loaded_dialog_id.as_deref() == Some(old_id.as_str()) {
                        dialog_state.loaded_dialog_id = Some(dialog_id.clone());
                    }
                    *dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Title:");
                *dirty |= ui
                    .add_sized(
                        [220.0, ui.spacing().interact_size.y],
                        egui::TextEdit::singleline(&mut dialog.title),
                    )
                    .changed();
            });
        });
        columns[1].vertical(|ui| {
            optional_or_required_node_picker(
                ui,
                "Entry Node:",
                &mut dialog.entry_node_id,
                &available_node_ids,
                dirty,
                false,
                ("dialog_entry_node", dialog.id.to_string()),
            );
            *dirty |= ui
                .checkbox(&mut dialog.allow_cancel, "Allow Cancel")
                .changed();
            *dirty |= ui
                .checkbox(
                    &mut dialog.gate_gameplay,
                    "Gate Gameplay While Dialog Is Open",
                )
                .changed();
        });
    });
}

fn render_dialog_validation_summary(
    ui: &mut Ui,
    dialog: &toki_core::dialog::DialogTree,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) {
    let validation = dialog.validate();
    ui.label(format!(
        "{} error(s), {} warning(s)",
        validation.errors.len(),
        validation.warnings.len()
    ));
    for error in &validation.errors {
        ui.colored_label(egui::Color32::from_rgb(255, 120, 120), error);
    }
    for warning in &validation.warnings {
        ui.colored_label(egui::Color32::from_rgb(255, 210, 80), warning);
    }
    for warning in undeclared_flag_warnings(dialog, declared_flags) {
        ui.colored_label(egui::Color32::from_rgb(255, 210, 80), warning);
    }
}

fn render_dialog_graph_workspace(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    dialog: &mut toki_core::dialog::DialogTree,
    dirty: &mut bool,
) {
    ui.heading("Dialog Graph");
    ui.horizontal_wrapped(|ui| {
        for (label, kind) in [
            ("+ Line", DialogNodeKindSelection::Line),
            ("+ Choice", DialogNodeKindSelection::Choice),
            ("+ Branch", DialogNodeKindSelection::Branch),
            ("+ End", DialogNodeKindSelection::End),
        ] {
            if ui.small_button(label).clicked() {
                let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
                let position = graph_spawn_position(dialog_state, dialog);
                let new_id = create_dialog_node(
                    dialog,
                    dialog_state.ensure_layout_for_dialog(dialog.id.as_ref()),
                    kind,
                    position,
                );
                dialog_state.select_dialog_node(new_id);
                *dirty = true;
            }
        }
        if ui.small_button("Auto Layout").clicked() {
            let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
            let view = (
                dialog_state.graph_canvas.zoom,
                dialog_state.graph_canvas.pan,
            );
            let mut layout = auto_layout_positions(dialog);
            layout.zoom = view.0;
            layout.pan = view.1;
            dialog_state
                .layouts_by_dialog
                .insert(dialog.id.to_string(), layout);
            dialog_state.layout_dirty = true;
        }
        if ui.small_button("Reset View").clicked() {
            let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
            dialog_state.graph_canvas.zoom = 1.0;
            dialog_state.graph_canvas.pan = [16.0, 16.0];
            dialog_state.persist_active_graph_view_into_layout();
        }
    });
    ui.separator();

    let dialog_state = &mut ui_state.dialog_editor_context_mut().dialog;
    let layout = dialog_state
        .layouts_by_dialog
        .get(dialog.id.as_ref())
        .cloned()
        .unwrap_or_else(|| DialogGraphLayout {
            zoom: dialog_state.graph_canvas.zoom,
            pan: dialog_state.graph_canvas.pan,
            ..auto_layout_positions(dialog)
        });
    let graph_document = DialogGraphDocument::from_dialog(dialog, Some(&layout));
    let actions = render_graph_canvas(
        ui,
        &mut dialog_state.graph_canvas,
        &graph_document.as_canvas_nodes(),
        &graph_document.as_canvas_edges(),
        dialog_state.selected_node_id.as_deref(),
    );
    for action in actions {
        match action {
            GraphCanvasAction::SelectNode(node_id) => {
                if let Some(node_id) = node_id {
                    dialog_state.select_dialog_node(node_id);
                } else {
                    dialog_state.selected_node_id = None;
                    dialog_state.sync_node_id_editor(None);
                }
            }
            GraphCanvasAction::MoveNode { node_id, position } => {
                set_layout_node_position(
                    dialog_state.ensure_layout_for_dialog(dialog.id.as_ref()),
                    &node_id,
                    position,
                );
                dialog_state.layout_dirty = true;
            }
            GraphCanvasAction::CreateNodeAt(position) => {
                let new_id = create_line_node_at(
                    dialog,
                    dialog_state.ensure_layout_for_dialog(dialog.id.as_ref()),
                    position,
                );
                dialog_state.select_dialog_node(new_id);
                *dirty = true;
            }
            GraphCanvasAction::Connect {
                from_node_id,
                from_port_id,
                to_node_id,
            } => {
                if let Some(edge_kind) = DialogGraphEdgeKind::from_port_id(&from_port_id) {
                    match connect_edge(dialog, &from_node_id, &edge_kind, &to_node_id) {
                        Ok(()) => *dirty = true,
                        Err(error) => dialog_state.status_message = Some(error),
                    }
                }
            }
        }
    }
    dialog_state.persist_active_graph_view_into_layout();
}

fn graph_spawn_position(
    dialog_state: &crate::ui::editor_ui::DialogEditorState,
    dialog: &toki_core::dialog::DialogTree,
) -> [f32; 2] {
    dialog_state
        .selected_node_id
        .as_ref()
        .and_then(|selected_node_id| {
            dialog_state
                .layouts_by_dialog
                .get(dialog.id.as_ref())
                .and_then(|layout| layout.node_positions.get(selected_node_id).copied())
        })
        .map(|[x, y]| [x + 80.0, y + 32.0])
        .unwrap_or([120.0, 120.0])
}

fn create_dialog_node(
    dialog: &mut toki_core::dialog::DialogTree,
    layout: &mut DialogGraphLayout,
    kind: DialogNodeKindSelection,
    position: [f32; 2],
) -> String {
    let new_id = unique_node_id(dialog, "node");
    dialog.nodes.push(DialogNode {
        id: new_id.clone(),
        speaker_name: None,
        conditions: Vec::new(),
        kind: default_node_kind(kind),
    });
    set_layout_node_position(layout, &new_id, position);
    new_id
}

fn undeclared_flag_warnings(
    dialog: &toki_core::dialog::DialogTree,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) -> Vec<String> {
    if declared_flags.is_empty() {
        return Vec::new();
    }
    let declared = declared_flags
        .iter()
        .map(|flag| flag.id.trim())
        .filter(|id| !id.is_empty())
        .collect::<std::collections::BTreeSet<_>>();
    let mut warnings = Vec::new();
    for node in &dialog.nodes {
        collect_undeclared_dialog_condition_warnings(
            &node.conditions,
            &declared,
            &mut warnings,
            format!("node '{}'", node.id),
        );
        match &node.kind {
            DialogNodeKind::Choice { choices, .. } => {
                for choice in choices {
                    collect_undeclared_dialog_condition_warnings(
                        &choice.conditions,
                        &declared,
                        &mut warnings,
                        format!("choice '{}' in node '{}'", choice.id, node.id),
                    );
                }
            }
            DialogNodeKind::Branch { branches, .. } => {
                for (index, branch) in branches.iter().enumerate() {
                    collect_undeclared_dialog_condition_warnings(
                        &branch.conditions,
                        &declared,
                        &mut warnings,
                        format!("branch {} in node '{}'", index + 1, node.id),
                    );
                }
            }
            DialogNodeKind::Line { .. } | DialogNodeKind::End { .. } => {}
        }
    }
    warnings
}

fn collect_undeclared_dialog_condition_warnings(
    conditions: &[DialogCondition],
    declared: &std::collections::BTreeSet<&str>,
    warnings: &mut Vec<String>,
    scope: String,
) {
    for condition in conditions {
        let Some(flag) = (match condition {
            DialogCondition::FlagEquals { flag, .. }
            | DialogCondition::FlagSet { flag }
            | DialogCondition::FlagGreaterThan { flag, .. } => Some(flag.trim()),
            _ => None,
        }) else {
            continue;
        };
        if !flag.is_empty() && !declared.contains(flag) {
            warnings.push(format!("{scope} references undeclared flag '{flag}'"));
        }
    }
}

fn render_node_editor(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    dialog: &mut toki_core::dialog::DialogTree,
    dirty: &mut bool,
) {
    let Some(selected_node_id) = crate::ui::editor_context::dialog_state_mut(ui_state)
        .selected_node_id
        .clone()
    else {
        ui.label("Select a node.");
        return;
    };
    ui_state
        .dialog_editor_context_mut()
        .dialog
        .sync_node_id_editor(Some(selected_node_id.as_str()));
    let available_node_ids = dialog
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<Vec<_>>();
    let Some(node_index) = dialog
        .nodes
        .iter()
        .position(|node| node.id == selected_node_id)
    else {
        ui.label("Selected node no longer exists.");
        return;
    };

    let mut deleted = false;
    ui.horizontal(|ui| {
        let delete_button =
            egui::Button::new("Delete Node").fill(egui::Color32::from_rgb(120, 40, 40));
        if ui
            .add_enabled(dialog.nodes.len() > 1, delete_button)
            .clicked()
        {
            let result = {
                let dialog_state = crate::ui::editor_context::dialog_state_mut(ui_state);
                let selected_node_id = &mut dialog_state.selected_node_id;
                let layout = dialog_state.layouts_by_dialog.get_mut(dialog.id.as_ref());
                delete_selected_node(dialog, selected_node_id, layout)
            };
            match result {
                Ok(status) => {
                    let selected_after_delete =
                        crate::ui::editor_context::dialog_state_mut(ui_state)
                            .selected_node_id
                            .clone();
                    ui_state
                        .dialog_editor_context_mut()
                        .dialog
                        .sync_node_id_editor(selected_after_delete.as_deref());
                    crate::ui::editor_context::dialog_state_mut(ui_state).status_message =
                        Some(status);
                    *dirty = true;
                    deleted = true;
                }
                Err(error) => {
                    crate::ui::editor_context::dialog_state_mut(ui_state).status_message =
                        Some(error);
                }
            }
        }
        if dialog.nodes.len() <= 1 {
            ui.label("Dialogs must keep at least one node.");
        }
    });
    if deleted {
        return;
    }

    ui.label("Node");
    let mut selected_node_dropdown = selected_node_id.clone();
    egui::ComboBox::from_id_salt(("dialog_selected_node", dialog.id.to_string()))
        .selected_text(selected_node_dropdown.clone())
        .width(180.0)
        .show_ui(ui, |ui| {
            for node_id in &available_node_ids {
                ui.selectable_value(&mut selected_node_dropdown, node_id.clone(), node_id);
            }
        });
    if selected_node_dropdown != selected_node_id {
        crate::ui::editor_context::dialog_state_mut(ui_state)
            .select_dialog_node(selected_node_dropdown);
        return;
    }

    ui.add_space(4.0);
    ui.label("Rename Node Id");
    ui.horizontal(|ui| {
        let response = ui.add_sized(
            [180.0, ui.spacing().interact_size.y],
            egui::TextEdit::singleline(
                &mut crate::ui::editor_context::dialog_state_mut(ui_state).node_id_edit_value,
            ),
        );
        let apply_clicked = ui.small_button("Apply").clicked();
        let pressed_enter =
            response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if apply_clicked || pressed_enter {
            let node_id_edit_value = crate::ui::editor_context::dialog_state(ui_state)
                .node_id_edit_value
                .clone();
            let result = {
                let dialog_state = crate::ui::editor_context::dialog_state_mut(ui_state);
                let selected_node = &mut dialog_state.selected_node_id;
                let layout = dialog_state.layouts_by_dialog.get_mut(dialog.id.as_ref());
                rename_dialog_node_id(
                    dialog,
                    selected_node,
                    selected_node_id.as_str(),
                    &node_id_edit_value,
                    layout,
                )
            };
            match result {
                Ok(Some(status)) => {
                    let committed_id = crate::ui::editor_context::dialog_state_mut(ui_state)
                        .selected_node_id
                        .clone()
                        .unwrap_or_default();
                    crate::ui::editor_context::dialog_state_mut(ui_state).node_id_edit_target =
                        Some(committed_id.clone());
                    crate::ui::editor_context::dialog_state_mut(ui_state).node_id_edit_value =
                        committed_id;
                    crate::ui::editor_context::dialog_state_mut(ui_state).status_message =
                        Some(status);
                    *dirty = true;
                }
                Ok(None) => {}
                Err(error) => {
                    crate::ui::editor_context::dialog_state_mut(ui_state).status_message =
                        Some(error);
                }
            }
        }
    });
    ui.small("Press Enter or Apply to rename the selected node.");

    let node_id = dialog.nodes[node_index].id.clone();
    ui.horizontal_wrapped(|ui| {
        if ui.small_button("Set As Entry").clicked() {
            dialog.entry_node_id = node_id.clone();
            *dirty = true;
        }
        if ui.small_button("Duplicate").clicked() {
            match duplicate_node(
                dialog,
                crate::ui::editor_context::dialog_state_mut(ui_state)
                    .ensure_layout_for_dialog(dialog.id.as_ref()),
                &node_id,
            ) {
                Ok(duplicate_id) => {
                    crate::ui::editor_context::dialog_state_mut(ui_state)
                        .select_dialog_node(duplicate_id.clone());
                    crate::ui::editor_context::dialog_state_mut(ui_state).status_message =
                        Some(format!("Duplicated node '{node_id}' to '{duplicate_id}'."));
                    *dirty = true;
                }
                Err(error) => {
                    crate::ui::editor_context::dialog_state_mut(ui_state).status_message =
                        Some(error);
                }
            }
        }
        if ui.small_button("Disconnect Outgoing").clicked() {
            match disconnect_all_outgoing(dialog, &node_id) {
                Ok(()) => *dirty = true,
                Err(error) => {
                    crate::ui::editor_context::dialog_state_mut(ui_state).status_message =
                        Some(error);
                }
            }
        }
    });

    let node = &mut dialog.nodes[node_index];
    ui.label("Speaker");
    ui.horizontal(|ui| {
        let speaker = node.speaker_name.get_or_insert_with(String::new);
        *dirty |= ui
            .add_sized(
                [180.0, ui.spacing().interact_size.y],
                egui::TextEdit::singleline(speaker),
            )
            .changed();
        if speaker.trim().is_empty() {
            node.speaker_name = None;
        }
    });

    let current_kind = kind_selection(&node.kind);
    let mut selected_kind = current_kind;
    ui.label("Kind");
    egui::ComboBox::from_id_salt(("dialog_node_kind", node_index))
        .selected_text(kind_label(current_kind))
        .width(180.0)
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
    if selected_kind != current_kind {
        node.kind = default_node_kind(selected_kind);
        *dirty = true;
    }

    ui.separator();
    match &mut node.kind {
        DialogNodeKind::Line { body, next_node_id } => {
            ui.label("Body:");
            *dirty |= ui.text_edit_multiline(body).changed();
            optional_node_picker(
                ui,
                "Next Node:",
                next_node_id,
                &available_node_ids,
                dirty,
                ("line_next_node", node_index),
            );
            render_conditions(
                ui,
                &mut node.conditions,
                dirty,
                ("node_conditions", node_index),
            );
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
            let mut index = 0usize;
            while index < choices.len() {
                let mut delete_choice = false;
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Choice {}", index + 1));
                    if ui
                        .small_button("Delete Choice")
                        .on_hover_text("Remove this choice from the dialog node")
                        .clicked()
                    {
                        delete_choice = true;
                    }
                });
                if delete_choice {
                    choices.remove(index);
                    *dirty = true;
                    continue;
                }
                let choice = &mut choices[index];
                ui.horizontal(|ui| {
                    ui.label("Id:");
                    *dirty |= ui.text_edit_singleline(&mut choice.id).changed();
                });
                ui.horizontal(|ui| {
                    ui.label("Label:");
                    *dirty |= ui.text_edit_singleline(&mut choice.label).changed();
                });
                required_node_picker(
                    ui,
                    "Next Node:",
                    &mut choice.next_node_id,
                    &available_node_ids,
                    dirty,
                    ("choice_next_node", node_index, index),
                );
                render_conditions(
                    ui,
                    &mut choice.conditions,
                    dirty,
                    ("choice_conditions", index),
                );
                index += 1;
            }
        }
        DialogNodeKind::Branch {
            branches,
            default_next_node_id,
        } => {
            optional_node_picker(
                ui,
                "Default Next:",
                default_next_node_id,
                &available_node_ids,
                dirty,
                ("branch_default_next", node_index),
            );
            if ui.small_button("+ Branch").clicked() {
                branches.push(DialogBranch {
                    conditions: Vec::new(),
                    next_node_id: String::new(),
                });
                *dirty = true;
            }
            let mut index = 0usize;
            while index < branches.len() {
                let mut delete_branch = false;
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(format!("Branch {}", index + 1));
                    if ui
                        .small_button("Delete Branch")
                        .on_hover_text("Remove this branch target from the dialog node")
                        .clicked()
                    {
                        delete_branch = true;
                    }
                });
                if delete_branch {
                    branches.remove(index);
                    *dirty = true;
                    continue;
                }
                let branch = &mut branches[index];
                required_node_picker(
                    ui,
                    "Next Node:",
                    &mut branch.next_node_id,
                    &available_node_ids,
                    dirty,
                    ("branch_next_node", node_index, index),
                );
                render_conditions(
                    ui,
                    &mut branch.conditions,
                    dirty,
                    ("branch_conditions", index),
                );
                index += 1;
            }
        }
        DialogNodeKind::End { body, outcome_id } => {
            ui.label("Body:");
            *dirty |= ui.text_edit_multiline(body).changed();
            optional_node_ref_editor(ui, "Outcome Id:", outcome_id, dirty);
            render_conditions(
                ui,
                &mut node.conditions,
                dirty,
                ("node_conditions", node_index),
            );
        }
    }
}

fn delete_selected_node(
    dialog: &mut toki_core::dialog::DialogTree,
    selected_node_id: &mut Option<String>,
    layout: Option<&mut DialogGraphLayout>,
) -> Result<String, String> {
    let Some(selected_node_id_value) = selected_node_id.clone() else {
        return Err("Select a node to delete.".to_string());
    };
    if dialog.nodes.len() <= 1 {
        return Err("Dialogs must keep at least one node.".to_string());
    }
    let Some(node_index) = dialog
        .nodes
        .iter()
        .position(|node| node.id == selected_node_id_value)
    else {
        return Err("Selected node no longer exists.".to_string());
    };

    let deleted_node_id = dialog.nodes[node_index].id.clone();
    let fallback_selection = dialog
        .nodes
        .get(node_index + 1)
        .or_else(|| {
            node_index
                .checked_sub(1)
                .and_then(|index| dialog.nodes.get(index))
        })
        .map(|node| node.id.clone());

    dialog.nodes.remove(node_index);
    *selected_node_id = fallback_selection.clone();
    if let Some(layout) = layout {
        remove_layout_node_key(layout, &deleted_node_id);
    }

    if dialog.entry_node_id == deleted_node_id {
        if let Some(fallback_selection) = fallback_selection {
            dialog.entry_node_id = fallback_selection;
        }
    }

    let mut cleared_optional_refs = 0usize;
    let mut removed_choices = 0usize;
    let mut removed_branches = 0usize;
    for node in &mut dialog.nodes {
        match &mut node.kind {
            DialogNodeKind::Line { next_node_id, .. } => {
                if next_node_id.as_deref() == Some(deleted_node_id.as_str()) {
                    *next_node_id = None;
                    cleared_optional_refs += 1;
                }
            }
            DialogNodeKind::Choice { choices, .. } => {
                let before = choices.len();
                choices.retain(|choice| choice.next_node_id != deleted_node_id);
                removed_choices += before - choices.len();
            }
            DialogNodeKind::Branch {
                branches,
                default_next_node_id,
            } => {
                let before = branches.len();
                branches.retain(|branch| branch.next_node_id != deleted_node_id);
                removed_branches += before - branches.len();
                if default_next_node_id.as_deref() == Some(deleted_node_id.as_str()) {
                    *default_next_node_id = None;
                    cleared_optional_refs += 1;
                }
            }
            DialogNodeKind::End { .. } => {}
        }
    }

    let mut details = Vec::new();
    if cleared_optional_refs > 0 {
        details.push(format!(
            "cleared {cleared_optional_refs} optional reference(s)"
        ));
    }
    if removed_choices > 0 {
        details.push(format!("removed {removed_choices} choice(s)"));
    }
    if removed_branches > 0 {
        details.push(format!("removed {removed_branches} branch target(s)"));
    }
    let suffix = if details.is_empty() {
        String::new()
    } else {
        format!(" and {}", details.join(", "))
    };

    Ok(format!("Deleted node '{deleted_node_id}'{suffix}."))
}

fn rename_dialog_node_id(
    dialog: &mut toki_core::dialog::DialogTree,
    selected_node_id: &mut Option<String>,
    old_node_id: &str,
    new_node_id: &str,
    layout: Option<&mut DialogGraphLayout>,
) -> Result<Option<String>, String> {
    if old_node_id == new_node_id {
        return Ok(None);
    }
    if new_node_id.trim().is_empty() {
        return Err("Node id must not be empty.".to_string());
    }
    if dialog
        .nodes
        .iter()
        .any(|node| node.id == new_node_id && node.id != old_node_id)
    {
        return Err(format!(
            "Dialog already contains a node named '{new_node_id}'."
        ));
    }
    let Some(node) = dialog.nodes.iter_mut().find(|node| node.id == old_node_id) else {
        return Err(format!("Selected node '{old_node_id}' no longer exists."));
    };
    node.id = new_node_id.to_string();

    if dialog.entry_node_id == old_node_id {
        dialog.entry_node_id = new_node_id.to_string();
    }
    for node in &mut dialog.nodes {
        match &mut node.kind {
            DialogNodeKind::Line { next_node_id, .. } => {
                if next_node_id.as_deref() == Some(old_node_id) {
                    *next_node_id = Some(new_node_id.to_string());
                }
            }
            DialogNodeKind::Choice { choices, .. } => {
                for choice in choices {
                    if choice.next_node_id == old_node_id {
                        choice.next_node_id = new_node_id.to_string();
                    }
                }
            }
            DialogNodeKind::Branch {
                branches,
                default_next_node_id,
            } => {
                for branch in branches {
                    if branch.next_node_id == old_node_id {
                        branch.next_node_id = new_node_id.to_string();
                    }
                }
                if default_next_node_id.as_deref() == Some(old_node_id) {
                    *default_next_node_id = Some(new_node_id.to_string());
                }
            }
            DialogNodeKind::End { .. } => {}
        }
    }
    *selected_node_id = Some(new_node_id.to_string());
    if let Some(layout) = layout {
        rename_layout_node_key(layout, old_node_id, new_node_id);
    }

    Ok(Some(format!(
        "Renamed node '{old_node_id}' to '{new_node_id}'."
    )))
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

    let mut index = 0usize;
    while index < conditions.len() {
        let mut delete_condition = false;
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Condition {}", index + 1));
                if ui
                    .small_button("Delete Condition")
                    .on_hover_text("Remove this condition")
                    .clicked()
                {
                    delete_condition = true;
                }
            });
            if delete_condition {
                return;
            }
            let condition = &mut conditions[index];
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
                        DialogConditionKind::FlagEquals,
                        DialogConditionKind::FlagSet,
                        DialogConditionKind::FlagGreaterThan,
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
                        if ui
                            .add(egui::DragValue::new(&mut count).range(0..=9999))
                            .changed()
                        {
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
                DialogCondition::FlagEquals { flag, value } => {
                    *dirty |= render_flag_name(ui, flag);
                    *dirty |= render_flag_value_editor(ui, (&id_salt, "flag_value", index), value);
                }
                DialogCondition::FlagSet { flag } => {
                    *dirty |= render_flag_name(ui, flag);
                }
                DialogCondition::FlagGreaterThan { flag, value } => {
                    *dirty |= render_flag_name(ui, flag);
                    ui.horizontal(|ui| {
                        ui.label("Threshold:");
                        *dirty |= ui.add(egui::DragValue::new(value).speed(1.0)).changed();
                    });
                }
            }
        });
        if delete_condition {
            conditions.remove(index);
            *dirty = true;
            continue;
        }
        index += 1;
    }
}

fn render_flag_name(ui: &mut Ui, flag: &mut String) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Flag:");
        changed |= ui.text_edit_singleline(flag).changed();
    });
    changed
}

fn render_flag_value_editor(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash,
    value: &mut FlagValue,
) -> bool {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum FlagValueKind {
        Bool,
        Int,
        String,
    }

    let mut changed = false;
    let current_kind = match value {
        FlagValue::Bool(_) => FlagValueKind::Bool,
        FlagValue::Int(_) => FlagValueKind::Int,
        FlagValue::String(_) => FlagValueKind::String,
    };
    let mut selected_kind = current_kind;

    egui::ComboBox::from_id_salt((&id_salt, "kind"))
        .selected_text(match current_kind {
            FlagValueKind::Bool => "Bool",
            FlagValueKind::Int => "Int",
            FlagValueKind::String => "String",
        })
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(&mut selected_kind, FlagValueKind::Bool, "Bool")
                .changed();
            changed |= ui
                .selectable_value(&mut selected_kind, FlagValueKind::Int, "Int")
                .changed();
            changed |= ui
                .selectable_value(&mut selected_kind, FlagValueKind::String, "String")
                .changed();
        });

    if selected_kind != current_kind {
        *value = match selected_kind {
            FlagValueKind::Bool => FlagValue::Bool(false),
            FlagValueKind::Int => FlagValue::Int(0),
            FlagValueKind::String => FlagValue::String(String::new()),
        };
    }

    match value {
        FlagValue::Bool(value) => {
            ui.horizontal(|ui| {
                ui.label("Value:");
                changed |= ui.checkbox(value, "Enabled").changed();
            });
        }
        FlagValue::Int(value) => {
            ui.horizontal(|ui| {
                ui.label("Value:");
                changed |= ui.add(egui::DragValue::new(value).speed(1.0)).changed();
            });
        }
        FlagValue::String(value) => {
            ui.horizontal(|ui| {
                ui.label("Value:");
                changed |= ui.text_edit_singleline(value).changed();
            });
        }
    }

    changed
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

fn optional_or_required_node_picker(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    available_node_ids: &[String],
    dirty: &mut bool,
    allow_none: bool,
    id_salt: impl std::hash::Hash,
) {
    let mut selected = value.clone();
    let current_label = if selected.trim().is_empty() {
        if allow_none {
            "None"
        } else {
            "Select node"
        }
    } else {
        selected.as_str()
    }
    .to_string();
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt((id_salt, "node_picker"))
            .selected_text(current_label)
            .show_ui(ui, |ui| {
                if allow_none {
                    ui.selectable_value(&mut selected, String::new(), "None");
                }
                if !selected.trim().is_empty()
                    && !available_node_ids.iter().any(|node| node == &selected)
                {
                    let missing_value = selected.clone();
                    let missing_label = format!("{missing_value} (missing)");
                    ui.selectable_value(&mut selected, missing_value, missing_label);
                }
                for node_id in available_node_ids {
                    ui.selectable_value(&mut selected, node_id.clone(), node_id);
                }
            });
    });
    if *value != selected {
        *value = selected;
        *dirty = true;
    }
}

fn optional_node_picker(
    ui: &mut Ui,
    label: &str,
    value: &mut Option<String>,
    available_node_ids: &[String],
    dirty: &mut bool,
    id_salt: impl std::hash::Hash,
) {
    let mut selected = value.clone().unwrap_or_default();
    optional_or_required_node_picker(
        ui,
        label,
        &mut selected,
        available_node_ids,
        dirty,
        true,
        id_salt,
    );
    if selected.trim().is_empty() {
        *value = None;
    } else {
        *value = Some(selected);
    }
}

fn required_node_picker(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    available_node_ids: &[String],
    dirty: &mut bool,
    id_salt: impl std::hash::Hash,
) {
    optional_or_required_node_picker(ui, label, value, available_node_ids, dirty, false, id_salt);
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
        DialogCondition::FlagEquals { .. } => DialogConditionKind::FlagEquals,
        DialogCondition::FlagSet { .. } => DialogConditionKind::FlagSet,
        DialogCondition::FlagGreaterThan { .. } => DialogConditionKind::FlagGreaterThan,
    }
}

fn condition_kind_label(kind: DialogConditionKind) -> &'static str {
    match kind {
        DialogConditionKind::HealthBelow => "HealthBelow",
        DialogConditionKind::HealthAbove => "HealthAbove",
        DialogConditionKind::HasInventoryItem => "HasInventoryItem",
        DialogConditionKind::EntityHasTag => "EntityHasTag",
        DialogConditionKind::EntityIsKind => "EntityIsKind",
        DialogConditionKind::FlagEquals => "FlagEquals",
        DialogConditionKind::FlagSet => "FlagSet",
        DialogConditionKind::FlagGreaterThan => "FlagGreaterThan",
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
        DialogConditionKind::FlagEquals => DialogCondition::FlagEquals {
            flag: String::new(),
            value: FlagValue::Bool(false),
        },
        DialogConditionKind::FlagSet => DialogCondition::FlagSet {
            flag: String::new(),
        },
        DialogConditionKind::FlagGreaterThan => DialogCondition::FlagGreaterThan {
            flag: String::new(),
            value: 0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::DialogGraphLayout;
    use std::collections::HashMap;

    #[test]
    fn delete_selected_node_reselects_neighbor_and_repairs_entry_node() {
        let mut dialog = toki_core::dialog::DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: String::new(),
                        next_node_id: Some("middle".to_string()),
                    },
                },
                DialogNode {
                    id: "middle".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };
        let mut selected = Some("start".to_string());

        let status =
            delete_selected_node(&mut dialog, &mut selected, None).expect("delete succeeds");

        assert_eq!(dialog.entry_node_id, "middle");
        assert_eq!(selected.as_deref(), Some("middle"));
        assert_eq!(dialog.nodes.len(), 1);
        assert_eq!(dialog.nodes[0].id, "middle");
        assert!(status.contains("Deleted node 'start'"));
    }

    #[test]
    fn delete_selected_node_removes_layout_entry() {
        let mut dialog = toki_core::dialog::DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: String::new(),
                        next_node_id: Some("end".to_string()),
                    },
                },
                DialogNode {
                    id: "end".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };
        let mut selected = Some("start".to_string());
        let mut layout = DialogGraphLayout {
            node_positions: HashMap::from([
                ("start".to_string(), [10.0, 20.0]),
                ("end".to_string(), [50.0, 80.0]),
            ]),
            ..DialogGraphLayout::default()
        };

        let _ = delete_selected_node(&mut dialog, &mut selected, Some(&mut layout))
            .expect("delete succeeds");

        assert!(!layout.node_positions.contains_key("start"));
        assert!(layout.node_positions.contains_key("end"));
    }

    #[test]
    fn delete_selected_node_cleans_references_to_deleted_node() {
        let mut dialog = toki_core::dialog::DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "line".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "line".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: String::new(),
                        next_node_id: Some("target".to_string()),
                    },
                },
                DialogNode {
                    id: "choice".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Choice {
                        body: String::new(),
                        choices: vec![
                            DialogChoice {
                                id: "remove".to_string(),
                                label: String::new(),
                                next_node_id: "target".to_string(),
                                conditions: Vec::new(),
                            },
                            DialogChoice {
                                id: "keep".to_string(),
                                label: String::new(),
                                next_node_id: "line".to_string(),
                                conditions: Vec::new(),
                            },
                        ],
                    },
                },
                DialogNode {
                    id: "branch".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Branch {
                        branches: vec![
                            DialogBranch {
                                conditions: Vec::new(),
                                next_node_id: "target".to_string(),
                            },
                            DialogBranch {
                                conditions: Vec::new(),
                                next_node_id: "line".to_string(),
                            },
                        ],
                        default_next_node_id: Some("target".to_string()),
                    },
                },
                DialogNode {
                    id: "target".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };
        let mut selected = Some("target".to_string());

        let status =
            delete_selected_node(&mut dialog, &mut selected, None).expect("delete succeeds");

        let line = dialog.node("line").expect("line survives");
        assert_eq!(
            line,
            &DialogNode {
                id: "line".to_string(),
                speaker_name: None,
                conditions: Vec::new(),
                kind: DialogNodeKind::Line {
                    body: String::new(),
                    next_node_id: None,
                },
            }
        );
        let choice = dialog.node("choice").expect("choice survives");
        let DialogNodeKind::Choice { choices, .. } = &choice.kind else {
            panic!("expected choice node");
        };
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].id, "keep");
        let branch = dialog.node("branch").expect("branch survives");
        let DialogNodeKind::Branch {
            branches,
            default_next_node_id,
        } = &branch.kind
        else {
            panic!("expected branch node");
        };
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].next_node_id, "line");
        assert_eq!(default_next_node_id, &None);
        assert!(status.contains("cleared 2 optional reference(s)"));
        assert!(status.contains("removed 1 choice(s)"));
        assert!(status.contains("removed 1 branch target(s)"));
    }

    #[test]
    fn delete_selected_node_rejects_last_remaining_node() {
        let mut dialog = toki_core::dialog::DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "only".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![DialogNode {
                id: "only".to_string(),
                speaker_name: None,
                conditions: Vec::new(),
                kind: DialogNodeKind::End {
                    body: String::new(),
                    outcome_id: None,
                },
            }],
        };
        let mut selected = Some("only".to_string());

        let error =
            delete_selected_node(&mut dialog, &mut selected, None).expect_err("delete fails");

        assert_eq!(error, "Dialogs must keep at least one node.");
        assert_eq!(dialog.nodes.len(), 1);
        assert_eq!(selected.as_deref(), Some("only"));
    }

    #[test]
    fn rename_dialog_node_id_updates_selection_entry_and_references() {
        let mut dialog = toki_core::dialog::DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: String::new(),
                        next_node_id: Some("target".to_string()),
                    },
                },
                DialogNode {
                    id: "choice".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Choice {
                        body: String::new(),
                        choices: vec![DialogChoice {
                            id: "next".to_string(),
                            label: String::new(),
                            next_node_id: "target".to_string(),
                            conditions: Vec::new(),
                        }],
                    },
                },
                DialogNode {
                    id: "branch".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Branch {
                        branches: vec![DialogBranch {
                            conditions: Vec::new(),
                            next_node_id: "target".to_string(),
                        }],
                        default_next_node_id: Some("target".to_string()),
                    },
                },
                DialogNode {
                    id: "target".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };
        let mut selected = Some("target".to_string());

        let status = rename_dialog_node_id(&mut dialog, &mut selected, "target", "done", None)
            .expect("rename succeeds");

        assert_eq!(selected.as_deref(), Some("done"));
        assert!(status
            .expect("status")
            .contains("Renamed node 'target' to 'done'."));
        assert!(dialog.node("target").is_none());
        assert!(dialog.node("done").is_some());
        let line = dialog.node("start").expect("line");
        let DialogNodeKind::Line { next_node_id, .. } = &line.kind else {
            panic!("expected line");
        };
        assert_eq!(next_node_id.as_deref(), Some("done"));
        let choice = dialog.node("choice").expect("choice");
        let DialogNodeKind::Choice { choices, .. } = &choice.kind else {
            panic!("expected choice");
        };
        assert_eq!(choices[0].next_node_id, "done");
        let branch = dialog.node("branch").expect("branch");
        let DialogNodeKind::Branch {
            branches,
            default_next_node_id,
        } = &branch.kind
        else {
            panic!("expected branch");
        };
        assert_eq!(branches[0].next_node_id, "done");
        assert_eq!(default_next_node_id.as_deref(), Some("done"));
    }

    #[test]
    fn rename_dialog_node_id_migrates_layout_key() {
        let mut dialog = toki_core::dialog::DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: String::new(),
                        next_node_id: Some("end".to_string()),
                    },
                },
                DialogNode {
                    id: "end".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };
        let mut selected = Some("start".to_string());
        let mut layout = DialogGraphLayout {
            node_positions: HashMap::from([("start".to_string(), [10.0, 20.0])]),
            ..DialogGraphLayout::default()
        };

        let _ = rename_dialog_node_id(
            &mut dialog,
            &mut selected,
            "start",
            "entry",
            Some(&mut layout),
        )
        .expect("rename succeeds");

        assert!(!layout.node_positions.contains_key("start"));
        assert_eq!(
            layout.node_positions.get("entry").copied(),
            Some([10.0, 20.0])
        );
    }

    #[test]
    fn rename_dialog_node_id_rejects_empty_and_duplicate_ids() {
        let mut dialog = toki_core::dialog::DialogTree {
            id: "intro".to_string().into(),
            title: String::new(),
            entry_node_id: "start".to_string(),
            allow_cancel: true,
            gate_gameplay: true,
            nodes: vec![
                DialogNode {
                    id: "start".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::Line {
                        body: String::new(),
                        next_node_id: Some("end".to_string()),
                    },
                },
                DialogNode {
                    id: "end".to_string(),
                    speaker_name: None,
                    conditions: Vec::new(),
                    kind: DialogNodeKind::End {
                        body: String::new(),
                        outcome_id: None,
                    },
                },
            ],
        };
        let mut selected = Some("start".to_string());

        let empty_error = rename_dialog_node_id(&mut dialog, &mut selected, "start", "", None)
            .expect_err("empty");
        assert_eq!(empty_error, "Node id must not be empty.");

        let duplicate_error =
            rename_dialog_node_id(&mut dialog, &mut selected, "start", "end", None)
                .expect_err("dup");
        assert_eq!(
            duplicate_error,
            "Dialog already contains a node named 'end'."
        );
        assert_eq!(selected.as_deref(), Some("start"));
    }
}
