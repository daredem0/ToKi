//! Shared editor widgets for rule graph editing.

use super::*;

impl InspectorSystem {
    pub(in super::super) fn render_rule_target_editor_with_salt(
        ui: &mut egui::Ui,
        id_salt: &str,
        target: &mut RuleTarget,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Target:");
            egui::ComboBox::from_id_salt((id_salt, "kind"))
                .selected_text(match target {
                    RuleTarget::Player => "Player",
                    RuleTarget::Entity(_) => "Entity",
                    RuleTarget::RuleOwner => "RuleOwner",
                    RuleTarget::TriggerSelf => "TriggerSelf",
                    RuleTarget::TriggerOther => "TriggerOther",
                })
                .show_ui(ui, |ui| {
                    changed |= Self::render_target_option(ui, target, RuleTarget::Player, "Player");
                    changed |=
                        Self::render_target_option(ui, target, RuleTarget::Entity(1), "Entity");
                    changed |= Self::render_target_option(
                        ui,
                        target,
                        RuleTarget::TriggerSelf,
                        "TriggerSelf",
                    );
                    changed |= Self::render_target_option(
                        ui,
                        target,
                        RuleTarget::TriggerOther,
                        "TriggerOther",
                    );
                    changed |=
                        Self::render_target_option(ui, target, RuleTarget::RuleOwner, "RuleOwner");
                });
        });

        if let RuleTarget::Entity(entity_id) = target {
            ui.horizontal(|ui| {
                ui.label("Entity Id:");
                let mut value = *entity_id as i64;
                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .speed(1.0)
                            .range(1..=u32::MAX as i64),
                    )
                    .changed()
                {
                    *entity_id = value as u32;
                    changed = true;
                }
            });
        }

        changed
    }

    fn render_target_option(
        ui: &mut egui::Ui,
        target: &mut RuleTarget,
        option: RuleTarget,
        label: &str,
    ) -> bool {
        let is_match = Self::target_kind_matches(target, &option);
        if ui.selectable_label(is_match, label).clicked() && !is_match {
            *target = option;
            return true;
        }
        false
    }

    fn target_kind_matches(target: &RuleTarget, option: &RuleTarget) -> bool {
        std::mem::discriminant(target) == std::mem::discriminant(option)
    }

    pub(in super::super) fn render_rule_key_editor_with_salt(
        ui: &mut egui::Ui,
        id_salt: &str,
        key: &mut RuleKey,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Key:");
            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(Self::rule_key_label(*key))
                .show_ui(ui, |ui| {
                    for candidate in RuleKey::iter() {
                        changed |= ui
                            .selectable_value(key, candidate, Self::rule_key_label(candidate))
                            .changed();
                    }
                });
        });
        changed
    }

    pub(super) fn interaction_mode_label(mode: InteractionMode) -> &'static str {
        match mode {
            InteractionMode::Overlap => "Overlap (Same Tile)",
            InteractionMode::Adjacent => "Adjacent (Within Reach)",
            InteractionMode::InFront => "In Front",
        }
    }

    pub(in super::super) fn render_rule_interaction_mode_editor_with_salt(
        ui: &mut egui::Ui,
        id_salt: &str,
        mode: &mut InteractionMode,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Mode:");
            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(Self::interaction_mode_label(*mode))
                .show_ui(ui, |ui| {
                    for candidate in InteractionMode::iter() {
                        changed |= ui
                            .selectable_value(
                                mode,
                                candidate,
                                Self::interaction_mode_label(candidate),
                            )
                            .changed();
                    }
                });
        });
        changed
    }

    pub(super) fn entity_kind_label(kind: EntityKind) -> &'static str {
        match kind {
            EntityKind::Player => "Player",
            EntityKind::Npc => "NPC",
            EntityKind::Item => "Item",
            EntityKind::Decoration => "Decoration",
            EntityKind::Trigger => "Trigger",
            EntityKind::Projectile => "Projectile",
        }
    }

    pub(in super::super) fn render_entity_kind_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        kind: &mut EntityKind,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Entity Kind:");
            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(Self::entity_kind_label(*kind))
                .show_ui(ui, |ui| {
                    for candidate in EntityKind::iter() {
                        changed |= ui
                            .selectable_value(kind, candidate, Self::entity_kind_label(candidate))
                            .changed();
                    }
                });
        });
        changed
    }

    /// Renders an optional entity filter editor for triggers like OnDamaged, OnDeath, OnCollision.
    ///
    /// When `None`, the trigger fires for all events. When `Some(target)`, it only fires
    /// when the resolved target matches the event entity.
    pub(in super::super) fn render_optional_entity_filter_editor(
        ui: &mut egui::Ui,
        id_salt: &str,
        entity: &mut Option<RuleTarget>,
    ) -> bool {
        let mut changed = false;
        let is_filtered = entity.is_some();

        ui.horizontal(|ui| {
            ui.label("Entity Filter:");
            let filter_label = if is_filtered {
                "Specific Entity"
            } else {
                "All Entities"
            };
            egui::ComboBox::from_id_salt((id_salt, "filter_toggle"))
                .selected_text(filter_label)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(!is_filtered, "All Entities").clicked() && is_filtered {
                        *entity = None;
                        changed = true;
                    }
                    if ui
                        .selectable_label(is_filtered, "Specific Entity")
                        .clicked()
                        && !is_filtered
                    {
                        *entity = Some(RuleTarget::Player);
                        changed = true;
                    }
                });
        });

        if let Some(target) = entity {
            changed |= Self::render_rule_target_editor_with_salt(
                ui,
                &format!("{}_target", id_salt),
                target,
            );
        }

        changed
    }
}
