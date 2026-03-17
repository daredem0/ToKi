use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::text::{TextAnchor, TextStyle, TextWeight};
pub use crate::ui::{UiAction, UiCommand};
use crate::ui::{UiBlock, UiComposition, UiRect, UiTextBlock};

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
    #[serde(default)]
    pub dialogs: Vec<MenuDialogDefinition>,
}

impl Default for MenuSettings {
    fn default() -> Self {
        Self {
            pause_root_screen_id: default_pause_root_screen_id(),
            gate_gameplay_when_open: default_gate_gameplay_when_open(),
            appearance: MenuAppearance::default(),
            screens: default_menu_screens(),
            dialogs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuAppearance {
    #[serde(default = "default_menu_font_family")]
    pub font_family: String,
    #[serde(default = "default_menu_font_size_px")]
    pub font_size_px: u16,
    #[serde(default = "default_menu_width_percent")]
    pub menu_width_percent: u16,
    #[serde(default = "default_menu_height_percent")]
    pub menu_height_percent: u16,
    #[serde(default = "default_menu_title_spacing_px")]
    pub title_spacing_px: u16,
    #[serde(default = "default_menu_button_spacing_px")]
    pub button_spacing_px: u16,
    #[serde(default = "default_menu_footer_spacing_px")]
    pub footer_spacing_px: u16,
    #[serde(default = "default_menu_opacity_percent")]
    pub opacity_percent: u16,
    #[serde(default = "default_menu_border_color_hex", alias = "color_hex")]
    pub border_color_hex: String,
    #[serde(default = "default_menu_text_color_hex")]
    pub text_color_hex: String,
    #[serde(default = "default_menu_background_color_hex")]
    pub menu_background_color_hex: String,
    #[serde(default)]
    pub menu_background_transparent: bool,
    #[serde(default = "default_menu_title_background_color_hex")]
    pub title_background_color_hex: String,
    #[serde(default)]
    pub title_background_transparent: bool,
    #[serde(default = "default_menu_entry_background_color_hex")]
    pub entry_background_color_hex: String,
    #[serde(default)]
    pub entry_background_transparent: bool,
    #[serde(default = "default_menu_footer_text")]
    pub footer_text: String,
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

pub type MenuRect = UiRect;

#[derive(Debug, Clone, PartialEq)]
pub struct MenuLayoutBlock {
    pub rect: MenuRect,
    pub text: String,
    pub border_style: MenuBorderStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuEntryLayout {
    pub rect: MenuRect,
    pub text: String,
    pub selected: bool,
    pub selectable: bool,
    pub border_style: MenuBorderStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuLayout {
    pub panel: MenuRect,
    pub title: MenuLayoutBlock,
    pub entries: Vec<MenuEntryLayout>,
    pub hint: MenuLayoutBlock,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuDialogLayout {
    pub panel: MenuRect,
    pub title: MenuLayoutBlock,
    pub body: MenuLayoutBlock,
    pub confirm_button: MenuEntryLayout,
    pub cancel_button: MenuEntryLayout,
}

impl Default for MenuAppearance {
    fn default() -> Self {
        Self {
            font_family: default_menu_font_family(),
            font_size_px: default_menu_font_size_px(),
            menu_width_percent: default_menu_width_percent(),
            menu_height_percent: default_menu_height_percent(),
            title_spacing_px: default_menu_title_spacing_px(),
            button_spacing_px: default_menu_button_spacing_px(),
            footer_spacing_px: default_menu_footer_spacing_px(),
            opacity_percent: default_menu_opacity_percent(),
            border_color_hex: default_menu_border_color_hex(),
            text_color_hex: default_menu_text_color_hex(),
            menu_background_color_hex: default_menu_background_color_hex(),
            menu_background_transparent: false,
            title_background_color_hex: default_menu_title_background_color_hex(),
            title_background_transparent: false,
            entry_background_color_hex: default_menu_entry_background_color_hex(),
            entry_background_transparent: false,
            footer_text: default_menu_footer_text(),
            border_style: MenuBorderStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MenuBorderStyle {
    None,
    #[default]
    Square,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuScreenDefinition {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_border_style_override: Option<MenuBorderStyle>,
    #[serde(default)]
    pub items: Vec<MenuItemDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuDialogDefinition {
    pub id: String,
    pub title: String,
    pub body: String,
    pub confirm_text: String,
    pub cancel_text: String,
    pub confirm_action: UiAction,
    pub cancel_action: UiAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MenuItemDefinition {
    Label {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        border_style_override: Option<MenuBorderStyle>,
    },
    Button {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        border_style_override: Option<MenuBorderStyle>,
        action: UiAction,
    },
    DynamicList {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        heading: Option<String>,
        source: MenuListSource,
        empty_text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        border_style_override: Option<MenuBorderStyle>,
    },
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
    pub title_border_style_override: Option<MenuBorderStyle>,
    pub entries: Vec<MenuViewEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuViewEntry {
    pub text: String,
    pub selected: bool,
    pub selectable: bool,
    pub border_style_override: Option<MenuBorderStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuDialogView {
    pub dialog_id: String,
    pub title: String,
    pub body: String,
    pub confirm_text: String,
    pub cancel_text: String,
    pub confirm_selected: bool,
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
    dialog_map: HashMap<String, MenuDialogDefinition>,
    stack: Vec<String>,
    selected_index_by_screen: HashMap<String, usize>,
    active_dialog: Option<ActiveDialogState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveDialogState {
    dialog_id: String,
    confirm_selected: bool,
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
        let mut controller = Self {
            settings,
            screen_map,
            dialog_map,
            stack: Vec::new(),
            selected_index_by_screen: HashMap::new(),
            active_dialog: None,
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

    pub fn is_dialog_open(&self) -> bool {
        self.active_dialog.is_some()
    }

    pub fn open_pause_root(&mut self) {
        let root = self.settings.pause_root_screen_id.clone();
        self.open_screen(&root);
    }

    pub fn open_screen(&mut self, screen_id: &str) {
        if self.screen_map.contains_key(screen_id) {
            self.stack.clear();
            self.stack.push(screen_id.to_string());
            self.active_dialog = None;
        }
    }

    pub fn close(&mut self) {
        self.stack.clear();
        self.active_dialog = None;
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
                MenuItemDefinition::Label {
                    text,
                    border_style_override,
                } => entries.push(MenuViewEntry {
                    text: text.clone(),
                    selected: false,
                    selectable: false,
                    border_style_override: *border_style_override,
                }),
                MenuItemDefinition::Button {
                    text,
                    border_style_override,
                    ..
                } => entries.push(MenuViewEntry {
                    text: text.clone(),
                    selected: index == selected_index,
                    selectable: true,
                    border_style_override: *border_style_override,
                }),
                MenuItemDefinition::DynamicList {
                    heading,
                    source,
                    empty_text,
                    border_style_override,
                } => {
                    if let Some(heading) = heading {
                        entries.push(MenuViewEntry {
                            text: heading.clone(),
                            selected: false,
                            selectable: false,
                            border_style_override: *border_style_override,
                        });
                    }
                    let dynamic_entries = match source {
                        MenuListSource::PlayerInventory => inventory
                            .iter()
                            .map(|entry| MenuViewEntry {
                                text: format!("{} x{}", entry.item_id, entry.count),
                                selected: false,
                                selectable: false,
                                border_style_override: *border_style_override,
                            })
                            .collect::<Vec<_>>(),
                    };
                    if dynamic_entries.is_empty() {
                        entries.push(MenuViewEntry {
                            text: empty_text.clone(),
                            selected: false,
                            selectable: false,
                            border_style_override: *border_style_override,
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
            title_border_style_override: screen.title_border_style_override,
            entries,
        })
    }

    pub fn current_dialog_view(&self) -> Option<MenuDialogView> {
        let active_dialog = self.active_dialog.as_ref()?;
        let dialog = self.dialog_map.get(&active_dialog.dialog_id)?;
        Some(MenuDialogView {
            dialog_id: dialog.id.clone(),
            title: dialog.title.clone(),
            body: dialog.body.clone(),
            confirm_text: dialog.confirm_text.clone(),
            cancel_text: dialog.cancel_text.clone(),
            confirm_selected: active_dialog.confirm_selected,
        })
    }

    pub fn handle_input(&mut self, input: MenuInput) -> Option<UiCommand> {
        if self.active_dialog.is_some() {
            return self.handle_dialog_input(input);
        }
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

fn default_menu_width_percent() -> u16 {
    88
}

fn default_menu_height_percent() -> u16 {
    70
}

fn default_menu_title_spacing_px() -> u16 {
    8
}

fn default_menu_button_spacing_px() -> u16 {
    8
}

fn default_menu_footer_spacing_px() -> u16 {
    16
}

fn default_menu_opacity_percent() -> u16 {
    100
}

fn default_menu_border_color_hex() -> String {
    "#7CFF7C".to_string()
}

fn default_menu_text_color_hex() -> String {
    "#FFFFFF".to_string()
}

fn default_menu_background_color_hex() -> String {
    "#142914".to_string()
}

fn default_menu_title_background_color_hex() -> String {
    "#143614".to_string()
}

fn default_menu_entry_background_color_hex() -> String {
    "#0F1F0F".to_string()
}

fn default_menu_footer_text() -> String {
    "Esc: Back   Enter/Space: Select".to_string()
}

fn default_menu_screens() -> Vec<MenuScreenDefinition> {
    vec![
        MenuScreenDefinition {
            id: "pause_menu".to_string(),
            title: "Paused".to_string(),
            title_border_style_override: None,
            items: vec![
                MenuItemDefinition::Button {
                    text: "Resume".to_string(),
                    border_style_override: None,
                    action: UiAction::CloseUi,
                },
                MenuItemDefinition::Button {
                    text: "Inventory".to_string(),
                    border_style_override: None,
                    action: UiAction::OpenSurface {
                        surface_id: "inventory_menu".to_string(),
                    },
                },
            ],
        },
        MenuScreenDefinition {
            id: "inventory_menu".to_string(),
            title: "Inventory".to_string(),
            title_border_style_override: None,
            items: vec![
                MenuItemDefinition::DynamicList {
                    heading: Some("Items".to_string()),
                    source: MenuListSource::PlayerInventory,
                    empty_text: "Inventory is empty".to_string(),
                    border_style_override: None,
                },
                MenuItemDefinition::Button {
                    text: "Back".to_string(),
                    border_style_override: None,
                    action: UiAction::Back,
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
    let panel = menu_panel_rect(view, appearance, viewport);
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
    let entries_start_y = title_rect.y + title_rect.height + appearance.title_spacing_px as f32;
    let entry_height = appearance.font_size_px as f32 + metrics.entry_padding_px.y * 2.0;
    let button_spacing = appearance.button_spacing_px as f32;
    let entries: Vec<MenuEntryLayout> = view
        .entries
        .iter()
        .enumerate()
        .map(|(index, entry)| MenuEntryLayout {
            rect: MenuRect {
                x: content_x,
                y: entries_start_y + index as f32 * (entry_height + button_spacing),
                width: content_width,
                height: entry_height,
            },
            text: entry.text.clone(),
            selected: entry.selected,
            selectable: entry.selectable,
            border_style: entry
                .border_style_override
                .unwrap_or(appearance.border_style),
        })
        .collect();
    let hint_font_size = (appearance.font_size_px as f32 - 2.0).max(10.0);
    let hint_height = hint_font_size + metrics.hint_padding_px.y * 2.0;
    let footer_y = entries
        .last()
        .map(|entry| entry.rect.y + entry.rect.height)
        .unwrap_or(title_rect.y + title_rect.height)
        + appearance.footer_spacing_px as f32;
    let hint_rect = MenuRect {
        x: content_x,
        y: footer_y,
        width: content_width,
        height: hint_height,
    };

    MenuLayout {
        panel,
        title: MenuLayoutBlock {
            rect: title_rect,
            text: view.title.clone(),
            border_style: view
                .title_border_style_override
                .unwrap_or(appearance.border_style),
        },
        entries,
        hint: MenuLayoutBlock {
            rect: hint_rect,
            text: appearance.footer_text.clone(),
            border_style: MenuBorderStyle::None,
        },
    }
}

pub fn compose_menu_ui(layout: &MenuLayout, appearance: &MenuAppearance) -> UiComposition {
    let border_color =
        menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]);
    let opacity_alpha = (appearance.opacity_percent.clamp(0, 100) as f32) / 100.0;
    let text_color = apply_menu_opacity(
        menu_hex_color_rgba(&appearance.text_color_hex).unwrap_or([1.0, 1.0, 1.0, 1.0]),
        appearance.opacity_percent,
    );
    let title_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32 + 4.0,
        weight: TextWeight::Bold,
        color: text_color,
        ..TextStyle::default()
    };
    let entry_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32,
        weight: TextWeight::Normal,
        color: text_color,
        ..TextStyle::default()
    };
    let selected_style = TextStyle {
        color: text_color,
        weight: TextWeight::Bold,
        ..entry_style.clone()
    };
    let hint_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: (appearance.font_size_px as f32 - 2.0).max(10.0),
        color: text_color,
        ..TextStyle::default()
    };

    let mut composition = UiComposition::default();
    composition.push(UiBlock {
        rect: layout.panel,
        fill_color: menu_fill_color_rgba(
            &appearance.menu_background_color_hex,
            appearance.menu_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: menu_border_color(appearance.border_style, border_color, opacity_alpha),
        text: None,
    });
    composition.push(UiBlock {
        rect: layout.title.rect,
        fill_color: menu_fill_color_rgba(
            &appearance.title_background_color_hex,
            appearance.title_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: menu_border_color(layout.title.border_style, border_color, opacity_alpha),
        text: Some(UiTextBlock {
            content: layout.title.text.clone(),
            position: glam::Vec2::new(layout.title.rect.center_x(), layout.title.rect.y + 10.0),
            anchor: TextAnchor::TopCenter,
            style: title_style,
            layer: 10,
        }),
    });

    for entry in &layout.entries {
        let style = if entry.selected {
            selected_style.clone()
        } else {
            entry_style.clone()
        };
        composition.push(UiBlock {
            rect: entry.rect,
            fill_color: menu_fill_color_rgba(
                &appearance.entry_background_color_hex,
                appearance.entry_background_transparent,
                appearance.opacity_percent,
            ),
            border_color: menu_border_color(entry.border_style, border_color, opacity_alpha),
            text: Some(UiTextBlock {
                content: if entry.selected {
                    format!("> {}", entry.text)
                } else {
                    format!("  {}", entry.text)
                },
                position: glam::Vec2::new(entry.rect.center_x(), entry.rect.y + 6.0),
                anchor: TextAnchor::TopCenter,
                style,
                layer: 10,
            }),
        });
    }

    composition.push(UiBlock {
        rect: layout.hint.rect,
        fill_color: None,
        border_color: None,
        text: Some(UiTextBlock {
            content: layout.hint.text.clone(),
            position: glam::Vec2::new(layout.hint.rect.center_x(), layout.hint.rect.y + 4.0),
            anchor: TextAnchor::BottomCenter,
            style: hint_style,
            layer: 10,
        }),
    });

    composition
}

pub fn build_dialog_layout(
    view: &MenuDialogView,
    appearance: &MenuAppearance,
    viewport: glam::Vec2,
) -> MenuDialogLayout {
    let metrics = menu_visual_metrics();
    let panel_width = (viewport.x * (appearance.menu_width_percent.clamp(20, 100) as f32 / 100.0))
        .clamp(160.0, (viewport.x - 16.0).max(160.0));
    let content_x = (viewport.x - panel_width) * 0.5 + metrics.panel_inner_margin_px;
    let content_width = (panel_width - metrics.panel_inner_margin_px * 2.0).max(1.0);
    let title_height = appearance.font_size_px as f32
        + metrics.title_size_delta_px
        + metrics.title_padding_px.y * 2.0;
    let title_rect = MenuRect {
        x: content_x,
        y: viewport.y * 0.18,
        width: content_width,
        height: title_height,
    };
    let body_height =
        ((appearance.font_size_px as f32 * 3.2).max(40.0)) + metrics.entry_padding_px.y * 2.0;
    let body_rect = MenuRect {
        x: content_x,
        y: title_rect.y + title_rect.height + appearance.title_spacing_px as f32,
        width: content_width,
        height: body_height,
    };
    let button_height = appearance.font_size_px as f32 + metrics.entry_padding_px.y * 2.0;
    let button_width =
        ((content_width - appearance.button_spacing_px as f32).max(2.0) * 0.5).floor();
    let confirm_rect = MenuRect {
        x: content_x,
        y: body_rect.y + body_rect.height + appearance.footer_spacing_px as f32,
        width: button_width,
        height: button_height,
    };
    let cancel_rect = MenuRect {
        x: content_x + button_width + appearance.button_spacing_px as f32,
        y: confirm_rect.y,
        width: button_width,
        height: button_height,
    };
    let panel = MenuRect {
        x: (viewport.x - panel_width) * 0.5,
        y: (title_rect.y - metrics.panel_inner_margin_px).max(8.0),
        width: panel_width,
        height: (cancel_rect.y + cancel_rect.height - title_rect.y)
            + metrics.panel_inner_margin_px * 2.0
            + appearance.title_spacing_px as f32,
    };

    MenuDialogLayout {
        panel,
        title: MenuLayoutBlock {
            rect: title_rect,
            text: view.title.clone(),
            border_style: appearance.border_style,
        },
        body: MenuLayoutBlock {
            rect: body_rect,
            text: view.body.clone(),
            border_style: appearance.border_style,
        },
        confirm_button: MenuEntryLayout {
            rect: confirm_rect,
            text: view.confirm_text.clone(),
            selected: view.confirm_selected,
            selectable: true,
            border_style: appearance.border_style,
        },
        cancel_button: MenuEntryLayout {
            rect: cancel_rect,
            text: view.cancel_text.clone(),
            selected: !view.confirm_selected,
            selectable: true,
            border_style: appearance.border_style,
        },
    }
}

pub fn compose_dialog_ui(layout: &MenuDialogLayout, appearance: &MenuAppearance) -> UiComposition {
    let border_color =
        menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]);
    let opacity_alpha = (appearance.opacity_percent.clamp(0, 100) as f32) / 100.0;
    let text_color = apply_menu_opacity(
        menu_hex_color_rgba(&appearance.text_color_hex).unwrap_or([1.0, 1.0, 1.0, 1.0]),
        appearance.opacity_percent,
    );
    let title_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32 + 4.0,
        weight: TextWeight::Bold,
        color: text_color,
        ..TextStyle::default()
    };
    let body_style = TextStyle {
        font_family: appearance.font_family.clone(),
        size_px: appearance.font_size_px as f32,
        weight: TextWeight::Normal,
        color: text_color,
        ..TextStyle::default()
    };
    let button_style = body_style.clone();
    let selected_button_style = TextStyle {
        weight: TextWeight::Bold,
        ..body_style.clone()
    };

    let mut composition = UiComposition::default();
    composition.push(UiBlock {
        rect: layout.panel,
        fill_color: menu_fill_color_rgba(
            &appearance.menu_background_color_hex,
            appearance.menu_background_transparent,
            appearance.opacity_percent,
        ),
        border_color: menu_border_color(appearance.border_style, border_color, opacity_alpha),
        text: None,
    });
    for block in [&layout.title, &layout.body] {
        composition.push(UiBlock {
            rect: block.rect,
            fill_color: menu_fill_color_rgba(
                &appearance.title_background_color_hex,
                appearance.title_background_transparent,
                appearance.opacity_percent,
            ),
            border_color: menu_border_color(block.border_style, border_color, opacity_alpha),
            text: Some(UiTextBlock {
                content: block.text.clone(),
                position: glam::Vec2::new(block.rect.center_x(), block.rect.y + 10.0),
                anchor: TextAnchor::TopCenter,
                style: if block.rect == layout.title.rect {
                    title_style.clone()
                } else {
                    body_style.clone()
                },
                layer: 11,
            }),
        });
    }
    for button in [&layout.confirm_button, &layout.cancel_button] {
        composition.push(UiBlock {
            rect: button.rect,
            fill_color: menu_fill_color_rgba(
                &appearance.entry_background_color_hex,
                appearance.entry_background_transparent,
                appearance.opacity_percent,
            ),
            border_color: menu_border_color(button.border_style, border_color, opacity_alpha),
            text: Some(UiTextBlock {
                content: if button.selected {
                    format!("> {}", button.text)
                } else {
                    format!("  {}", button.text)
                },
                position: glam::Vec2::new(button.rect.center_x(), button.rect.y + 6.0),
                anchor: TextAnchor::TopCenter,
                style: if button.selected {
                    selected_button_style.clone()
                } else {
                    button_style.clone()
                },
                layer: 11,
            }),
        });
    }
    composition
}

fn menu_panel_rect(view: &MenuView, appearance: &MenuAppearance, viewport: glam::Vec2) -> MenuRect {
    let metrics = menu_visual_metrics();
    let font_size_px = appearance.font_size_px as f32;
    let title_height =
        font_size_px + metrics.title_size_delta_px + metrics.title_padding_px.y * 2.0;
    let entries_start_y =
        metrics.title_top_y_px + title_height + appearance.title_spacing_px as f32;
    let entry_height = font_size_px + metrics.entry_padding_px.y * 2.0;
    let button_spacing = appearance.button_spacing_px as f32;
    let last_entry_bottom = if view.entries.is_empty() {
        metrics.title_top_y_px + title_height
    } else {
        entries_start_y
            + (view.entries.len() - 1) as f32 * (entry_height + button_spacing)
            + entry_height
    };
    let hint_size_px = (font_size_px - 2.0).max(10.0);
    let hint_height = hint_size_px + metrics.hint_padding_px.y * 2.0;
    let content_bottom = last_entry_bottom + appearance.footer_spacing_px as f32 + hint_height;
    let requested_panel_width =
        viewport.x * (appearance.menu_width_percent.clamp(20, 100) as f32 / 100.0);
    let requested_panel_height =
        viewport.y * (appearance.menu_height_percent.clamp(20, 100) as f32 / 100.0);
    let max_panel_width = (viewport.x - 16.0).max(40.0);
    let panel_width = requested_panel_width.clamp(40.0, max_panel_width);
    let x = (viewport.x - panel_width) * 0.5;
    let y = (metrics.title_top_y_px - metrics.panel_inner_margin_px).max(8.0);
    let content_height = (content_bottom - y + metrics.panel_inner_margin_px).max(80.0);
    MenuRect {
        x,
        y,
        width: panel_width,
        height: content_height.max(requested_panel_height),
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

pub fn apply_menu_opacity(mut color: [f32; 4], opacity_percent: u16) -> [f32; 4] {
    color[3] *= (opacity_percent.clamp(0, 100) as f32) / 100.0;
    color
}

pub fn menu_fill_color_rgba(
    hex: &str,
    transparent: bool,
    opacity_percent: u16,
) -> Option<[f32; 4]> {
    let mut color = menu_hex_color_rgba(hex)?;
    color[3] = if transparent { 0.0 } else { 1.0 };
    color = apply_menu_opacity(color, opacity_percent);
    Some(color)
}

pub fn menu_border_color(
    border_style: MenuBorderStyle,
    accent: [f32; 4],
    alpha: f32,
) -> Option<[f32; 4]> {
    match border_style {
        MenuBorderStyle::None => None,
        MenuBorderStyle::Square if alpha > 0.0 => Some([accent[0], accent[1], accent[2], alpha]),
        MenuBorderStyle::Square => None,
    }
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
