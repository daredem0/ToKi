use crate::project::Project;
use crate::ui::editor_ui::EditorUI;
use crate::ui::template_workflow::{
    build_apply_template_command, built_in_template_descriptors, default_value_for_kind,
    preview_selected_template, selected_descriptor, sync_template_editor_state,
    TemplateAssetChoices,
};
use toki_templates::{AssetReferenceKind, TemplateParameter, TemplateParameterKind, TemplateValue};

use super::InspectorSystem;

impl InspectorSystem {
    pub(super) fn render_template_editor_inspector(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
        template_asset_choices: Option<&TemplateAssetChoices>,
    ) {
        let Some(project) = project else {
            ui.heading("Templates");
            ui.separator();
            ui.label("Open a project to edit template parameters.");
            return;
        };

        let descriptors = built_in_template_descriptors();
        sync_template_editor_state(&mut ui_state.template, &descriptors);

        ui.heading("Template Editor");
        ui.separator();

        let Some(descriptor) = selected_descriptor(&ui_state.template, &descriptors).cloned()
        else {
            ui.label("Select a template in the Templates tab.");
            return;
        };

        ui.label(egui::RichText::new(&descriptor.display_name).strong());
        ui.small(format!("{} · {}", descriptor.category, descriptor.id));
        ui.small(&descriptor.description);
        ui.separator();

        egui::CollapsingHeader::new("Parameters")
            .default_open(true)
            .show(ui, |ui| {
                let values = ui_state
                    .template
                    .parameters_by_template
                    .entry(descriptor.id.clone())
                    .or_default();
                for parameter in &descriptor.parameters {
                    let value = values.entry(parameter.id.clone()).or_insert_with(|| {
                        parameter
                            .default
                            .clone()
                            .unwrap_or_else(|| default_value_for_kind(&parameter.kind))
                    });
                    render_parameter_editor(
                        ui,
                        parameter,
                        value,
                        template_asset_choices,
                        &descriptor.id,
                    );
                    ui.add_space(8.0);
                }
            });

        egui::CollapsingHeader::new("Preview")
            .default_open(true)
            .show(ui, |ui| {
                match preview_selected_template(&ui_state.template, &project.path) {
                    Ok(preview) => {
                        ui.label("Semantic Output");
                        for line in &preview.semantic_summary_lines {
                            ui.small(format!("- {line}"));
                        }
                        if !preview.lowered_summary_lines.is_empty() {
                            ui.add_space(6.0);
                            ui.label("Authored Changes");
                            for line in &preview.lowered_summary_lines {
                                ui.small(format!("- {line}"));
                            }
                        }
                    }
                    Err(error) => {
                        ui.colored_label(egui::Color32::from_rgb(215, 120, 120), &error.message);
                    }
                }
            });

        egui::CollapsingHeader::new("Apply")
            .default_open(true)
            .show(ui, |ui| {
                if ui.button("Apply Template").clicked() {
                    match build_apply_template_command(
                        &ui_state.template,
                        &project.path,
                        ui_state.selection.clone(),
                    ) {
                        Ok(command) => {
                            if ui_state.execute_command_with_project(project, command) {
                                ui_state.template.last_error = None;
                            } else {
                                ui_state.template.last_error =
                                    Some("template apply command failed to execute".to_string());
                            }
                        }
                        Err(error) => {
                            ui_state.template.last_error = Some(error.message);
                        }
                    }
                }

                if let Some(error) = &ui_state.template.last_error {
                    ui.add_space(6.0);
                    ui.colored_label(egui::Color32::from_rgb(215, 120, 120), error);
                }
            });
    }
}

fn render_parameter_editor(
    ui: &mut egui::Ui,
    parameter: &TemplateParameter,
    value: &mut TemplateValue,
    template_asset_choices: Option<&TemplateAssetChoices>,
    id_prefix: &str,
) {
    ui.push_id((id_prefix, parameter.id.as_str()), |ui| {
        ui.label(egui::RichText::new(&parameter.label).strong());
        if let Some(description) = &parameter.description {
            ui.small(description);
        }
        render_value_editor(
            ui,
            &parameter.kind,
            value,
            template_asset_choices,
            parameter.id.as_str(),
        );
    });
}

