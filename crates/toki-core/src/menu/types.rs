//! Menu type definitions.

use serde::{Deserialize, Serialize};

use crate::ui::{UiAction, UiRect};
use crate::ui_layout::UiTheme;

use super::constants::{
    default_dialog_body_spacing_px, default_dialog_button_spacing_px,
    default_dialog_opacity_percent, default_dialog_title_spacing_px,
    default_dialog_width_percent, default_gate_gameplay_when_open,
    default_menu_button_spacing_px, default_menu_font_family, default_menu_font_size_px,
    default_menu_footer_spacing_px, default_menu_footer_text, default_menu_height_percent,
    default_menu_opacity_percent, default_menu_screens, default_menu_title_spacing_px,
    default_menu_width_percent, default_pause_root_screen_id,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuSettings {
    #[serde(default = "default_pause_root_screen_id")]
    pub pause_root_screen_id: String,
    #[serde(default = "default_gate_gameplay_when_open")]
    pub gate_gameplay_when_open: bool,
    #[serde(default)]
    pub theme_override: MenuThemeOverride,
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
            theme_override: MenuThemeOverride::default(),
            screens: default_menu_screens(),
            dialogs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuThemeOverride {
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
    #[serde(default = "default_menu_footer_text")]
    pub footer_text: String,
}

impl Default for MenuThemeOverride {
    fn default() -> Self {
        Self {
            menu_width_percent: default_menu_width_percent(),
            menu_height_percent: default_menu_height_percent(),
            title_spacing_px: default_menu_title_spacing_px(),
            button_spacing_px: default_menu_button_spacing_px(),
            footer_spacing_px: default_menu_footer_spacing_px(),
            opacity_percent: default_menu_opacity_percent(),
            footer_text: default_menu_footer_text(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DialogThemeOverride {
    #[serde(default = "default_dialog_position")]
    pub position: MenuDialogPosition,
    #[serde(default = "default_dialog_width_percent")]
    pub width_percent: u16,
    #[serde(default = "default_dialog_title_spacing_px")]
    pub title_spacing_px: u16,
    #[serde(default = "default_dialog_body_spacing_px")]
    pub body_spacing_px: u16,
    #[serde(default = "default_dialog_button_spacing_px")]
    pub button_spacing_px: u16,
    #[serde(default = "default_dialog_opacity_percent")]
    pub opacity_percent: u16,
}

impl Default for DialogThemeOverride {
    fn default() -> Self {
        Self {
            position: default_dialog_position(),
            width_percent: default_dialog_width_percent(),
            title_spacing_px: default_dialog_title_spacing_px(),
            body_spacing_px: default_dialog_body_spacing_px(),
            button_spacing_px: default_dialog_button_spacing_px(),
            opacity_percent: default_dialog_opacity_percent(),
        }
    }
}

fn default_dialog_position() -> MenuDialogPosition {
    MenuDialogPosition::Bottom
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuAppearance {
    pub font_family: String,
    pub font_size_px: u16,
    pub menu_width_percent: u16,
    pub menu_height_percent: u16,
    pub title_spacing_px: u16,
    pub button_spacing_px: u16,
    pub footer_spacing_px: u16,
    pub opacity_percent: u16,
    pub border_thickness_px: u16,
    pub border_color_hex: String,
    pub text_color_hex: String,
    pub menu_background_color_hex: String,
    pub menu_background_transparent: bool,
    pub title_background_color_hex: String,
    pub title_background_transparent: bool,
    pub entry_background_color_hex: String,
    pub entry_background_transparent: bool,
    pub footer_text: String,
    pub border_style: MenuBorderStyle,
    pub dialog_position: MenuDialogPosition,
    pub dialog_speaker_text: MenuTextAppearance,
    pub dialog_body_text: MenuTextAppearance,
}

impl Default for MenuAppearance {
    fn default() -> Self {
        resolve_menu_appearance(&UiTheme::default(), &MenuThemeOverride::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuTextAppearance {
    pub font_family: String,
    pub font_size_px: u16,
    pub bold: bool,
    pub cursive: bool,
}

impl Default for MenuTextAppearance {
    fn default() -> Self {
        Self {
            font_family: default_menu_font_family(),
            font_size_px: default_menu_font_size_px(),
            bold: false,
            cursive: false,
        }
    }
}

pub fn resolve_menu_appearance(theme: &UiTheme, theme_override: &MenuThemeOverride) -> MenuAppearance {
    let border_color = color_hex(theme.border_color);
    let text_color = color_hex(theme.foreground_color);
    let background_color = color_hex(theme.background_color);
    let title_background_color = color_hex(theme.accent_color);
    let entry_background_color = color_hex(theme.background_color);

    MenuAppearance {
        font_family: theme.font_family.clone(),
        font_size_px: theme.menu_font_size_px,
        menu_width_percent: theme_override.menu_width_percent,
        menu_height_percent: theme_override.menu_height_percent,
        title_spacing_px: theme_override.title_spacing_px,
        button_spacing_px: theme_override.button_spacing_px,
        footer_spacing_px: theme_override.footer_spacing_px,
        opacity_percent: theme_override.opacity_percent,
        border_thickness_px: theme.border_thickness_px,
        border_color_hex: border_color,
        text_color_hex: text_color,
        menu_background_color_hex: background_color.clone(),
        menu_background_transparent: theme.background_color[3] == 0,
        title_background_color_hex: title_background_color,
        title_background_transparent: theme.accent_color[3] == 0,
        entry_background_color_hex: entry_background_color,
        entry_background_transparent: theme.background_color[3] == 0,
        footer_text: theme_override.footer_text.clone(),
        border_style: MenuBorderStyle::Square,
        dialog_position: MenuDialogPosition::Top,
        dialog_speaker_text: default_dialog_speaker_text_appearance_for_theme(theme),
        dialog_body_text: default_dialog_body_text_appearance_for_theme(theme),
    }
}

pub fn resolve_dialog_appearance(
    theme: &UiTheme,
    theme_override: &DialogThemeOverride,
) -> MenuAppearance {
    let mut appearance = resolve_menu_appearance(theme, &MenuThemeOverride::default());
    appearance.font_size_px = theme.base_font_size_px;
    appearance.menu_width_percent = theme_override.width_percent;
    appearance.title_spacing_px = theme_override.title_spacing_px;
    appearance.footer_spacing_px = theme_override.body_spacing_px;
    appearance.button_spacing_px = theme_override.button_spacing_px;
    appearance.opacity_percent = theme_override.opacity_percent;
    appearance.dialog_position = theme_override.position;
    appearance.footer_text.clear();
    appearance.dialog_speaker_text = default_dialog_speaker_text_appearance_for_theme(theme);
    appearance.dialog_body_text = default_dialog_body_text_appearance_for_theme(theme);
    appearance
}

fn default_dialog_speaker_text_appearance_for_theme(theme: &UiTheme) -> MenuTextAppearance {
    MenuTextAppearance {
        font_family: theme.font_family.clone(),
        font_size_px: theme.dialog_speaker_font_size_px,
        bold: true,
        cursive: false,
    }
}

fn default_dialog_body_text_appearance_for_theme(theme: &UiTheme) -> MenuTextAppearance {
    MenuTextAppearance {
        font_family: theme.font_family.clone(),
        font_size_px: theme.dialog_body_font_size_px,
        bold: false,
        cursive: false,
    }
}

fn color_hex(color: [u8; 4]) -> String {
    format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2])
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
    pub entries: Vec<MenuEntryLayout>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MenuBorderStyle {
    None,
    #[default]
    Square,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MenuDialogPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
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
    Left,
    Right,
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
    pub entries: Vec<MenuViewEntry>,
    pub hide_main_menu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryEntry {
    pub item_id: String,
    pub count: u32,
}
