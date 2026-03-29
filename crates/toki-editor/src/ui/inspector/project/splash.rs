pub(super) fn render_splash_section(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.collapsing("Splash", |ui| {
        ui.horizontal(|ui| {
            ui.label("Splash Duration (ms):");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.splash_duration_ms)
                        .speed(25.0)
                        .range(0..=u64::MAX),
                )
                .changed();
        });
    });
    changed
}
