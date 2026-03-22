//! Menu type definitions.

use serde::{Deserialize, Serialize};

use crate::ui::{UiAction, UiRect};

use super::constants::{
    default_gate_gameplay_when_open, default_menu_background_color_hex,
    default_menu_border_color_hex, default_menu_button_spacing_px,
    default_menu_entry_background_color_hex, default_menu_font_family, default_menu_font_size_px,
    default_menu_footer_spacing_px, default_menu_footer_text, default_menu_height_percent,
    default_menu_opacity_percent, default_menu_screens, default_menu_text_color_hex,
    default_menu_title_background_color_hex, default_menu_title_spacing_px,
    default_menu_width_percent, default_pause_root_screen_id,
};

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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hide_main_menu: bool,
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
    pub hide_main_menu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEntry {
    pub item_id: String,
    pub count: u32,
}
