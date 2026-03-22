//! Helper functions for menu editor operations.

use super::*;

impl InspectorSystem {
    pub(super) fn selected_menu_screen_index(project: &Project, screen_id: &str) -> Option<usize> {
        project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .position(|screen| screen.id == screen_id)
    }

    pub(super) fn selected_menu_dialog_index(project: &Project, dialog_id: &str) -> Option<usize> {
        project
            .metadata
            .runtime
            .menu
            .dialogs
            .iter()
            .position(|dialog| dialog.id == dialog_id)
    }

    pub(super) fn normalize_menu_screen_id(input: &str) -> String {
        let normalized = input
            .trim()
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect::<String>();
        normalized
            .split('_')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub(super) fn next_menu_screen_id(settings: &MenuSettings) -> String {
        Self::next_menu_screen_id_for_base(settings, "new_menu")
    }

    pub(super) fn next_menu_dialog_id(settings: &MenuSettings) -> String {
        Self::next_menu_dialog_id_for_base(settings, "new_dialog")
    }

    pub(super) fn next_menu_screen_id_for_base(settings: &MenuSettings, base: &str) -> String {
        if !Self::menu_screen_exists(settings, base) {
            return base.to_string();
        }
        let mut index = 2usize;
        loop {
            let candidate = format!("{base}_{index}");
            if !Self::menu_screen_exists(settings, &candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    pub(super) fn next_menu_dialog_id_for_base(settings: &MenuSettings, base: &str) -> String {
        if !Self::menu_dialog_exists(settings, base) {
            return base.to_string();
        }
        let mut index = 2usize;
        loop {
            let candidate = format!("{base}_{index}");
            if !Self::menu_dialog_exists(settings, &candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    fn menu_screen_exists(settings: &MenuSettings, screen_id: &str) -> bool {
        settings.screens.iter().any(|screen| screen.id == screen_id)
    }

    fn menu_dialog_exists(settings: &MenuSettings, dialog_id: &str) -> bool {
        settings.dialogs.iter().any(|dialog| dialog.id == dialog_id)
    }

    pub(crate) fn rewrite_ui_action_surface_targets(
        settings: &mut MenuSettings,
        previous_id: &str,
        next_id: &str,
    ) {
        for screen in &mut settings.screens {
            for item in &mut screen.items {
                if let MenuItemDefinition::Button {
                    action: UiAction::OpenSurface { surface_id },
                    ..
                } = item
                {
                    if surface_id == previous_id {
                        *surface_id = next_id.to_string();
                    }
                }
            }
        }
        for dialog in &mut settings.dialogs {
            if let UiAction::OpenSurface { surface_id } = &mut dialog.confirm_action {
                if surface_id == previous_id {
                    *surface_id = next_id.to_string();
                }
            }
            if let UiAction::OpenSurface { surface_id } = &mut dialog.cancel_action {
                if surface_id == previous_id {
                    *surface_id = next_id.to_string();
                }
            }
        }
    }

    pub(super) fn remove_ui_action_surface_targets(settings: &mut MenuSettings, removed_id: &str) {
        for screen in &mut settings.screens {
            for item in &mut screen.items {
                Self::remove_button_surface_target(item, removed_id);
            }
        }
        for dialog in &mut settings.dialogs {
            for action in [&mut dialog.confirm_action, &mut dialog.cancel_action] {
                if matches!(action, UiAction::OpenSurface { surface_id } if surface_id == removed_id)
                {
                    *action = UiAction::CloseSurface;
                }
            }
        }
    }

    fn remove_button_surface_target(item: &mut MenuItemDefinition, removed_id: &str) {
        if let MenuItemDefinition::Button { action, text, .. } = item {
            if matches!(action, UiAction::OpenSurface { surface_id } if surface_id == removed_id) {
                *item = MenuItemDefinition::Button {
                    text: text.clone(),
                    border_style_override: None,
                    action: UiAction::Back,
                };
            }
        }
    }

    pub(super) fn commit_menu_settings_change(
        ui_state: &mut EditorUI,
        project: &mut Project,
        before_settings: MenuSettings,
    ) {
        let after_settings = project.metadata.runtime.menu.clone();
        if before_settings == after_settings {
            return;
        }

        ui_state.execute_command_with_project(
            project,
            EditorCommand::update_menu_settings(before_settings, after_settings),
        );
    }

    pub(super) fn is_valid_menu_hex_color(hex: &str) -> bool {
        let trimmed = hex.trim().trim_start_matches('#');
        trimmed.len() == 6 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit())
    }

    pub(super) fn render_menu_border_override_editor(
        ui: &mut egui::Ui,
        label: &str,
        border_style_override: &mut Option<MenuBorderStyle>,
    ) -> bool {
        let mut selected = *border_style_override;
        egui::ComboBox::from_label(label)
            .selected_text(match selected {
                None => "Inherit",
                Some(MenuBorderStyle::None) => "None",
                Some(MenuBorderStyle::Square) => "Square",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, None, "Inherit");
                ui.selectable_value(&mut selected, Some(MenuBorderStyle::None), "None");
                ui.selectable_value(&mut selected, Some(MenuBorderStyle::Square), "Square");
            });
        if *border_style_override != selected {
            *border_style_override = selected;
            return true;
        }
        false
    }
}
