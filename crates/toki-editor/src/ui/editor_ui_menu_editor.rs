use super::{EditorUI, Selection};
use crate::project::Project;

pub(crate) fn sync_menu_editor_selection(ui_state: &mut EditorUI, project: Option<&Project>) {
    let Some(project) = project else {
        clear_menu_editor_selection(ui_state);
        return;
    };

    let screens = &project.metadata.runtime.menu.screens;
    let dialogs = &project.metadata.runtime.menu.dialogs;
    if screens.is_empty() && dialogs.is_empty() {
        clear_menu_editor_selection(ui_state);
        return;
    }

    match ui_state.selection.clone() {
        Some(Selection::MenuEntry {
            screen_id,
            item_index,
        }) => {
            if let Some(screen) = screens.iter().find(|screen| screen.id == screen_id) {
                if item_index < screen.items.len() {
                    return;
                }
                ui_state.selection = Some(Selection::MenuScreen(screen_id));
                return;
            }
        }
        Some(Selection::MenuScreen(screen_id)) => {
            if screens.iter().any(|screen| screen.id == screen_id) {
                return;
            }
        }
        Some(Selection::MenuDialog(dialog_id)) => {
            if dialogs.iter().any(|dialog| dialog.id == dialog_id) {
                return;
            }
        }
        _ => {}
    }

    if let Some(first_screen_id) = screens.first().map(|screen| screen.id.clone()) {
        ui_state.selection = Some(Selection::MenuScreen(first_screen_id));
    } else if let Some(first_dialog_id) = dialogs.first().map(|dialog| dialog.id.clone()) {
        ui_state.selection = Some(Selection::MenuDialog(first_dialog_id));
    }
    ui_state.clear_entity_selection_state();
}

pub(crate) fn clear_menu_editor_selection(ui_state: &mut EditorUI) {
    if matches!(
        ui_state.selection,
        Some(Selection::MenuScreen(_))
            | Some(Selection::MenuDialog(_))
            | Some(Selection::MenuEntry { .. })
    ) {
        ui_state.selection = None;
    }
}

pub(crate) fn select_menu_screen(ui_state: &mut EditorUI, screen_id: impl Into<String>) {
    ui_state.clear_entity_selection_state();
    ui_state.selection = Some(Selection::MenuScreen(screen_id.into()));
}

pub(crate) fn select_menu_dialog(ui_state: &mut EditorUI, dialog_id: impl Into<String>) {
    ui_state.clear_entity_selection_state();
    ui_state.selection = Some(Selection::MenuDialog(dialog_id.into()));
}

pub(crate) fn select_menu_entry(
    ui_state: &mut EditorUI,
    screen_id: impl Into<String>,
    item_index: usize,
) {
    ui_state.clear_entity_selection_state();
    ui_state.selection = Some(Selection::MenuEntry {
        screen_id: screen_id.into(),
        item_index,
    });
}

pub(crate) fn selected_menu_screen_id(ui_state: &EditorUI) -> Option<&str> {
    match ui_state.selection.as_ref() {
        Some(Selection::MenuScreen(screen_id)) => Some(screen_id.as_str()),
        Some(Selection::MenuEntry { screen_id, .. }) => Some(screen_id.as_str()),
        _ => None,
    }
}

pub(crate) fn selected_menu_dialog_id(ui_state: &EditorUI) -> Option<&str> {
    match ui_state.selection.as_ref() {
        Some(Selection::MenuDialog(dialog_id)) => Some(dialog_id.as_str()),
        _ => None,
    }
}
