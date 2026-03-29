use super::*;
use crate::project::ProjectAssets;
use chrono::Utc;

impl InspectorSystem {
    pub(crate) fn render_dialog_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
        project_assets: Option<&mut ProjectAssets>,
    ) {
        let Some(project_assets) = project_assets else {
            ui.heading("Dialog Editor");
            ui.separator();
            ui.label("Open a project to edit dialog assets.");
            return;
        };

        ui.heading("Dialog Editor");
        ui.separator();
        let mut project_ref = None::<&Project>;
        if let Some(project) = project {
            ui.heading("Dialog Style");
            let changed = Self::render_menu_appearance_editor(
                ui_state,
                ui,
                &mut project.metadata.runtime.dialog_appearance,
                true,
            );

            if changed {
                project.metadata.project.modified = Utc::now();
                project.is_dirty = true;
            }
            project_ref = Some(&*project);
        } else {
            ui.heading("Dialog Style");
            ui.separator();
            ui.label("Open a project to edit dialog appearance.");
        }
        ui.separator();
        crate::ui::panels::dialog_editor::render_dialog_inspector_panel(
            ui,
            ui_state,
            project_assets,
            project_ref,
        );
    }
}
