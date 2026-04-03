//! Entity property editor UI components.

use super::super::InspectorSystem;
use super::helpers::{
    ai_behavior_label, ai_behavior_needs_detection_radius, control_role_label,
    movement_profile_label, movement_sound_trigger_label,
};
use super::types::EntityPropertyDraft;
use crate::config::EditorConfig;
use crate::ui::editor_ui::EditorUI;
use crate::ui::entity_kind_policy::{
    default_collision_for_kind, effective_kind_for_category, kind_supports_audio,
    kind_supports_combat_defaults, kind_supports_movement,
};
use crate::ui::object_sheet_browser::{
    build_decoration_placement_draft, ensure_object_sheet_preview_texture,
    render_object_gallery_item, resolve_object_sheet_browser_source, sync_selected_object_name,
    sync_selected_sheet_name,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use toki_core::entity::{AiBehavior, ControlRole, MovementProfile, MovementSoundTrigger};
use toki_core::palette::Palette4;

/// Whether the movement section (speed, can_move, movement_profile, control_role) is relevant.
pub fn should_show_movement_section(draft: &EntityPropertyDraft) -> bool {
    let kind = effective_kind_for_category(&draft.category);
    draft.movement_component_present || draft.ai_component_present || kind_supports_movement(kind)
}

/// Whether the audio section (footstep sounds, hearing) is relevant.
pub fn should_show_audio_section(draft: &EntityPropertyDraft) -> bool {
    let kind = effective_kind_for_category(&draft.category);
    draft.movement_component_present
        || draft.ai_component_present
        || !draft.movement_sound.trim().is_empty()
        || kind_supports_audio(kind)
}

/// Whether the combat/stats section (health, attack power) is relevant.
pub fn should_show_combat_section(draft: &EntityPropertyDraft) -> bool {
    let kind = effective_kind_for_category(&draft.category);
    draft.combat_component_present
        || draft.health_enabled
        || draft.attack_power_enabled
        || kind_supports_combat_defaults(kind)
}

pub fn should_show_pickup_section(draft: &EntityPropertyDraft) -> bool {
    let kind = effective_kind_for_category(&draft.category);
    draft.pickup_present || kind == toki_core::entity::EntityKind::Item
}

impl InspectorSystem {
    pub(in super::super) fn render_entity_property_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        available_palettes: &BTreeMap<String, Palette4>,
        config: Option<&EditorConfig>,
        show_position: bool,
        allow_control_role_edit: bool,
        section_label: &str,
    ) -> bool {
        let mut changed = false;
        let show_movement = should_show_movement_section(draft);
        let show_audio = should_show_audio_section(draft);
        let show_combat = should_show_combat_section(draft);
        let show_pickup = should_show_pickup_section(draft);

        ui.label(section_label);
        ui.separator();

        if show_position {
            changed |= render_position_row(ui, draft);
        }

        changed |= render_size_row(ui, draft);
        changed |= render_render_layer_row(ui, draft);
        render_static_render_row(ui, draft);

        if show_movement {
            changed |= render_speed_row(ui, draft);
        }

        changed |= ui.checkbox(&mut draft.visible, "Visible").changed();
        changed |= ui.checkbox(&mut draft.has_shadow, "Has Shadow").changed();
        changed |= ui.checkbox(&mut draft.has_drop_shadow, "Has Drop Shadow").changed();
        changed |= render_palette_override_row(ui, draft, available_palettes);
        changed |= ui.checkbox(&mut draft.active, "Active").changed();
        changed |= ui.checkbox(&mut draft.solid, "Solid").changed();

        if show_pickup {
            changed |= render_pickup_section(ui, draft);
        }

        changed |= ui
            .checkbox(&mut draft.interactable, "Interactable")
            .changed();

        if draft.interactable {
            changed |= render_interaction_reach_row(ui, draft);
        }

        if show_movement {
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
        if show_position {
            changed |= ui
                .checkbox(
                    &mut draft.persistent_across_saves,
                    "Persistent Across Saves",
                )
                .changed();
        }

        if show_audio {
            ui.separator();
            changed |= render_audio_section(ui, draft, config);
        }

        if show_combat {
            ui.separator();
            changed |= render_stats_section(ui, draft);
        }

        ui.separator();
        changed |= render_collision_section(ui, draft);

        changed
    }

    pub(in super::super) fn render_scene_entity_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        available_palettes: &BTreeMap<String, Palette4>,
        config: Option<&EditorConfig>,
    ) -> bool {
        Self::render_entity_property_editor(
            ui,
            draft,
            available_palettes,
            config,
            true,
            true,
            "Scene Entity Properties",
        )
    }

    pub(in super::super) fn render_scene_entity_decoration_editor(
        ui: &mut egui::Ui,
        ui_state: &mut EditorUI,
        draft: &mut EntityPropertyDraft,
        config: Option<&EditorConfig>,
    ) -> bool {
        let Some(selected_sheet_name) = draft.static_object_sheet.clone() else {
            return false;
        };
        let Some(project_path) = config.and_then(EditorConfig::current_project_path) else {
            ui.separator();
            ui.label("Open a project to edit decoration object sheets.");
            return false;
        };
        let Some(source) =
            resolve_object_sheet_browser_source(project_path, Some(selected_sheet_name.as_str()))
        else {
            ui.separator();
            ui.label("No object sheets found in assets/sprites.");
            return false;
        };

        let mut changed = false;
        let mut selected_sheet = draft
            .static_object_sheet
            .clone()
            .unwrap_or_else(|| source.selected_sheet_name.clone());
        let mut selected_object = draft
            .static_object_name
            .clone()
            .or_else(|| source.object_names.first().cloned())
            .unwrap_or_default();

        let mut normalized_sheet = draft.static_object_sheet.clone();
        let mut normalized_object = draft.static_object_name.clone();
        sync_selected_sheet_name(&mut normalized_sheet, &source.sheet_names);
        sync_selected_object_name(&mut normalized_object, &source.object_names);
        if normalized_sheet != draft.static_object_sheet {
            draft.static_object_sheet = normalized_sheet.clone();
            changed = true;
        }
        if normalized_object != draft.static_object_name {
            draft.static_object_name = normalized_object.clone();
            changed = true;
        }
        if let Some(normalized_sheet) = normalized_sheet {
            selected_sheet = normalized_sheet;
        }
        if let Some(normalized_object) = normalized_object {
            selected_object = normalized_object;
        }

        let active_source =
            resolve_object_sheet_browser_source(project_path, Some(selected_sheet.as_str()))
                .unwrap_or(source);
        if !active_source
            .object_names
            .iter()
            .any(|name| name == &selected_object)
        {
            selected_object = active_source
                .object_names
                .first()
                .cloned()
                .unwrap_or_default();
            changed = true;
        }

        ui.separator();
        ui.label("Decoration");
        ui.horizontal(|ui| {
            ui.label("Object Sheet:");
            egui::ComboBox::from_id_salt("scene_entity_static_object_sheet")
                .selected_text(selected_sheet.as_str())
                .show_ui(ui, |ui| {
                    for sheet_name in &active_source.sheet_names {
                        changed |= ui
                            .selectable_value(
                                &mut selected_sheet,
                                sheet_name.clone(),
                                sheet_name.as_str(),
                            )
                            .changed();
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Object:");
            egui::ComboBox::from_id_salt("scene_entity_static_object_name")
                .selected_text(selected_object.as_str())
                .show_ui(ui, |ui| {
                    for object_name in &active_source.object_names {
                        changed |= ui
                            .selectable_value(
                                &mut selected_object,
                                object_name.clone(),
                                object_name.as_str(),
                            )
                            .changed();
                    }
                });
        });

        if changed {
            draft.static_object_sheet = Some(selected_sheet.clone());
            draft.static_object_name = Some(selected_object.clone());

            if let Some(placement_draft) =
                build_decoration_placement_draft(project_path, &selected_sheet, &selected_object)
            {
                draft.size_x = placement_draft.size_px.x as i64;
                draft.size_y = placement_draft.size_px.y as i64;
            }

            let toolbox =
                &mut crate::ui::editor_context::scene_viewport_context_mut(ui_state).toolbox;
            toolbox.selected_object_sheet = Some(selected_sheet.clone());
            toolbox.selected_object_name = Some(selected_object.clone());
        }

        let texture = {
            let toolbox =
                &mut crate::ui::editor_context::scene_viewport_context_mut(ui_state).toolbox;
            ensure_object_sheet_preview_texture(
                &mut toolbox.preview_image_path,
                &mut toolbox.preview_texture,
                ui.ctx(),
                &active_source.texture_path,
            )
        };
        if let Some(texture) = texture {
            if let Some(texture_size) = active_source.object_sheet.image_size() {
                ui.horizontal(|ui| {
                    render_object_gallery_item(
                        ui,
                        texture.id(),
                        texture_size,
                        &active_source.object_sheet,
                        &selected_object,
                        true,
                        72.0,
                    );
                    ui.label(selected_object.as_str());
                });
            }
        }

        changed
    }

    pub(in super::super) fn render_entity_definition_property_editor(
        ui: &mut egui::Ui,
        draft: &mut EntityPropertyDraft,
        available_palettes: &BTreeMap<String, Palette4>,
        config: Option<&EditorConfig>,
    ) -> bool {
        Self::render_entity_property_editor(
            ui,
            draft,
            available_palettes,
            config,
            false,
            false,
            "Entity Properties",
        )
    }
}

fn render_palette_override_row(
    ui: &mut egui::Ui,
    draft: &mut EntityPropertyDraft,
    available_palettes: &BTreeMap<String, Palette4>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Palette Override:");
        egui::ComboBox::from_id_salt("entity_palette_override")
            .selected_text(if draft.palette_override.trim().is_empty() {
                "None"
            } else {
                draft.palette_override.as_str()
            })
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(&mut draft.palette_override, String::new(), "None")
                    .changed();
                for palette_id in available_palettes.keys() {
                    changed |= ui
                        .selectable_value(
                            &mut draft.palette_override,
                            palette_id.clone(),
                            palette_id,
                        )
                        .changed();
                }
            });
    });
    ui.label("Optional. Ignored for true-color atlases.");
    changed
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

