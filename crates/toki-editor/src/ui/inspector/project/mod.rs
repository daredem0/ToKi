mod display;
mod flags;
mod general;
mod metadata;
mod palettes;
mod runtime;
mod splash;
mod transitions;

use super::*;
use crate::project::apply_project_settings_draft;
use crate::project::{validate_project_settings_draft, ProjectAssets, ProjectViewportModeDraft};
#[cfg(test)]
use crate::project::ProjectSettingsDraft;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use toki_core::dialog::{DialogCondition, DialogNodeKind};
use toki_core::palette::{save_palette_asset_to_path, Palette4};
use toki_core::project_assets::load_project_palettes;

impl InspectorSystem {
    pub(super) fn render_project_settings_panel(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
        _project_assets: Option<&mut ProjectAssets>,
        _config: Option<&EditorConfig>,
    ) {
        let Some(project) = project else {
            ui.heading("Project");
            ui.separator();
            ui.label("No project open.");
            ui.label("Open or create a project to edit project-wide settings.");
            return;
        };

        ui.heading("Project");
        ui.separator();

        let mut draft = ui_state.project_settings_draft_for(project).clone();
        let mut changed = false;
        let mut palette_files_changed = false;
        let previous_flags = project.metadata.runtime.flags.declarations.clone();
        changed |= general::render_general_section(ui, &mut draft);
        ui.separator();
        changed |= flags::render_flags_section(ui, &mut draft);
        ui.separator();
        changed |= transitions::render_transitions_section(ui, &mut draft);
        ui.separator();
        changed |= display::render_display_section(ui_state, ui, &mut draft);
        ui.separator();
        changed |= splash::render_splash_section(ui, &mut draft);
        ui.separator();
        changed |= runtime::render_runtime_section(ui, project, &mut draft);
        ui.separator();
        let palette_outcome = palettes::render_palettes_section(ui_state, ui, project, &mut draft);
        changed |= palette_outcome.changed;
        palette_files_changed |= palette_outcome.palette_files_changed;
        ui.separator();
        metadata::render_metadata_section(ui, project);

        if changed {
            propagate_flag_renames(ui_state, &previous_flags, &draft.flag_declarations);
        }

        if changed && apply_project_settings_draft(project, &draft) {
            ui_state.set_title(&project.name);
            ui_state
                .project
                .set_available_palettes(&load_project_palette_files(project));
        } else if palette_files_changed {
            ui_state
                .project
                .set_available_palettes(&load_project_palette_files(project));
        }

        ui_state.project_settings_draft = Some((project.path.clone(), draft));
    }

    #[cfg(test)]
    pub(super) fn apply_project_settings_draft(
        project: &mut Project,
        draft: &ProjectSettingsDraft,
    ) -> bool {
        apply_project_settings_draft(project, draft)
    }
}

fn load_project_palette_files(project: &Project) -> BTreeMap<String, Palette4> {
    load_project_palettes(&project.path).unwrap_or_else(|error| {
        tracing::warn!(
            "Failed to load project palettes from '{}': {}",
            project.path.display(),
            error
        );
        BTreeMap::new()
    })
}

fn project_palette_file_path(project: &Project, palette_id: &str) -> PathBuf {
    project
        .path
        .join("palettes")
        .join(format!("{palette_id}.json"))
}

fn save_project_palette_file(
    project: &Project,
    palette_id: &str,
    palette: Palette4,
) -> anyhow::Result<()> {
    let path = project_palette_file_path(project, palette_id);
    save_palette_asset_to_path(&path, palette).map_err(anyhow::Error::from)
}

fn remove_project_palette_file(project: &Project, palette_id: &str) -> anyhow::Result<()> {
    let path = project_palette_file_path(project, palette_id);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

struct FlagRegistryIssue {
    color: egui::Color32,
    message: String,
}

fn validate_flag_registry(
    declarations: &[toki_core::project_runtime::ProjectFlagDefinition],
) -> Vec<FlagRegistryIssue> {
    let mut seen = std::collections::BTreeSet::new();
    let mut issues = Vec::new();
    for declaration in declarations {
        let id = declaration.id.trim();
        if id.is_empty() {
            issues.push(FlagRegistryIssue {
                color: egui::Color32::from_rgb(255, 120, 120),
                message: "Flag ids must not be empty".to_string(),
            });
        } else if !seen.insert(id.to_string()) {
            issues.push(FlagRegistryIssue {
                color: egui::Color32::from_rgb(255, 120, 120),
                message: format!("Duplicate flag id '{id}'"),
            });
        }
    }
    issues
}

fn propagate_flag_renames(
    ui_state: &mut EditorUI,
    previous: &[toki_core::project_runtime::ProjectFlagDefinition],
    next: &[toki_core::project_runtime::ProjectFlagDefinition],
) {
    for (before, after) in previous.iter().zip(next.iter()) {
        let old_id = before.id.trim();
        let new_id = after.id.trim();
        if old_id.is_empty() || new_id.is_empty() || old_id == new_id {
            continue;
        }
        rename_flag_references(ui_state, old_id, new_id);
    }
}

fn rename_flag_references(ui_state: &mut EditorUI, old_id: &str, new_id: &str) {
    for scene in &mut ui_state.scenes {
        for rule in &mut scene.rules.rules {
            for condition in &mut rule.conditions {
                rename_rule_condition_flag(condition, old_id, new_id);
            }
            for action in &mut rule.actions {
                rename_rule_action_flag(action, old_id, new_id);
            }
        }
    }

    if let Some(dialog) = crate::ui::editor_context::dialog_state_mut(ui_state).draft.as_mut() {
        for node in &mut dialog.nodes {
            rename_dialog_conditions(&mut node.conditions, old_id, new_id);
            match &mut node.kind {
                DialogNodeKind::Choice { choices, .. } => {
                    for choice in choices {
                        rename_dialog_conditions(&mut choice.conditions, old_id, new_id);
                    }
                }
                DialogNodeKind::Branch { branches, .. } => {
                    for branch in branches {
                        rename_dialog_conditions(&mut branch.conditions, old_id, new_id);
                    }
                }
                DialogNodeKind::Line { .. } | DialogNodeKind::End { .. } => {}
            }
        }
    }
}

fn rename_rule_condition_flag(
    condition: &mut toki_core::rules::RuleCondition,
    old_id: &str,
    new_id: &str,
) {
    match condition {
        toki_core::rules::RuleCondition::FlagEquals { flag, .. }
        | toki_core::rules::RuleCondition::FlagSet { flag }
        | toki_core::rules::RuleCondition::FlagGreaterThan { flag, .. }
            if flag.trim() == old_id =>
        {
            *flag = new_id.to_string();
        }
        _ => {}
    }
}

fn rename_rule_action_flag(
    action: &mut toki_core::rules::RuleAction,
    old_id: &str,
    new_id: &str,
) {
    match action {
        toki_core::rules::RuleAction::SetFlag { flag, .. }
        | toki_core::rules::RuleAction::IncrementFlag { flag, .. }
        | toki_core::rules::RuleAction::ClearFlag { flag }
            if flag.trim() == old_id =>
        {
            *flag = new_id.to_string();
        }
        _ => {}
    }
}

fn rename_dialog_conditions(conditions: &mut [DialogCondition], old_id: &str, new_id: &str) {
    for condition in conditions {
        match condition {
            DialogCondition::FlagEquals { flag, .. }
            | DialogCondition::FlagSet { flag }
            | DialogCondition::FlagGreaterThan { flag, .. }
                if flag.trim() == old_id =>
            {
                *flag = new_id.to_string();
            }
            _ => {}
        }
    }
}
