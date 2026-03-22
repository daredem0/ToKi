//! Entity property editor UI components.

use super::super::InspectorSystem;
use super::helpers::{
    ai_behavior_label, ai_behavior_needs_detection_radius, control_role_label,
    movement_profile_label, movement_sound_trigger_label,
};
use super::types::EntityPropertyDraft;
use crate::config::EditorConfig;
use std::path::PathBuf;
use toki_core::entity::{AiBehavior, ControlRole, MovementProfile, MovementSoundTrigger};

impl InspectorSystem {
    pub(in super::super) fn render_entity_property_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        config: Option<&EditorConfig>,
        show_position: bool,
        allow_control_role_edit: bool,
        section_label: &str,
    ) -> bool {
        let mut changed = false;
        let is_static_item = draft.category == "item" && draft.static_object_sheet.is_some();

        ui.label(section_label);
        ui.separator();

        if show_position {
            changed |= render_position_row(ui, draft);
        }

        changed |= render_size_row(ui, draft);
        changed |= render_render_layer_row(ui, draft);
        render_static_render_row(ui, draft);

        if !is_static_item {
            changed |= render_speed_row(ui, draft);
        }

        changed |= ui.checkbox(&mut draft.visible, "Visible").changed();
        changed |= ui.checkbox(&mut draft.has_shadow, "Has Shadow").changed();
        changed |= ui.checkbox(&mut draft.active, "Active").changed();
        changed |= ui.checkbox(&mut draft.solid, "Solid").changed();
        changed |= ui
            .checkbox(&mut draft.interactable, "Interactable")
            .changed();

        if draft.interactable {
            changed |= render_interaction_reach_row(ui, draft);
        }

        if !is_static_item {
            changed |= ui.checkbox(&mut draft.can_move, "Can Move").changed();
            changed |= render_control_role_row(ui, draft, allow_control_role_edit);
            changed |= render_movement_profile_row(ui, draft);
            changed |= render_ai_behavior_row(ui, draft);

            if ai_behavior_needs_detection_radius(draft.ai_config.behavior) {
                changed |= render_detection_radius_row(ui, draft);
            }
        }

        changed |= ui
            .checkbox(&mut draft.has_inventory, "Has Inventory")
            .changed();

        if !is_static_item {
            ui.separator();
            changed |= render_audio_section(ui, draft, config);
        }

        ui.separator();
        changed |= render_stats_section(ui, draft);

        ui.separator();
        changed |= render_collision_section(ui, draft);

        changed
    }

    pub(in super::super) fn render_scene_entity_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        config: Option<&EditorConfig>,
    ) -> bool {
        Self::render_entity_property_editor(
            ui,
            draft,
            config,
            true,
            true,
            "Scene Entity Properties",
        )
    }

    pub(in super::super) fn render_entity_definition_property_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        config: Option<&EditorConfig>,
    ) -> bool {
        Self::render_entity_property_editor(ui, draft, config, false, false, "Entity Properties")
    }
}

fn render_position_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Position:");
        changed |= ui
            .add(egui::DragValue::new(&mut draft.position_x).speed(1.0))
            .changed();
        changed |= ui
            .add(egui::DragValue::new(&mut draft.position_y).speed(1.0))
            .changed();
    });
    changed
}

fn render_size_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Size:");
        changed |= ui
            .add(
                egui::DragValue::new(&mut draft.size_x)
                    .speed(1.0)
                    .range(1..=i64::MAX),
            )
            .changed();
        changed |= ui
            .add(
                egui::DragValue::new(&mut draft.size_y)
                    .speed(1.0)
                    .range(1..=i64::MAX),
            )
            .changed();
    });
    changed
}

fn render_render_layer_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Render Layer:");
        changed |= ui
            .add(egui::DragValue::new(&mut draft.render_layer).speed(1.0))
            .changed();
    });
    changed
}

fn render_static_render_row(ui: &mut egui::Ui, draft: &EntityPropertyDraft) {
    if let (Some(sheet), Some(object_name)) =
        (&draft.static_object_sheet, &draft.static_object_name)
    {
        ui.horizontal(|ui| {
            ui.label("Static Render:");
            ui.label(format!("{sheet}/{object_name}"));
        });
    }
}

fn render_speed_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Speed:");
        changed |= ui
            .add(
                egui::DragValue::new(&mut draft.speed)
                    .speed(0.1)
                    .range(0.0..=1000.0),
            )
            .changed();
    });
    changed
}

fn render_interaction_reach_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Interaction Reach:");
        let mut value = draft.interaction_reach as i64;
        if ui
            .add(egui::DragValue::new(&mut value).speed(1.0).range(0..=256))
            .changed()
        {
            draft.interaction_reach = value as u32;
            changed = true;
        }
        ui.label("px");
    });
    changed
}

fn render_control_role_row(
    ui: &mut egui::Ui,
    draft: &mut EntityPropertyDraft,
    allow_edit: bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Control Role:");
        if allow_edit {
            egui::ComboBox::from_id_salt("entity_control_role")
                .selected_text(control_role_label(draft.control_role))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut draft.control_role, ControlRole::None, "None")
                        .changed();
                    changed |= ui
                        .selectable_value(
                            &mut draft.control_role,
                            ControlRole::PlayerCharacter,
                            "Player Character",
                        )
                        .changed();
                });
        } else {
            ui.label(control_role_label(ControlRole::PlayerCharacter));
        }
    });
    changed
}

fn render_movement_profile_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Movement:");
        egui::ComboBox::from_id_salt("entity_movement_profile")
            .selected_text(movement_profile_label(
                draft.control_role,
                draft.movement_profile,
            ))
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(&mut draft.movement_profile, MovementProfile::None, "None")
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.movement_profile,
                        MovementProfile::PlayerWasd,
                        "Player WASD",
                    )
                    .changed();
            });
    });
    changed
}