fn render_pickup_section(ui: &mut egui::Ui, draft: &mut EntityPropertyDraft) -> bool {
    let mut changed = false;
    let pickup_toggled = ui.checkbox(&mut draft.pickup_present, "Pickup").changed();
    changed |= pickup_toggled;
    if pickup_toggled {
        let kind = effective_kind_for_category(&draft.category);
        let defaults = default_collision_for_kind(
            kind,
            [draft.size_x as u32, draft.size_y as u32],
            draft.pickup_present,
        );
        draft.collision.enabled = defaults.enabled;
        draft.collision.trigger = defaults.trigger;
        draft.collision.offset_x = defaults.offset[0];
        draft.collision.offset_y = defaults.offset[1];
        draft.collision.size_x = defaults.size[0] as i64;
        draft.collision.size_y = defaults.size[1] as i64;
        changed = true;
    }
    if draft.pickup_present {
        ui.horizontal(|ui| {
            ui.label("Pickup Item:");
            changed |= ui.text_edit_singleline(&mut draft.pickup_item_id).changed();
        });
        ui.horizontal(|ui| {
            ui.label("Pickup Count:");
            let mut count = draft.pickup_count as i64;
            if ui
                .add(egui::DragValue::new(&mut count).range(1..=9999))
                .changed()
            {
                draft.pickup_count = count.max(1) as u32;
                changed = true;
            }
        });
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::inspector::{CollisionDraft, EntityPropertyDraft};
    use toki_core::entity::{AiConfig, ControlRole, MovementProfile, MovementSoundTrigger};

    fn draft_with_category(category: &str) -> EntityPropertyDraft {
        EntityPropertyDraft {
            category: category.to_string(),
            static_object_sheet: None,
            static_object_name: None,
            control_role: ControlRole::None,
            position_x: 0,
            position_y: 0,
            size_x: 16,
            size_y: 16,
            visible: true,
            has_shadow: true,
            has_drop_shadow: false,
            palette_override: String::new(),
            active: true,
            solid: false,
            interactable: false,
            interaction_reach: 0,
            pickup_present: false,
            pickup_item_id: String::new(),
            pickup_count: 1,
            movement_component_present: false,
            can_move: false,
            ai_component_present: false,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::LegacyDefault,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound: String::new(),
            has_inventory: false,
            speed: 0.0,
            render_layer: 0,
            persistent_across_saves: false,
            combat_component_present: false,
            health_enabled: false,
            health_value: 0,
            attack_power_enabled: false,
            attack_power_value: 0,
            collision: CollisionDraft {
                enabled: false,
                offset_x: 0,
                offset_y: 0,
                size_x: 16,
                size_y: 16,
                trigger: false,
            },
        }
    }

    #[test]
    fn movement_section_visible_for_player_and_npc() {
        assert!(should_show_movement_section(&draft_with_category("player")));
        assert!(should_show_movement_section(&draft_with_category(
            "creature"
        )));
    }

    #[test]
    fn movement_section_stays_visible_for_promoted_decoration() {
        let mut draft = draft_with_category("decoration");
        draft.movement_component_present = true;
        assert!(should_show_movement_section(&draft));
    }

    #[test]
    fn movement_section_hidden_for_plain_passive_decoration() {
        assert!(!should_show_movement_section(&draft_with_category(
            "decoration"
        )));
    }

    #[test]
    fn audio_section_visible_for_promoted_decoration_with_movement() {
        let mut draft = draft_with_category("decoration");
        draft.movement_component_present = true;
        assert!(should_show_audio_section(&draft));
    }

    #[test]
    fn audio_section_hidden_for_plain_passive_decoration() {
        assert!(!should_show_audio_section(&draft_with_category(
            "decoration"
        )));
    }

    #[test]
    fn combat_section_visible_for_promoted_decorations() {
        let mut draft = draft_with_category("decoration");
        draft.combat_component_present = true;
        assert!(should_show_combat_section(&draft));
    }

    #[test]
    fn combat_section_hidden_for_plain_non_combat_decoration() {
        assert!(!should_show_combat_section(&draft_with_category(
            "decoration"
        )));
    }

    #[test]
    fn combat_section_hidden_for_plain_items_without_combat_component() {
        assert!(!should_show_combat_section(&draft_with_category("item")));
    }

    #[test]
    fn pickup_section_visible_for_items_and_hidden_for_plain_decorations() {
        assert!(should_show_pickup_section(&draft_with_category("item")));
        assert!(!should_show_pickup_section(&draft_with_category(
            "decoration"
        )));
    }
}
