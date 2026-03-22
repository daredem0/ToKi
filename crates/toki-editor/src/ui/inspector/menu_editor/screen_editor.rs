//! Screen editing UI.

use super::*;

impl InspectorSystem {
    pub(super) fn render_menu_screen_editor(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: &mut Project,
        screen_id: &str,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let Some(screen_index) = Self::selected_menu_screen_index(project, screen_id) else {
            ui.label("Selected screen no longer exists.");
            return;
        };

        let mut screen_deleted = false;
        let mut changed = false;
        let mut renamed_to = None;
        egui::CollapsingHeader::new("Screen Settings")
            .default_open(false)
            .show(ui, |ui| {
                let screen = &mut project.metadata.runtime.menu.screens[screen_index];
                let mut title = screen.title.clone();
                ui.label("Title");
                if ui.text_edit_singleline(&mut title).changed() && title != screen.title {
                    screen.title = title;
                    changed = true;
                }

                changed |= Self::render_menu_border_override_editor(
                    ui,
                    "Title Border Style",
                    &mut screen.title_border_style_override,
                );

                let mut id = screen.id.clone();
                ui.label("Screen ID");
                if ui.text_edit_singleline(&mut id).changed() {
                    let normalized = Self::normalize_menu_screen_id(&id);
                    if !normalized.is_empty() && normalized != screen.id {
                        screen.id = normalized.clone();
                        renamed_to = Some(normalized);
                        changed = true;
                    }
                }
            });
        if let Some(normalized) = renamed_to {
            if project.metadata.runtime.menu.pause_root_screen_id == *screen_id {
                project.metadata.runtime.menu.pause_root_screen_id = normalized.clone();
            }
            Self::rewrite_ui_action_surface_targets(
                &mut project.metadata.runtime.menu,
                screen_id,
                &normalized,
            );
            ui_state.select_menu_screen(normalized);
        }
        if changed {
            Self::commit_menu_settings_change(ui_state, project, before_settings);
        }

        egui::CollapsingHeader::new("Screen Actions")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Duplicate Screen").clicked() {
                        Self::duplicate_menu_screen(ui_state, project, screen_index);
                    }
                    if ui.button("Delete Screen").clicked() {
                        screen_deleted = Self::delete_menu_screen(ui_state, project, screen_index);
                    }
                });
            });

        if screen_deleted {
            return;
        }

        Self::render_screen_entries_section(ui, ui_state, project, screen_index);
    }

    fn render_screen_entries_section(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
    ) {
        egui::CollapsingHeader::new("Entries")
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("+ Text").clicked() {
                        Self::add_menu_item_to_selected_screen(
                            ui_state,
                            project,
                            MenuItemDefinition::Label {
                                text: "New Text".to_string(),
                                border_style_override: None,
                            },
                        );
                    }
                    if ui.button("+ Button").clicked() {
                        Self::add_menu_item_to_selected_screen(
                            ui_state,
                            project,
                            MenuItemDefinition::Button {
                                text: "New Button".to_string(),
                                border_style_override: None,
                                action: UiAction::CloseUi,
                            },
                        );
                    }
                    if ui.button("+ Inventory List").clicked() {
                        Self::add_menu_item_to_selected_screen(
                            ui_state,
                            project,
                            MenuItemDefinition::DynamicList {
                                heading: Some("Inventory".to_string()),
                                source: MenuListSource::PlayerInventory,
                                empty_text: "Inventory is empty".to_string(),
                                border_style_override: None,
                            },
                        );
                    }
                });

                let item_count = project.metadata.runtime.menu.screens[screen_index]
                    .items
                    .len();
                ui.label(format!("{item_count} item(s) on this screen"));
            });
    }
}
