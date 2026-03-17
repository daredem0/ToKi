use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuSettings {
    #[serde(default = "default_pause_root_screen_id")]
    pub pause_root_screen_id: String,
    #[serde(default = "default_gate_gameplay_when_open")]
    pub gate_gameplay_when_open: bool,
    #[serde(default)]
    pub appearance: MenuAppearance,
    #[serde(default = "default_menu_screens")]
    pub screens: Vec<MenuScreenDefinition>,
}

impl Default for MenuSettings {
    fn default() -> Self {
        Self {
            pause_root_screen_id: default_pause_root_screen_id(),
            gate_gameplay_when_open: default_gate_gameplay_when_open(),
            appearance: MenuAppearance::default(),
            screens: default_menu_screens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuAppearance {
    #[serde(default = "default_menu_font_family")]
    pub font_family: String,
    #[serde(default = "default_menu_font_size_px")]
    pub font_size_px: u16,
    #[serde(default = "default_menu_color_hex")]
    pub color_hex: String,
    #[serde(default)]
    pub border_style: MenuBorderStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuVisualMetrics {
    pub panel_width_px: f32,
    pub panel_inner_margin_px: f32,
    pub title_size_delta_px: f32,
    pub title_top_y_px: f32,
    pub entries_start_y_px: f32,
    pub entry_spacing_y_px: f32,
    pub hint_bottom_padding_px: f32,
    pub title_padding_px: glam::Vec2,
    pub entry_padding_px: glam::Vec2,
    pub hint_padding_px: glam::Vec2,
}

impl Default for MenuVisualMetrics {
    fn default() -> Self {
        Self {
            panel_width_px: 280.0,
            panel_inner_margin_px: 16.0,
            title_size_delta_px: 4.0,
            title_top_y_px: 22.0,
            entries_start_y_px: 52.0,
            entry_spacing_y_px: 20.0,
            hint_bottom_padding_px: 18.0,
            title_padding_px: glam::Vec2::new(14.0, 10.0),
            entry_padding_px: glam::Vec2::new(10.0, 6.0),
            hint_padding_px: glam::Vec2::new(8.0, 4.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl MenuRect {
    pub fn center_x(&self) -> f32 {
        self.x + self.width * 0.5
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuLayoutBlock {
    pub rect: MenuRect,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuEntryLayout {
    pub rect: MenuRect,
    pub text: String,
    pub selected: bool,
    pub selectable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuLayout {
    pub panel: MenuRect,
    pub title: MenuLayoutBlock,
    pub entries: Vec<MenuEntryLayout>,
    pub hint: MenuLayoutBlock,
}

impl Default for MenuAppearance {
    fn default() -> Self {
        Self {
            font_family: default_menu_font_family(),
            font_size_px: default_menu_font_size_px(),
            color_hex: default_menu_color_hex(),
            border_style: MenuBorderStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MenuBorderStyle {
    #[default]
    Square,
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
    ExitGame,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    ExitRuntime,
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
        self.open_screen(&root);
    }

    pub fn open_screen(&mut self, screen_id: &str) {
        if self.screen_map.contains_key(screen_id) {
            self.stack.clear();
            self.stack.push(screen_id.to_string());
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

    pub fn handle_input(&mut self, input: MenuInput) -> Option<MenuCommand> {
        let current_screen_id = self.current_screen_id().map(str::to_string)?;

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

    fn go_back(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        } else {
            self.close();
        }
    }

    fn confirm_current_selection(&mut self, current_screen_id: &str) -> Option<MenuCommand> {
        let screen = self.screen_map.get(current_screen_id)?;
        let selected_index = *self
            .selected_index_by_screen
            .get(current_screen_id)
            .unwrap_or(&0);
        let Some(MenuItemDefinition::Button { action, .. }) = screen.items.get(selected_index)
        else {
            return None;
        };

        match action {
            MenuAction::CloseMenu => {
                self.close();
                None
            }
            MenuAction::OpenScreen { screen_id } => {
                if self.screen_map.contains_key(screen_id) {
                    self.stack.push(screen_id.clone());
                }
                None
            }
            MenuAction::Back => {
                self.go_back();
                None
            }
            MenuAction::ExitGame => Some(MenuCommand::ExitRuntime),
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

fn default_menu_font_family() -> String {
    "Sans".to_string()
}

fn default_menu_font_size_px() -> u16 {
    14
}

fn default_menu_color_hex() -> String {
    "#7CFF7C".to_string()
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

pub fn menu_visual_metrics() -> MenuVisualMetrics {
    MenuVisualMetrics::default()
}

pub fn build_menu_layout(
    view: &MenuView,
    appearance: &MenuAppearance,
    viewport: glam::Vec2,
) -> MenuLayout {
    let metrics = menu_visual_metrics();
    let panel = menu_panel_rect(view, appearance.font_size_px as f32, viewport);
    let content_x = panel.x + metrics.panel_inner_margin_px;
    let content_width = (panel.width - metrics.panel_inner_margin_px * 2.0).max(1.0);
    let title_height = appearance.font_size_px as f32
        + metrics.title_size_delta_px
        + metrics.title_padding_px.y * 2.0;
    let title_rect = MenuRect {
        x: content_x,
        y: metrics.title_top_y_px,
        width: content_width,
        height: title_height,
    };
    let entry_height = appearance.font_size_px as f32 + metrics.entry_padding_px.y * 2.0;
    let entries = view
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| MenuEntryLayout {
            rect: MenuRect {
                x: content_x,
                y: metrics.entries_start_y_px + index as f32 * metrics.entry_spacing_y_px,
                width: content_width,
                height: entry_height,
            },
            text: entry.text.clone(),
            selected: entry.selected,
            selectable: entry.selectable,
        })
        .collect();
    let hint_font_size = (appearance.font_size_px as f32 - 2.0).max(10.0);
    let hint_height = hint_font_size + metrics.hint_padding_px.y * 2.0;
    let hint_rect = MenuRect {
        x: content_x,
        y: viewport.y - metrics.hint_bottom_padding_px - hint_height,
        width: content_width,
        height: hint_height,
    };

    MenuLayout {
        panel,
        title: MenuLayoutBlock {
            rect: title_rect,
            text: view.title.clone(),
        },
        entries,
        hint: MenuLayoutBlock {
            rect: hint_rect,
            text: "Esc: Back   Enter/Space: Select".to_string(),
        },
    }
}

fn menu_panel_rect(view: &MenuView, font_size_px: f32, viewport: glam::Vec2) -> MenuRect {
    let metrics = menu_visual_metrics();
    let last_entry_y = if view.entries.is_empty() {
        metrics.entries_start_y_px
    } else {
        metrics.entries_start_y_px + (view.entries.len() - 1) as f32 * metrics.entry_spacing_y_px
    };
    let hint_size_px = (font_size_px - 2.0).max(10.0);
    let bottom = (last_entry_y + font_size_px + metrics.entry_padding_px.y * 2.0 + 12.0).max(
        viewport.y
            - metrics.hint_bottom_padding_px
            - hint_size_px
            - metrics.hint_padding_px.y * 2.0
            - 8.0,
    );
    let x = (viewport.x - metrics.panel_width_px) * 0.5;
    let y = (metrics.title_top_y_px - metrics.panel_inner_margin_px).max(8.0);
    MenuRect {
        x,
        y,
        width: metrics.panel_width_px,
        height: (bottom - y + metrics.panel_inner_margin_px).max(80.0),
    }
}

pub fn menu_hex_color_rgba(hex: &str) -> Option<[f32; 4]> {
    let trimmed = hex.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let green = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some([
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        1.0,
    ])
}

pub fn tinted_menu_background(accent: [f32; 4], shade: f32, alpha: f32) -> [f32; 4] {
    [
        accent[0] * shade,
        accent[1] * shade,
        accent[2] * shade,
        alpha,
    ]
}

pub fn menu_border_color(
    border_style: MenuBorderStyle,
    accent: [f32; 4],
    alpha: f32,
) -> Option<[f32; 4]> {
    match border_style {
        MenuBorderStyle::Square if alpha > 0.0 => Some([accent[0], accent[1], accent[2], alpha]),
        MenuBorderStyle::Square => None,
    }
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
