use super::*;
use crate::project::assets::ProjectAudioAssetKind;
use crate::ui::editor_domain::{
    default_rule_action as shared_default_rule_action,
    default_rule_condition as shared_default_rule_condition,
    default_rule_trigger as shared_default_rule_trigger,
    rule_action_kind as shared_rule_action_kind,
    rule_action_kind_label as shared_rule_action_kind_label,
    rule_condition_kind as shared_rule_condition_kind,
    rule_condition_kind_label as shared_rule_condition_kind_label,
    rule_key_label as shared_rule_key_label,
    rule_sound_channel_label as shared_rule_sound_channel_label,
    rule_spawn_entity_type_label as shared_rule_spawn_entity_type_label,
    rule_trigger_kind as shared_rule_trigger_kind,
    rule_trigger_kind_label as shared_rule_trigger_kind_label,
};
use toki_core::rules::{RuleFlagValueSource, RuleIntSource, RuleVec2IntSource};

impl InspectorSystem {
    pub(in super::super) fn render_flag_name_editor(ui: &mut egui::Ui, flag: &mut String) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Flag:");
            changed |= ui.text_edit_singleline(flag).changed();
        });
        changed
    }

    pub(in super::super) fn render_flag_value_editor(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        value: &mut FlagValue,
    ) -> bool {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum FlagValueKind {
            Bool,
            Int,
            String,
        }

        let mut changed = false;
        let current_kind = match value {
            FlagValue::Bool(_) => FlagValueKind::Bool,
            FlagValue::Int(_) => FlagValueKind::Int,
            FlagValue::String(_) => FlagValueKind::String,
        };
        let mut selected_kind = current_kind;
        ui.horizontal(|ui| {
            ui.label("Value Type:");
            egui::ComboBox::from_id_salt((&id_salt, "kind"))
                .selected_text(match current_kind {
                    FlagValueKind::Bool => "Bool",
                    FlagValueKind::Int => "Int",
                    FlagValueKind::String => "String",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut selected_kind, FlagValueKind::Bool, "Bool")
                        .changed();
                    changed |= ui
                        .selectable_value(&mut selected_kind, FlagValueKind::Int, "Int")
                        .changed();
                    changed |= ui
                        .selectable_value(&mut selected_kind, FlagValueKind::String, "String")
                        .changed();
                });
        });
        if selected_kind != current_kind {
            *value = match selected_kind {
                FlagValueKind::Bool => FlagValue::Bool(false),
                FlagValueKind::Int => FlagValue::Int(0),
                FlagValueKind::String => FlagValue::String(String::new()),
            };
        }

        match value {
            FlagValue::Bool(flag) => {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    changed |= ui.checkbox(flag, "Enabled").changed();
                });
            }
            FlagValue::Int(flag) => {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    changed |= ui.add(egui::DragValue::new(flag).speed(1.0)).changed();
                });
            }
            FlagValue::String(flag) => {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    changed |= ui.text_edit_singleline(flag).changed();
                });
            }
        }

        changed
    }

    pub(in super::super) fn render_rule_int_source_editor(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        label: &str,
        value: &mut RuleIntSource,
    ) -> bool {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum IntSourceMode {
            Literal,
            Expression,
        }

        let mut changed = false;
        let current_mode = match value {
            RuleIntSource::Literal(_) => IntSourceMode::Literal,
            RuleIntSource::Expression { .. } => IntSourceMode::Expression,
        };
        let mut selected_mode = current_mode;
        ui.horizontal(|ui| {
            ui.label(label);
            egui::ComboBox::from_id_salt((&id_salt, "mode"))
                .selected_text(match current_mode {
                    IntSourceMode::Literal => "Literal",
                    IntSourceMode::Expression => "Expr",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut selected_mode, IntSourceMode::Literal, "Literal")
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_mode,
                            IntSourceMode::Expression,
                            "Expression",
                        )
                        .changed();
                });
        });

        if selected_mode != current_mode {
            match selected_mode {
                IntSourceMode::Literal => value.set_literal(value.as_literal().unwrap_or_default()),
                IntSourceMode::Expression => {
                    value.set_expression(value.as_literal().unwrap_or_default().to_string())
                }
            }
        }

        match value {
            RuleIntSource::Literal(literal) => {
                ui.horizontal(|ui| {
                    ui.label("Value:");
                    changed |= ui
                        .add(egui::DragValue::new(literal).speed(1.0))
                        .changed();
                });
            }
            RuleIntSource::Expression { expr } => {
                ui.horizontal(|ui| {
                    ui.label("Expr:");
                    changed |= ui.text_edit_singleline(expr).changed();
                });
            }
        }

        changed
    }

    pub(in super::super) fn render_rule_vec2_source_editor(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        label: &str,
        value: &mut RuleVec2IntSource,
    ) -> bool {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum VecSourceMode {
            Literal,
            Components,
        }

        let mut changed = false;
        let current_mode = match value {
            RuleVec2IntSource::Literal(_) => VecSourceMode::Literal,
            RuleVec2IntSource::Expression { .. } => VecSourceMode::Components,
        };
        let mut selected_mode = current_mode;
        ui.horizontal(|ui| {
            ui.label(label);
            egui::ComboBox::from_id_salt((&id_salt, "mode"))
                .selected_text(match current_mode {
                    VecSourceMode::Literal => "Literal",
                    VecSourceMode::Components => "Per Axis",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut selected_mode, VecSourceMode::Literal, "Literal")
                        .changed();
                    changed |= ui
                        .selectable_value(&mut selected_mode, VecSourceMode::Components, "Per Axis")
                        .changed();
                });
        });

        if selected_mode != current_mode {
            match selected_mode {
                VecSourceMode::Literal => value.set_literal(value.as_literal().unwrap_or([0, 0])),
                VecSourceMode::Components => {
                    let [x, y] = value.as_literal().unwrap_or([0, 0]);
                    value.set_expression(RuleIntSource::Literal(x), RuleIntSource::Literal(y));
                }
            }
        }

        match value {
            RuleVec2IntSource::Literal(literal) => {
                ui.horizontal(|ui| {
                    ui.label("X:");
                    changed |= ui
                        .add(egui::DragValue::new(&mut literal[0]).speed(1.0))
                        .changed();
                    ui.label("Y:");
                    changed |= ui
                        .add(egui::DragValue::new(&mut literal[1]).speed(1.0))
                        .changed();
                });
            }
            RuleVec2IntSource::Expression { x, y } => {
                changed |= Self::render_rule_int_source_editor(ui, (&id_salt, "x"), "X:", x);
                changed |= Self::render_rule_int_source_editor(ui, (&id_salt, "y"), "Y:", y);
            }
        }

        changed
    }

    pub(in super::super) fn render_rule_flag_value_source_editor(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        value: &mut RuleFlagValueSource,
    ) -> bool {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum FlagSourceMode {
            Literal,
            Expression,
        }

        let mut changed = false;
        let current_mode = match value {
            RuleFlagValueSource::Literal(_) => FlagSourceMode::Literal,
            RuleFlagValueSource::Expression { .. } => FlagSourceMode::Expression,
        };
        let mut selected_mode = current_mode;
        ui.horizontal(|ui| {
            ui.label("Value Source:");
                egui::ComboBox::from_id_salt((&id_salt, "mode"))
                .selected_text(match current_mode {
                    FlagSourceMode::Literal => "Literal",
                    FlagSourceMode::Expression => "Expression",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut selected_mode, FlagSourceMode::Literal, "Literal")
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut selected_mode,
                            FlagSourceMode::Expression,
                            "Expression",
                        )
                        .changed();
                });
        });

        if selected_mode != current_mode {
            match selected_mode {
                FlagSourceMode::Literal => value.set_literal(FlagValue::Bool(false)),
                FlagSourceMode::Expression => value.set_expression(String::new()),
            }
        }

        match value {
            RuleFlagValueSource::Literal(literal) => {
                changed |= Self::render_flag_value_editor(ui, (&id_salt, "literal"), literal);
            }
            RuleFlagValueSource::Expression { expr } => {
                ui.horizontal(|ui| {
                    ui.label("Expr:");
                    changed |= ui.text_edit_singleline(expr).changed();
                });
            }
        }

        changed
    }

    fn validate_expression_string(expression: &str) -> Option<String> {
        toki_core::expression::Expression::parse(expression)
            .err()
            .map(|error| error.to_string())
    }

    pub(in super::super) fn render_save_slot_editor(ui: &mut egui::Ui, slot: &mut u8) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Slot:");
            let mut slot_i32 = (*slot).clamp(1, 3) as i32;
            if ui
                .add(egui::DragValue::new(&mut slot_i32).speed(1.0).range(1..=3))
                .changed()
            {
                *slot = slot_i32.clamp(1, 3) as u8;
                changed = true;
            }
        });
        changed
    }

    pub(in super::super) fn next_rule_id(rule_set: &RuleSet) -> String {
        let mut index = 1usize;
        loop {
            let candidate = format!("rule_{}", index);
            if !rule_set.rules.iter().any(|rule| rule.id == candidate) {
                return candidate;
            }
            index += 1;
        }
    }

    pub(in super::super) fn add_default_rule(rule_set: &mut RuleSet) -> String {
        let id = Self::next_rule_id(rule_set);
        let rule = Rule {
            id: id.clone(),
            enabled: true,
            priority: 0,
            once: false,
            log_enabled: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            }],
        };
        rule_set.rules.push(rule);
        id
    }

    pub(in super::super) fn duplicate_rule(
        rule_set: &mut RuleSet,
        rule_index: usize,
    ) -> Option<usize> {
        let source_rule = rule_set.rules.get(rule_index)?.clone();
        let mut duplicated = source_rule;
        duplicated.id = Self::next_rule_id(rule_set);
        let insert_index = (rule_index + 1).min(rule_set.rules.len());
        rule_set.rules.insert(insert_index, duplicated);
        Some(insert_index)
    }

    pub(in super::super) fn remove_rule(
        rule_set: &mut RuleSet,
        rule_index: usize,
    ) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }

        rule_set.rules.remove(rule_index);
        if rule_set.rules.is_empty() {
            None
        } else if rule_index < rule_set.rules.len() {
            Some(rule_index)
        } else {
            Some(rule_set.rules.len() - 1)
        }
    }

    pub(in super::super) fn move_rule_up(
        rule_set: &mut RuleSet,
        rule_index: usize,
    ) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }
        if rule_index == 0 {
            return Some(0);
        }

        rule_set.rules.swap(rule_index - 1, rule_index);
        Some(rule_index - 1)
    }

    pub(in super::super) fn move_rule_down(
        rule_set: &mut RuleSet,
        rule_index: usize,
    ) -> Option<usize> {
        if rule_index >= rule_set.rules.len() {
            return None;
        }
        if rule_index + 1 >= rule_set.rules.len() {
            return Some(rule_index);
        }

        rule_set.rules.swap(rule_index, rule_index + 1);
        Some(rule_index + 1)
    }

    pub(in super::super) fn add_action(rule: &mut Rule, action_kind: RuleActionKind) {
        rule.actions.push(shared_default_rule_action(action_kind));
    }

    pub(in super::super) fn add_condition(rule: &mut Rule, condition_kind: RuleConditionKind) {
        rule.conditions
            .push(shared_default_rule_condition(condition_kind));
    }

    pub(in super::super) fn remove_condition(rule: &mut Rule, condition_index: usize) -> bool {
        if condition_index >= rule.conditions.len() {
            return false;
        }
        rule.conditions.remove(condition_index);
        if rule.conditions.is_empty() {
            rule.conditions.push(RuleCondition::Always);
        }
        true
    }

    pub(in super::super) fn switch_condition_kind(
        condition: &mut RuleCondition,
        condition_kind: RuleConditionKind,
    ) {
        *condition = shared_default_rule_condition(condition_kind);
    }

    pub(in super::super) fn remove_action(rule: &mut Rule, action_index: usize) -> bool {
        if action_index >= rule.actions.len() {
            return false;
        }
        rule.actions.remove(action_index);
        true
    }

    pub(in super::super) fn switch_action_kind(
        action: &mut RuleAction,
        action_kind: RuleActionKind,
    ) {
        *action = shared_default_rule_action(action_kind);
    }

    #[cfg(test)]
    pub(in super::super) fn validate_rule_set(rule_set: &RuleSet) -> Vec<RuleValidationIssue> {
        Self::validate_rule_set_for_scene(rule_set, "", &[], &[])
    }

    pub(in super::super) fn validate_rule_set_for_scene(
        rule_set: &RuleSet,
        _current_scene_name: &str,
        scenes: &[toki_core::Scene],
        declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    ) -> Vec<RuleValidationIssue> {
        let mut issues = Vec::new();
        let declared_flag_ids = declared_flags
            .iter()
            .map(|declaration| declaration.id.trim())
            .filter(|id| !id.is_empty())
            .collect::<std::collections::BTreeSet<_>>();

        let mut id_to_indices: HashMap<&str, Vec<usize>> = HashMap::new();
        for (rule_index, rule) in rule_set.rules.iter().enumerate() {
            id_to_indices
                .entry(rule.id.as_str())
                .or_default()
                .push(rule_index);
        }

        for (rule_id, indices) in id_to_indices {
            if indices.len() > 1 {
                for rule_index in indices {
                    issues.push(RuleValidationIssue {
                        rule_index,
                        action_index: None,
                        message: format!("Duplicate rule id '{rule_id}'"),
                    });
                }
            }
        }

        for (rule_index, rule) in rule_set.rules.iter().enumerate() {
            if rule.id.trim().is_empty() {
                issues.push(RuleValidationIssue {
                    rule_index,
                    action_index: None,
                    message: "Rule id must not be empty".to_string(),
                });
            }

            if let RuleTrigger::OnDialogComplete {
                dialog_id,
                outcome_id,
            } = &rule.trigger
            {
                if dialog_id.trim().is_empty() {
                    issues.push(RuleValidationIssue {
                        rule_index,
                        action_index: None,
                        message: "OnDialogComplete requires a dialog id".to_string(),
                    });
                }
                if outcome_id.trim().is_empty() {
                    issues.push(RuleValidationIssue {
                        rule_index,
                        action_index: None,
                        message: "OnDialogComplete requires an outcome id".to_string(),
                    });
                }
            }

            for (condition_index, condition) in rule.conditions.iter().enumerate() {
                match condition {
                    RuleCondition::Always
                    | RuleCondition::KeyHeld { .. }
                    | RuleCondition::TriggerOtherIsPlayer
                    | RuleCondition::TriggerOtherIsKind { .. }
                    | RuleCondition::TriggerOtherHasTag { .. } => {}
                    RuleCondition::Expression { expression } => {
                        if expression.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: None,
                                message: format!(
                                    "Condition {} expression must not be empty",
                                    condition_index + 1
                                ),
                            });
                        } else if let Some(message) =
                            Self::validate_expression_string(expression)
                        {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: None,
                                message: format!(
                                    "Condition {} has invalid expression: {}",
                                    condition_index + 1,
                                    message
                                ),
                            });
                        }
                    }
                    RuleCondition::TargetExists { target }
                    | RuleCondition::EntityActive { target, .. }
                    | RuleCondition::HealthBelow { target, .. }
                    | RuleCondition::HealthAbove { target, .. }
                    | RuleCondition::EntityIsKind { target, .. }
                    | RuleCondition::EntityHasTag { target, .. }
                    | RuleCondition::HasInventoryItem { target, .. } => {
                        if let RuleTarget::Entity(entity_id) = target {
                            if *entity_id == 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: None,
                                    message: format!(
                                        "Condition {} entity target must be non-zero",
                                        condition_index + 1
                                    ),
                                });
                            }
                        }
                    }
                    RuleCondition::FlagEquals { flag, .. }
                    | RuleCondition::FlagSet { flag }
                    | RuleCondition::FlagGreaterThan { flag, .. } => {
                        if flag.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: None,
                                message: format!(
                                    "Condition {} flag name must not be empty",
                                    condition_index + 1
                                ),
                            });
                        } else if !declared_flag_ids.is_empty()
                            && !declared_flag_ids.contains(flag.trim())
                        {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: None,
                                message: format!(
                                    "Condition {} references undeclared flag '{}'",
                                    condition_index + 1,
                                    flag.trim()
                                ),
                            });
                        }
                    }
                }
            }

            for (action_index, action) in rule.actions.iter().enumerate() {
                match action {
                    RuleAction::PlaySound { sound_id, .. } => {
                        if sound_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "PlaySound requires a non-empty sound id".to_string(),
                            });
                        }
                    }
                    RuleAction::PlayMusic { track_id } => {
                        if track_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "PlayMusic requires a non-empty track id".to_string(),
                            });
                        }
                    }
                    RuleAction::PlayAnimation { .. } => {}
                    RuleAction::SetVelocity { target, velocity } => {
                        if let RuleTarget::Entity(entity_id) = target {
                            if *entity_id == 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: "SetVelocity entity target must be non-zero"
                                        .to_string(),
                                });
                            }
                        }
                        if let RuleVec2IntSource::Expression { x, y } = velocity {
                            for (axis_label, source) in
                                [("x", x as &RuleIntSource), ("y", y as &RuleIntSource)]
                            {
                                if let Some(expression) = source.expression() {
                                    if let Some(message) =
                                        Self::validate_expression_string(expression)
                                    {
                                        issues.push(RuleValidationIssue {
                                            rule_index,
                                            action_index: Some(action_index),
                                            message: format!(
                                                "SetVelocity {} has invalid expression: {}",
                                                axis_label, message
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    RuleAction::Spawn { position, .. } => {
                        if let RuleVec2IntSource::Expression { x, y } = position {
                            for source in [x as &RuleIntSource, y as &RuleIntSource] {
                                if let Some(expression) = source.expression() {
                                    if let Some(message) =
                                        Self::validate_expression_string(expression)
                                    {
                                        issues.push(RuleValidationIssue {
                                            rule_index,
                                            action_index: Some(action_index),
                                            message: format!(
                                                "Spawn position has invalid expression: {}",
                                                message
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    RuleAction::DestroySelf { target } => {
                        if let RuleTarget::Entity(entity_id) = target {
                            if *entity_id == 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: "DestroySelf entity target must be non-zero"
                                        .to_string(),
                                });
                            }
                        }
                    }
                    RuleAction::SwitchScene {
                        scene_name,
                        spawn_point_id,
                        duration_ms,
                        ..
                    } => {
                        if scene_name.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "SwitchScene requires a scene name".to_string(),
                            });
                        }
                        if spawn_point_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "SwitchScene requires a spawn point id".to_string(),
                            });
                        }
                        if matches!(duration_ms, Some(0)) {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "SwitchScene override duration must be positive"
                                    .to_string(),
                            });
                        }
                        if !scenes.is_empty() && !scene_name.trim().is_empty() {
                            let Some(target_scene) =
                                scenes.iter().find(|scene| scene.name == scene_name.trim())
                            else {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: format!(
                                        "SwitchScene target scene '{}' does not exist",
                                        scene_name.trim()
                                    ),
                                });
                                continue;
                            };

                            if !spawn_point_id.trim().is_empty()
                                && target_scene
                                    .anchors
                                    .iter()
                                    .filter(|anchor| {
                                        matches!(
                                            anchor.kind,
                                            toki_core::scene::SceneAnchorKind::SpawnPoint
                                        )
                                    })
                                    .all(|anchor| anchor.id != spawn_point_id.trim())
                            {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: format!(
                                        "SwitchScene target spawn point '{}' does not exist in scene '{}'",
                                        spawn_point_id.trim(),
                                        target_scene.name
                                    ),
                                });
                            }
                        }
                    }
                    RuleAction::StartDialog { dialog_id } => {
                        if dialog_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "StartDialog requires a dialog id".to_string(),
                            });
                        }
                    }
                    RuleAction::DamageEntity { amount, .. } => {
                        if let Some(value) = amount.as_literal() {
                            if value <= 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: "DamageEntity amount must be positive".to_string(),
                                });
                            }
                        } else if let Some(expression) = amount.expression() {
                            if let Some(message) = Self::validate_expression_string(expression) {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: format!(
                                        "DamageEntity amount has invalid expression: {}",
                                        message
                                    ),
                                });
                            }
                        }
                    }
                    RuleAction::HealEntity { amount, .. } => {
                        if let Some(value) = amount.as_literal() {
                            if value <= 0 {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: "HealEntity amount must be positive".to_string(),
                                });
                            }
                        } else if let Some(expression) = amount.expression() {
                            if let Some(message) = Self::validate_expression_string(expression) {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: format!(
                                        "HealEntity amount has invalid expression: {}",
                                        message
                                    ),
                                });
                            }
                        }
                    }
                    RuleAction::SetFlag { flag, value } => {
                        if flag.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "Flag actions require a non-empty flag name".to_string(),
                            });
                        } else if !declared_flag_ids.is_empty()
                            && !declared_flag_ids.contains(flag.trim())
                        {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: format!(
                                    "Flag action references undeclared flag '{}'",
                                    flag.trim()
                                ),
                            });
                        }
                        if let Some(expression) = value.expression() {
                            if let Some(message) = Self::validate_expression_string(expression) {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: format!(
                                        "SetFlag value has invalid expression: {}",
                                        message
                                    ),
                                });
                            }
                        }
                    }
                    RuleAction::IncrementFlag { flag, amount } => {
                        if flag.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "Flag actions require a non-empty flag name".to_string(),
                            });
                        } else if !declared_flag_ids.is_empty()
                            && !declared_flag_ids.contains(flag.trim())
                        {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: format!(
                                    "Flag action references undeclared flag '{}'",
                                    flag.trim()
                                ),
                            });
                        }
                        if let Some(expression) = amount.expression() {
                            if let Some(message) = Self::validate_expression_string(expression) {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: format!(
                                        "IncrementFlag amount has invalid expression: {}",
                                        message
                                    ),
                                });
                            }
                        }
                    }
                    RuleAction::TeleportEntity { tile_x, tile_y, .. } => {
                        for (axis_label, source) in [("tile_x", tile_x), ("tile_y", tile_y)] {
                            if let Some(expression) = source.expression() {
                                if let Some(message) =
                                    Self::validate_expression_string(expression)
                                {
                                    issues.push(RuleValidationIssue {
                                        rule_index,
                                        action_index: Some(action_index),
                                        message: format!(
                                            "TeleportEntity {} has invalid expression: {}",
                                            axis_label, message
                                        ),
                                    });
                                }
                            }
                        }
                    }
                    RuleAction::AddInventoryItem { item_id, count, .. } => {
                        if item_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "AddInventoryItem requires a non-empty item id"
                                    .to_string(),
                            });
                        }
                        if *count == 0 {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "AddInventoryItem count must be at least 1".to_string(),
                            });
                        }
                    }
                    RuleAction::RemoveInventoryItem { item_id, count, .. } => {
                        if item_id.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "RemoveInventoryItem requires a non-empty item id"
                                    .to_string(),
                            });
                        }
                        if *count == 0 {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "RemoveInventoryItem count must be at least 1".to_string(),
                            });
                        }
                    }
                    RuleAction::SetEntityActive { .. } => {}
                    RuleAction::ClearFlag { flag } => {
                        if flag.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "Flag actions require a non-empty flag name".to_string(),
                            });
                        } else if !declared_flag_ids.is_empty()
                            && !declared_flag_ids.contains(flag.trim())
                        {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: format!(
                                    "Flag action references undeclared flag '{}'",
                                    flag.trim()
                                ),
                            });
                        }
                    }
                    RuleAction::SaveGame { slot } | RuleAction::LoadGame { slot } => {
                        if !(1..=3).contains(slot) {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "Save/load slot must be between 1 and 3".to_string(),
                            });
                        }
                    }
                    RuleAction::ShowUi { ui_id } | RuleAction::HideUi { ui_id } => {
                        if ui_id.as_str().trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "UI actions require a non-empty UI id".to_string(),
                            });
                        }
                    }
                    RuleAction::UpdateUiBinding {
                        ui_id,
                        binding_key,
                        value,
                    } => {
                        if ui_id.as_str().trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "UI actions require a non-empty UI id".to_string(),
                            });
                        }
                        if binding_key.trim().is_empty() {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "UpdateUiBinding requires a non-empty binding key"
                                    .to_string(),
                            });
                        }
                        if let Some(expression) = value.expression() {
                            if let Some(message) = Self::validate_expression_string(expression) {
                                issues.push(RuleValidationIssue {
                                    rule_index,
                                    action_index: Some(action_index),
                                    message: format!(
                                        "UpdateUiBinding value has invalid expression: {}",
                                        message
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        issues
    }

    pub(in super::super) fn scene_switch_target_scene_names(
        scenes: &[toki_core::Scene],
    ) -> Vec<String> {
        let mut names = scenes
            .iter()
            .map(|scene| scene.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    pub(in super::super) fn scene_switch_spawn_point_ids(
        scenes: &[toki_core::Scene],
        target_scene_name: &str,
    ) -> Vec<String> {
        let Some(scene) = scenes.iter().find(|scene| scene.name == target_scene_name) else {
            return Vec::new();
        };
        let mut ids = scene.spawn_point_ids();
        ids.sort();
        ids
    }

    pub(in super::super) fn render_switch_scene_editor(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        scene_name: &mut toki_core::SceneId,
        spawn_point_id: &mut String,
        transition: &mut Option<toki_core::SceneTransitionEffect>,
        duration_ms: &mut Option<u32>,
        scenes: &[toki_core::Scene],
    ) -> bool {
        let mut changed = false;
        let scene_names = Self::scene_switch_target_scene_names(scenes);
        let mut scene_name_value = scene_name.to_string();
        ui.horizontal(|ui| {
            ui.label("Scene:");
            egui::ComboBox::from_id_salt((&id_salt, "scene"))
                .selected_text(if scene_name_value.is_empty() {
                    "<select scene>"
                } else {
                    scene_name_value.as_str()
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut scene_name_value, String::new(), "<select scene>")
                        .changed();
                    for candidate in &scene_names {
                        changed |= ui
                            .selectable_value(&mut scene_name_value, candidate.clone(), candidate)
                            .changed();
                    }
                });
        });
        if changed {
            *scene_name = scene_name_value.into();
        }

        let spawn_ids = Self::scene_switch_spawn_point_ids(scenes, scene_name.as_str());
        ui.horizontal(|ui| {
            ui.label("Spawn Point:");
            egui::ComboBox::from_id_salt((&id_salt, "spawn"))
                .selected_text(if spawn_point_id.is_empty() {
                    "<select spawn>"
                } else {
                    spawn_point_id.as_str()
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(spawn_point_id, String::new(), "<select spawn>")
                        .changed();
                    for candidate in &spawn_ids {
                        changed |= ui
                            .selectable_value(spawn_point_id, candidate.clone(), candidate)
                            .changed();
                    }
                });
        });

        let mut use_override = duration_ms.is_some();
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut use_override, "Override Fade").changed();
        });
        if use_override {
            if duration_ms.is_none() {
                *duration_ms = Some(250);
                changed = true;
            }
            if transition.is_none() {
                *transition = Some(toki_core::SceneTransitionEffect::Fade);
                changed = true;
            }
        } else {
            if duration_ms.take().is_some() {
                changed = true;
            }
            if transition.take().is_some() {
                changed = true;
            }
        }
        if let Some(override_duration_ms) = duration_ms.as_mut() {
            ui.horizontal(|ui| {
                ui.label("Fade Duration:");
                let mut value = *override_duration_ms as i64;
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .speed(1.0)
                            .range(1..=60_000),
                    )
                    .changed()
                {
                    *override_duration_ms = value.max(1) as u32;
                    changed = true;
                }
                ui.label("ms");
            });
        }

        changed
    }

    pub(in super::super) fn condition_kind(condition: &RuleCondition) -> RuleConditionKind {
        shared_rule_condition_kind(condition)
    }

    pub(in super::super) fn condition_kind_label(
        condition_kind: RuleConditionKind,
    ) -> &'static str {
        shared_rule_condition_kind_label(condition_kind)
    }

    pub(in super::super) fn action_kind(action: &RuleAction) -> RuleActionKind {
        shared_rule_action_kind(action)
    }

    pub(in super::super) fn action_kind_label(action_kind: RuleActionKind) -> &'static str {
        shared_rule_action_kind_label(action_kind)
    }

    pub(in super::super) fn spawn_entity_type_label(
        entity_type: RuleSpawnEntityType,
    ) -> &'static str {
        shared_rule_spawn_entity_type_label(entity_type)
    }

    pub(in super::super) fn sound_channel_label(channel: RuleSoundChannel) -> &'static str {
        shared_rule_sound_channel_label(channel)
    }

    pub(in super::super) fn trigger_kind(trigger: &RuleTrigger) -> RuleTriggerKind {
        shared_rule_trigger_kind(trigger)
    }

    pub(in super::super) fn trigger_kind_label(kind: RuleTriggerKind) -> &'static str {
        shared_rule_trigger_kind_label(kind)
    }

    pub(in super::super) fn set_rule_trigger_kind(rule: &mut Rule, kind: RuleTriggerKind) {
        rule.trigger = shared_default_rule_trigger(kind);
    }

    pub(in super::super) fn rule_key_label(key: RuleKey) -> &'static str {
        shared_rule_key_label(key)
    }

    pub(in super::super) fn load_rule_audio_choices(
        config: Option<&EditorConfig>,
    ) -> RuleAudioChoices {
        let Some(project_path) = config.and_then(|cfg| cfg.current_project_path()) else {
            return RuleAudioChoices::default();
        };

        RuleAudioChoices {
            sfx: crate::project::ProjectAssets::discover_project_audio_names(
                project_path,
                ProjectAudioAssetKind::Sfx,
            ),
            music: crate::project::ProjectAssets::discover_project_audio_names(
                project_path,
                ProjectAudioAssetKind::Music,
            ),
        }
    }
}
