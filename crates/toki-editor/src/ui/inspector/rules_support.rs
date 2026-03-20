use super::*;

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
        rule.actions.push(Self::default_action(action_kind));
    }

    pub(in super::super) fn add_condition(rule: &mut Rule, condition_kind: RuleConditionKind) {
        rule.conditions
            .push(Self::default_condition(condition_kind));
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
        *condition = Self::default_condition(condition_kind);
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
        *action = Self::default_action(action_kind);
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
                                message: "AddInventoryItem requires a non-empty item id".to_string(),
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
                                message: "RemoveInventoryItem requires a non-empty item id".to_string(),
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

    pub(in super::super) fn default_action(action_kind: RuleActionKind) -> RuleAction {
        match action_kind {
            RuleActionKind::PlaySound => RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_placeholder".to_string(),
            },
            RuleActionKind::PlayMusic => RuleAction::PlayMusic {
                track_id: "music_placeholder".to_string(),
            },
            RuleActionKind::PlayAnimation => RuleAction::PlayAnimation {
                target: RuleTarget::Player,
                state: AnimationState::Idle,
            },
            RuleActionKind::SetVelocity => RuleAction::SetVelocity {
                target: RuleTarget::Player,
                velocity: [0, 0],
            },
            RuleActionKind::Spawn => RuleAction::Spawn {
                entity_type: RuleSpawnEntityType::Npc,
                position: [0, 0],
            },
            RuleActionKind::DestroySelf => RuleAction::DestroySelf {
                target: RuleTarget::Player,
            },
            RuleActionKind::SwitchScene => RuleAction::SwitchScene {
                scene_name: String::new(),
                spawn_point_id: String::new(),
            },
            RuleActionKind::DamageEntity => RuleAction::DamageEntity {
                target: RuleTarget::Player,
                amount: 10,
            },
            RuleActionKind::HealEntity => RuleAction::HealEntity {
                target: RuleTarget::Player,
                amount: 10,
            },
            RuleActionKind::AddInventoryItem => RuleAction::AddInventoryItem {
                target: RuleTarget::Player,
                item_id: String::new(),
                count: 1,
            },
            RuleActionKind::RemoveInventoryItem => RuleAction::RemoveInventoryItem {
                target: RuleTarget::Player,
                item_id: String::new(),
                count: 1,
            },
            RuleActionKind::SetEntityActive => RuleAction::SetEntityActive {
                target: RuleTarget::Player,
                active: false,
            },
            RuleActionKind::TeleportEntity => RuleAction::TeleportEntity {
                target: RuleTarget::Player,
                position: [0, 0],
            },
        }
    }

    pub(in super::super) fn default_condition(condition_kind: RuleConditionKind) -> RuleCondition {
        match condition_kind {
            RuleConditionKind::Always => RuleCondition::Always,
            RuleConditionKind::TargetExists => RuleCondition::TargetExists {
                target: RuleTarget::Player,
            },
            RuleConditionKind::KeyHeld => RuleCondition::KeyHeld { key: RuleKey::Up },
            RuleConditionKind::EntityActive => RuleCondition::EntityActive {
                target: RuleTarget::Player,
                is_active: true,
            },
            RuleConditionKind::HealthBelow => RuleCondition::HealthBelow {
                target: RuleTarget::Player,
                threshold: 50,
            },
            RuleConditionKind::HealthAbove => RuleCondition::HealthAbove {
                target: RuleTarget::Player,
                threshold: 50,
            },
            RuleConditionKind::TriggerOtherIsPlayer => RuleCondition::TriggerOtherIsPlayer,
            RuleConditionKind::EntityIsKind => RuleCondition::EntityIsKind {
                target: RuleTarget::Player,
                kind: toki_core::entity::EntityKind::Player,
            },
            RuleConditionKind::TriggerOtherIsKind => RuleCondition::TriggerOtherIsKind {
                kind: toki_core::entity::EntityKind::Npc,
            },
            RuleConditionKind::EntityHasTag => RuleCondition::EntityHasTag {
                target: RuleTarget::Player,
                tag: String::new(),
            },
            RuleConditionKind::TriggerOtherHasTag => RuleCondition::TriggerOtherHasTag {
                tag: String::new(),
            },
            RuleConditionKind::HasInventoryItem => RuleCondition::HasInventoryItem {
                target: RuleTarget::Player,
                item_id: String::new(),
                min_count: 1,
            },
        }
    }

    pub(in super::super) fn condition_kind(condition: &RuleCondition) -> RuleConditionKind {
        match condition {
            RuleCondition::Always => RuleConditionKind::Always,
            RuleCondition::TargetExists { .. } => RuleConditionKind::TargetExists,
            RuleCondition::KeyHeld { .. } => RuleConditionKind::KeyHeld,
            RuleCondition::EntityActive { .. } => RuleConditionKind::EntityActive,
            RuleCondition::HealthBelow { .. } => RuleConditionKind::HealthBelow,
            RuleCondition::HealthAbove { .. } => RuleConditionKind::HealthAbove,
            RuleCondition::TriggerOtherIsPlayer => RuleConditionKind::TriggerOtherIsPlayer,
            RuleCondition::EntityIsKind { .. } => RuleConditionKind::EntityIsKind,
            RuleCondition::TriggerOtherIsKind { .. } => RuleConditionKind::TriggerOtherIsKind,
            RuleCondition::EntityHasTag { .. } => RuleConditionKind::EntityHasTag,
            RuleCondition::TriggerOtherHasTag { .. } => RuleConditionKind::TriggerOtherHasTag,
            RuleCondition::HasInventoryItem { .. } => RuleConditionKind::HasInventoryItem,
        }
    }

    pub(in super::super) fn condition_kind_label(
        condition_kind: RuleConditionKind,
    ) -> &'static str {
        match condition_kind {
            RuleConditionKind::Always => "Always",
            RuleConditionKind::TargetExists => "TargetExists",
            RuleConditionKind::KeyHeld => "KeyHeld",
            RuleConditionKind::EntityActive => "EntityActive",
            RuleConditionKind::HealthBelow => "HealthBelow",
            RuleConditionKind::HealthAbove => "HealthAbove",
            RuleConditionKind::TriggerOtherIsPlayer => "TriggerOtherIsPlayer",
            RuleConditionKind::EntityIsKind => "EntityIsKind",
            RuleConditionKind::TriggerOtherIsKind => "TriggerOtherIsKind",
            RuleConditionKind::EntityHasTag => "EntityHasTag",
            RuleConditionKind::TriggerOtherHasTag => "TriggerOtherHasTag",
            RuleConditionKind::HasInventoryItem => "HasInventoryItem",
        }
    }

    pub(in super::super) fn action_kind(action: &RuleAction) -> RuleActionKind {
        match action {
            RuleAction::PlaySound { .. } => RuleActionKind::PlaySound,
            RuleAction::PlayMusic { .. } => RuleActionKind::PlayMusic,
            RuleAction::PlayAnimation { .. } => RuleActionKind::PlayAnimation,
            RuleAction::SetVelocity { .. } => RuleActionKind::SetVelocity,
            RuleAction::Spawn { .. } => RuleActionKind::Spawn,
            RuleAction::DestroySelf { .. } => RuleActionKind::DestroySelf,
            RuleAction::SwitchScene { .. } => RuleActionKind::SwitchScene,
            RuleAction::DamageEntity { .. } => RuleActionKind::DamageEntity,
            RuleAction::HealEntity { .. } => RuleActionKind::HealEntity,
            RuleAction::AddInventoryItem { .. } => RuleActionKind::AddInventoryItem,
            RuleAction::RemoveInventoryItem { .. } => RuleActionKind::RemoveInventoryItem,
            RuleAction::SetEntityActive { .. } => RuleActionKind::SetEntityActive,
            RuleAction::TeleportEntity { .. } => RuleActionKind::TeleportEntity,
        }
    }

    pub(in super::super) fn action_kind_label(action_kind: RuleActionKind) -> &'static str {
        match action_kind {
            RuleActionKind::PlaySound => "PlaySound",
            RuleActionKind::PlayMusic => "PlayMusic",
            RuleActionKind::PlayAnimation => "PlayAnimation",
            RuleActionKind::SetVelocity => "SetVelocity",
            RuleActionKind::Spawn => "Spawn",
            RuleActionKind::DestroySelf => "DestroySelf",
            RuleActionKind::SwitchScene => "SwitchScene",
            RuleActionKind::DamageEntity => "DamageEntity",
            RuleActionKind::HealEntity => "HealEntity",
            RuleActionKind::AddInventoryItem => "AddInventoryItem",
            RuleActionKind::RemoveInventoryItem => "RemoveInventoryItem",
            RuleActionKind::SetEntityActive => "SetEntityActive",
            RuleActionKind::TeleportEntity => "TeleportEntity",
        }
    }

    pub(in super::super) fn spawn_entity_type_label(
        entity_type: RuleSpawnEntityType,
    ) -> &'static str {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => "PlayerLikeNpc",
            RuleSpawnEntityType::Npc => "Npc",
            RuleSpawnEntityType::Item => "Item",
            RuleSpawnEntityType::Decoration => "Decoration",
            RuleSpawnEntityType::Trigger => "Trigger",
        }
    }

    pub(in super::super) fn trigger_kind(trigger: &RuleTrigger) -> RuleTriggerKind {
        match trigger {
            RuleTrigger::OnStart => RuleTriggerKind::Start,
            RuleTrigger::OnUpdate => RuleTriggerKind::Update,
            RuleTrigger::OnPlayerMove => RuleTriggerKind::PlayerMove,
            RuleTrigger::OnKey { .. } => RuleTriggerKind::Key,
            RuleTrigger::OnCollision { .. } => RuleTriggerKind::Collision,
            RuleTrigger::OnDamaged { .. } => RuleTriggerKind::Damaged,
            RuleTrigger::OnDeath { .. } => RuleTriggerKind::Death,
            RuleTrigger::OnTrigger => RuleTriggerKind::Trigger,
            RuleTrigger::OnInteract { .. } => RuleTriggerKind::Interact,
            RuleTrigger::OnTileEnter { .. } => RuleTriggerKind::TileEnter,
            RuleTrigger::OnTileExit { .. } => RuleTriggerKind::TileExit,
        }
    }

    pub(in super::super) fn trigger_kind_label(kind: RuleTriggerKind) -> &'static str {
        match kind {
            RuleTriggerKind::Start => "OnStart",
            RuleTriggerKind::Update => "OnUpdate",
            RuleTriggerKind::PlayerMove => "OnPlayerMove",
            RuleTriggerKind::Key => "OnKey",
            RuleTriggerKind::Collision => "OnCollision",
            RuleTriggerKind::Damaged => "OnDamaged",
            RuleTriggerKind::Death => "OnDeath",
            RuleTriggerKind::Trigger => "OnTrigger",
            RuleTriggerKind::Interact => "OnInteract",
            RuleTriggerKind::TileEnter => "OnTileEnter",
            RuleTriggerKind::TileExit => "OnTileExit",
        }
    }

    pub(in super::super) fn set_rule_trigger_kind(rule: &mut Rule, kind: RuleTriggerKind) {
        rule.trigger = match kind {
            RuleTriggerKind::Start => RuleTrigger::OnStart,
            RuleTriggerKind::Update => RuleTrigger::OnUpdate,
            RuleTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
            RuleTriggerKind::Key => RuleTrigger::OnKey { key: RuleKey::Up },
            RuleTriggerKind::Collision => RuleTrigger::OnCollision { entity: None },
            RuleTriggerKind::Damaged => RuleTrigger::OnDamaged { entity: None },
            RuleTriggerKind::Death => RuleTrigger::OnDeath { entity: None },
            RuleTriggerKind::Trigger => RuleTrigger::OnTrigger,
            RuleTriggerKind::Interact => RuleTrigger::OnInteract {
                mode: toki_core::rules::InteractionMode::default(),
                entity: None,
            },
            RuleTriggerKind::TileEnter => RuleTrigger::OnTileEnter { x: 0, y: 0 },
            RuleTriggerKind::TileExit => RuleTrigger::OnTileExit { x: 0, y: 0 },
        };
    }

    pub(in super::super) fn rule_key_label(key: RuleKey) -> &'static str {
        match key {
            RuleKey::Up => "Up",
            RuleKey::Down => "Down",
            RuleKey::Left => "Left",
            RuleKey::Right => "Right",
            RuleKey::DebugToggle => "DebugToggle",
            RuleKey::Interact => "Interact",
            RuleKey::AttackPrimary => "AttackPrimary",
            RuleKey::AttackSecondary => "AttackSecondary",
            RuleKey::Inventory => "Inventory",
            RuleKey::Pause => "Pause",
        }
    }

    pub(in super::super) fn load_rule_audio_choices(
        config: Option<&EditorConfig>,
    ) -> RuleAudioChoices {
        let Some(project_path) = config.and_then(|cfg| cfg.current_project_path()) else {
            return RuleAudioChoices::default();
        };

        RuleAudioChoices {
            sfx: Self::discover_audio_asset_names(project_path.join("assets/audio/sfx").as_path()),
            music: Self::discover_audio_asset_names(
                project_path.join("assets/audio/music").as_path(),
            ),
        }
    }
}
