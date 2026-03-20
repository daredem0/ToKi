use super::*;

#[path = "rules_flat.rs"]
mod rules_flat;
#[path = "rules_graph.rs"]
mod rules_graph;
#[path = "rules_support.rs"]
mod rules_support;

impl InspectorSystem {
    pub(super) fn render_scene_rules_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        rule_set: &mut RuleSet,
        scenes: &[toki_core::Scene],
        config: Option<&EditorConfig>,
        map_size: Option<(u32, u32)>,
    ) -> bool {
        let mut changed = false;
        let validation_issues = Self::validate_rule_set_for_scene(rule_set, scene_name, scenes);
        let audio_choices = Self::load_rule_audio_choices(config);

        ui.label("Visual Rules");
        ui.horizontal(|ui| {
            ui.label("Count:");
            ui.label(rule_set.rules.len().to_string());
        });

        if ui.button("➕ Add Rule").clicked() {
            let rule_id = Self::add_default_rule(rule_set);
            tracing::info!("Added rule '{}' to scene '{}'", rule_id, scene_name);
            changed = true;
        }

        if !validation_issues.is_empty() {
            ui.colored_label(
                egui::Color32::from_rgb(255, 210, 80),
                format!("⚠ {} validation issues", validation_issues.len()),
            );
        }

        if rule_set.rules.is_empty() {
            ui.label("No rules configured");
            return changed;
        }

        let mut pending_command = None;
        for (rule_index, rule) in rule_set.rules.iter_mut().enumerate() {
            let outcome = Self::render_rule_editor(
                ui,
                scene_name,
                rule_index,
                rule,
                &validation_issues,
                &audio_choices,
                scenes,
                map_size,
            );
            changed |= outcome.changed;
            if pending_command.is_none() {
                pending_command = outcome.command;
            }
        }

        if let Some(command) = pending_command {
            match command {
                RuleEditorCommand::Remove(rule_index) => {
                    if Self::remove_rule(rule_set, rule_index).is_some() {
                        changed = true;
                    }
                }
                RuleEditorCommand::Duplicate(rule_index) => {
                    if Self::duplicate_rule(rule_set, rule_index).is_some() {
                        changed = true;
                    }
                }
                RuleEditorCommand::MoveUp(rule_index) => {
                    if let Some(new_index) = Self::move_rule_up(rule_set, rule_index) {
                        changed |= new_index != rule_index;
                    }
                }
                RuleEditorCommand::MoveDown(rule_index) => {
                    if let Some(new_index) = Self::move_rule_down(rule_set, rule_index) {
                        changed |= new_index != rule_index;
                    }
                }
            }
        }

        changed
    }
}