fn render_value_editor(
    ui: &mut egui::Ui,
    kind: &TemplateParameterKind,
    value: &mut TemplateValue,
    template_asset_choices: Option<&TemplateAssetChoices>,
    id_hint: &str,
) {
    match kind {
        TemplateParameterKind::String { multiline, .. } => {
            let TemplateValue::String(current) = value else {
                *value = default_value_for_kind(kind);
                let TemplateValue::String(current) = value else {
                    unreachable!()
                };
                render_string_editor(ui, *multiline, current);
                return;
            };
            render_string_editor(ui, *multiline, current);
        }
        TemplateParameterKind::Integer { min, max, step } => {
            let TemplateValue::Integer(current) = value else {
                *value = default_value_for_kind(kind);
                let TemplateValue::Integer(current) = value else {
                    unreachable!()
                };
                render_integer_editor(ui, min, max, step, current);
                return;
            };
            render_integer_editor(ui, min, max, step, current);
        }
        TemplateParameterKind::Float { min, max, step } => {
            let TemplateValue::Float(current) = value else {
                *value = default_value_for_kind(kind);
                let TemplateValue::Float(current) = value else {
                    unreachable!()
                };
                render_float_editor(ui, min, max, step, current);
                return;
            };
            render_float_editor(ui, min, max, step, current);
        }
        TemplateParameterKind::Boolean => {
            let TemplateValue::Boolean(current) = value else {
                *value = TemplateValue::Boolean(false);
                let TemplateValue::Boolean(current) = value else {
                    unreachable!()
                };
                ui.checkbox(current, "Enabled");
                return;
            };
            ui.checkbox(current, "Enabled");
        }
        TemplateParameterKind::Enum { options } => {
            let TemplateValue::Enum(current) = value else {
                *value = default_value_for_kind(kind);
                let TemplateValue::Enum(current) = value else {
                    unreachable!()
                };
                render_enum_editor(ui, options, current, id_hint);
                return;
            };
            render_enum_editor(ui, options, current, id_hint);
        }
        TemplateParameterKind::AssetReference { asset_kind } => {
            let TemplateValue::AssetReference(current) = value else {
                *value = TemplateValue::AssetReference(String::new());
                let TemplateValue::AssetReference(current) = value else {
                    unreachable!()
                };
                render_asset_reference_editor(
                    ui,
                    *asset_kind,
                    current,
                    template_asset_choices,
                    id_hint,
                );
                return;
            };
            render_asset_reference_editor(
                ui,
                *asset_kind,
                current,
                template_asset_choices,
                id_hint,
            );
        }
        TemplateParameterKind::EntityDefinitionReference => {
            let TemplateValue::EntityDefinitionReference(current) = value else {
                *value = TemplateValue::EntityDefinitionReference(String::new());
                let TemplateValue::EntityDefinitionReference(current) = value else {
                    unreachable!()
                };
                render_dropdown_or_text(
                    ui,
                    "Entity Definition",
                    current,
                    template_asset_choices.map(|choices| choices.entity_definition_ids.clone()),
                    id_hint,
                );
                return;
            };
            render_dropdown_or_text(
                ui,
                "Entity Definition",
                current,
                template_asset_choices.map(|choices| choices.entity_definition_ids.clone()),
                id_hint,
            );
        }
        TemplateParameterKind::SceneReference => {
            let TemplateValue::SceneReference(current) = value else {
                *value = TemplateValue::SceneReference(String::new());
                let TemplateValue::SceneReference(current) = value else {
                    unreachable!()
                };
                render_dropdown_or_text(
                    ui,
                    "Scene",
                    current,
                    template_asset_choices.map(|choices| choices.scene_ids.clone()),
                    id_hint,
                );
                return;
            };
            render_dropdown_or_text(
                ui,
                "Scene",
                current,
                template_asset_choices.map(|choices| choices.scene_ids.clone()),
                id_hint,
            );
        }
        TemplateParameterKind::Optional { inner } => {
            let TemplateValue::Optional(current) = value else {
                *value = TemplateValue::Optional(None);
                let TemplateValue::Optional(current) = value else {
                    unreachable!()
                };
                render_optional_editor(ui, inner, current, template_asset_choices, id_hint);
                return;
            };
            render_optional_editor(ui, inner, current, template_asset_choices, id_hint);
        }
        TemplateParameterKind::List { item_kind, .. } => {
            let TemplateValue::List(values) = value else {
                *value = TemplateValue::List(Vec::new());
                let TemplateValue::List(values) = value else {
                    unreachable!()
                };
                render_list_editor(ui, item_kind, values, template_asset_choices, id_hint);
                return;
            };
            render_list_editor(ui, item_kind, values, template_asset_choices, id_hint);
        }
    }
}

