//! Menu system constants and default values.

use super::types::{MenuItemDefinition, MenuListSource, MenuScreenDefinition};
use crate::ui::UiAction;

/// Default font family for menus.
pub const DEFAULT_FONT_FAMILY: &str = "Sans";

/// Default font size in pixels.
pub const DEFAULT_FONT_SIZE_PX: u16 = 14;

/// Default menu width as percentage of screen width.
pub const DEFAULT_WIDTH_PERCENT: u16 = 88;

/// Default menu height as percentage of screen height.
pub const DEFAULT_HEIGHT_PERCENT: u16 = 70;

/// Default spacing between title and content in pixels.
pub const DEFAULT_TITLE_SPACING_PX: u16 = 8;

/// Default spacing between buttons in pixels.
pub const DEFAULT_BUTTON_SPACING_PX: u16 = 8;

/// Default spacing for footer section in pixels.
pub const DEFAULT_FOOTER_SPACING_PX: u16 = 16;

/// Default menu opacity (100 = fully opaque).
pub const DEFAULT_OPACITY_PERCENT: u16 = 100;

/// Default menu border color (green tint).
pub const DEFAULT_BORDER_COLOR: &str = "#7CFF7C";

/// Default menu text color (white).
pub const DEFAULT_TEXT_COLOR: &str = "#FFFFFF";

/// Default menu background color (dark green).
pub const DEFAULT_BACKGROUND_COLOR: &str = "#142914";

/// Default title background color (slightly lighter green).
pub const DEFAULT_TITLE_BACKGROUND_COLOR: &str = "#143614";

/// Default entry background color (darker green).
pub const DEFAULT_ENTRY_BACKGROUND_COLOR: &str = "#0F1F0F";

/// Default footer text.
pub const DEFAULT_FOOTER_TEXT: &str = "Esc: Back   Enter/Space: Select";

pub fn default_pause_root_screen_id() -> String {
    "pause_menu".to_string()
}

pub fn default_gate_gameplay_when_open() -> bool {
    true
}

pub fn default_menu_font_family() -> String {
    DEFAULT_FONT_FAMILY.to_string()
}

pub fn default_menu_font_size_px() -> u16 {
    DEFAULT_FONT_SIZE_PX
}

pub fn default_menu_width_percent() -> u16 {
    DEFAULT_WIDTH_PERCENT
}

pub fn default_menu_height_percent() -> u16 {
    DEFAULT_HEIGHT_PERCENT
}

pub fn default_menu_title_spacing_px() -> u16 {
    DEFAULT_TITLE_SPACING_PX
}

pub fn default_menu_button_spacing_px() -> u16 {
    DEFAULT_BUTTON_SPACING_PX
}

pub fn default_menu_footer_spacing_px() -> u16 {
    DEFAULT_FOOTER_SPACING_PX
}

pub fn default_menu_opacity_percent() -> u16 {
    DEFAULT_OPACITY_PERCENT
}

pub fn default_menu_border_color_hex() -> String {
    DEFAULT_BORDER_COLOR.to_string()
}

pub fn default_menu_text_color_hex() -> String {
    DEFAULT_TEXT_COLOR.to_string()
}

pub fn default_menu_background_color_hex() -> String {
    DEFAULT_BACKGROUND_COLOR.to_string()
}

pub fn default_menu_title_background_color_hex() -> String {
    DEFAULT_TITLE_BACKGROUND_COLOR.to_string()
}

pub fn default_menu_entry_background_color_hex() -> String {
    DEFAULT_ENTRY_BACKGROUND_COLOR.to_string()
}

pub fn default_menu_footer_text() -> String {
    DEFAULT_FOOTER_TEXT.to_string()
}

pub fn default_menu_screens() -> Vec<MenuScreenDefinition> {
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
