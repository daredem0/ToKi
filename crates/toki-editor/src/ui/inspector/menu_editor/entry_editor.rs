//! Entry editing UI.

use super::*;
use crate::ui::ui_event_registry::{
    render_required_ui_event_id_editor, validate_ui_event_reference,
};

impl InspectorSystem {
    pub(super) fn render_menu_entry_editor(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: &mut Project,
        screen_id: &str,
        item_index: usize,
    ) {
        let before_settings = project.metadata.runtime.menu.clone();
        let Some(screen_index) = Self::selected_menu_screen_index(project, screen_id) else {
            ui.label("Selected screen no longer exists.");
            return;
        };
        if item_index
            >= project.metadata.runtime.menu.screens[screen_index]
                .items
                .len()
        {
            ui.label("Selected entry no longer exists.");
            return;
        }

        let mut item_kind = {
            let item = &project.metadata.runtime.menu.screens[screen_index].items[item_index];
            MenuEditorItemKind::from_item(item)
        };
        let available_surface_ids = Self::collect_available_surface_ids(project);
        let declared_ui_events = project.metadata.runtime.ui.event_declarations.clone();
        let mut changed = false;
        let mut has_missing_target_validation = false;
        let mut missing_target_id = String::new();
        let mut invalid_event_issue = None::<String>;

        egui::CollapsingHeader::new("Entry")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(format!("Screen: {screen_id}"));
                ui.label(format!("Position: {}", item_index + 1));
                ui.separator();

                Self::render_entry_move_buttons(ui, ui_state, project, screen_index, item_index);
                Self::render_entry_action_buttons(ui, ui_state, project, screen_index, item_index);

                ui.separator();
                Self::render_entry_type_combo(
                    ui,
                    ui_state,
                    project,
                    screen_index,
                    item_index,
                    &mut item_kind,
                );

                ui.separator();
                let (ch, validation) = Self::render_entry_fields(
                    ui,
                    project,
                    screen_index,
                    item_index,
                    &available_surface_ids,
                    &declared_ui_events,
                );
                changed |= ch;
                if let Some(target_id) = validation.0 {
                    has_missing_target_validation = true;
                    missing_target_id = target_id;
                }
                invalid_event_issue = validation.1;
            });

        if changed {
            Self::commit_menu_settings_change(ui_state, project, before_settings);
        }

        if has_missing_target_validation {
            ui.colored_label(
                egui::Color32::from_rgb(215, 120, 120),
                format!("Target surface '{missing_target_id}' does not exist."),
            );
        }
        if let Some(issue) = invalid_event_issue {
            ui.colored_label(egui::Color32::from_rgb(215, 120, 120), issue);
        }
    }

    fn collect_available_surface_ids(project: &Project) -> Vec<String> {
        let screen_ids: Vec<_> = project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .map(|screen| screen.id.clone())
            .collect();
        let dialog_ids: Vec<_> = project
            .metadata
            .runtime
            .menu
            .dialogs
            .iter()
            .map(|dialog| dialog.id.clone())
            .collect();
        screen_ids
            .iter()
            .chain(dialog_ids.iter())
            .cloned()
            .collect()
    }

    fn render_entry_move_buttons(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
    ) {
        ui.horizontal(|ui| {
            if ui.button("Move Up").clicked() {
                Self::move_menu_item(ui_state, project, screen_index, item_index, -1);
            }
            if ui.button("Move Down").clicked() {
                Self::move_menu_item(ui_state, project, screen_index, item_index, 1);
            }
        });
    }

    fn render_entry_action_buttons(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
    ) {
        ui.horizontal(|ui| {
            if ui.button("Duplicate Entry").clicked() {
                Self::duplicate_menu_item(ui_state, project, screen_index, item_index);
            }
            if ui.button("Delete Entry").clicked() {
                Self::delete_menu_item(ui_state, project, screen_index, item_index);
            }
        });
    }

    fn render_entry_type_combo(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
        item_kind: &mut MenuEditorItemKind,
    ) {
        egui::ComboBox::from_label("Type")
            .selected_text(item_kind.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(item_kind, MenuEditorItemKind::Label, "Text");
                ui.selectable_value(item_kind, MenuEditorItemKind::Button, "Button");
                ui.selectable_value(
                    item_kind,
                    MenuEditorItemKind::InventoryList,
                    "Inventory List",
                );
            });
        Self::coerce_menu_item_kind(ui_state, project, screen_index, item_index, *item_kind);
    }

    fn render_entry_fields(
        ui: &mut egui::Ui,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
        available_surface_ids: &[String],
        declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
    ) -> (bool, (Option<String>, Option<String>)) {
        let mut changed = false;
        let mut missing_target = None;
        let mut invalid_event_issue = None;

        match &mut project.metadata.runtime.menu.screens[screen_index].items[item_index] {
            MenuItemDefinition::Label {
                text,
                border_style_override,
            } => {
                ui.label("Text");
                if ui.text_edit_singleline(text).changed() {
                    changed = true;
                }
                changed |= Self::render_menu_border_override_editor(
                    ui,
                    "Entry Border Style",
                    border_style_override,
                );
            }
            MenuItemDefinition::Button {
                text,
                border_style_override,
                action,
            } => {
                ui.label("Label");
                if ui.text_edit_singleline(text).changed() {
                    changed = true;
                }
                changed |= Self::render_menu_border_override_editor(
                    ui,
                    "Entry Border Style",
                    border_style_override,
                );
                changed |=
                    Self::render_menu_action_editor(ui, available_surface_ids, declared_ui_events, action);
                if let UiAction::OpenSurface { surface_id } = action {
                    if !available_surface_ids.iter().any(|id| id == surface_id) {
                        missing_target = Some(surface_id.clone());
                    }
                }
                if let UiAction::EmitEvent { event_id } = action {
                    invalid_event_issue =
                        validate_ui_event_reference(event_id, declared_ui_events, "Menu action");
                }
            }
            MenuItemDefinition::DynamicList {
                heading,
                source,
                empty_text,
                border_style_override,
            } => {
                changed |= Self::render_dynamic_list_fields(
                    ui,
                    heading,
                    source,
                    empty_text,
                    border_style_override,
                );
            }
        }

        (changed, (missing_target, invalid_event_issue))
    }

    fn render_dynamic_list_fields(
        ui: &mut egui::Ui,
        heading: &mut Option<String>,
        source: &mut MenuListSource,
        empty_text: &mut String,
        border_style_override: &mut Option<MenuBorderStyle>,
    ) -> bool {
        let mut changed = false;

        let mut show_heading = heading.is_some();
        if ui.checkbox(&mut show_heading, "Show Heading").changed() {
            *heading = if show_heading {
                Some("Inventory".to_string())
            } else {
                None
            };
            changed = true;
        }
        if let Some(heading_text) = heading.as_mut() {
            ui.label("Heading");
            if ui.text_edit_singleline(heading_text).changed() {
                changed = true;
            }
        }

        let mut selected_source = source.clone();
        egui::ComboBox::from_label("List Source")
            .selected_text("Player Inventory")
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut selected_source,
                    MenuListSource::PlayerInventory,
                    "Player Inventory",
                );
            });
        if *source != selected_source {
            *source = selected_source;
            changed = true;
        }

        ui.label("Empty Text");
        if ui.text_edit_singleline(empty_text).changed() {
            changed = true;
        }
        changed |= Self::render_menu_border_override_editor(
            ui,
            "Entry Border Style",
            border_style_override,
        );

        changed
    }

    pub(super) fn render_menu_action_editor(
        ui: &mut egui::Ui,
        available_surface_ids: &[String],
        declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
        action: &mut UiAction,
    ) -> bool {
        let mut changed = false;
        let mut action_kind = Self::ui_action_to_kind(action);

        egui::ComboBox::from_label("Action")
            .selected_text(Self::ui_action_kind_label(action_kind))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut action_kind, 0, "Close UI");
                ui.selectable_value(&mut action_kind, 1, "Close Surface");
                ui.selectable_value(&mut action_kind, 2, "Open Surface");
                ui.selectable_value(&mut action_kind, 3, "Back");
                ui.selectable_value(&mut action_kind, 4, "Exit Runtime");
                ui.selectable_value(&mut action_kind, 5, "Open Audio Settings");
                ui.selectable_value(&mut action_kind, 6, "Open Display Settings");
                ui.selectable_value(&mut action_kind, 7, "Open Graphics Settings");
                ui.selectable_value(&mut action_kind, 8, "Save Game");
                ui.selectable_value(&mut action_kind, 9, "Load Game");
                ui.selectable_value(&mut action_kind, 10, "Emit Event");
            });

        changed |= Self::apply_action_kind_change(
            ui,
            action,
            action_kind,
            available_surface_ids,
            declared_ui_events,
        );
        changed
    }

    fn ui_action_to_kind(action: &UiAction) -> u8 {
        match action {
            UiAction::CloseUi => 0,
            UiAction::CloseSurface => 1,
            UiAction::OpenSurface { .. } => 2,
            UiAction::Back => 3,
            UiAction::ExitRuntime => 4,
            UiAction::OpenAudioSettings => 5,
            UiAction::OpenDisplaySettings => 6,
            UiAction::OpenGraphicsSettings => 7,
            UiAction::SaveGame { .. } => 8,
            UiAction::LoadGame { .. } => 9,
            UiAction::EmitEvent { .. } => 10,
        }
    }

    fn ui_action_kind_label(kind: u8) -> &'static str {
        match kind {
            0 => "Close UI",
            1 => "Close Surface",
            2 => "Open Surface",
            3 => "Back",
            4 => "Exit Runtime",
            5 => "Open Audio Settings",
            6 => "Open Display Settings",
            7 => "Open Graphics Settings",
            8 => "Save Game",
            9 => "Load Game",
            _ => "Emit Event",
        }
    }

    fn apply_action_kind_change(
        ui: &mut egui::Ui,
        action: &mut UiAction,
        kind: u8,
        available_surface_ids: &[String],
        declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
    ) -> bool {
        let mut changed = false;
        match kind {
            0 if *action != UiAction::CloseUi => {
                *action = UiAction::CloseUi;
                changed = true;
            }
            1 if *action != UiAction::CloseSurface => {
                *action = UiAction::CloseSurface;
                changed = true;
            }
            2 => {
                let mut target = match action {
                    UiAction::OpenSurface { surface_id } => surface_id.clone(),
                    _ => available_surface_ids.first().cloned().unwrap_or_default(),
                };
                egui::ComboBox::from_label("Target Surface")
                    .selected_text(target.clone())
                    .show_ui(ui, |ui| {
                        for surface_id in available_surface_ids {
                            ui.selectable_value(&mut target, surface_id.clone(), surface_id);
                        }
                    });
                let next_action = UiAction::OpenSurface { surface_id: target };
                if *action != next_action {
                    *action = next_action;
                    changed = true;
                }
            }
            3 if *action != UiAction::Back => {
                *action = UiAction::Back;
                changed = true;
            }
            4 if *action != UiAction::ExitRuntime => {
                *action = UiAction::ExitRuntime;
                changed = true;
            }
            5 if *action != UiAction::OpenAudioSettings => {
                *action = UiAction::OpenAudioSettings;
                changed = true;
            }
            6 if *action != UiAction::OpenDisplaySettings => {
                *action = UiAction::OpenDisplaySettings;
                changed = true;
            }
            7 if *action != UiAction::OpenGraphicsSettings => {
                *action = UiAction::OpenGraphicsSettings;
                changed = true;
            }
            8 => {
                let mut slot = match action {
                    UiAction::SaveGame { slot } => *slot,
                    _ => 1,
                };
                changed |= Self::render_save_slot_editor(ui, &mut slot);
                let next_action = UiAction::SaveGame { slot };
                if *action != next_action {
                    *action = next_action;
                    changed = true;
                }
            }
            9 => {
                let mut slot = match action {
                    UiAction::LoadGame { slot } => *slot,
                    _ => 1,
                };
                changed |= Self::render_save_slot_editor(ui, &mut slot);
                let next_action = UiAction::LoadGame { slot };
                if *action != next_action {
                    *action = next_action;
                    changed = true;
                }
            }
            _ => {
                let mut event_id = match action {
                    UiAction::EmitEvent { event_id } => event_id.clone(),
                    _ => String::new(),
                };
                if render_required_ui_event_id_editor(
                    ui,
                    "menu_emit_event",
                    "Event ID",
                    &mut event_id,
                    declared_ui_events,
                ) {
                    *action = UiAction::EmitEvent {
                        event_id: event_id.trim().to_string(),
                    };
                    changed = true;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::InspectorSystem;
    use toki_core::ui::UiAction;

    #[test]
    fn display_settings_action_has_stable_kind_and_label() {
        assert_eq!(
            InspectorSystem::ui_action_to_kind(&UiAction::OpenDisplaySettings),
            6
        );
        assert_eq!(
            InspectorSystem::ui_action_kind_label(6),
            "Open Display Settings"
        );
    }
}
