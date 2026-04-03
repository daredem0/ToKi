use super::super::editor_ui::{EditorUI, Selection};
use crate::fonts::resolve_preview_font_family;
use crate::project::Project;
use egui::text::{LayoutJob, TextFormat};
use toki_core::menu::{
    build_dialog_layout, build_menu_layout, compose_dialog_output, compose_menu_output,
    resolve_dialog_appearance, resolve_menu_appearance, MenuItemDefinition, MenuView,
    MenuViewEntry,
};
use toki_core::ui::{UiBlock, UiComposition};

pub(crate) fn render_menu_editor(
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

    crate::ui::editor_ui::sync_menu_editor_selection(ui_state, Some(project));

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
                        crate::ui::editor_ui::select_menu_screen(ui_state, screen.id.clone());
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
                        crate::ui::editor_ui::select_menu_dialog(ui_state, dialog.id.clone());
                    }
                }
            });

        if let Some(screen_id) = crate::ui::editor_ui::selected_menu_screen_id(ui_state) {
            ui.small(format!("id: {screen_id}"));
        } else if let Some(dialog_id) = crate::ui::editor_ui::selected_menu_dialog_id(ui_state) {
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
            let appearance = resolve_dialog_appearance(
                &project.metadata.runtime.ui.theme,
                &project.metadata.runtime.dialog_theme_override,
            );
            let layout = build_dialog_layout(
                &toki_core::menu::MenuDialogView {
                    dialog_id: dialog.id.clone(),
                    title: dialog.title.clone(),
                    body: dialog.body.clone(),
                    entries: vec![
                        toki_core::menu::MenuViewEntry {
                            text: dialog.confirm_text.clone(),
                            selected: true,
                            selectable: true,
                            border_style_override: None,
                        },
                        toki_core::menu::MenuViewEntry {
                            text: dialog.cancel_text.clone(),
                            selected: false,
                            selectable: true,
                            border_style_override: None,
                        },
                    ],
                    hide_main_menu: dialog.hide_main_menu,
                },
                &appearance,
                viewport,
            );
            let composition = compose_dialog_output(&layout, &appearance).composition;
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
            if let Some(clipped_panel_rect) = clipped_interaction_rect(panel_rect, rect) {
                if ui
                    .interact(
                        clipped_panel_rect,
                        ui.id().with(("menu_dialog", &dialog.id)),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    crate::ui::editor_ui::select_menu_dialog(ui_state, dialog.id.clone());
                }
            }
        }
        _ => {
            let Some(selected_screen_id) =
                crate::ui::editor_ui::selected_menu_screen_id(ui_state).map(str::to_string)
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
            let appearance = resolve_menu_appearance(
                &project.metadata.runtime.ui.theme,
                &project.metadata.runtime.menu.theme_override,
            );
            let layout = build_menu_layout(
                &MenuView {
                    screen_id: screen.id.clone(),
                    title: screen.title.clone(),
                    title_border_style_override: screen.title_border_style_override,
                    entries,
                },
                &appearance,
                viewport,
            );
            let composition = compose_menu_output(&layout, &appearance).composition;
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
            if let Some(clipped_title_rect) = clipped_interaction_rect(title_rect, rect) {
                if ui
                    .interact(
                        clipped_title_rect,
                        ui.id().with(("menu_title", &screen.id)),
                        egui::Sense::click(),
                    )
                    .clicked()
                {
                    crate::ui::editor_ui::select_menu_screen(ui_state, screen.id.clone());
                }
            }

            for (item_index, entry) in layout.entries.iter().enumerate() {
                let entry_rect = translated_rect(&entry.rect, origin);
                let id = ui.id().with(("menu_entry", &screen.id, item_index));
                if let Some(clipped_entry_rect) = clipped_interaction_rect(entry_rect, rect) {
                    if ui
                        .interact(clipped_entry_rect, id, egui::Sense::click())
                        .clicked()
                    {
                        crate::ui::editor_ui::select_menu_entry(
                            ui_state,
                            screen.id.clone(),
                            item_index,
                        );
                    }
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
            egui::Stroke::new(
                block.border_thickness.max(1.0),
                menu_preview_color32(border),
            ),
            egui::StrokeKind::Outside,
        );
    }
    if let Some(text) = &block.text {
        let pos = egui::pos2(text.position.x + origin.x, text.position.y + origin.y);
        let font_family = resolve_preview_font_family(&text.style.font_family, available_fonts);
        let mut format = TextFormat::simple(
            egui::FontId::new(text.style.size_px, font_family),
            menu_preview_color32(text.style.color),
        );
        format.italics = matches!(text.style.slant, toki_core::text::TextSlant::Italic);
        let mut job = LayoutJob::default();
        job.wrap.max_width = text.max_width.unwrap_or(f32::INFINITY);
        job.append(&text.content, 0.0, format);
        let galley = painter.layout_job(job);
        let galley_pos = text_anchor_to_align2(text.anchor)
            .anchor_rect(egui::Rect::from_min_size(pos, galley.size()))
            .min;
        painter.galley(galley_pos, galley, menu_preview_color32(text.style.color));
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

fn clipped_interaction_rect(
    interaction_rect: egui::Rect,
    bounds: egui::Rect,
) -> Option<egui::Rect> {
    let clipped = interaction_rect.intersect(bounds);
    (clipped.width() > 0.0 && clipped.height() > 0.0).then_some(clipped)
}

#[cfg(test)]
mod tests {
    use super::clipped_interaction_rect;

    #[test]
    fn clipped_interaction_rect_discards_non_overlapping_rects() {
        let a = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0));
        let b = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(5.0, 5.0));
        assert_eq!(clipped_interaction_rect(a, b), None);
    }

    #[test]
    fn clipped_interaction_rect_clamps_to_bounds() {
        let interaction = egui::Rect::from_min_size(egui::pos2(-5.0, -4.0), egui::vec2(20.0, 20.0));
        let bounds = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(8.0, 6.0));
        let clipped = clipped_interaction_rect(interaction, bounds).expect("should overlap");
        assert_eq!(clipped.min, bounds.min);
        assert_eq!(clipped.max, bounds.max);
    }
}