fn render_string_editor(ui: &mut egui::Ui, multiline: bool, current: &mut String) {
    if multiline {
        ui.text_edit_multiline(current);
    } else {
        ui.text_edit_singleline(current);
    }
}

fn render_integer_editor(
    ui: &mut egui::Ui,
    min: &Option<i64>,
    max: &Option<i64>,
    step: &Option<i64>,
    current: &mut i64,
) {
    let mut drag = egui::DragValue::new(current).speed(step.unwrap_or(1) as f64);
    if let (Some(min), Some(max)) = (min, max) {
        drag = drag.range(*min..=*max);
    }
    ui.add(drag);
}

fn render_float_editor(
    ui: &mut egui::Ui,
    min: &Option<f64>,
    max: &Option<f64>,
    step: &Option<f64>,
    current: &mut f64,
) {
    let mut drag = egui::DragValue::new(current).speed(step.unwrap_or(0.1));
    if let (Some(min), Some(max)) = (min, max) {
        drag = drag.range(*min..=*max);
    }
    ui.add(drag);
}

fn render_enum_editor(
    ui: &mut egui::Ui,
    options: &[toki_templates::TemplateEnumOption],
    current: &mut String,
    id_hint: &str,
) {
    let selected_text = options
        .iter()
        .find(|option| option.id == *current)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| "<select>".to_string());
    egui::ComboBox::from_id_salt(("template_enum", id_hint))
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for option in options {
                ui.selectable_value(current, option.id.clone(), &option.label);
            }
        });
}

fn render_asset_reference_editor(
    ui: &mut egui::Ui,
    asset_kind: AssetReferenceKind,
    current: &mut String,
    template_asset_choices: Option<&TemplateAssetChoices>,
    id_hint: &str,
) {
    let choices = template_asset_choices.map(|choices| choices.asset_ids_for_kind(asset_kind));
    render_dropdown_or_text(ui, "Asset", current, choices, id_hint);
}

fn render_dropdown_or_text(
    ui: &mut egui::Ui,
    fallback_label: &str,
    current: &mut String,
    choices: Option<Vec<String>>,
    id_hint: &str,
) {
    if let Some(choices) = choices.filter(|choices| !choices.is_empty()) {
        let selected_text = if current.trim().is_empty() {
            format!("<select {fallback_label}>")
        } else {
            current.clone()
        };
        egui::ComboBox::from_id_salt(("template_ref", id_hint))
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for choice in choices {
                    ui.selectable_value(current, choice.clone(), choice);
                }
            });
    } else {
        ui.text_edit_singleline(current);
    }
}

fn render_optional_editor(
    ui: &mut egui::Ui,
    inner: &TemplateParameterKind,
    current: &mut Option<Box<TemplateValue>>,
    template_asset_choices: Option<&TemplateAssetChoices>,
    id_hint: &str,
) {
    let mut enabled = current.is_some();
    if ui.checkbox(&mut enabled, "Set value").changed() {
        if enabled {
            *current = Some(Box::new(default_value_for_kind(inner)));
        } else {
            *current = None;
        }
    }

    if let Some(inner_value) = current.as_mut() {
        ui.indent(("template_optional_indent", id_hint), |ui| {
            render_value_editor(ui, inner, inner_value, template_asset_choices, id_hint);
        });
    }
}

fn render_list_editor(
    ui: &mut egui::Ui,
    item_kind: &TemplateParameterKind,
    values: &mut Vec<TemplateValue>,
    template_asset_choices: Option<&TemplateAssetChoices>,
    id_hint: &str,
) {
    let mut remove_index = None;
    for (index, value) in values.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Item {}", index + 1));
                if ui.button("Remove").clicked() {
                    remove_index = Some(index);
                }
            });
            render_value_editor(
                ui,
                item_kind,
                value,
                template_asset_choices,
                &format!("{id_hint}_{index}"),
            );
        });
    }
    if let Some(index) = remove_index {
        values.remove(index);
    }
    if ui.button("Add Item").clicked() {
        values.push(default_value_for_kind(item_kind));
    }
}
