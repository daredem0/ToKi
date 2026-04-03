//! Menu system for in-game UI.
//!
//! This module provides the runtime menu system that renders pause menus,
//! dialog boxes, and other UI overlays.
//!
//! # Module Structure
//!
//! - `constants`: Default values and constants
//! - `types`: Core types (MenuSettings, theme overrides, resolved appearance)
//! - `utilities`: Color parsing and utility functions
//! - `layout`: Layout building functions
//! - `composition`: UI composition functions
//! - `controller`: Menu navigation and input handling

pub mod composition;
pub mod constants;
pub mod controller;
pub mod layout;
pub mod types;
pub mod utilities;

// Re-export commonly used items
pub use composition::{
    build_dialog_widget_tree, build_menu_widget_tree, compose_dialog_output, compose_dialog_ui,
    compose_menu_output, compose_menu_ui, dialog_entry_index_from_widget_id,
    menu_entry_index_from_widget_id, theme_from_menu_appearance,
};
pub use constants::{
    DEFAULT_BACKGROUND_COLOR, DEFAULT_BORDER_COLOR, DEFAULT_BORDER_THICKNESS_PX,
    DEFAULT_BUTTON_SPACING_PX, DEFAULT_ENTRY_BACKGROUND_COLOR, DEFAULT_FONT_FAMILY,
    DEFAULT_FONT_SIZE_PX, DEFAULT_FOOTER_SPACING_PX, DEFAULT_FOOTER_TEXT, DEFAULT_HEIGHT_PERCENT,
    DEFAULT_OPACITY_PERCENT, DEFAULT_TEXT_COLOR, DEFAULT_TITLE_BACKGROUND_COLOR,
    DEFAULT_TITLE_SPACING_PX, DEFAULT_WIDTH_PERCENT,
};
pub use controller::MenuController;
pub use layout::{build_dialog_layout, build_menu_layout};
pub use types::{
    resolve_dialog_appearance, resolve_menu_appearance, DialogThemeOverride, InventoryEntry,
    MenuAppearance, MenuBorderStyle, MenuDialogDefinition, MenuDialogLayout, MenuDialogPosition,
    MenuDialogView, MenuEntryLayout, MenuInput, MenuItemDefinition, MenuLayout, MenuLayoutBlock,
    MenuListSource, MenuRect, MenuScreenDefinition, MenuSettings, MenuTextAppearance,
    MenuThemeOverride, MenuView, MenuViewEntry, MenuVisualMetrics,
};
pub use utilities::{
    apply_menu_opacity, menu_border_color, menu_fill_color_rgba, menu_hex_color_rgba,
    menu_visual_metrics,
};

// Re-export UI types used in menu API
pub use crate::ui::{UiAction, UiCommand};

#[cfg(test)]
#[path = "../menu_tests.rs"]
mod tests;
