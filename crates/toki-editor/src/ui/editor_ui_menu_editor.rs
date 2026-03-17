use super::{EditorUI, Selection};
use crate::project::Project;

impl EditorUI {
    pub fn sync_menu_editor_selection(&mut self, project: Option<&Project>) {
        let Some(project) = project else {
            self.clear_menu_editor_selection();
            return;
        };

        let screens = &project.metadata.runtime.menu.screens;
        if screens.is_empty() {
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
            _ => {}
        }

        let first_screen_id = screens[0].id.clone();
        self.selection = Some(Selection::MenuScreen(first_screen_id));
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
    }

    pub fn clear_menu_editor_selection(&mut self) {
        if matches!(
            self.selection,
            Some(Selection::MenuScreen(_)) | Some(Selection::MenuEntry { .. })
        ) {
            self.selection = None;
        }
    }

    pub fn select_menu_screen(&mut self, screen_id: impl Into<String>) {
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
        self.selection = Some(Selection::MenuScreen(screen_id.into()));
    }

    pub fn select_menu_entry(&mut self, screen_id: impl Into<String>, item_index: usize) {
        self.selected_entity_id = None;
        self.selected_entity_ids.clear();
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
}
