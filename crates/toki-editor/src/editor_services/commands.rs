use crate::project::Project;
use crate::ui::editor_ui::{CenterPanelTab, EditorUI};
use crate::ui::undo_redo::EditorCommand;

pub fn execute(ui_state: &mut EditorUI, command: EditorCommand) -> bool {
    let mut history = std::mem::take(&mut ui_state.command_history);
    let changed = history.execute(command, ui_state, None);
    ui_state.command_history = history;
    changed
}

pub fn execute_with_project(
    ui_state: &mut EditorUI,
    project: &mut Project,
    command: EditorCommand,
) -> bool {
    let mut history = std::mem::take(&mut ui_state.command_history);
    let changed = history.execute(command, ui_state, Some(project));
    ui_state.command_history = history;
    changed
}

pub fn undo(ui_state: &mut EditorUI) -> bool {
    if ui_state.center_panel_tab == CenterPanelTab::MapEditor && ui_state.map.history.can_undo() {
        let mut history = std::mem::take(&mut ui_state.map.history);
        let undone = history.undo(ui_state);
        ui_state.map.history = history;
        return undone;
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let undone = history.undo(ui_state, None);
    ui_state.command_history = history;
    undone
}

pub fn undo_with_project(ui_state: &mut EditorUI, project: &mut Project) -> bool {
    if ui_state.center_panel_tab == CenterPanelTab::MapEditor && ui_state.map.history.can_undo() {
        let mut history = std::mem::take(&mut ui_state.map.history);
        let undone = history.undo(ui_state);
        ui_state.map.history = history;
        return undone;
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let undone = history.undo(ui_state, Some(project));
    ui_state.command_history = history;
    undone
}

pub fn redo(ui_state: &mut EditorUI) -> bool {
    if ui_state.center_panel_tab == CenterPanelTab::MapEditor && ui_state.map.history.can_redo() {
        let mut history = std::mem::take(&mut ui_state.map.history);
        let redone = history.redo(ui_state);
        ui_state.map.history = history;
        return redone;
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let redone = history.redo(ui_state, None);
    ui_state.command_history = history;
    redone
}

pub fn redo_with_project(ui_state: &mut EditorUI, project: &mut Project) -> bool {
    if ui_state.center_panel_tab == CenterPanelTab::MapEditor && ui_state.map.history.can_redo() {
        let mut history = std::mem::take(&mut ui_state.map.history);
        let redone = history.redo(ui_state);
        ui_state.map.history = history;
        return redone;
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let redone = history.redo(ui_state, Some(project));
    ui_state.command_history = history;
    redone
}

pub fn can_undo(ui_state: &EditorUI) -> bool {
    if ui_state.center_panel_tab == CenterPanelTab::MapEditor {
        ui_state.map.history.can_undo()
    } else {
        ui_state.command_history.can_undo()
    }
}

pub fn can_redo(ui_state: &EditorUI) -> bool {
    if ui_state.center_panel_tab == CenterPanelTab::MapEditor {
        ui_state.map.history.can_redo()
    } else {
        ui_state.command_history.can_redo()
    }
}
