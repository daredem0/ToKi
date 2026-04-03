//! Menu editor inspector UI.
//!
//! This module provides UI for editing runtime menu settings, screens, dialogs, and entries.
//!
//! # Module Structure
//!
//! - `appearance`: Menu appearance settings editors
//! - `screen_editor`: Screen editing UI
//! - `dialog_editor`: Dialog editing UI
//! - `entry_editor`: Entry editing UI
//! - `operations`: CRUD operations for screens, dialogs, entries
//! - `helpers`: Helper functions and utilities

mod appearance;
mod dialog_editor;
mod entry_editor;
mod helpers;
mod operations;
mod screen_editor;

use super::{EditorUI, InspectorSystem, Selection};
use crate::project::Project;
use crate::ui::undo_redo::EditorCommand;
use toki_core::menu::{
    DialogThemeOverride, MenuBorderStyle, MenuDialogDefinition, MenuDialogPosition,
    MenuItemDefinition, MenuListSource, MenuScreenDefinition, MenuSettings, MenuThemeOverride,
    UiAction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MenuEditorItemKind {
    Label,
    Button,
    InventoryList,
}

impl MenuEditorItemKind {
    pub fn from_item(item: &MenuItemDefinition) -> Self {
        match item {
            MenuItemDefinition::Label { .. } => Self::Label,
            MenuItemDefinition::Button { .. } => Self::Button,
            MenuItemDefinition::DynamicList { .. } => Self::InventoryList,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Label => "Text",
            Self::Button => "Button",
            Self::InventoryList => "Inventory List",
        }
    }
}

impl InspectorSystem {
    pub(super) fn render_menu_theme_override_editor(
        ui: &mut egui::Ui,
        theme_override: &mut MenuThemeOverride,
    ) -> bool {
        Self::render_menu_theme_override_layout(ui, theme_override)
    }

    pub(super) fn render_dialog_theme_override_editor(
        ui: &mut egui::Ui,
        theme_override: &mut DialogThemeOverride,
    ) -> bool {
        Self::render_dialog_theme_override_layout(ui, theme_override)
    }

    pub(crate) fn render_menu_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
    ) {
        let Some(project) = project else {
            ui.heading("Menu Editor");
            ui.separator();
            ui.label("Open a project to edit runtime menus.");
            return;
        };

        crate::ui::editor_ui::sync_menu_editor_selection(ui_state, Some(project));

        ui.heading("Menu Editor");
        ui.separator();
        Self::render_menu_global_settings(ui_state, ui, project);
        ui.separator();

        match ui_state.selection.clone() {
            Some(Selection::MenuScreen(screen_id)) => {
                Self::render_menu_screen_editor(ui_state, ui, project, &screen_id);
            }
            Some(Selection::MenuDialog(dialog_id)) => {
                Self::render_menu_dialog_editor(ui_state, ui, project, &dialog_id);
            }
            Some(Selection::MenuEntry {
                screen_id,
                item_index,
            }) => {
                Self::render_menu_entry_editor(ui_state, ui, project, &screen_id, item_index);
            }
            _ => {
                ui.label("Select a menu screen or entry in the Menu Editor tab.");
            }
        }
    }

    fn render_menu_global_settings(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: &mut Project,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let available_screen_ids: Vec<_> = project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .map(|s| s.id.clone())
            .collect();

        Self::render_runtime_menu_header(ui, project, &available_screen_ids);

        Self::render_menu_theme_override_editor(ui, &mut project.metadata.runtime.menu.theme_override);
        Self::commit_menu_settings_change(ui_state, project, before_settings);

        Self::render_screens_dialogs_headers(ui, ui_state, project);
    }

    fn render_runtime_menu_header(ui: &mut egui::Ui, project: &mut Project, screen_ids: &[String]) {
        egui::CollapsingHeader::new("Runtime Menu Settings")
            .default_open(false)
            .show(ui, |ui| {
                ui.checkbox(
                    &mut project.metadata.runtime.menu.gate_gameplay_when_open,
                    "Gate gameplay while menu is open",
                );
                let mut pause_root = project.metadata.runtime.menu.pause_root_screen_id.clone();
                egui::ComboBox::from_label("Pause Root Screen")
                    .selected_text(pause_root.clone())
                    .show_ui(ui, |ui| {
                        for screen_id in screen_ids {
                            ui.selectable_value(&mut pause_root, screen_id.clone(), screen_id);
                        }
                    });
                if pause_root != project.metadata.runtime.menu.pause_root_screen_id {
                    project.metadata.runtime.menu.pause_root_screen_id = pause_root;
                }
            });
    }

    fn render_screens_dialogs_headers(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        project: &mut Project,
    ) {
        egui::CollapsingHeader::new("Screens")
            .default_open(false)
            .show(ui, |ui| {
                if ui.button("+ Add Screen").clicked() {
                    Self::add_menu_screen(ui_state, project);
                }
            });
        egui::CollapsingHeader::new("Dialogs")
            .default_open(false)
            .show(ui, |ui| {
                if ui.button("+ Add Dialog").clicked() {
                    Self::add_menu_dialog(ui_state, project);
                }
            });
    }
}
