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

impl InspectorSystem {
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
        rule.conditions.push(shared_default_rule_condition(condition_kind));
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

    pub(in super::super) fn validate_rule_set(rule_set: &RuleSet) -> Vec<RuleValidationIssue> {
        Self::validate_rule_set_for_scene(rule_set, "", &[])
    }

    pub(in super::super) fn validate_rule_set_for_scene(
        rule_set: &RuleSet,
        _current_scene_name: &str,
        scenes: &[toki_core::Scene],
    ) -> Vec<RuleValidationIssue> {
        let mut issues = Vec::new();

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

            for (condition_index, condition) in rule.conditions.iter().enumerate() {
                match condition {
                    RuleCondition::Always
                    | RuleCondition::KeyHeld { .. }
                    | RuleCondition::TriggerOtherIsPlayer
                    | RuleCondition::TriggerOtherIsKind { .. }
                    | RuleCondition::TriggerOtherHasTag { .. } => {}
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
                    RuleAction::SetVelocity { target, .. } => {
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
                    }
                    RuleAction::Spawn { .. } => {}
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
                    RuleAction::DamageEntity { amount, .. } => {
                        if *amount <= 0 {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "DamageEntity amount must be positive".to_string(),
                            });
                        }
                    }
                    RuleAction::HealEntity { amount, .. } => {
                        if *amount <= 0 {
                            issues.push(RuleValidationIssue {
                                rule_index,
                                action_index: Some(action_index),
                                message: "HealEntity amount must be positive".to_string(),
                            });
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
                    RuleAction::TeleportEntity { .. } => {}
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
        let mut ids = scene
            .anchors
            .iter()
            .filter(|anchor| matches!(anchor.kind, toki_core::scene::SceneAnchorKind::SpawnPoint))
            .map(|anchor| anchor.id.clone())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    pub(in super::super) fn render_switch_scene_editor(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        scene_name: &mut String,
        spawn_point_id: &mut String,
        scenes: &[toki_core::Scene],
    ) -> bool {
        let mut changed = false;
        let scene_names = Self::scene_switch_target_scene_names(scenes);
        ui.horizontal(|ui| {
            ui.label("Scene:");
            egui::ComboBox::from_id_salt((&id_salt, "scene"))
                .selected_text(if scene_name.is_empty() {
                    "<select scene>"
                } else {
                    scene_name.as_str()
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(scene_name, String::new(), "<select scene>")
                        .changed();
                    for candidate in &scene_names {
                        changed |= ui
                            .selectable_value(scene_name, candidate.clone(), candidate)
                            .changed();
                    }
                });
        });

        let spawn_ids = Self::scene_switch_spawn_point_ids(scenes, scene_name);
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
