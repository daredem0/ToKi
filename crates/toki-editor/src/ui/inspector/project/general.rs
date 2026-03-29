pub(super) fn render_general_section(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.collapsing("General", |ui| {
        ui.horizontal(|ui| {
            ui.label("Name:");
            changed |= ui.text_edit_singleline(&mut draft.name).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Version:");
            changed |= ui.text_edit_singleline(&mut draft.version).changed();
        });
        ui.label("Description:");
        changed |= ui
            .add(
                egui::TextEdit::multiline(&mut draft.description)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            )
            .changed();
    });
    changed
}
