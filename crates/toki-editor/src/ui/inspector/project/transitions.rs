pub(super) fn render_transitions_section(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.collapsing("Scene Transitions", |ui| {
        ui.label("Effect: Fade");
        ui.horizontal(|ui| {
            ui.label("Default Fade Duration:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.transition_default_duration_ms)
                        .speed(1.0)
                        .range(1..=60_000),
                )
                .changed();
            ui.label("ms");
        });
    });
    changed
}
