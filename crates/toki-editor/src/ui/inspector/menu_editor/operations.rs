//! CRUD operations for menu screens, dialogs, and entries.

use super::*;

impl InspectorSystem {
    pub(crate) fn add_menu_screen(ui_state: &mut EditorUI, project: &mut Project) {
        let before_settings = project.metadata.runtime.menu.clone();
        let next_id = Self::next_menu_screen_id(&project.metadata.runtime.menu);
        project
            .metadata
            .runtime
            .menu
            .screens
            .push(MenuScreenDefinition {
                id: next_id.clone(),
                title: "New Menu".to_string(),
                title_border_style_override: None,
                items: vec![MenuItemDefinition::Button {
                    text: "Resume".to_string(),
                    border_style_override: None,
                    action: UiAction::CloseUi,
                }],
            });
        if project.metadata.runtime.menu.screens.len() == 1 {
            project.metadata.runtime.menu.pause_root_screen_id = next_id.clone();
        }
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.select_menu_screen(next_id);
    }

    pub(crate) fn add_menu_dialog(ui_state: &mut EditorUI, project: &mut Project) {
        let before_settings = project.metadata.runtime.menu.clone();
        let next_id = Self::next_menu_dialog_id(&project.metadata.runtime.menu);
        project
            .metadata
            .runtime
            .menu
            .dialogs
            .push(MenuDialogDefinition {
                id: next_id.clone(),
                title: "New Dialog".to_string(),
                body: "Are you sure?".to_string(),
                confirm_text: "Confirm".to_string(),
                cancel_text: "Cancel".to_string(),
                confirm_action: UiAction::CloseSurface,
                cancel_action: UiAction::CloseSurface,
                hide_main_menu: false,
            });
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.select_menu_dialog(next_id);
    }

    pub(super) fn duplicate_menu_screen(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let original = project.metadata.runtime.menu.screens[screen_index].clone();
        let mut duplicate = original.clone();
        duplicate.id =
            Self::next_menu_screen_id_for_base(&project.metadata.runtime.menu, &original.id);
        duplicate.title = format!("{} Copy", original.title);
        let insert_index = screen_index + 1;
        project
            .metadata
            .runtime
            .menu
            .screens
            .insert(insert_index, duplicate.clone());
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.select_menu_screen(duplicate.id);
    }

    pub(super) fn duplicate_menu_dialog(
        ui_state: &mut EditorUI,
        project: &mut Project,
        dialog_index: usize,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let original = project.metadata.runtime.menu.dialogs[dialog_index].clone();
        let mut duplicate = original.clone();
        duplicate.id =
            Self::next_menu_dialog_id_for_base(&project.metadata.runtime.menu, &original.id);
        duplicate.title = format!("{} Copy", original.title);
        let insert_index = dialog_index + 1;
        project
            .metadata
            .runtime
            .menu
            .dialogs
            .insert(insert_index, duplicate.clone());
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.select_menu_dialog(duplicate.id);
    }

    pub(super) fn delete_menu_screen(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
    ) -> bool {
        let before_settings = project.metadata.runtime.menu.clone();
        let removed = project.metadata.runtime.menu.screens.remove(screen_index);
        Self::remove_ui_action_surface_targets(&mut project.metadata.runtime.menu, &removed.id);
        if project.metadata.runtime.menu.pause_root_screen_id == removed.id {
            project.metadata.runtime.menu.pause_root_screen_id = project
                .metadata
                .runtime
                .menu
                .screens
                .first()
                .map(|screen| screen.id.clone())
                .unwrap_or_default();
        }
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.sync_menu_editor_selection(Some(project));
        true
    }

    pub(super) fn delete_menu_dialog(
        ui_state: &mut EditorUI,
        project: &mut Project,
        dialog_index: usize,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let removed = project.metadata.runtime.menu.dialogs.remove(dialog_index);
        Self::remove_ui_action_surface_targets(&mut project.metadata.runtime.menu, &removed.id);
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.sync_menu_editor_selection(Some(project));
    }

    pub(super) fn add_menu_item_to_selected_screen(
        ui_state: &mut EditorUI,
        project: &mut Project,
        item: MenuItemDefinition,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let Some(screen_id) = ui_state.selected_menu_screen_id().map(str::to_string) else {
            return;
        };
        let Some(screen_index) = Self::selected_menu_screen_index(project, &screen_id) else {
            return;
        };
        let screen = &mut project.metadata.runtime.menu.screens[screen_index];
        let item_index = screen.items.len();
        screen.items.push(item);
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.select_menu_entry(screen_id, item_index);
    }

    pub(super) fn duplicate_menu_item(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let item = project.metadata.runtime.menu.screens[screen_index].items[item_index].clone();
        project.metadata.runtime.menu.screens[screen_index]
            .items
            .insert(item_index + 1, item);
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        let screen_id = project.metadata.runtime.menu.screens[screen_index]
            .id
            .clone();
        ui_state.select_menu_entry(screen_id, item_index + 1);
    }

    pub(crate) fn delete_menu_item(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let (screen_id, remaining_len) = {
            let screen = &mut project.metadata.runtime.menu.screens[screen_index];
            screen.items.remove(item_index);
            (screen.id.clone(), screen.items.len())
        };
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        if item_index < remaining_len {
            ui_state.select_menu_entry(screen_id, item_index);
        } else {
            ui_state.select_menu_screen(screen_id);
        }
    }

    pub(super) fn move_menu_item(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
        direction: isize,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let next_index = item_index as isize + direction;
        let item_count = project.metadata.runtime.menu.screens[screen_index]
            .items
            .len();
        if next_index < 0 || next_index as usize >= item_count {
            return;
        }
        let screen_id = {
            let screen = &mut project.metadata.runtime.menu.screens[screen_index];
            screen.items.swap(item_index, next_index as usize);
            screen.id.clone()
        };
        Self::commit_menu_settings_change(ui_state, project, before_settings);
        ui_state.select_menu_entry(screen_id, next_index as usize);
    }

    pub(super) fn coerce_menu_item_kind(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
        kind: MenuEditorItemKind,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let next_item = match kind {
            MenuEditorItemKind::Label => MenuItemDefinition::Label {
                text: "Text".to_string(),
                border_style_override: None,
            },
            MenuEditorItemKind::Button => MenuItemDefinition::Button {
                text: "Button".to_string(),
                border_style_override: None,
                action: UiAction::CloseUi,
            },
            MenuEditorItemKind::InventoryList => MenuItemDefinition::DynamicList {
                heading: Some("Inventory".to_string()),
                source: MenuListSource::PlayerInventory,
                empty_text: "Inventory is empty".to_string(),
                border_style_override: None,
            },
        };

        let current_kind = {
            let current = &project.metadata.runtime.menu.screens[screen_index].items[item_index];
            MenuEditorItemKind::from_item(current)
        };
        if current_kind == kind {
            return;
        }
        project.metadata.runtime.menu.screens[screen_index].items[item_index] = next_item;
        Self::commit_menu_settings_change(ui_state, project, before_settings);
    }
}
