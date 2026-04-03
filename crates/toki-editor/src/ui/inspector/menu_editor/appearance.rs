//! Menu and dialog theme-override editors.

use super::*;

impl InspectorSystem {
    pub(super) fn render_menu_theme_override_layout(
        ui: &mut egui::Ui,
        theme_override: &mut MenuThemeOverride,
    ) -> bool {
        let mut changed = false;

        egui::CollapsingHeader::new("Layout")
            .default_open(false)
            .show(ui, |ui| {
                changed |= Self::render_drag_value(
                    ui,
                    "Menu Width %",
                    &mut theme_override.menu_width_percent,
                    20..=100,
                );
                changed |= Self::render_drag_value(
                    ui,
                    "Menu Height %",
                    &mut theme_override.menu_height_percent,
                    20..=100,
                );
                changed |= Self::render_drag_value(
                    ui,
                    "Title Spacing",
                    &mut theme_override.title_spacing_px,
                    0..=64,
                );
                changed |= Self::render_drag_value(
                    ui,
                    "Button Spacing",
                    &mut theme_override.button_spacing_px,
                    0..=64,
                );
                changed |= Self::render_drag_value(
                    ui,
                    "Footer Spacing",
                    &mut theme_override.footer_spacing_px,
                    0..=128,
                );
            });

        egui::CollapsingHeader::new("Style")
            .default_open(false)
            .show(ui, |ui| {
                changed |= Self::render_opacity_slider(ui, &mut theme_override.opacity_percent);
            });

        egui::CollapsingHeader::new("Footer")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Footer Text");
                changed |= ui
                    .add(
                        egui::TextEdit::multiline(&mut theme_override.footer_text)
                            .desired_rows(3)
                            .lock_focus(true),
                    )
                    .changed();
            });

        changed
    }

    pub(super) fn render_dialog_theme_override_layout(
        ui: &mut egui::Ui,
        theme_override: &mut DialogThemeOverride,
    ) -> bool {
        let mut changed = false;

        egui::CollapsingHeader::new("Layout")
            .default_open(false)
            .show(ui, |ui| {
                changed |= Self::render_drag_value(
                    ui,
                    "Dialog Width %",
                    &mut theme_override.width_percent,
                    20..=100,
                );
                changed |= Self::render_drag_value(
                    ui,
                    "Title Spacing",
                    &mut theme_override.title_spacing_px,
                    0..=64,
                );
                changed |= Self::render_drag_value(
                    ui,
                    "Body Spacing",
                    &mut theme_override.body_spacing_px,
                    0..=128,
                );
                changed |= Self::render_drag_value(
                    ui,
                    "Button Spacing",
                    &mut theme_override.button_spacing_px,
                    0..=64,
                );
                changed |= Self::render_dialog_position_combo(ui, &mut theme_override.position);
            });

        egui::CollapsingHeader::new("Style")
            .default_open(false)
            .show(ui, |ui| {
                changed |= Self::render_opacity_slider(ui, &mut theme_override.opacity_percent);
            });

        changed
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

    fn render_opacity_slider(ui: &mut egui::Ui, value: &mut u16) -> bool {
        let mut changed = false;
        let mut val = *value;
        ui.horizontal(|ui| {
            ui.label("Opacity %");
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

    fn render_dialog_position_combo(ui: &mut egui::Ui, position: &mut MenuDialogPosition) -> bool {
        let mut selected = *position;
        egui::ComboBox::from_label("Dialog Position")
            .selected_text(match selected {
                MenuDialogPosition::Top => "Top",
                MenuDialogPosition::Bottom => "Bottom",
                MenuDialogPosition::Left => "Left",
                MenuDialogPosition::Right => "Right",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, MenuDialogPosition::Top, "Top");
                ui.selectable_value(&mut selected, MenuDialogPosition::Bottom, "Bottom");
                ui.selectable_value(&mut selected, MenuDialogPosition::Left, "Left");
                ui.selectable_value(&mut selected, MenuDialogPosition::Right, "Right");
            });
        if selected != *position {
            *position = selected;
            return true;
        }
        false
    }
}
