use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuSettings {
    #[serde(default = "default_pause_root_screen_id")]
    pub pause_root_screen_id: String,
    #[serde(default = "default_gate_gameplay_when_open")]
    pub gate_gameplay_when_open: bool,
    #[serde(default = "default_menu_screens")]
    pub screens: Vec<MenuScreenDefinition>,
}

impl Default for MenuSettings {
    fn default() -> Self {
        Self {
            pause_root_screen_id: default_pause_root_screen_id(),
            gate_gameplay_when_open: default_gate_gameplay_when_open(),
            screens: default_menu_screens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuScreenDefinition {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub items: Vec<MenuItemDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MenuItemDefinition {
    Label {
        text: String,
    },
    Button {
        text: String,
        action: MenuAction,
    },
    DynamicList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        heading: Option<String>,
        source: MenuListSource,
        empty_text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MenuAction {
    CloseMenu,
    OpenScreen { screen_id: String },
    Back,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenuListSource {
    PlayerInventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    Up,
    Down,
    Confirm,
    Back,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuView {
    pub screen_id: String,
    pub title: String,
    pub entries: Vec<MenuViewEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuViewEntry {
    pub text: String,
    pub selected: bool,
    pub selectable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEntry {
    pub item_id: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuController {
    settings: MenuSettings,
    screen_map: HashMap<String, MenuScreenDefinition>,
    stack: Vec<String>,
    selected_index_by_screen: HashMap<String, usize>,
}

impl MenuController {
    pub fn new(settings: MenuSettings) -> Self {
        let screen_map = settings
            .screens
            .iter()
            .cloned()
            .map(|screen| (screen.id.clone(), screen))
            .collect::<HashMap<_, _>>();
        let mut controller = Self {
            settings,
            screen_map,
            stack: Vec::new(),
            selected_index_by_screen: HashMap::new(),
        };
        for screen_id in controller.screen_map.keys().cloned().collect::<Vec<_>>() {
            let selected = controller.first_selectable_index(&screen_id).unwrap_or(0);
            controller
                .selected_index_by_screen
                .insert(screen_id, selected);
        }
        controller
    }

    pub fn settings(&self) -> &MenuSettings {
        &self.settings
    }

    pub fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn open_pause_root(&mut self) {
        let root = self.settings.pause_root_screen_id.clone();
        if self.screen_map.contains_key(&root) {
            self.stack.clear();
            self.stack.push(root);
        }
    }

    pub fn close(&mut self) {
        self.stack.clear();
    }

    pub fn current_screen_id(&self) -> Option<&str> {
        self.stack.last().map(String::as_str)
    }

    pub fn current_view(&self, inventory: &[InventoryEntry]) -> Option<MenuView> {
        let screen_id = self.current_screen_id()?.to_string();
        let screen = self.screen_map.get(&screen_id)?;
        let selected_index = *self.selected_index_by_screen.get(&screen_id).unwrap_or(&0);
        let mut entries = Vec::new();
        for (index, item) in screen.items.iter().enumerate() {
            match item {
                MenuItemDefinition::Label { text } => entries.push(MenuViewEntry {
                    text: text.clone(),
                    selected: false,
                    selectable: false,
                }),
                MenuItemDefinition::Button { text, .. } => entries.push(MenuViewEntry {
                    text: text.clone(),
                    selected: index == selected_index,
                    selectable: true,
                }),
                MenuItemDefinition::DynamicList {
                    heading,
                    source,
                    empty_text,
                } => {
                    if let Some(heading) = heading {
                        entries.push(MenuViewEntry {
                            text: heading.clone(),
                            selected: false,
                            selectable: false,
                        });
                    }
                    let dynamic_entries = match source {
                        MenuListSource::PlayerInventory => inventory
                            .iter()
                            .map(|entry| MenuViewEntry {
                                text: format!("{} x{}", entry.item_id, entry.count),
                                selected: false,
                                selectable: false,
                            })
                            .collect::<Vec<_>>(),
                    };
                    if dynamic_entries.is_empty() {
                        entries.push(MenuViewEntry {
                            text: empty_text.clone(),
                            selected: false,
                            selectable: false,
                        });
                    } else {
                        entries.extend(dynamic_entries);
                    }
                }
            }
        }
        Some(MenuView {
            screen_id,
            title: screen.title.clone(),
            entries,
        })
    }

    pub fn handle_input(&mut self, input: MenuInput) {
        let Some(current_screen_id) = self.current_screen_id().map(str::to_string) else {
            return;
        };

        match input {
            MenuInput::Up => self.move_selection(&current_screen_id, -1),
            MenuInput::Down => self.move_selection(&current_screen_id, 1),
            MenuInput::Confirm => self.confirm_current_selection(&current_screen_id),
            MenuInput::Back => self.go_back(),
        }
    }

    fn go_back(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        } else {
            self.close();
        }
    }

    fn confirm_current_selection(&mut self, current_screen_id: &str) {
        let Some(screen) = self.screen_map.get(current_screen_id) else {
            return;
        };
        let selected_index = *self
            .selected_index_by_screen
            .get(current_screen_id)
            .unwrap_or(&0);
        let Some(MenuItemDefinition::Button { action, .. }) = screen.items.get(selected_index)
        else {
            return;
        };

        match action {
            MenuAction::CloseMenu => self.close(),
            MenuAction::OpenScreen { screen_id } => {
                if self.screen_map.contains_key(screen_id) {
                    self.stack.push(screen_id.clone());
                }
            }
            MenuAction::Back => self.go_back(),
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
        let next_pos = ((current_pos as isize + direction).rem_euclid(selectable_indices.len() as isize))
            as usize;
        self.selected_index_by_screen
            .insert(screen_id.to_string(), selectable_indices[next_pos]);
    }

    fn first_selectable_index(&self, screen_id: &str) -> Option<usize> {
        self.screen_map.get(screen_id).and_then(|screen| {
            screen
                .items
                .iter()
                .position(|item| matches!(item, MenuItemDefinition::Button { .. }))
        })
    }
}

fn default_pause_root_screen_id() -> String {
    "pause_menu".to_string()
}

fn default_gate_gameplay_when_open() -> bool {
    true
}

fn default_menu_screens() -> Vec<MenuScreenDefinition> {
    vec![
        MenuScreenDefinition {
            id: "pause_menu".to_string(),
            title: "Paused".to_string(),
            items: vec![
                MenuItemDefinition::Button {
                    text: "Resume".to_string(),
                    action: MenuAction::CloseMenu,
                },
                MenuItemDefinition::Button {
                    text: "Inventory".to_string(),
                    action: MenuAction::OpenScreen {
                        screen_id: "inventory_menu".to_string(),
                    },
                },
            ],
        },
        MenuScreenDefinition {
            id: "inventory_menu".to_string(),
            title: "Inventory".to_string(),
            items: vec![
                MenuItemDefinition::DynamicList {
                    heading: Some("Items".to_string()),
                    source: MenuListSource::PlayerInventory,
                    empty_text: "Inventory is empty".to_string(),
                },
                MenuItemDefinition::Button {
                    text: "Back".to_string(),
                    action: MenuAction::Back,
                },
            ],
        },
    ]
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