fn render_ai_behavior_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("AI:");
        egui::ComboBox::from_id_salt("entity_ai_behavior")
            .selected_text(ai_behavior_label(draft.ai_config.behavior))
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(&mut draft.ai_config.behavior, AiBehavior::None, "None")
                    .changed();
                changed |= ui
                    .selectable_value(&mut draft.ai_config.behavior, AiBehavior::Wander, "Wander")
                    .changed();
                changed |= ui
                    .selectable_value(&mut draft.ai_config.behavior, AiBehavior::Chase, "Chase")
                    .changed();
                changed |= ui
                    .selectable_value(&mut draft.ai_config.behavior, AiBehavior::Run, "Run")
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.ai_config.behavior,
                        AiBehavior::RunAndMultiply,
                        "Run And Multiply",
                    )
                    .changed();
            });
    });
    changed
}

fn render_detection_radius_row(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Detection Radius:");
        let mut radius = draft.ai_config.detection_radius as i32;
        if ui
            .add(egui::DragValue::new(&mut radius).range(0..=1000).speed(1))
            .changed()
        {
            draft.ai_config.detection_radius = radius.max(0) as u32;
            changed = true;
        }
    });
    changed
}

fn render_audio_section(
    ui: &mut egui::Ui,
    draft: &mut EntityPropertyDraft,
    config: Option<&EditorConfig>,
) -> bool {
    let mut changed = false;
    ui.label("Audio");

    ui.horizontal(|ui| {
        ui.label("Movement Trigger:");
        egui::ComboBox::from_id_salt("entity_movement_sound_trigger")
            .selected_text(movement_sound_trigger_label(draft.movement_sound_trigger))
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(
                        &mut draft.movement_sound_trigger,
                        MovementSoundTrigger::Distance,
                        "Distance",
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.movement_sound_trigger,
                        MovementSoundTrigger::AnimationLoop,
                        "Animation Loop",
                    )
                    .changed();
            });
    });

    let uses_distance_trigger =
        matches!(draft.movement_sound_trigger, MovementSoundTrigger::Distance);
    ui.horizontal(|ui| {
        ui.label("Footstep Distance:");
        ui.add_enabled_ui(uses_distance_trigger, |ui| {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.footstep_trigger_distance)
                        .speed(0.5)
                        .range(0.0..=f32::MAX),
                )
                .changed();
        });
    });

    changed |= render_movement_sound_dropdown(ui, draft, config);

    ui.horizontal(|ui| {
        ui.label("Hearing Radius:");
        changed |= ui
            .add(
                egui::DragValue::new(&mut draft.hearing_radius)
                    .speed(1.0)
                    .range(0..=u32::MAX),
            )
            .changed();
    });

    changed
}

fn render_movement_sound_dropdown(
    ui: &mut egui::Ui,
    draft: &mut EntityPropertyDraft,
    config: Option<&EditorConfig>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Movement Sound:");
        let mut sfx_names = config
            .and_then(|cfg: &EditorConfig| cfg.current_project_path())
            .map(|project_path: &PathBuf| {
                crate::project::ProjectAssets::discover_project_audio_names(
                    project_path,
                    crate::project::assets::ProjectAudioAssetKind::Sfx,
                )
            })
            .unwrap_or_default();

        if !draft.movement_sound.trim().is_empty()
            && !sfx_names.iter().any(|name| name == &draft.movement_sound)
        {
            sfx_names.push(draft.movement_sound.clone());
            sfx_names.sort();
            sfx_names.dedup();
        }

        egui::ComboBox::from_id_salt("entity_movement_sound")
            .selected_text(if draft.movement_sound.trim().is_empty() {
                "None".to_string()
            } else {
                draft.movement_sound.clone()
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(&mut draft.movement_sound, String::new(), "None")
                    .changed();
                for sound_name in sfx_names.iter() {
                    changed |= ui
                        .selectable_value(
                            &mut draft.movement_sound,
                            sound_name.clone(),
                            sound_name.as_str(),
                        )
                        .changed();
                }
            });
    });
    changed
}

fn render_stats_section(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.label("Stats");

    ui.horizontal(|ui| {
        ui.label("Health:");
        changed |= ui.checkbox(&mut draft.health_enabled, "Enabled").changed();
        if draft.health_enabled {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.health_value)
                        .speed(1.0)
                        .range(0..=i64::MAX),
                )
                .changed();
        }
    });

    ui.horizontal(|ui| {
        ui.label("Attack Power:");
        changed |= ui
            .checkbox(&mut draft.attack_power_enabled, "Enabled")
            .changed();
        if draft.attack_power_enabled {
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.attack_power_value)
                        .speed(1.0)
                        .range(0..=i64::MAX),
                )
                .changed();
        }
    });

    changed
}

fn render_collision_section(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    ui.label("Collision");

    changed |= ui
        .checkbox(&mut draft.collision.enabled, "Enabled")
        .changed();

    if draft.collision.enabled {
        ui.horizontal(|ui| {
            ui.label("Offset:");
            changed |= ui
                .add(egui::DragValue::new(&mut draft.collision.offset_x).speed(1.0))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut draft.collision.offset_y).speed(1.0))
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.collision.size_x)
                        .speed(1.0)
                        .range(1..=i64::MAX),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.collision.size_y)
                        .speed(1.0)
                        .range(1..=i64::MAX),
                )
                .changed();
        });

        changed |= ui
            .checkbox(&mut draft.collision.trigger, "Trigger")
            .changed();
    }

    changed
}
