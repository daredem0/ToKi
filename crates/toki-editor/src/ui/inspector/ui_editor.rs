use super::*;
use crate::project::ProjectAssets;
use chrono::Utc;

impl InspectorSystem {
    pub(crate) fn render_ui_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
        project_assets: Option<&mut ProjectAssets>,
    ) {
        let Some(project_assets) = project_assets else {
            ui.heading("UI Editor");
            ui.separator();
            ui.label("Open a project to edit UI layouts.");
            return;
        };

        ui.heading("UI Editor");
        ui.separator();
        if let Some(project) = project {
            ui.collapsing("UI Theme", |ui| {
                let theme = &mut project.metadata.runtime.ui.theme;
                ui.horizontal(|ui| {
                    ui.label("Font Family:");
                    if ui.text_edit_singleline(&mut theme.font_family).changed() {
                        project.metadata.project.modified = Utc::now();
                        project.is_dirty = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Base Font:");
                    let mut font_size = theme.base_font_size_px as i32;
                    if ui
                        .add(egui::DragValue::new(&mut font_size).range(8..=64))
                        .changed()
                    {
                        theme.base_font_size_px = font_size.max(8) as u16;
                        project.metadata.project.modified = Utc::now();
                        project.is_dirty = true;
                    }
                });
            });
            ui.separator();
            crate::ui::panels::ui_editor::render_ui_editor_inspector_panel(
                ui,
                ui_state,
                project_assets,
                Some(&*project),
            );
        } else {
            crate::ui::panels::ui_editor::render_ui_editor_inspector_panel(
                ui,
                ui_state,
                project_assets,
                None,
            );
        }
    }
}
