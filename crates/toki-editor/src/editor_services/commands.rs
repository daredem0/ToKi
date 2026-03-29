use crate::project::Project;
use crate::ui::EditorUI;
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
    if ui_state.with_active_context(|context, shell| context.can_undo(shell)) {
        return ui_state.with_active_context(|context, shell| context.undo(shell, None));
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let undone = history.undo(ui_state, None);
    ui_state.command_history = history;
    undone
}

pub fn undo_with_project(ui_state: &mut EditorUI, project: &mut Project) -> bool {
    if ui_state.with_active_context(|context, shell| context.can_undo(shell)) {
        return ui_state.with_active_context(|context, shell| context.undo(shell, Some(project)));
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let undone = history.undo(ui_state, Some(project));
    ui_state.command_history = history;
    undone
}

pub fn redo(ui_state: &mut EditorUI) -> bool {
    if ui_state.with_active_context(|context, shell| context.can_redo(shell)) {
        return ui_state.with_active_context(|context, shell| context.redo(shell, None));
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let redone = history.redo(ui_state, None);
    ui_state.command_history = history;
    redone
}

pub fn redo_with_project(ui_state: &mut EditorUI, project: &mut Project) -> bool {
    if ui_state.with_active_context(|context, shell| context.can_redo(shell)) {
        return ui_state.with_active_context(|context, shell| context.redo(shell, Some(project)));
    }

    let mut history = std::mem::take(&mut ui_state.command_history);
    let redone = history.redo(ui_state, Some(project));
    ui_state.command_history = history;
    redone
}

pub fn can_undo(ui_state: &EditorUI) -> bool {
    ui_state.active_context_ref().can_undo(ui_state) || ui_state.command_history.can_undo()
}

pub fn can_redo(ui_state: &EditorUI) -> bool {
    ui_state.active_context_ref().can_redo(ui_state) || ui_state.command_history.can_redo()
}
