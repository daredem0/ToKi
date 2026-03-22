//! Read-only runtime entity view for inspector.

use super::super::InspectorSystem;
use super::helpers::{ai_behavior_label, ai_behavior_needs_detection_radius, control_role_label, movement_profile_label};
use super::property_apply::ATTACK_POWER_STAT_ID;
use toki_core::entity::EntityId;

impl InspectorSystem {
    pub(in super::super) fn render_runtime_entity_read_only(
        ui: &mut egui::Ui,
        game_state: Option<&toki_core::GameState>,
        entity_id: EntityId,
    ) {
        let Some(game_state) = game_state else {
            ui.label("No game state available");
            return;
        };

        let Some(entity) = game_state.entity_manager().get_entity(entity_id) else {
            ui.label("Entity not found in game state");
            return;
        };

        render_basic_properties(ui, entity);
        render_type_and_role(ui, entity);
        render_visibility_state(ui, entity);
        render_stats(ui, entity);
        render_behavior(ui, entity);
        render_collision_box(ui, entity);
        render_animation_state(ui, entity);
    }
}

fn render_basic_properties(ui: &mut egui::Ui, entity: &toki_core::entity::Entity) {
    ui.horizontal(|ui| {
        ui.label("Position:");
        ui.label(format!("({}, {})", entity.position.x, entity.position.y));
    });

    ui.horizontal(|ui| {
        ui.label("Size:");
        ui.label(format!("{}x{}", entity.size.x, entity.size.y));
    });
}

fn render_type_and_role(ui: &mut egui::Ui, entity: &toki_core::entity::Entity) {
    ui.horizontal(|ui| {
        ui.label("Type:");
        if entity.category.is_empty() {
            ui.label(format!("{:?}", entity.entity_kind));
        } else {
            ui.label(entity.category.as_str());
        }
    });

    ui.horizontal(|ui| {
        ui.label("Control Role:");
        ui.label(control_role_label(entity.control_role));
    });
}

fn render_visibility_state(ui: &mut egui::Ui, entity: &toki_core::entity::Entity) {
    ui.horizontal(|ui| {
        ui.label("Visible:");
        ui.label(format!("{}", entity.attributes.visible));
    });

    ui.horizontal(|ui| {
        ui.label("Active:");
        ui.label(format!("{}", entity.attributes.active));
    });
}

fn render_stats(ui: &mut egui::Ui, entity: &toki_core::entity::Entity) {
    if let Some(health) = entity.attributes.health {
        ui.horizontal(|ui| {
            ui.label("Health:");
            ui.label(format!("{}", health));
        });
    }

    if let Some(attack_power) = entity.attributes.current_stat(ATTACK_POWER_STAT_ID) {
        ui.horizontal(|ui| {
            ui.label("Attack Power:");
            ui.label(format!("{}", attack_power));
        });
    }

    if entity.attributes.has_inventory {
        ui.horizontal(|ui| {
            ui.label("Has Inventory:");
            ui.label("Yes");
        });
    }
}

fn render_behavior(ui: &mut egui::Ui, entity: &toki_core::entity::Entity) {
    let is_static_item =
        entity.category == "item" && entity.attributes.static_object_render.is_some();

    if let Some(static_render) = &entity.attributes.static_object_render {
        ui.horizontal(|ui| {
            ui.label("Static Render:");
            ui.label(format!(
                "{}/{}",
                static_render.sheet, static_render.object_name
            ));
        });
    }

    if !is_static_item {
        ui.horizontal(|ui| {
            ui.label("AI:");
            ui.label(ai_behavior_label(entity.attributes.ai_config.behavior));
            if ai_behavior_needs_detection_radius(entity.attributes.ai_config.behavior) {
                ui.label(format!(
                    "(radius: {})",
                    entity.attributes.ai_config.detection_radius
                ));
            }
        });

        ui.horizontal(|ui| {
            ui.label("Movement:");
            ui.label(movement_profile_label(
                entity.control_role,
                entity.attributes.movement_profile,
            ));
        });
    }
}

fn render_collision_box(ui: &mut egui::Ui, entity: &toki_core::entity::Entity) {
    let Some(collision_box) = &entity.collision_box else {
        return;
    };

    ui.separator();
    ui.label("Collision Box:");

    ui.horizontal(|ui| {
        ui.label("Offset:");
        ui.label(format!(
            "({}, {})",
            collision_box.offset.x, collision_box.offset.y
        ));
    });

    ui.horizontal(|ui| {
        ui.label("Size:");
        ui.label(format!("{}x{}", collision_box.size.x, collision_box.size.y));
    });

    ui.horizontal(|ui| {
        ui.label("Trigger:");
        ui.label(format!("{}", collision_box.trigger));
    });
}

fn render_animation_state(ui: &mut egui::Ui, entity: &toki_core::entity::Entity) {
    let Some(animation_controller) = &entity.attributes.animation_controller else {
        return;
    };

    ui.separator();
    ui.label("Animation:");

    ui.horizontal(|ui| {
        ui.label("Current State:");
        ui.label(format!("{:?}", animation_controller.current_clip_state));
    });

    ui.horizontal(|ui| {
        ui.label("Frame:");
        ui.label(format!("{}", animation_controller.current_frame_index));
    });

    ui.horizontal(|ui| {
        ui.label("Finished:");
        ui.label(format!("{}", animation_controller.is_finished));
    });
}
