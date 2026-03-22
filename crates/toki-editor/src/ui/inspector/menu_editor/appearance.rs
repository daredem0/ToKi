//! Menu appearance settings editors.

use super::*;

impl InspectorSystem {
    pub(super) fn render_typography_header(
        ui: &mut egui::Ui,
        ui_state: &EditorUI,
        ctx: &mut AppearanceEditContext,
    ) {
        egui::CollapsingHeader::new("Typography")
            .default_open(false)
            .show(ui, |ui| {
                let font_choices = if ui_state.menu_preview_font_families.is_empty() {
                    vec!["Sans".to_string(), "Serif".to_string(), "Mono".to_string()]
                } else {
                    ui_state.menu_preview_font_families.clone()
                };
                ctx.changed |= Self::render_font_family_combo(
                    ui,
                    &mut ctx.appearance.font_family,
                    &font_choices,
                );
                ctx.changed |= Self::render_drag_value(
                    ui,
                    "Font Size",
                    &mut ctx.appearance.font_size_px,
                    8..=64,
                );
            });
    }

    pub(super) fn render_font_family_combo(
        ui: &mut egui::Ui,
        current: &mut String,
        choices: &[String],
    ) -> bool {
        let mut selected = current.clone();
        egui::ComboBox::from_label("Font Family")
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                for family in choices {
                    ui.selectable_value(&mut selected, family.clone(), family);
                }
            });
        if selected != *current {
            *current = selected;
            return true;
        }
        false
    }

    pub(super) fn render_drag_value(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut u16,
        range: std::ops::RangeInclusive<u16>,
    ) -> bool {
        let mut changed = false;
        let mut val = *value;
        ui.horizontal(|ui| {
            ui.label(label);
            if ui
                .add(egui::DragValue::new(&mut val).range(range).speed(1.0))
                .changed()
            {
                *value = val;
                changed = true;
            }
        });
        changed
    }

    pub(super) fn render_layout_header(ui: &mut egui::Ui, ctx: &mut AppearanceEditContext) {
        egui::CollapsingHeader::new("Layout")
            .default_open(false)
            .show(ui, |ui| {
                ctx.changed |= Self::render_drag_value(
                    ui,
                    "Menu Width %",
                    &mut ctx.appearance.menu_width_percent,
                    20..=100,
                );
                ctx.changed |= Self::render_drag_value(
                    ui,
                    "Menu Height %",
                    &mut ctx.appearance.menu_height_percent,
                    20..=100,
                );
                ctx.changed |= Self::render_drag_value(
                    ui,
                    "Title Spacing",
                    &mut ctx.appearance.title_spacing_px,
                    0..=64,
                );
                ctx.changed |= Self::render_drag_value(
                    ui,
                    "Button Spacing",
                    &mut ctx.appearance.button_spacing_px,
                    0..=64,
                );
                ctx.changed |= Self::render_drag_value(
                    ui,
                    "Footer Spacing",
                    &mut ctx.appearance.footer_spacing_px,
                    0..=128,
                );
            });
    }

    pub(super) fn render_style_header(ui: &mut egui::Ui, ctx: &mut AppearanceEditContext) {
        egui::CollapsingHeader::new("Style")
            .default_open(false)
            .show(ui, |ui| {
                ctx.changed |= Self::render_opacity_slider(ui, &mut ctx.appearance.opacity_percent);
                ctx.changed |=
                    Self::render_border_style_combo(ui, &mut ctx.appearance.border_style);
                ctx.changed |= Self::render_hex_color_field(
                    ui,
                    "Border Color Hex",
                    &mut ctx.appearance.border_color_hex,
                    "#7CFF7C",
                );
                ctx.changed |= Self::render_hex_color_field(
                    ui,
                    "Text Color Hex",
                    &mut ctx.appearance.text_color_hex,
                    "#FFFFFF",
                );
            });
    }

    fn render_opacity_slider(ui: &mut egui::Ui, value: &mut u16) -> bool {
        let mut changed = false;
        let mut val = *value;
        ui.horizontal(|ui| {
            ui.label("Menu Opacity %");
            if ui
                .add(egui::Slider::new(&mut val, 0..=100).clamping(egui::SliderClamping::Always))
                .changed()
            {
                *value = val;
                changed = true;
            }
        });
        changed
    }

    pub(super) fn render_border_style_combo(
        ui: &mut egui::Ui,
        style: &mut MenuBorderStyle,
    ) -> bool {
        let mut selected = *style;
        egui::ComboBox::from_label("Border Style")
            .selected_text(match selected {
                MenuBorderStyle::None => "None",
                MenuBorderStyle::Square => "Square",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, MenuBorderStyle::None, "None");
                ui.selectable_value(&mut selected, MenuBorderStyle::Square, "Square");
            });
        if selected != *style {
            *style = selected;
            return true;
        }
        false
    }

    pub(super) fn render_hex_color_field(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut String,
        example: &str,
    ) -> bool {
        ui.label(label);
        let changed = ui.text_edit_singleline(value).changed();
        if !Self::is_valid_menu_hex_color(value) {
            ui.colored_label(
                egui::Color32::from_rgb(215, 120, 120),
                format!("Use a 6-digit hex color like {}", example),
            );
        }
        changed
    }

    pub(super) fn render_backgrounds_header(ui: &mut egui::Ui, ctx: &mut AppearanceEditContext) {
        egui::CollapsingHeader::new("Backgrounds")
            .default_open(false)
            .show(ui, |ui| {
                ctx.changed |= Self::render_background_section(
                    ui,
                    "Menu Background",
                    "Transparent Menu Background",
                    &mut ctx.appearance.menu_background_transparent,
                    &mut ctx.appearance.menu_background_color_hex,
                    "#142914",
                );
                ctx.changed |= Self::render_background_section(
                    ui,
                    "Title Background",
                    "Transparent Title Background",
                    &mut ctx.appearance.title_background_transparent,
                    &mut ctx.appearance.title_background_color_hex,
                    "#143614",
                );
                ctx.changed |= Self::render_background_section(
                    ui,
                    "Entry Background",
                    "Transparent Entry Background",
                    &mut ctx.appearance.entry_background_transparent,
                    &mut ctx.appearance.entry_background_color_hex,
                    "#0F1F0F",
                );
            });
    }

    fn render_background_section(
        ui: &mut egui::Ui,
        label: &str,
        checkbox_label: &str,
        transparent: &mut bool,
        color_hex: &mut String,
        example: &str,
    ) -> bool {
        ui.label(label);
        let mut changed = ui.checkbox(transparent, checkbox_label).changed();
        changed |= ui.text_edit_singleline(color_hex).changed();
        if !Self::is_valid_menu_hex_color(color_hex) {
            ui.colored_label(
                egui::Color32::from_rgb(215, 120, 120),
                format!("Use a 6-digit hex color like {}", example),
            );
        }
        changed
    }

    pub(super) fn render_footer_header(ui: &mut egui::Ui, ctx: &mut AppearanceEditContext) {
        egui::CollapsingHeader::new("Footer")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Footer Text");
                ctx.changed |= ui
                    .add(
                        egui::TextEdit::multiline(&mut ctx.appearance.footer_text)
                            .desired_rows(3)
                            .lock_focus(true),
                    )
                    .changed();
            });
    }
}
