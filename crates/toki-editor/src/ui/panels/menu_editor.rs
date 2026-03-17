use super::super::editor_ui::{EditorUI, Selection};
use crate::project::Project;
use toki_core::menu::{MenuBorderStyle, MenuItemDefinition};

#[derive(Debug, Clone)]
struct MenuPreviewTheme {
    accent: egui::Color32,
    font_family: egui::FontFamily,
    font_size: f32,
    border_style: MenuBorderStyle,
}

pub(super) fn render_menu_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    project: Option<&mut Project>,
) {
    let Some(project) = project else {
        ui.heading("Menu Editor");
        ui.separator();
        ui.label("Open a project to edit runtime menus.");
        return;
    };

    ui_state.sync_menu_editor_selection(Some(project));

    ui.columns(2, |columns| {
        columns[0].set_min_width(180.0);
        columns[0].heading("Screens");
        columns[0].separator();
        for screen in &project.metadata.runtime.menu.screens {
            let selected = matches!(
                ui_state.selection.as_ref(),
                Some(Selection::MenuScreen(screen_id)) if screen_id == &screen.id
            ) || matches!(
                ui_state.selection.as_ref(),
                Some(Selection::MenuEntry { screen_id, .. }) if screen_id == &screen.id
            );
            if columns[0]
                .selectable_label(selected, screen.title.to_string())
                .clicked()
            {
                ui_state.select_menu_screen(screen.id.clone());
            }
            columns[0].small(format!("id: {}", screen.id));
            columns[0].add_space(6.0);
        }

        columns[1].heading("Preview");
        columns[1].separator();
        let Some(selected_screen_id) = ui_state.selected_menu_screen_id().map(str::to_string)
        else {
            columns[1].label("Select a screen to preview it.");
            return;
        };
        let Some(screen) = project
            .metadata
            .runtime
            .menu
            .screens
            .iter()
            .find(|screen| screen.id == selected_screen_id)
        else {
            columns[1].label("Selected screen no longer exists.");
            return;
        };
        let theme = MenuPreviewTheme {
            accent: menu_preview_color(&project.metadata.runtime.menu.appearance.color_hex)
                .unwrap_or(egui::Color32::from_rgb(124, 255, 124)),
            font_family: egui::FontFamily::Name(
                project
                    .metadata
                    .runtime
                    .menu
                    .appearance
                    .font_family
                    .clone()
                    .into(),
            ),
            font_size: project.metadata.runtime.menu.appearance.font_size_px as f32,
            border_style: project.metadata.runtime.menu.appearance.border_style,
        };

        columns[1].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(16))
                .fill(egui::Color32::from_rgba_unmultiplied(
                    ((theme.accent.r() as f32) * 0.16) as u8,
                    ((theme.accent.g() as f32) * 0.16) as u8,
                    ((theme.accent.b() as f32) * 0.16) as u8,
                    224,
                ))
                .stroke(menu_preview_stroke(theme.border_style, theme.accent, 1.5))
                .show(ui, |ui| {
                    let screen_selected = matches!(
                        ui_state.selection.as_ref(),
                        Some(Selection::MenuScreen(screen_id)) if screen_id == &screen.id
                    );
                    let title_text = if screen_selected {
                        egui::RichText::new(&screen.title)
                            .strong()
                            .family(theme.font_family.clone())
                            .size(theme.font_size + 4.0)
                            .color(theme.accent)
                    } else {
                        egui::RichText::new(&screen.title)
                            .family(theme.font_family.clone())
                            .size(theme.font_size + 4.0)
                    };
                    if ui.add(egui::Button::new(title_text).frame(false)).clicked() {
                        ui_state.select_menu_screen(screen.id.clone());
                    }
                    ui.add_space(8.0);

                    for (item_index, item) in screen.items.iter().enumerate() {
                        let selected = matches!(
                            ui_state.selection.as_ref(),
                            Some(Selection::MenuEntry {
                                screen_id,
                                item_index: selected_index,
                            }) if screen_id == &screen.id && *selected_index == item_index
                        );
                        render_menu_item_preview(
                            ui, ui_state, &screen.id, item_index, item, selected, &theme,
                        );
                        ui.add_space(6.0);
                    }
                });
        });
    });
}

fn render_menu_item_preview(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    screen_id: &str,
    item_index: usize,
    item: &MenuItemDefinition,
    selected: bool,
    theme: &MenuPreviewTheme,
) {
    match item {
        MenuItemDefinition::Label { text } => {
            let text = if selected {
                egui::RichText::new(text)
                    .strong()
                    .family(theme.font_family.clone())
                    .size(theme.font_size)
                    .color(theme.accent)
            } else {
                egui::RichText::new(text)
                    .family(theme.font_family.clone())
                    .size(theme.font_size)
            };
            if ui.add(egui::Button::new(text).frame(false)).clicked() {
                ui_state.select_menu_entry(screen_id.to_string(), item_index);
            }
        }
        MenuItemDefinition::Button { text, .. } => {
            let button = egui::Button::new(
                egui::RichText::new(text)
                    .family(theme.font_family.clone())
                    .size(theme.font_size)
                    .color(if selected {
                        theme.accent
                    } else {
                        ui.visuals().text_color()
                    }),
            )
            .selected(selected)
            .stroke(menu_preview_stroke(theme.border_style, theme.accent, 1.2));
            if ui.add_sized([220.0, 28.0], button).clicked() {
                ui_state.select_menu_entry(screen_id.to_string(), item_index);
            }
        }
        MenuItemDefinition::DynamicList {
            heading,
            empty_text,
            ..
        } => {
            let frame = egui::Frame::group(ui.style()).stroke(if selected {
                menu_preview_stroke(theme.border_style, theme.accent, 1.5)
            } else {
                menu_preview_stroke(
                    theme.border_style,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                    1.0,
                )
            });
            let response = frame
                .show(ui, |ui| {
                    ui.set_width(220.0);
                    if let Some(heading) = heading {
                        ui.label(
                            egui::RichText::new(heading)
                                .strong()
                                .family(theme.font_family.clone())
                                .size(theme.font_size),
                        );
                    }
                    ui.label(
                        egui::RichText::new(empty_text)
                            .family(theme.font_family.clone())
                            .size(theme.font_size),
                    );
                    ui.small("Runtime list items appear here.");
                })
                .response;
            if response.clicked() {
                ui_state.select_menu_entry(screen_id.to_string(), item_index);
            }
        }
    }
}

fn menu_preview_color(hex: &str) -> Option<egui::Color32> {
    let trimmed = hex.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let green = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some(egui::Color32::from_rgb(red, green, blue))
}

fn menu_preview_stroke(
    border_style: MenuBorderStyle,
    color: egui::Color32,
    width: f32,
) -> egui::Stroke {
    match border_style {
        MenuBorderStyle::Square => egui::Stroke::new(width, color),
    }
}
