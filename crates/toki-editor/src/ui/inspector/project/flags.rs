use super::*;

pub(super) fn render_flags_section(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.collapsing("Flags", |ui| {
        for issue in validate_flag_registry(&draft.flag_declarations) {
            ui.colored_label(issue.color, issue.message);
        }

        let mut remove_index = None;
        for (index, declaration) in draft.flag_declarations.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Flag {}", index + 1));
                    if ui.small_button("Delete").clicked() {
                        remove_index = Some(index);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Id:");
                    changed |= ui.text_edit_singleline(&mut declaration.id).changed();
                });
                changed |= InspectorSystem::render_flag_value_editor(
                    ui,
                    format!("project_flag_default_{index}"),
                    &mut declaration.default_value,
                );
            });
        }
        if let Some(index) = remove_index {
            draft.flag_declarations.remove(index);
            changed = true;
        }
        if ui.button("+ Add Flag").clicked() {
            draft
                .flag_declarations
                .push(toki_core::project_runtime::ProjectFlagDefinition {
                    id: String::new(),
                    default_value: toki_core::FlagValue::Bool(false),
                });
            changed = true;
        }
    });
    changed
}

