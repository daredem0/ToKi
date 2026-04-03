use egui::Ui;
use std::hash::Hash;
use toki_core::project_runtime::ProjectUiEventDefinition;

pub(crate) fn declared_ui_event_choices(declarations: &[ProjectUiEventDefinition]) -> Vec<String> {
    let mut choices = declarations
        .iter()
        .map(|declaration| declaration.id.trim())
        .filter(|id| !id.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    choices.sort();
    choices.dedup();
    choices
}

pub(crate) fn validate_ui_event_registry(declarations: &[ProjectUiEventDefinition]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut issues = Vec::new();
    for declaration in declarations {
        let id = declaration.id.trim();
        if id.is_empty() {
            issues.push("UI event ids must not be empty".to_string());
        } else if !seen.insert(id.to_string()) {
            issues.push(format!("Duplicate UI event id '{id}'"));
        }
    }
    issues
}

pub(crate) fn validate_ui_event_reference(
    event_id: &str,
    declarations: &[ProjectUiEventDefinition],
    label: &str,
) -> Option<String> {
    let trimmed = event_id.trim();
    if trimmed.is_empty() {
        return Some(format!("{label} requires a non-empty event id"));
    }
    let declared = declared_ui_event_choices(declarations);
    if !declared.is_empty() && !declared.iter().any(|candidate| candidate == trimmed) {
        return Some(format!(
            "{label} references undeclared UI event '{trimmed}'"
        ));
    }
    None
}

pub(crate) fn render_optional_ui_event_id_editor(
    ui: &mut Ui,
    id_salt: impl Hash + Copy,
    label: &str,
    event_id: &mut Option<String>,
    declarations: &[ProjectUiEventDefinition],
) -> bool {
    let before = event_id.clone();
    let mut value = event_id.clone().unwrap_or_default();
    render_ui_event_id_editor_inner(ui, id_salt, label, &mut value, declarations, true);
    *event_id = if value.trim().is_empty() {
        None
    } else {
        Some(value.trim().to_string())
    };
    *event_id != before
}

pub(crate) fn render_required_ui_event_id_editor(
    ui: &mut Ui,
    id_salt: impl Hash + Copy,
    label: &str,
    event_id: &mut String,
    declarations: &[ProjectUiEventDefinition],
) -> bool {
    let before = event_id.clone();
    render_ui_event_id_editor_inner(ui, id_salt, label, event_id, declarations, false);
    *event_id = event_id.trim().to_string();
    *event_id != before
}

fn render_ui_event_id_editor_inner(
    ui: &mut Ui,
    id_salt: impl Hash + Copy,
    label: &str,
    event_id: &mut String,
    declarations: &[ProjectUiEventDefinition],
    allow_none: bool,
) {
    let choices = declared_ui_event_choices(declarations);
    let trimmed = event_id.trim().to_string();
    let current_choice = if trimmed.is_empty() && allow_none {
        "None".to_string()
    } else if choices.iter().any(|choice| choice == &trimmed) {
        trimmed.clone()
    } else {
        "Custom".to_string()
    };
    let mut selected = current_choice;
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(("ui_event_choice", id_salt))
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                if allow_none {
                    ui.selectable_value(&mut selected, "None".to_string(), "None");
                }
                for choice in &choices {
                    ui.selectable_value(&mut selected, choice.clone(), choice);
                }
                ui.selectable_value(&mut selected, "Custom".to_string(), "Custom");
            });
    });
    if selected == "None" {
        event_id.clear();
    } else if selected != "Custom" {
        *event_id = selected.clone();
    } else if choices.iter().any(|choice| choice == &trimmed) {
        event_id.clear();
    }
    if selected == "Custom" || (!allow_none && event_id.trim().is_empty()) {
        ui.horizontal(|ui| {
            ui.label("Custom:");
            ui.text_edit_singleline(event_id);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ui_event_registry_reports_empty_and_duplicate_ids() {
        let issues = validate_ui_event_registry(&[
            ProjectUiEventDefinition { id: String::new() },
            ProjectUiEventDefinition {
                id: "open_inventory".to_string(),
            },
            ProjectUiEventDefinition {
                id: "open_inventory".to_string(),
            },
        ]);

        assert!(issues
            .iter()
            .any(|issue| issue.contains("must not be empty")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("Duplicate UI event id")));
    }

    #[test]
    fn validate_ui_event_reference_flags_undeclared_events() {
        let issues = validate_ui_event_reference(
            "missing_event",
            &[ProjectUiEventDefinition {
                id: "open_inventory".to_string(),
            }],
            "Widget event",
        );

        assert_eq!(
            issues.as_deref(),
            Some("Widget event references undeclared UI event 'missing_event'")
        );
    }
}
