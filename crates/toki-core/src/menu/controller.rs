//! Menu controller and navigation.

use std::collections::HashMap;

use crate::ui::{UiAction, UiCommand};

use super::types::{
    InventoryEntry, MenuDialogDefinition, MenuDialogView, MenuInput, MenuItemDefinition,
    MenuListSource, MenuScreenDefinition, MenuSettings, MenuView, MenuViewEntry,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveDialogState {
    dialog_id: String,
    confirm_selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuController {
    settings: MenuSettings,
    screen_map: HashMap<String, MenuScreenDefinition>,
    dialog_map: HashMap<String, MenuDialogDefinition>,
    stack: Vec<String>,
    selected_index_by_screen: HashMap<String, usize>,
    active_dialog: Option<ActiveDialogState>,
}

impl MenuController {
    pub fn new(settings: MenuSettings) -> Self {
        let screen_map = settings
            .screens
            .iter()
            .cloned()
            .map(|screen| (screen.id.clone(), screen))
            .collect::<HashMap<_, _>>();
        let dialog_map = settings
            .dialogs
            .iter()
            .cloned()
            .map(|dialog| (dialog.id.clone(), dialog))
            .collect::<HashMap<_, _>>();
        Self {
            settings,
            screen_map,
            dialog_map,
            stack: Vec::new(),
            selected_index_by_screen: HashMap::new(),
            active_dialog: None,
        }
    }

    pub fn settings(&self) -> &MenuSettings {
        &self.settings
    }

    pub fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn open(&mut self) {
        if self.stack.is_empty() {
            self.stack.push(self.settings.pause_root_screen_id.clone());
        }
    }

    pub fn close(&mut self) {
        self.stack.clear();
        self.active_dialog = None;
    }

    /// Opens the pause root menu screen.
    pub fn open_pause_root(&mut self) {
        self.stack.clear();
        self.stack.push(self.settings.pause_root_screen_id.clone());
        self.active_dialog = None;
    }

    /// Returns whether a dialog is currently active.
    pub fn is_dialog_open(&self) -> bool {
        self.active_dialog.is_some()
    }

    pub fn current_view(&self, inventory: &[InventoryEntry]) -> Option<MenuView> {
        let screen_id = self.stack.last()?;
        let screen = self.screen_map.get(screen_id)?;

        // Get stored selection or default to first selectable item
        let selected_index = self
            .selected_index_by_screen
            .get(screen_id)
            .copied()
            .or_else(|| self.first_selectable_index(screen_id))
            .unwrap_or(0);

        let mut entries: Vec<MenuViewEntry> = Vec::new();

        for (item_index, item) in screen.items.iter().enumerate() {
            match item {
                MenuItemDefinition::Label {
                    text,
                    border_style_override,
                } => {
                    entries.push(MenuViewEntry {
                        text: text.clone(),
                        selected: false,
                        selectable: false,
                        border_style_override: *border_style_override,
                    });
                }
                MenuItemDefinition::Button {
                    text,
                    border_style_override,
                    ..
                } => {
                    entries.push(MenuViewEntry {
                        text: text.clone(),
                        selected: item_index == selected_index,
                        selectable: true,
                        border_style_override: *border_style_override,
                    });
                }
                MenuItemDefinition::DynamicList {
                    heading,
                    source,
                    empty_text,
                    border_style_override,
                } => {
                    // Add heading if present
                    if let Some(heading_text) = heading {
                        entries.push(MenuViewEntry {
                            text: heading_text.clone(),
                            selected: false,
                            selectable: false,
                            border_style_override: *border_style_override,
                        });
                    }

                    // Expand inventory items
                    match source {
                        MenuListSource::PlayerInventory => {
                            if inventory.is_empty() {
                                entries.push(MenuViewEntry {
                                    text: empty_text.clone(),
                                    selected: false,
                                    selectable: false,
                                    border_style_override: *border_style_override,
                                });
                            } else {
                                for inv_entry in inventory {
                                    entries.push(MenuViewEntry {
                                        text: format!("{} x{}", inv_entry.item_id, inv_entry.count),
                                        selected: false,
                                        selectable: false,
                                        border_style_override: *border_style_override,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Some(MenuView {
            screen_id: screen.id.clone(),
            title: screen.title.clone(),
            title_border_style_override: screen.title_border_style_override,
            entries,
        })
    }

    pub fn current_dialog_view(&self) -> Option<MenuDialogView> {
        let active = self.active_dialog.as_ref()?;
        let dialog = self.dialog_map.get(&active.dialog_id)?;
        Some(MenuDialogView {
            dialog_id: dialog.id.clone(),
            title: dialog.title.clone(),
            body: dialog.body.clone(),
            confirm_text: dialog.confirm_text.clone(),
            cancel_text: dialog.cancel_text.clone(),
            confirm_selected: active.confirm_selected,
            hide_main_menu: dialog.hide_main_menu,
        })
    }

    pub fn handle_input(&mut self, input: MenuInput) -> Option<UiCommand> {
        if self.active_dialog.is_some() {
            return self.handle_dialog_input(input);
        }

        let current_screen_id = self.stack.last().cloned()?;

        match input {
            MenuInput::Up => {
                self.move_selection(&current_screen_id, -1);
                None
            }
            MenuInput::Down => {
                self.move_selection(&current_screen_id, 1);
                None
            }
            MenuInput::Confirm => self.confirm_current_selection(&current_screen_id),
            MenuInput::Back => {
                self.go_back();
                None
            }
        }
    }

    fn handle_dialog_input(&mut self, input: MenuInput) -> Option<UiCommand> {
        match input {
            MenuInput::Up | MenuInput::Down => {
                if let Some(active_dialog) = &mut self.active_dialog {
                    active_dialog.confirm_selected = !active_dialog.confirm_selected;
                }
                None
            }
            MenuInput::Confirm => {
                let active_dialog = self.active_dialog.clone()?;
                let dialog = self.dialog_map.get(&active_dialog.dialog_id)?.clone();
                let action = if active_dialog.confirm_selected {
                    dialog.confirm_action
                } else {
                    dialog.cancel_action
                };
                self.apply_action(&action)
            }
            MenuInput::Back => {
                let active_dialog = self.active_dialog.clone()?;
                let dialog = self.dialog_map.get(&active_dialog.dialog_id)?.clone();
                self.apply_action(&dialog.cancel_action)
            }
        }
    }

    fn go_back(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        } else {
            self.close();
        }
    }

    fn confirm_current_selection(&mut self, current_screen_id: &str) -> Option<UiCommand> {
        let selected_index = *self
            .selected_index_by_screen
            .get(current_screen_id)
            .unwrap_or(&0);
        let action = {
            let screen = self.screen_map.get(current_screen_id)?;
            let Some(MenuItemDefinition::Button { action, .. }) = screen.items.get(selected_index)
            else {
                return None;
            };
            action.clone()
        };

        self.apply_action(&action)
    }

    fn apply_action(&mut self, action: &UiAction) -> Option<UiCommand> {
        match action {
            UiAction::CloseUi => {
                self.close();
                None
            }
            UiAction::CloseSurface => {
                if self.active_dialog.is_some() {
                    self.active_dialog = None;
                } else if self.stack.len() > 1 {
                    self.stack.pop();
                } else {
                    self.close();
                }
                None
            }
            UiAction::OpenSurface { surface_id } => {
                if self.dialog_map.contains_key(surface_id) {
                    self.active_dialog = Some(ActiveDialogState {
                        dialog_id: surface_id.clone(),
                        confirm_selected: true,
                    });
                } else if self.screen_map.contains_key(surface_id) {
                    self.active_dialog = None;
                    self.stack.push(surface_id.clone());
                }
                None
            }
            UiAction::Back => {
                if self.active_dialog.is_some() {
                    self.active_dialog = None;
                } else {
                    self.go_back();
                }
                None
            }
            UiAction::ExitRuntime => Some(UiCommand::ExitRuntime),
            UiAction::EmitEvent { event_id } => Some(UiCommand::EmitEvent {
                event_id: event_id.clone(),
            }),
        }
    }

    fn move_selection(&mut self, screen_id: &str, direction: isize) {
        let Some(screen) = self.screen_map.get(screen_id) else {
            return;
        };
        let selectable_indices = screen
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item {
                MenuItemDefinition::Button { .. } => Some(index),
                MenuItemDefinition::Label { .. } | MenuItemDefinition::DynamicList { .. } => None,
            })
            .collect::<Vec<_>>();

        if selectable_indices.is_empty() {
            return;
        }

        let current_index = *self.selected_index_by_screen.get(screen_id).unwrap_or(&0);
        let current_pos = selectable_indices
            .iter()
            .position(|&index| index == current_index)
            .unwrap_or(0);
        let next_pos = ((current_pos as isize + direction)
            .rem_euclid(selectable_indices.len() as isize)) as usize;
        self.selected_index_by_screen
            .insert(screen_id.to_string(), selectable_indices[next_pos]);
    }

    #[allow(dead_code)]
    fn first_selectable_index(&self, screen_id: &str) -> Option<usize> {
        self.screen_map.get(screen_id).and_then(|screen| {
            screen
                .items
                .iter()
                .position(|item| matches!(item, MenuItemDefinition::Button { .. }))
        })
    }
}
