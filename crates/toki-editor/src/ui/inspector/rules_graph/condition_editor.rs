//! Condition node editing UI.

use super::*;

impl InspectorSystem {
    pub(in super::super) fn render_rule_graph_condition_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        condition: &mut RuleCondition,
    ) -> bool {
        let mut changed = false;

        let current_kind = Self::condition_kind(condition);
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_condition_kind_{}_{}",
                scene_name, node_key
            ))
            .selected_text(Self::condition_kind_label(current_kind))
            .show_ui(ui, |ui| {
                for candidate in RuleConditionKind::iter() {
                    changed |= ui
                        .selectable_value(
                            &mut selected_kind,
                            candidate,
                            Self::condition_kind_label(candidate),
                        )
                        .changed();
                }
            });
        });

        if selected_kind != current_kind {
            Self::switch_condition_kind(condition, selected_kind);
            changed = true;
        }

        changed |= Self::render_condition_parameters(ui, scene_name, node_key, condition);

        changed
    }

    fn render_condition_parameters(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        condition: &mut RuleCondition,
    ) -> bool {
        let mut changed = false;

        match condition {
            RuleCondition::Always | RuleCondition::TriggerOtherIsPlayer => {}
            RuleCondition::TargetExists { target } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_target_{}_{}", scene_name, node_key),
                    target,
                );
            }
            RuleCondition::KeyHeld { key } => {
                changed |= Self::render_rule_key_editor_with_salt(
                    ui,
                    &format!("graph_node_condition_key_{}_{}", scene_name, node_key),
                    key,
                );
            }
            RuleCondition::EntityActive { target, is_active } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!(
                        "graph_node_condition_entity_target_{}_{}",
                        scene_name, node_key
                    ),
                    target,
                );
                changed |= ui.checkbox(is_active, "Target Is Active").changed();
            }
            RuleCondition::HealthBelow { target, threshold }
            | RuleCondition::HealthAbove { target, threshold } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!(
                        "graph_node_condition_health_target_{}_{}",
                        scene_name, node_key
                    ),
                    target,
                );
                changed |= Self::render_threshold_editor(ui, threshold);
            }
            RuleCondition::EntityIsKind { target, kind } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!(
                        "graph_node_condition_entity_kind_target_{}_{}",
                        scene_name, node_key
                    ),
                    target,
                );
                changed |= Self::render_entity_kind_editor(
                    ui,
                    &format!(
                        "graph_node_condition_entity_kind_{}_{}",
                        scene_name, node_key
                    ),
                    kind,
                );
            }
            RuleCondition::TriggerOtherIsKind { kind } => {
                changed |= Self::render_entity_kind_editor(
                    ui,
                    &format!(
                        "graph_node_condition_other_kind_{}_{}",
                        scene_name, node_key
                    ),
                    kind,
                );
            }
            RuleCondition::EntityHasTag { target, tag } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!(
                        "graph_node_condition_tag_target_{}_{}",
                        scene_name, node_key
                    ),
                    target,
                );
                changed |= Self::render_tag_editor(ui, tag);
            }
            RuleCondition::TriggerOtherHasTag { tag } => {
                changed |= Self::render_tag_editor(ui, tag);
            }
            RuleCondition::HasInventoryItem {
                target,
                item_id,
                min_count,
            } => {
                changed |= Self::render_rule_target_editor_with_salt(
                    ui,
                    &format!(
                        "graph_node_condition_inv_target_{}_{}",
                        scene_name, node_key
                    ),
                    target,
                );
                changed |= Self::render_item_id_editor(ui, item_id);
                changed |= Self::render_min_count_editor(ui, min_count);
            }
        }

        changed
    }

    fn render_threshold_editor(ui: &mut egui::Ui, threshold: &mut i32) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Threshold:");
            changed |= ui
                .add(egui::DragValue::new(threshold).range(0..=1000))
                .changed();
        });
        changed
    }

    fn render_tag_editor(ui: &mut egui::Ui, tag: &mut String) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Tag:");
            changed |= ui.text_edit_singleline(tag).changed();
        });
        changed
    }

    fn render_item_id_editor(ui: &mut egui::Ui, item_id: &mut String) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Item ID:");
            changed |= ui.text_edit_singleline(item_id).changed();
        });
        changed
    }

    fn render_min_count_editor(ui: &mut egui::Ui, min_count: &mut u32) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Min Count:");
            changed |= ui
                .add(egui::DragValue::new(min_count).range(1..=999))
                .changed();
        });
        changed
    }
}
