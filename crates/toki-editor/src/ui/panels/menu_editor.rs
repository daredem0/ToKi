use super::super::editor_ui::{EditorUI, Selection};
use crate::fonts::resolve_preview_font_family;
use crate::project::Project;
use toki_core::menu::{
    build_dialog_layout, build_menu_layout, compose_dialog_ui, compose_menu_ui, MenuItemDefinition,
    MenuView, MenuViewEntry,
};
use toki_core::ui::{UiBlock, UiComposition};

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

    ui.horizontal(|ui| {
        ui.label("Surface");

        let selected_surface_label = match ui_state.selection.as_ref() {
            Some(Selection::MenuScreen(screen_id))
            | Some(Selection::MenuEntry { screen_id, .. }) => project
                .metadata
                .runtime
                .menu
                .screens
                .iter()
                .find(|screen| &screen.id == screen_id)
                .map(|screen| format!("Screen: {}", screen.title))
                .unwrap_or_else(|| "Select surface".to_string()),
            Some(Selection::MenuDialog(dialog_id)) => project
                .metadata
                .runtime
                .menu
                .dialogs
                .iter()
                .find(|dialog| &dialog.id == dialog_id)
                .map(|dialog| format!("Dialog: {}", dialog.title))
                .unwrap_or_else(|| "Select surface".to_string()),
            _ => project
                .metadata
                .runtime
                .menu
                .screens
                .first()
                .map(|screen| format!("Screen: {}", screen.title))
                .or_else(|| {
                    project
                        .metadata
                        .runtime
                        .menu
                        .dialogs
                        .first()
                        .map(|dialog| format!("Dialog: {}", dialog.title))
                })
                .unwrap_or_else(|| "Select surface".to_string()),
        };

        egui::ComboBox::from_id_salt("menu_editor_screen_selector")
            .selected_text(selected_surface_label)
            .width(220.0)
            .show_ui(ui, |ui| {
                for screen in &project.metadata.runtime.menu.screens {
                    let selected = matches!(
                        ui_state.selection.as_ref(),
                        Some(Selection::MenuScreen(id)) | Some(Selection::MenuEntry { screen_id: id, .. })
                            if id == &screen.id
                    );
                    if ui.selectable_label(selected, &screen.title).clicked() {
                        ui_state.select_menu_screen(screen.id.clone());
                    }
                }
                if !project.metadata.runtime.menu.dialogs.is_empty()
                    && !project.metadata.runtime.menu.screens.is_empty()
                {
                    ui.separator();
                }
                for dialog in &project.metadata.runtime.menu.dialogs {
                    let selected = matches!(
                        ui_state.selection.as_ref(),
                        Some(Selection::MenuDialog(id)) if id == &dialog.id
                    );
                    if ui
                        .selectable_label(selected, format!("Dialog: {}", dialog.title))
                        .clicked()
                    {
                        ui_state.select_menu_dialog(dialog.id.clone());
                    }
                }
            });

        if let Some(screen_id) = ui_state.selected_menu_screen_id() {
            ui.small(format!("id: {screen_id}"));
        } else if let Some(dialog_id) = ui_state.selected_menu_dialog_id() {
            ui.small(format!("id: {dialog_id}"));
        }
    });
    ui.separator();

    let available = ui.available_size();
    let viewport = glam::Vec2::new(available.x.max(320.0), available.y.max(240.0));

    let (rect, _response) = ui.allocate_exact_size(available, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    match ui_state.selection.as_ref() {
        Some(Selection::MenuDialog(dialog_id)) => {
            let Some(dialog) = project
                .metadata
                .runtime
                .menu
                .dialogs
                .iter()
                .find(|dialog| dialog.id == *dialog_id)
            else {
                ui.label("Selected dialog no longer exists.");
                return;
            };
            let layout = build_dialog_layout(
                &toki_core::menu::MenuDialogView {
                    dialog_id: dialog.id.clone(),
                    title: dialog.title.clone(),
                    body: dialog.body.clone(),
                    confirm_text: dialog.confirm_text.clone(),
                    cancel_text: dialog.cancel_text.clone(),
                    confirm_selected: true,
                    hide_main_menu: dialog.hide_main_menu,
                },
                &project.metadata.runtime.menu.appearance,
                viewport,
            );
            let composition = compose_dialog_ui(&layout, &project.metadata.runtime.menu.appearance);
            let origin = egui::vec2(
                rect.center().x - layout.panel.width * 0.5 - layout.panel.x,
                rect.center().y - layout.panel.height * 0.5 - layout.panel.y,
            );
            paint_ui_composition(
                &painter,
                &composition,
                origin,
                &ui_state.menu_preview_font_families,
            );
            let panel_rect = translated_rect(&layout.panel, origin);
            if ui
                .interact(
                    panel_rect,
                    ui.id().with(("menu_dialog", &dialog.id)),
                    egui::Sense::click(),
                )
                .clicked()
            {
                ui_state.select_menu_dialog(dialog.id.clone());
            }
        }
        _ => {
            let Some(selected_screen_id) = ui_state.selected_menu_screen_id().map(str::to_string)
            else {
                ui.label("Select a screen or dialog to preview it.");
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
                ui.label("Selected screen no longer exists.");
                return;
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
                    MenuItemDefinition::Label {
                        text,
                        border_style_override,
                    } => entries.push(MenuViewEntry {
                        text: text.clone(),
                        selected: false,
                        selectable: false,
                        border_style_override: *border_style_override,
                    }),
                    MenuItemDefinition::Button {
                        text,
                        border_style_override,
                        ..
                    } => entries.push(MenuViewEntry {
                        text: text.clone(),
                        selected: selected_entry_index == Some(item_index),
                        selectable: true,
                        border_style_override: *border_style_override,
                    }),
                    MenuItemDefinition::DynamicList {
                        heading,
                        empty_text,
                        border_style_override,
                        ..
                    } => {
                        if let Some(heading) = heading {
                            entries.push(MenuViewEntry {
                                text: heading.clone(),
                                selected: false,
                                selectable: false,
                                border_style_override: *border_style_override,
                            });
                        }
                        entries.push(MenuViewEntry {
                            text: empty_text.clone(),
                            selected: false,
                            selectable: false,
                            border_style_override: *border_style_override,
                        });
                    }
                }
            }
            let layout = build_menu_layout(
                &MenuView {
                    screen_id: screen.id.clone(),
                    title: screen.title.clone(),
                    title_border_style_override: screen.title_border_style_override,
                    entries,
                },
                &project.metadata.runtime.menu.appearance,
                viewport,
            );
            let composition = compose_menu_ui(&layout, &project.metadata.runtime.menu.appearance);
            let origin = egui::vec2(
                rect.center().x - layout.panel.width * 0.5 - layout.panel.x,
                rect.center().y - layout.panel.height * 0.5 - layout.panel.y,
            );
            paint_ui_composition(
                &painter,
                &composition,
                origin,
                &ui_state.menu_preview_font_families,
            );

            let title_rect = translated_rect(&layout.title.rect, origin);
            if ui
                .interact(
                    title_rect,
                    ui.id().with(("menu_title", &screen.id)),
                    egui::Sense::click(),
                )
                .clicked()
            {
                ui_state.select_menu_screen(screen.id.clone());
            }

            for (item_index, entry) in layout.entries.iter().enumerate() {
                let entry_rect = translated_rect(&entry.rect, origin);
                let id = ui.id().with(("menu_entry", &screen.id, item_index));
                if ui.interact(entry_rect, id, egui::Sense::click()).clicked() {
                    ui_state.select_menu_entry(screen.id.clone(), item_index);
                }
            }
        }
    }
}

