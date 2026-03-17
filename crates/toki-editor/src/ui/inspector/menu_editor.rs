use super::{EditorUI, InspectorSystem, Selection};
use crate::project::Project;
use chrono::Utc;
use toki_core::menu::{
    MenuAction, MenuBorderStyle, MenuDialogDefinition, MenuItemDefinition, MenuListSource,
    MenuScreenDefinition, MenuSettings,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuEditorItemKind {
    Label,
    Button,
    InventoryList,
}

impl MenuEditorItemKind {
    fn from_item(item: &MenuItemDefinition) -> Self {
        match item {
            MenuItemDefinition::Label { .. } => Self::Label,
            MenuItemDefinition::Button { .. } => Self::Button,
            MenuItemDefinition::DynamicList { .. } => Self::InventoryList,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Label => "Text",
            Self::Button => "Button",
            Self::InventoryList => "Inventory List",
        }
    }
}

impl InspectorSystem {
    pub(super) fn render_menu_editor_inspector(
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

        ui_state.sync_menu_editor_selection(Some(project));

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
        let available_screen_ids = project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .map(|screen| screen.id.clone())
            .collect::<Vec<_>>();
        let mut appearance = project.metadata.runtime.menu.appearance.clone();
        let mut appearance_changed = false;
        egui::CollapsingHeader::new("Runtime Menu Settings")
            .default_open(false)
            .show(ui, |ui| {
                if ui
                    .checkbox(
                        &mut project.metadata.runtime.menu.gate_gameplay_when_open,
                        "Gate gameplay while menu is open",
                    )
                    .changed()
                {
                    Self::mark_menu_settings_changed(project);
                }

                let mut pause_root = project.metadata.runtime.menu.pause_root_screen_id.clone();
                egui::ComboBox::from_label("Pause Root Screen")
                    .selected_text(pause_root.clone())
                    .show_ui(ui, |ui| {
                        for screen_id in &available_screen_ids {
                            ui.selectable_value(&mut pause_root, screen_id.clone(), screen_id);
                        }
                    });
                if pause_root != project.metadata.runtime.menu.pause_root_screen_id {
                    project.metadata.runtime.menu.pause_root_screen_id = pause_root;
                    Self::mark_menu_settings_changed(project);
                }
            });

        egui::CollapsingHeader::new("Typography")
            .default_open(false)
            .show(ui, |ui| {
                let font_choices = if ui_state.menu_preview_font_families.is_empty() {
                    vec!["Sans".to_string(), "Serif".to_string(), "Mono".to_string()]
                } else {
                    ui_state.menu_preview_font_families.clone()
                };
                let mut selected_font_family = appearance.font_family.clone();
                egui::ComboBox::from_label("Font Family")
                    .selected_text(selected_font_family.clone())
                    .show_ui(ui, |ui| {
                        for family in &font_choices {
                            ui.selectable_value(&mut selected_font_family, family.clone(), family);
                        }
                    });
                if selected_font_family != appearance.font_family {
                    appearance.font_family = selected_font_family;
                    appearance_changed = true;
                }

                let mut font_size = appearance.font_size_px;
                ui.horizontal(|ui| {
                    ui.label("Font Size");
                    if ui
                        .add(
                            egui::DragValue::new(&mut font_size)
                                .range(8..=64)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        appearance.font_size_px = font_size;
                        appearance_changed = true;
                    }
                });
            });

        egui::CollapsingHeader::new("Layout")
            .default_open(false)
            .show(ui, |ui| {
                let mut menu_width_percent = appearance.menu_width_percent;
                ui.horizontal(|ui| {
                    ui.label("Menu Width %");
                    if ui
                        .add(
                            egui::DragValue::new(&mut menu_width_percent)
                                .range(20..=100)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        appearance.menu_width_percent = menu_width_percent;
                        appearance_changed = true;
                    }
                });

                let mut menu_height_percent = appearance.menu_height_percent;
                ui.horizontal(|ui| {
                    ui.label("Menu Height %");
                    if ui
                        .add(
                            egui::DragValue::new(&mut menu_height_percent)
                                .range(20..=100)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        appearance.menu_height_percent = menu_height_percent;
                        appearance_changed = true;
                    }
                });

                let mut title_spacing = appearance.title_spacing_px;
                ui.horizontal(|ui| {
                    ui.label("Title Spacing");
                    if ui
                        .add(
                            egui::DragValue::new(&mut title_spacing)
                                .range(0..=64)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        appearance.title_spacing_px = title_spacing;
                        appearance_changed = true;
                    }
                });

                let mut button_spacing = appearance.button_spacing_px;
                ui.horizontal(|ui| {
                    ui.label("Button Spacing");
                    if ui
                        .add(
                            egui::DragValue::new(&mut button_spacing)
                                .range(0..=64)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        appearance.button_spacing_px = button_spacing;
                        appearance_changed = true;
                    }
                });

                let mut footer_spacing = appearance.footer_spacing_px;
                ui.horizontal(|ui| {
                    ui.label("Footer Spacing");
                    if ui
                        .add(
                            egui::DragValue::new(&mut footer_spacing)
                                .range(0..=128)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        appearance.footer_spacing_px = footer_spacing;
                        appearance_changed = true;
                    }
                });
            });

        egui::CollapsingHeader::new("Style")
            .default_open(false)
            .show(ui, |ui| {
                let mut opacity_percent = appearance.opacity_percent;
                ui.horizontal(|ui| {
                    ui.label("Menu Opacity %");
                    if ui
                        .add(
                            egui::Slider::new(&mut opacity_percent, 0..=100)
                                .clamping(egui::SliderClamping::Always),
                        )
                        .changed()
                    {
                        appearance.opacity_percent = opacity_percent;
                        appearance_changed = true;
                    }
                });

                let mut border_style = appearance.border_style;
                egui::ComboBox::from_label("Border Style")
                    .selected_text(match border_style {
                        MenuBorderStyle::None => "None",
                        MenuBorderStyle::Square => "Square",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut border_style, MenuBorderStyle::None, "None");
                        ui.selectable_value(&mut border_style, MenuBorderStyle::Square, "Square");
                    });
                if border_style != appearance.border_style {
                    appearance.border_style = border_style;
                    appearance_changed = true;
                }

                ui.label("Border Color Hex");
                if ui
                    .text_edit_singleline(&mut appearance.border_color_hex)
                    .changed()
                {
                    appearance_changed = true;
                }
                if !Self::is_valid_menu_hex_color(&appearance.border_color_hex) {
                    ui.colored_label(
                        egui::Color32::from_rgb(215, 120, 120),
                        "Use a 6-digit hex color like #7CFF7C",
                    );
                }

                ui.label("Text Color Hex");
                if ui
                    .text_edit_singleline(&mut appearance.text_color_hex)
                    .changed()
                {
                    appearance_changed = true;
                }
                if !Self::is_valid_menu_hex_color(&appearance.text_color_hex) {
                    ui.colored_label(
                        egui::Color32::from_rgb(215, 120, 120),
                        "Use a 6-digit hex color like #FFFFFF",
                    );
                }
            });

        egui::CollapsingHeader::new("Backgrounds")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Menu Background");
                if ui
                    .checkbox(
                        &mut appearance.menu_background_transparent,
                        "Transparent Menu Background",
                    )
                    .changed()
                {
                    appearance_changed = true;
                }
                if ui
                    .text_edit_singleline(&mut appearance.menu_background_color_hex)
                    .changed()
                {
                    appearance_changed = true;
                }
                if !Self::is_valid_menu_hex_color(&appearance.menu_background_color_hex) {
                    ui.colored_label(
                        egui::Color32::from_rgb(215, 120, 120),
                        "Use a 6-digit hex color like #142914",
                    );
                }

                ui.label("Title Background");
                if ui
                    .checkbox(
                        &mut appearance.title_background_transparent,
                        "Transparent Title Background",
                    )
                    .changed()
                {
                    appearance_changed = true;
                }
                if ui
                    .text_edit_singleline(&mut appearance.title_background_color_hex)
                    .changed()
                {
                    appearance_changed = true;
                }
                if !Self::is_valid_menu_hex_color(&appearance.title_background_color_hex) {
                    ui.colored_label(
                        egui::Color32::from_rgb(215, 120, 120),
                        "Use a 6-digit hex color like #143614",
                    );
                }

                ui.label("Entry Background");
                if ui
                    .checkbox(
                        &mut appearance.entry_background_transparent,
                        "Transparent Entry Background",
                    )
                    .changed()
                {
                    appearance_changed = true;
                }
                if ui
                    .text_edit_singleline(&mut appearance.entry_background_color_hex)
                    .changed()
                {
                    appearance_changed = true;
                }
                if !Self::is_valid_menu_hex_color(&appearance.entry_background_color_hex) {
                    ui.colored_label(
                        egui::Color32::from_rgb(215, 120, 120),
                        "Use a 6-digit hex color like #0F1F0F",
                    );
                }
            });

        egui::CollapsingHeader::new("Footer")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Footer Text");
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut appearance.footer_text)
                            .desired_rows(3)
                            .lock_focus(true),
                    )
                    .changed()
                {
                    appearance_changed = true;
                }
            });

        if appearance_changed {
            project.metadata.runtime.menu.appearance = appearance;
            Self::mark_menu_settings_changed(project);
        }

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

    fn render_menu_screen_editor(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: &mut Project,
        screen_id: &str,
    ) {
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
            Self::rewrite_menu_action_screen_targets(
                &mut project.metadata.runtime.menu,
                screen_id,
                &normalized,
            );
            ui_state.select_menu_screen(normalized);
        }
        if changed {
            Self::mark_menu_settings_changed(project);
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
                                action: MenuAction::CloseMenu,
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

    fn render_menu_dialog_editor(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: &mut Project,
        dialog_id: &str,
    ) {
        let Some(dialog_index) = Self::selected_menu_dialog_index(project, dialog_id) else {
            ui.label("Selected dialog no longer exists.");
            return;
        };

        let available_screen_ids = project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .map(|screen| screen.id.clone())
            .collect::<Vec<_>>();
        let available_dialog_ids = project
            .metadata
            .runtime
            .menu
            .dialogs
            .iter()
            .map(|dialog| dialog.id.clone())
            .collect::<Vec<_>>();
        let mut changed = false;
        let mut renamed_to = None;
        let mut duplicate_dialog = false;
        let mut delete_dialog = false;

        egui::CollapsingHeader::new("Dialog")
            .default_open(false)
            .show(ui, |ui| {
                let dialog = &mut project.metadata.runtime.menu.dialogs[dialog_index];
                ui.label("Title");
                if ui.text_edit_singleline(&mut dialog.title).changed() {
                    changed = true;
                }

                ui.label("Dialog ID");
                let mut id = dialog.id.clone();
                if ui.text_edit_singleline(&mut id).changed() {
                    let normalized = Self::normalize_menu_screen_id(&id);
                    if !normalized.is_empty() && normalized != dialog.id {
                        dialog.id = normalized.clone();
                        renamed_to = Some(normalized);
                        changed = true;
                    }
                }

                ui.label("Body");
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut dialog.body)
                            .desired_rows(3)
                            .lock_focus(true),
                    )
                    .changed()
                {
                    changed = true;
                }

                ui.separator();
                ui.label("Confirm Button");
                if ui.text_edit_singleline(&mut dialog.confirm_text).changed() {
                    changed = true;
                }
                ui.push_id("dialog_confirm_action", |ui| {
                    changed |= Self::render_menu_action_editor(
                        ui,
                        &available_screen_ids,
                        &available_dialog_ids,
                        &mut dialog.confirm_action,
                    );
                });

                ui.separator();
                ui.label("Cancel Button");
                if ui.text_edit_singleline(&mut dialog.cancel_text).changed() {
                    changed = true;
                }
                ui.push_id("dialog_cancel_action", |ui| {
                    changed |= Self::render_menu_action_editor(
                        ui,
                        &available_screen_ids,
                        &available_dialog_ids,
                        &mut dialog.cancel_action,
                    );
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Duplicate Dialog").clicked() {
                        duplicate_dialog = true;
                    }
                    if ui.button("Delete Dialog").clicked() {
                        delete_dialog = true;
                    }
                });
            });

        if let Some(normalized) = renamed_to {
            Self::rewrite_menu_action_dialog_targets(
                &mut project.metadata.runtime.menu,
                dialog_id,
                &normalized,
            );
            ui_state.select_menu_dialog(normalized);
        }
        if changed {
            Self::mark_menu_settings_changed(project);
        }
        if duplicate_dialog {
            Self::duplicate_menu_dialog(ui_state, project, dialog_index);
        }
        if delete_dialog {
            Self::delete_menu_dialog(ui_state, project, dialog_index);
        }
    }

    fn render_menu_entry_editor(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: &mut Project,
        screen_id: &str,
        item_index: usize,
    ) {
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
        let available_screen_ids = project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .map(|screen| screen.id.clone())
            .collect::<Vec<_>>();
        let available_dialog_ids = project
            .metadata
            .runtime
            .menu
            .dialogs
            .iter()
            .map(|dialog| dialog.id.clone())
            .collect::<Vec<_>>();
        let mut changed = false;
        let mut has_missing_target_validation = false;
        let mut missing_target_id = String::new();

        egui::CollapsingHeader::new("Entry")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(format!("Screen: {screen_id}"));
                ui.label(format!("Position: {}", item_index + 1));
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Move Up").clicked() {
                        Self::move_menu_item(ui_state, project, screen_index, item_index, -1);
                    }
                    if ui.button("Move Down").clicked() {
                        Self::move_menu_item(ui_state, project, screen_index, item_index, 1);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Duplicate Entry").clicked() {
                        Self::duplicate_menu_item(ui_state, project, screen_index, item_index);
                    }
                    if ui.button("Delete Entry").clicked() {
                        Self::delete_menu_item(ui_state, project, screen_index, item_index);
                    }
                });

                ui.separator();
                egui::ComboBox::from_label("Type")
                    .selected_text(item_kind.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut item_kind, MenuEditorItemKind::Label, "Text");
                        ui.selectable_value(&mut item_kind, MenuEditorItemKind::Button, "Button");
                        ui.selectable_value(
                            &mut item_kind,
                            MenuEditorItemKind::InventoryList,
                            "Inventory List",
                        );
                    });
                Self::coerce_menu_item_kind(project, screen_index, item_index, item_kind);

                ui.separator();
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
                        changed |= Self::render_menu_action_editor(
                            ui,
                            &available_screen_ids,
                            &available_dialog_ids,
                            action,
                        );
                        match action {
                            MenuAction::OpenScreen { screen_id }
                                if !available_screen_ids.iter().any(|id| id == screen_id) =>
                            {
                                has_missing_target_validation = true;
                                missing_target_id = screen_id.clone();
                            }
                            MenuAction::OpenDialog { dialog_id }
                                if !available_dialog_ids.iter().any(|id| id == dialog_id) =>
                            {
                                has_missing_target_validation = true;
                                missing_target_id = dialog_id.clone();
                            }
                            _ => {}
                        }
                    }
                    MenuItemDefinition::DynamicList {
                        heading,
                        source,
                        empty_text,
                        border_style_override,
                    } => {
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
                    }
                }
            });
        if changed {
            Self::mark_menu_settings_changed(project);
        }

        if has_missing_target_validation {
            ui.colored_label(
                egui::Color32::from_rgb(215, 120, 120),
                format!("Target surface '{missing_target_id}' does not exist."),
            );
        }
    }

    fn render_menu_action_editor(
        ui: &mut egui::Ui,
        available_screen_ids: &[String],
        available_dialog_ids: &[String],
        action: &mut MenuAction,
    ) -> bool {
        let mut changed = false;
        let mut action_kind = match action {
            MenuAction::CloseMenu => 0,
            MenuAction::CloseDialog => 1,
            MenuAction::OpenScreen { .. } => 2,
            MenuAction::OpenDialog { .. } => 3,
            MenuAction::Back => 4,
            MenuAction::ExitGame => 5,
        };
        egui::ComboBox::from_label("Action")
            .selected_text(match action_kind {
                0 => "Resume / Close Menu",
                1 => "Close Dialog",
                2 => "Open Screen",
                3 => "Open Dialog",
                4 => "Back",
                _ => "Exit Game",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut action_kind, 0, "Resume / Close Menu");
                ui.selectable_value(&mut action_kind, 1, "Close Dialog");
                ui.selectable_value(&mut action_kind, 2, "Open Screen");
                ui.selectable_value(&mut action_kind, 3, "Open Dialog");
                ui.selectable_value(&mut action_kind, 4, "Back");
                ui.selectable_value(&mut action_kind, 5, "Exit Game");
            });

        match action_kind {
            0 => {
                if *action != MenuAction::CloseMenu {
                    *action = MenuAction::CloseMenu;
                    changed = true;
                }
            }
            1 => {
                if *action != MenuAction::CloseDialog {
                    *action = MenuAction::CloseDialog;
                    changed = true;
                }
            }
            2 => {
                let mut target = match action {
                    MenuAction::OpenScreen { screen_id } => screen_id.clone(),
                    _ => available_screen_ids.first().cloned().unwrap_or_default(),
                };
                egui::ComboBox::from_label("Target Screen")
                    .selected_text(target.clone())
                    .show_ui(ui, |ui| {
                        for screen_id in available_screen_ids {
                            ui.selectable_value(&mut target, screen_id.clone(), screen_id);
                        }
                    });
                let next_action = MenuAction::OpenScreen { screen_id: target };
                if *action != next_action {
                    *action = next_action;
                    changed = true;
                }
            }
            3 => {
                let mut target = match action {
                    MenuAction::OpenDialog { dialog_id } => dialog_id.clone(),
                    _ => available_dialog_ids.first().cloned().unwrap_or_default(),
                };
                egui::ComboBox::from_label("Target Dialog")
                    .selected_text(target.clone())
                    .show_ui(ui, |ui| {
                        for dialog_id in available_dialog_ids {
                            ui.selectable_value(&mut target, dialog_id.clone(), dialog_id);
                        }
                    });
                let next_action = MenuAction::OpenDialog { dialog_id: target };
                if *action != next_action {
                    *action = next_action;
                    changed = true;
                }
            }
            _ => {
                let next_action = if action_kind == 4 {
                    MenuAction::Back
                } else {
                    MenuAction::ExitGame
                };
                if *action != next_action {
                    *action = next_action;
                    changed = true;
                }
            }
        }
        changed
    }

    fn selected_menu_screen_index(project: &Project, screen_id: &str) -> Option<usize> {
        project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .position(|screen| screen.id == screen_id)
    }

    fn selected_menu_dialog_index(project: &Project, dialog_id: &str) -> Option<usize> {
        project
            .metadata
            .runtime
            .menu
            .dialogs
            .iter()
            .position(|dialog| dialog.id == dialog_id)
    }

    pub(crate) fn add_menu_screen(ui_state: &mut EditorUI, project: &mut Project) {
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
                    action: MenuAction::CloseMenu,
                }],
            });
        if project.metadata.runtime.menu.screens.len() == 1 {
            project.metadata.runtime.menu.pause_root_screen_id = next_id.clone();
        }
        Self::mark_menu_settings_changed(project);
        ui_state.select_menu_screen(next_id);
    }

    pub(crate) fn add_menu_dialog(ui_state: &mut EditorUI, project: &mut Project) {
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
                confirm_action: MenuAction::CloseDialog,
                cancel_action: MenuAction::CloseDialog,
            });
        Self::mark_menu_settings_changed(project);
        ui_state.select_menu_dialog(next_id);
    }

    fn duplicate_menu_screen(ui_state: &mut EditorUI, project: &mut Project, screen_index: usize) {
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
        Self::mark_menu_settings_changed(project);
        ui_state.select_menu_screen(duplicate.id);
    }

    fn duplicate_menu_dialog(ui_state: &mut EditorUI, project: &mut Project, dialog_index: usize) {
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
        Self::mark_menu_settings_changed(project);
        ui_state.select_menu_dialog(duplicate.id);
    }

    fn delete_menu_screen(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
    ) -> bool {
        let removed = project.metadata.runtime.menu.screens.remove(screen_index);
        Self::remove_menu_action_targets(&mut project.metadata.runtime.menu, &removed.id);
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
        Self::mark_menu_settings_changed(project);
        ui_state.sync_menu_editor_selection(Some(project));
        true
    }

    fn delete_menu_dialog(ui_state: &mut EditorUI, project: &mut Project, dialog_index: usize) {
        let removed = project.metadata.runtime.menu.dialogs.remove(dialog_index);
        Self::remove_menu_dialog_action_targets(&mut project.metadata.runtime.menu, &removed.id);
        Self::mark_menu_settings_changed(project);
        ui_state.sync_menu_editor_selection(Some(project));
    }

    fn add_menu_item_to_selected_screen(
        ui_state: &mut EditorUI,
        project: &mut Project,
        item: MenuItemDefinition,
    ) {
        let Some(screen_id) = ui_state.selected_menu_screen_id().map(str::to_string) else {
            return;
        };
        let Some(screen_index) = Self::selected_menu_screen_index(project, &screen_id) else {
            return;
        };
        let screen = &mut project.metadata.runtime.menu.screens[screen_index];
        let item_index = screen.items.len();
        screen.items.push(item);
        Self::mark_menu_settings_changed(project);
        ui_state.select_menu_entry(screen_id, item_index);
    }

    fn duplicate_menu_item(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
    ) {
        let item = project.metadata.runtime.menu.screens[screen_index].items[item_index].clone();
        project.metadata.runtime.menu.screens[screen_index]
            .items
            .insert(item_index + 1, item);
        Self::mark_menu_settings_changed(project);
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
        let (screen_id, remaining_len) = {
            let screen = &mut project.metadata.runtime.menu.screens[screen_index];
            screen.items.remove(item_index);
            (screen.id.clone(), screen.items.len())
        };
        Self::mark_menu_settings_changed(project);
        if item_index < remaining_len {
            ui_state.select_menu_entry(screen_id, item_index);
        } else {
            ui_state.select_menu_screen(screen_id);
        }
    }

    fn move_menu_item(
        ui_state: &mut EditorUI,
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
        direction: isize,
    ) {
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
        Self::mark_menu_settings_changed(project);
        ui_state.select_menu_entry(screen_id, next_index as usize);
    }

    fn coerce_menu_item_kind(
        project: &mut Project,
        screen_index: usize,
        item_index: usize,
        kind: MenuEditorItemKind,
    ) {
        let next_item = match kind {
            MenuEditorItemKind::Label => MenuItemDefinition::Label {
                text: "Text".to_string(),
                border_style_override: None,
            },
            MenuEditorItemKind::Button => MenuItemDefinition::Button {
                text: "Button".to_string(),
                border_style_override: None,
                action: MenuAction::CloseMenu,
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
        Self::mark_menu_settings_changed(project);
    }

    fn normalize_menu_screen_id(input: &str) -> String {
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

    fn next_menu_screen_id(settings: &MenuSettings) -> String {
        Self::next_menu_screen_id_for_base(settings, "new_menu")
    }

    fn next_menu_dialog_id(settings: &MenuSettings) -> String {
        Self::next_menu_dialog_id_for_base(settings, "new_dialog")
    }

    fn next_menu_screen_id_for_base(settings: &MenuSettings, base: &str) -> String {
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

    fn next_menu_dialog_id_for_base(settings: &MenuSettings, base: &str) -> String {
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

    pub(crate) fn rewrite_menu_action_screen_targets(
        settings: &mut MenuSettings,
        previous_id: &str,
        next_id: &str,
    ) {
        for screen in &mut settings.screens {
            for item in &mut screen.items {
                if let MenuItemDefinition::Button {
                    action: MenuAction::OpenScreen { screen_id },
                    ..
                } = item
                {
                    if screen_id == previous_id {
                        *screen_id = next_id.to_string();
                    }
                }
            }
        }
        for dialog in &mut settings.dialogs {
            if let MenuAction::OpenScreen { screen_id } = &mut dialog.confirm_action {
                if screen_id == previous_id {
                    *screen_id = next_id.to_string();
                }
            }
            if let MenuAction::OpenScreen { screen_id } = &mut dialog.cancel_action {
                if screen_id == previous_id {
                    *screen_id = next_id.to_string();
                }
            }
        }
    }

    pub(crate) fn rewrite_menu_action_dialog_targets(
        settings: &mut MenuSettings,
        previous_id: &str,
        next_id: &str,
    ) {
        for screen in &mut settings.screens {
            for item in &mut screen.items {
                if let MenuItemDefinition::Button {
                    action: MenuAction::OpenDialog { dialog_id },
                    ..
                } = item
                {
                    if dialog_id == previous_id {
                        *dialog_id = next_id.to_string();
                    }
                }
            }
        }
        for dialog in &mut settings.dialogs {
            for action in [&mut dialog.confirm_action, &mut dialog.cancel_action] {
                if let MenuAction::OpenDialog { dialog_id } = action {
                    if dialog_id == previous_id {
                        *dialog_id = next_id.to_string();
                    }
                }
            }
        }
    }

    fn remove_menu_action_targets(settings: &mut MenuSettings, removed_id: &str) {
        for screen in &mut settings.screens {
            for item in &mut screen.items {
                if let MenuItemDefinition::Button { action, .. } = item {
                    if matches!(action, MenuAction::OpenScreen { screen_id } if screen_id == removed_id)
                    {
                        *item = MenuItemDefinition::Button {
                            text: match item {
                                MenuItemDefinition::Button { text, .. } => text.clone(),
                                _ => "Button".to_string(),
                            },
                            border_style_override: None,
                            action: MenuAction::Back,
                        };
                    }
                }
            }
        }
        for dialog in &mut settings.dialogs {
            for action in [&mut dialog.confirm_action, &mut dialog.cancel_action] {
                if matches!(action, MenuAction::OpenScreen { screen_id } if screen_id == removed_id)
                {
                    *action = MenuAction::Back;
                }
            }
        }
    }

    fn remove_menu_dialog_action_targets(settings: &mut MenuSettings, removed_id: &str) {
        for screen in &mut settings.screens {
            for item in &mut screen.items {
                if let MenuItemDefinition::Button { action, .. } = item {
                    if matches!(action, MenuAction::OpenDialog { dialog_id } if dialog_id == removed_id)
                    {
                        *action = MenuAction::Back;
                    }
                }
            }
        }
        for dialog in &mut settings.dialogs {
            for action in [&mut dialog.confirm_action, &mut dialog.cancel_action] {
                if matches!(action, MenuAction::OpenDialog { dialog_id } if dialog_id == removed_id)
                {
                    *action = MenuAction::CloseDialog;
                }
            }
        }
    }

    fn mark_menu_settings_changed(project: &mut Project) {
        project.metadata.project.modified = Utc::now();
        project.is_dirty = true;
    }

    fn is_valid_menu_hex_color(hex: &str) -> bool {
        let trimmed = hex.trim().trim_start_matches('#');
        trimmed.len() == 6 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit())
    }

    fn render_menu_border_override_editor(
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
