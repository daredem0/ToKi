use super::{EditorUI, Selection};
use crate::project::Project;

impl EditorUI {
    pub fn sync_menu_editor_selection(&mut self, project: Option<&Project>) {
        let Some(project) = project else {
            self.clear_menu_editor_selection();
            return;
        };

        let screens = &project.metadata.runtime.menu.screens;
        let dialogs = &project.metadata.runtime.menu.dialogs;
        if screens.is_empty() && dialogs.is_empty() {
            self.clear_menu_editor_selection();
            return;
        }

        match self.selection.clone() {
            Some(Selection::MenuEntry {
                screen_id,
                item_index,
            }) => {
                if let Some(screen) = screens.iter().find(|screen| screen.id == screen_id) {
                    if item_index < screen.items.len() {
                        return;
                    }
                    self.selection = Some(Selection::MenuScreen(screen_id));
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
            self.selection = Some(Selection::MenuScreen(first_screen_id));
        } else if let Some(first_dialog_id) = dialogs.first().map(|dialog| dialog.id.clone()) {
            self.selection = Some(Selection::MenuDialog(first_dialog_id));
        }
        self.clear_entity_selection_state();
    }

    pub fn clear_menu_editor_selection(&mut self) {
        if matches!(
            self.selection,
            Some(Selection::MenuScreen(_))
                | Some(Selection::MenuDialog(_))
                | Some(Selection::MenuEntry { .. })
        ) {
            self.selection = None;
        }
    }

    pub fn select_menu_screen(&mut self, screen_id: impl Into<String>) {
        self.clear_entity_selection_state();
        self.selection = Some(Selection::MenuScreen(screen_id.into()));
    }

    pub fn select_menu_dialog(&mut self, dialog_id: impl Into<String>) {
        self.clear_entity_selection_state();
        self.selection = Some(Selection::MenuDialog(dialog_id.into()));
    }

    pub fn select_menu_entry(&mut self, screen_id: impl Into<String>, item_index: usize) {
        self.clear_entity_selection_state();
        self.selection = Some(Selection::MenuEntry {
            screen_id: screen_id.into(),
            item_index,
        });
    }

    pub fn selected_menu_screen_id(&self) -> Option<&str> {
        match self.selection.as_ref() {
            Some(Selection::MenuScreen(screen_id)) => Some(screen_id.as_str()),
            Some(Selection::MenuEntry { screen_id, .. }) => Some(screen_id.as_str()),
            _ => None,
        }
    }

    pub fn selected_menu_dialog_id(&self) -> Option<&str> {
        match self.selection.as_ref() {
            Some(Selection::MenuDialog(dialog_id)) => Some(dialog_id.as_str()),
            _ => None,
        }
    }
}
