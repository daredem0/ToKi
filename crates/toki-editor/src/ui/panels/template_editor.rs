use crate::project::Project;
use crate::ui::editor_ui::EditorUI;
use crate::ui::template_workflow::{
    available_template_catalog, filtered_descriptors, preview_selected_template,
    summary_line_for_parameter_value, sync_template_editor_state, template_categories,
    TemplateAssetChoices,
};

pub(super) fn render_template_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    project: Option<&Project>,
    _template_asset_choices: Option<&TemplateAssetChoices>,
) {
    let Some(project) = project else {
        ui.heading("Templates");
        ui.separator();
        ui.label("Open a project to browse and apply templates.");
        return;
    };

    let catalog = available_template_catalog(Some(project));
    let descriptors = catalog.descriptors;
    sync_template_editor_state(&mut ui_state.template, &descriptors);
    let categories = template_categories(&descriptors);

    ui.horizontal(|ui| {
        ui.label("Category");
        egui::ComboBox::from_id_salt("template_category_filter")
            .selected_text(
                ui_state
                    .template
                    .category_filter
                    .clone()
                    .unwrap_or_else(|| "All".to_string()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut ui_state.template.category_filter, None, "All");
                for category in &categories {
                    ui.selectable_value(
                        &mut ui_state.template.category_filter,
                        Some(category.clone()),
                        category,
                    );
                }
            });
    });
    ui.separator();
    for diagnostic in &catalog.diagnostics {
        ui.colored_label(egui::Color32::from_rgb(215, 180, 110), diagnostic);
    }
    if !catalog.diagnostics.is_empty() {
        ui.separator();
    }

    let filtered = filtered_descriptors(&descriptors, ui_state.template.category_filter.as_deref());
    sync_template_editor_state(&mut ui_state.template, &filtered);

    ui.columns(2, |columns| {
        columns[0].heading("Templates");
        columns[0].separator();
        egui::ScrollArea::vertical().show(&mut columns[0], |ui| {
            for descriptor in &filtered {
                let selected = ui_state
                    .template
                    .selected_template_id
                    .as_ref()
                    .is_some_and(|selected| selected == &descriptor.id);
                if ui
                    .selectable_label(selected, &descriptor.display_name)
                    .clicked()
                {
                    ui_state.template.selected_template_id = Some(descriptor.id.clone());
                    ui_state.template.last_error = None;
                }
                ui.small(format!("{} · {}", descriptor.category, descriptor.id));
                ui.small(&descriptor.description);
                ui.add_space(8.0);
            }
        });

        columns[1].heading("Preview");
        columns[1].separator();
        let Some(descriptor) =
            crate::ui::template_workflow::selected_descriptor(&ui_state.template, &descriptors)
        else {
            columns[1].label("Select a template.");
            return;
        };

        columns[1].label(egui::RichText::new(&descriptor.display_name).strong());
        columns[1].small(&descriptor.description);
        columns[1].add_space(8.0);
        columns[1].label("Parameters");
        for parameter in &descriptor.parameters {
            let value = ui_state
                .template
                .parameters_by_template
                .get(&descriptor.id)
                .and_then(|values| values.get(&parameter.id));
            columns[1].small(summary_line_for_parameter_value(parameter, value));
        }

        columns[1].add_space(10.0);
        match preview_selected_template(&ui_state.template, project) {
            Ok(preview) => {
                if !preview.semantic_summary_lines.is_empty() {
                    columns[1].label("Semantic Output");
                    for line in &preview.semantic_summary_lines {
                        columns[1].small(format!("- {line}"));
                    }
                }
                columns[1].add_space(8.0);
                if !preview.lowered_summary_lines.is_empty() {
                    columns[1].label("Authored Changes");
                    for line in &preview.lowered_summary_lines {
                        columns[1].small(format!("- {line}"));
                    }
                }
            }
            Err(error) => {
                columns[1].colored_label(egui::Color32::from_rgb(215, 120, 120), &error.message);
            }
        }

        columns[1].add_space(10.0);
        columns[1].label("Active Templates");
        if project.metadata.editor.template_applications.is_empty() {
            columns[1].small("No active templates recorded yet.");
        } else {
            for application in &project.metadata.editor.template_applications {
                columns[1].small(format!(
                    "- {} ({})",
                    application.template_display_name, application.application_id
                ));
            }
        }
    });
}
