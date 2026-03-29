use super::*;

pub(super) fn render_metadata_section(ui: &mut egui::Ui, project: &Project) {
    ui.collapsing("Metadata", |ui| {
        ui.horizontal(|ui| {
            ui.label("Created:");
            ui.monospace(project.metadata.project.created.to_rfc3339());
        });
        ui.horizontal(|ui| {
            ui.label("Modified:");
            ui.monospace(project.metadata.project.modified.to_rfc3339());
        });
        ui.horizontal(|ui| {
            ui.label("Current Editor Version:");
            ui.monospace(env!("TOKI_VERSION"));
        });
        ui.horizontal(|ui| {
            ui.label("Project Created With:");
            ui.monospace(&project.metadata.project.toki_editor_version);
        });
    });
}
