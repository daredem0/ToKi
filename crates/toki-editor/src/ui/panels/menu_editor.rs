use super::super::editor_ui::{EditorUI, Selection};
use crate::fonts::resolve_preview_font_family;
use crate::project::Project;
use toki_core::menu::{
    build_menu_layout, menu_border_color, menu_hex_color_rgba, tinted_menu_background,
    MenuBorderStyle, MenuItemDefinition, MenuView, MenuViewEntry,
};

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
            accent: menu_hex_color_rgba(&project.metadata.runtime.menu.appearance.color_hex)
                .map(menu_preview_color32)
                .unwrap_or(egui::Color32::from_rgb(124, 255, 124)),
            font_family: resolve_preview_font_family(
                &project.metadata.runtime.menu.appearance.font_family,
                &ui_state.menu_preview_font_families,
            ),
            font_size: project.metadata.runtime.menu.appearance.font_size_px as f32,
            border_style: project.metadata.runtime.menu.appearance.border_style,
        };
        let selected_entry_index = match ui_state.selection.as_ref() {
            Some(Selection::MenuEntry {
                screen_id,
                item_index,
            }) if screen_id == &screen.id => Some(*item_index),
            _ => None,
        };
        let mut entries = Vec::new();
        for (item_index, item) in screen.items.iter().enumerate() {
            match item {
                MenuItemDefinition::Label { text } => entries.push(MenuViewEntry {
                    text: text.clone(),
                    selected: false,
                    selectable: false,
                }),
                MenuItemDefinition::Button { text, .. } => entries.push(MenuViewEntry {
                    text: text.clone(),
                    selected: selected_entry_index == Some(item_index),
                    selectable: true,
                }),
                MenuItemDefinition::DynamicList {
                    heading,
                    empty_text,
                    ..
                } => {
                    if let Some(heading) = heading {
                        entries.push(MenuViewEntry {
                            text: heading.clone(),
                            selected: false,
                            selectable: false,
                        });
                    }
                    entries.push(MenuViewEntry {
                        text: empty_text.clone(),
                        selected: false,
                        selectable: false,
                    });
                }
            }
        }
        let layout = build_menu_layout(
            &MenuView {
                screen_id: screen.id.clone(),
                title: screen.title.clone(),
                entries,
            },
            &project.metadata.runtime.menu.appearance,
            glam::Vec2::new(360.0, 240.0),
        );

        columns[1].with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            let desired_size = egui::vec2(layout.panel.width + 40.0, 240.0);
            let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let origin = egui::vec2(
                rect.center().x - layout.panel.width * 0.5,
                rect.top() + 8.0 - layout.panel.y,
            );
            let panel_rect = translated_rect(&layout.panel, origin);
            painter.rect_filled(
                panel_rect,
                0.0,
                menu_preview_color32(tinted_menu_background(
                    color32_to_rgba(theme.accent),
                    0.16,
                    0.88,
                )),
            );
            painter.rect_stroke(
                panel_rect,
                0.0,
                menu_preview_stroke(theme.border_style, theme.accent, 1.5),
                egui::StrokeKind::Outside,
            );

            let screen_selected = matches!(
                ui_state.selection.as_ref(),
                Some(Selection::MenuScreen(screen_id)) if screen_id == &screen.id
            );
            let title_rect = translated_rect(&layout.title.rect, origin);
            painter.rect_filled(
                title_rect,
                0.0,
                menu_preview_color32(tinted_menu_background(
                    color32_to_rgba(theme.accent),
                    0.16,
                    0.9,
                )),
            );
            if let Some(border) = menu_border_color(
                theme.border_style,
                color32_to_rgba(theme.accent),
                0.95,
            ) {
                painter.rect_stroke(
                    title_rect,
                    0.0,
                    egui::Stroke::new(1.5, menu_preview_color32(border)),
                    egui::StrokeKind::Outside,
                );
            }
            if ui
                .interact(title_rect, ui.id().with(("menu_title", &screen.id)), egui::Sense::click())
                .clicked()
            {
                ui_state.select_menu_screen(screen.id.clone());
            }
            painter.text(
                title_rect.center_top() + egui::vec2(0.0, 10.0),
                egui::Align2::CENTER_TOP,
                &layout.title.text,
                egui::FontId::new(theme.font_size + 4.0, theme.font_family.clone()),
                if screen_selected { theme.accent } else { ui.visuals().text_color() },
            );

            for (item_index, entry) in layout.entries.iter().enumerate() {
                let entry_rect = translated_rect(&entry.rect, origin);
                let id = ui.id().with(("menu_entry", &screen.id, item_index));
                if ui.interact(entry_rect, id, egui::Sense::click()).clicked() {
                    ui_state.select_menu_entry(screen.id.clone(), item_index);
                }
                let fill = if entry.selected {
                    tinted_menu_background(color32_to_rgba(theme.accent), 0.22, 0.88)
                } else if entry.selectable {
                    tinted_menu_background(color32_to_rgba(theme.accent), 0.08, 0.72)
                } else {
                    [0.0, 0.0, 0.0, 0.45]
                };
                painter.rect_filled(entry_rect, 0.0, menu_preview_color32(fill));
                if entry.selectable {
                    let alpha = if entry.selected { 0.95 } else { 0.55 };
                    if let Some(border) =
                        menu_border_color(theme.border_style, color32_to_rgba(theme.accent), alpha)
                    {
                        painter.rect_stroke(
                            entry_rect,
                            0.0,
                            egui::Stroke::new(
                                if entry.selected { 1.5 } else { 1.2 },
                                menu_preview_color32(border),
                            ),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
                painter.text(
                    entry_rect.center_top() + egui::vec2(0.0, 6.0),
                    egui::Align2::CENTER_TOP,
                    if entry.selected {
                        format!("> {}", entry.text)
                    } else {
                        format!("  {}", entry.text)
                    },
                    egui::FontId::new(theme.font_size, theme.font_family.clone()),
                    if entry.selected { theme.accent } else { ui.visuals().text_color() },
                );
            }

            let hint_rect = translated_rect(&layout.hint.rect, origin);
            painter.rect_filled(hint_rect, 0.0, menu_preview_color32([0.0, 0.0, 0.0, 0.65]));
            painter.text(
                hint_rect.center_top() + egui::vec2(0.0, 4.0),
                egui::Align2::CENTER_TOP,
                &layout.hint.text,
                egui::FontId::new((theme.font_size - 2.0).max(10.0), theme.font_family.clone()),
                egui::Color32::from_rgb(217, 217, 217),
            );
        });
    });
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

fn menu_preview_color32(rgba: [f32; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn color32_to_rgba(color: egui::Color32) -> [f32; 4] {
    [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
        color.a() as f32 / 255.0,
    ]
}

fn translated_rect(rect: &toki_core::menu::MenuRect, origin: egui::Vec2) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(rect.x + origin.x, rect.y + origin.y),
        egui::vec2(rect.width, rect.height),
    )
}