fn paint_ui_composition(
    painter: &egui::Painter,
    composition: &UiComposition,
    origin: egui::Vec2,
    available_fonts: &[String],
) {
    for block in &composition.blocks {
        paint_ui_block(painter, block, origin, available_fonts);
    }
}

fn paint_ui_block(
    painter: &egui::Painter,
    block: &UiBlock,
    origin: egui::Vec2,
    available_fonts: &[String],
) {
    let rect = translated_rect(&block.rect, origin);
    if let Some(fill) = block.fill_color {
        painter.rect_filled(rect, 0.0, menu_preview_color32(fill));
    }
    if let Some(border) = block.border_color {
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.5, menu_preview_color32(border)),
            egui::StrokeKind::Outside,
        );
    }
    if let Some(text) = &block.text {
        let pos = egui::pos2(text.position.x + origin.x, text.position.y + origin.y);
        let font_family = resolve_preview_font_family(&text.style.font_family, available_fonts);
        painter.text(
            pos,
            text_anchor_to_align2(text.anchor),
            &text.content,
            egui::FontId::new(text.style.size_px, font_family),
            menu_preview_color32(text.style.color),
        );
    }
}

fn text_anchor_to_align2(anchor: toki_core::text::TextAnchor) -> egui::Align2 {
    match anchor {
        toki_core::text::TextAnchor::TopLeft => egui::Align2::LEFT_TOP,
        toki_core::text::TextAnchor::TopCenter => egui::Align2::CENTER_TOP,
        toki_core::text::TextAnchor::TopRight => egui::Align2::RIGHT_TOP,
        toki_core::text::TextAnchor::CenterLeft => egui::Align2::LEFT_CENTER,
        toki_core::text::TextAnchor::Center => egui::Align2::CENTER_CENTER,
        toki_core::text::TextAnchor::CenterRight => egui::Align2::RIGHT_CENTER,
        toki_core::text::TextAnchor::BottomLeft => egui::Align2::LEFT_BOTTOM,
        toki_core::text::TextAnchor::BottomCenter => egui::Align2::CENTER_BOTTOM,
        toki_core::text::TextAnchor::BottomRight => egui::Align2::RIGHT_BOTTOM,
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

fn translated_rect(rect: &toki_core::menu::MenuRect, origin: egui::Vec2) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(rect.x + origin.x, rect.y + origin.y),
        egui::vec2(rect.width, rect.height),
    )
}
