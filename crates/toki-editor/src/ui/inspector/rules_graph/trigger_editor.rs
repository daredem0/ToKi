//! Trigger node editing UI.

use super::*;

impl InspectorSystem {
    pub(in super::super) fn render_rule_graph_trigger_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        trigger: &mut RuleTrigger,
        map_size: Option<(u32, u32)>,
    ) -> bool {
        let mut changed = false;
        let mut trigger_kind = Self::trigger_kind(trigger);
        ui.horizontal(|ui| {
            ui.label("Type:");
            egui::ComboBox::from_id_salt(format!(
                "graph_node_trigger_kind_{}_{}",
                scene_name, node_key
            ))
            .selected_text(Self::trigger_kind_label(trigger_kind))
            .show_ui(ui, |ui| {
                for candidate in RuleTriggerKind::iter() {
                    changed |= ui
                        .selectable_value(
                            &mut trigger_kind,
                            candidate,
                            Self::trigger_kind_label(candidate),
                        )
                        .changed();
                }
            });
        });

        if trigger_kind != Self::trigger_kind(trigger) {
            *trigger = Self::default_trigger_for_kind(trigger_kind);
            changed = true;
        }

        changed |= Self::render_trigger_parameters(ui, scene_name, node_key, trigger);
        changed |= Self::render_tile_coordinates(ui, trigger, map_size);

        changed
    }

    fn default_trigger_for_kind(kind: RuleTriggerKind) -> RuleTrigger {
        match kind {
            RuleTriggerKind::Start => RuleTrigger::OnStart,
            RuleTriggerKind::Update => RuleTrigger::OnUpdate,
            RuleTriggerKind::PlayerMove => RuleTrigger::OnPlayerMove,
            RuleTriggerKind::Key => RuleTrigger::OnKey {
                key: toki_core::rules::RuleKey::Up,
            },
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
        }
    }

    fn render_trigger_parameters(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        trigger: &mut RuleTrigger,
    ) -> bool {
        let mut changed = false;

        if let RuleTrigger::OnKey { key } = trigger {
            changed |= Self::render_rule_key_editor_with_salt(
                ui,
                &format!("graph_node_trigger_key_{}_{}", scene_name, node_key),
                key,
            );
        }

        if let RuleTrigger::OnInteract { mode, .. } = trigger {
            changed |= Self::render_rule_interaction_mode_editor_with_salt(
                ui,
                &format!(
                    "graph_node_trigger_interact_mode_{}_{}",
                    scene_name, node_key
                ),
                mode,
            );
        }

        // Entity filter editors for triggers that support them
        changed |= Self::render_trigger_entity_filter(ui, scene_name, node_key, trigger);

        changed
    }

    fn render_trigger_entity_filter(
        ui: &mut egui::Ui,
        scene_name: &str,
        node_key: &str,
        trigger: &mut RuleTrigger,
    ) -> bool {
        let mut changed = false;

        if let RuleTrigger::OnCollision { entity } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!(
                    "graph_node_trigger_collision_entity_{}_{}",
                    scene_name, node_key
                ),
                entity,
            );
        }
        if let RuleTrigger::OnDamaged { entity } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!(
                    "graph_node_trigger_damaged_entity_{}_{}",
                    scene_name, node_key
                ),
                entity,
            );
        }
        if let RuleTrigger::OnDeath { entity } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!(
                    "graph_node_trigger_death_entity_{}_{}",
                    scene_name, node_key
                ),
                entity,
            );
        }
        if let RuleTrigger::OnInteract { entity, .. } = trigger {
            changed |= Self::render_optional_entity_filter_editor(
                ui,
                &format!(
                    "graph_node_trigger_interact_entity_{}_{}",
                    scene_name, node_key
                ),
                entity,
            );
        }

        changed
    }

    fn render_tile_coordinates(
        ui: &mut egui::Ui,
        trigger: &mut RuleTrigger,
        map_size: Option<(u32, u32)>,
    ) -> bool {
        let mut changed = false;

        let (x, y) = match trigger {
            RuleTrigger::OnTileEnter { x, y } | RuleTrigger::OnTileExit { x, y } => (x, y),
            _ => return false,
        };

        changed |= Self::render_tile_coordinate(ui, "Tile X:", x);
        if let Some((map_width, _)) = map_size {
            Self::render_coordinate_warning(ui, *x, map_width, "X", "width");
        }

        changed |= Self::render_tile_coordinate(ui, "Tile Y:", y);
        if let Some((_, map_height)) = map_size {
            Self::render_coordinate_warning(ui, *y, map_height, "Y", "height");
        }

        changed
    }

    fn render_tile_coordinate(ui: &mut egui::Ui, label: &str, value: &mut u32) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(label);
            let mut int_val = *value as i32;
            if ui
                .add(egui::DragValue::new(&mut int_val).speed(1.0).range(0..=9999))
                .changed()
            {
                *value = int_val.max(0) as u32;
                changed = true;
            }
        });
        changed
    }

    fn render_coordinate_warning(
        ui: &mut egui::Ui,
        value: u32,
        max: u32,
        coord_name: &str,
        dimension: &str,
    ) {
        if value >= max {
            ui.colored_label(
                egui::Color32::from_rgb(255, 150, 80),
                format!(
                    "\u{26a0} {} coordinate {} is out of bounds (map {}: {})",
                    coord_name, value, dimension, max
                ),
            );
        }
    }
}
