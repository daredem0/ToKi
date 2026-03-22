//! Component section rendering for entity editor.

use crate::ui::EditorUI;
use toki_core::entity::AiBehavior;

use super::widgets::{render_sfx_dropdown, show_field_error};

pub fn render_component_toggles(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.heading("Components");
    ui.separator();

    super::components_core::render_rendering_section(ui, ui_state);
    super::components_core::render_attributes_section(ui, ui_state);
    render_collision_section(ui, ui_state);
    render_health_section(ui, ui_state);
    render_ai_section(ui, ui_state);
    render_inventory_section(ui, ui_state);
    render_projectile_section(ui, ui_state);
    render_pickup_section(ui, ui_state);
    render_audio_section(ui, ui_state);
}

fn render_collision_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut edit.toggles.collision_enabled, "Collision")
            .changed()
        {
            edit.definition.collision.enabled = edit.toggles.collision_enabled;
            edit.mark_dirty();
        }
    });

    if edit.toggles.collision_enabled {
        egui::CollapsingHeader::new("  Collision Settings")
            .default_open(false)
            .show(ui, |ui| {
                render_collision_settings(ui, edit);
            });
    }
}

fn render_collision_settings(
    ui: &mut egui::Ui,
    edit: &mut crate::ui::editor_ui::EntityEditState,
) {
    // Offset
    ui.horizontal(|ui| {
        ui.label("Offset:");
        if ui
            .add(egui::DragValue::new(
                &mut edit.definition.collision.offset[0],
            ))
            .changed()
        {
            edit.mark_dirty();
        }
        ui.label(",");
        if ui
            .add(egui::DragValue::new(
                &mut edit.definition.collision.offset[1],
            ))
            .changed()
        {
            edit.mark_dirty();
        }
    });

    // Size
    ui.horizontal(|ui| {
        ui.label("Size:");
        let mut w = edit.definition.collision.size[0] as i32;
        let mut h = edit.definition.collision.size[1] as i32;
        if ui
            .add(egui::DragValue::new(&mut w).range(1..=1024))
            .changed()
        {
            edit.definition.collision.size[0] = w.max(1) as u32;
            edit.mark_dirty();
        }
        ui.label("x");
        if ui
            .add(egui::DragValue::new(&mut h).range(1..=1024))
            .changed()
        {
            edit.definition.collision.size[1] = h.max(1) as u32;
            edit.mark_dirty();
        }
    });
    show_field_error(ui, edit, "collision_size");

    // Trigger
    if ui
        .checkbox(&mut edit.definition.collision.trigger, "Is Trigger")
        .changed()
    {
        edit.mark_dirty();
    }
}

fn render_health_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.health_enabled;
    if ui.checkbox(&mut toggle, "Health").changed() {
        edit.toggle_health();
    }

    if edit.toggles.health_enabled {
        ui.horizontal(|ui| {
            ui.label("  Max HP:");
            let mut hp = edit.definition.attributes.health.unwrap_or(100) as i32;
            if ui
                .add(egui::DragValue::new(&mut hp).range(1..=99999))
                .changed()
            {
                edit.definition.attributes.health = Some(hp.max(1) as u32);
                edit.mark_dirty();
            }
        });
        show_field_error(ui, edit, "health");
    }
}

fn render_ai_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.ai_enabled;
    if ui.checkbox(&mut toggle, "AI").changed() {
        edit.toggle_ai();
    }

    if edit.toggles.ai_enabled {
        egui::CollapsingHeader::new("  AI Settings")
            .default_open(false)
            .show(ui, |ui| {
                render_ai_settings(ui, edit);
            });
    }
}

fn render_ai_settings(ui: &mut egui::Ui, edit: &mut crate::ui::editor_ui::EntityEditState) {
    // Behavior dropdown
    ui.horizontal(|ui| {
        ui.label("Behavior:");
        let current = format!("{:?}", edit.definition.attributes.ai_config.behavior);
        egui::ComboBox::from_id_salt("ai_behavior")
            .selected_text(&current)
            .show_ui(ui, |ui| {
                for behavior in [
                    AiBehavior::Wander,
                    AiBehavior::Chase,
                    AiBehavior::Run,
                    AiBehavior::RunAndMultiply,
                ] {
                    let label = format!("{:?}", behavior);
                    if ui
                        .selectable_value(
                            &mut edit.definition.attributes.ai_config.behavior,
                            behavior,
                            &label,
                        )
                        .changed()
                    {
                        edit.mark_dirty();
                    }
                }
            });
    });

    // Detection radius
    ui.horizontal(|ui| {
        ui.label("Detection Radius:");
        let mut radius = edit.definition.attributes.ai_config.detection_radius as i32;
        if ui
            .add(egui::DragValue::new(&mut radius).range(0..=1024))
            .changed()
        {
            edit.definition.attributes.ai_config.detection_radius = radius.max(0) as u32;
            edit.mark_dirty();
        }
    });
}

fn render_inventory_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.inventory_enabled;
    if ui.checkbox(&mut toggle, "Inventory").changed() {
        edit.toggle_inventory();
    }
}

fn render_projectile_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.projectile_enabled;
    if ui.checkbox(&mut toggle, "Projectile").changed() {
        edit.toggle_projectile();
    }

    if edit.toggles.projectile_enabled {
        if let Some(proj) = edit.definition.attributes.primary_projectile.as_mut() {
            egui::CollapsingHeader::new("  Projectile Settings")
                .default_open(false)
                .show(ui, |ui| {
                    render_projectile_settings(ui, proj, &mut edit.dirty);
                });
        }
    }
}

fn render_projectile_settings(
    ui: &mut egui::Ui,
    proj: &mut toki_core::entity::PrimaryProjectileDef,
    dirty: &mut bool,
) {
    // Sheet
    ui.horizontal(|ui| {
        ui.label("Sheet:");
        if ui.text_edit_singleline(&mut proj.sheet).changed() {
            *dirty = true;
        }
    });

    // Object name
    ui.horizontal(|ui| {
        ui.label("Object:");
        if ui.text_edit_singleline(&mut proj.object_name).changed() {
            *dirty = true;
        }
    });

    // Size
    ui.horizontal(|ui| {
        ui.label("Size:");
        let mut w = proj.size[0] as i32;
        let mut h = proj.size[1] as i32;
        if ui
            .add(egui::DragValue::new(&mut w).range(1..=256))
            .changed()
        {
            proj.size[0] = w.max(1) as u32;
            *dirty = true;
        }
        ui.label("x");
        if ui
            .add(egui::DragValue::new(&mut h).range(1..=256))
            .changed()
        {
            proj.size[1] = h.max(1) as u32;
            *dirty = true;
        }
    });

    // Speed
    ui.horizontal(|ui| {
        ui.label("Speed:");
        let mut speed = proj.speed as i32;
        if ui
            .add(egui::DragValue::new(&mut speed).range(1..=9999))
            .changed()
        {
            proj.speed = speed.max(1) as u32;
            *dirty = true;
        }
    });

    // Damage
    ui.horizontal(|ui| {
        ui.label("Damage:");
        if ui.add(egui::DragValue::new(&mut proj.damage)).changed() {
            *dirty = true;
        }
    });

    // Lifetime
    ui.horizontal(|ui| {
        ui.label("Lifetime (ticks):");
        let mut lifetime = proj.lifetime_ticks as i32;
        if ui
            .add(egui::DragValue::new(&mut lifetime).range(1..=9999))
            .changed()
        {
            proj.lifetime_ticks = lifetime.max(1) as u32;
            *dirty = true;
        }
    });
}

fn render_pickup_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.pickup_enabled;
    if ui.checkbox(&mut toggle, "Pickup").changed() {
        edit.toggle_pickup();
    }

    if edit.toggles.pickup_enabled {
        if let Some(pickup) = edit.definition.attributes.pickup.as_mut() {
            egui::CollapsingHeader::new("  Pickup Settings")
                .default_open(false)
                .show(ui, |ui| {
                    render_pickup_settings(ui, pickup, &mut edit.dirty);
                });
        }
    }
}

fn render_pickup_settings(
    ui: &mut egui::Ui,
    pickup: &mut toki_core::entity::PickupDef,
    dirty: &mut bool,
) {
    // Item ID
    ui.horizontal(|ui| {
        ui.label("Item ID:");
        if ui.text_edit_singleline(&mut pickup.item_id).changed() {
            *dirty = true;
        }
    });

    // Count
    ui.horizontal(|ui| {
        ui.label("Count:");
        let mut count = pickup.count as i32;
        if ui
            .add(egui::DragValue::new(&mut count).range(1..=9999))
            .changed()
        {
            pickup.count = count.max(1) as u32;
            *dirty = true;
        }
    });
}

fn render_audio_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let available_sfx = ui_state.entity_editor.available_sfx.clone();
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.audio_enabled;
    if ui.checkbox(&mut toggle, "Audio").changed() {
        edit.toggle_audio();
    }

    if edit.toggles.audio_enabled {
        egui::CollapsingHeader::new("  Audio Settings")
            .default_open(false)
            .show(ui, |ui| {
                render_audio_settings(ui, edit, &available_sfx);
            });
    }
}

fn render_audio_settings(
    ui: &mut egui::Ui,
    edit: &mut crate::ui::editor_ui::EntityEditState,
    available_sfx: &[String],
) {
    // Movement sound - dropdown from discovered SFX
    ui.horizontal(|ui| {
        ui.label("Movement Sound:");
        render_sfx_dropdown(
            ui,
            "movement_sound",
            &mut edit.definition.audio.movement_sound,
            available_sfx,
            &mut edit.dirty,
        );
    });

    // Collision sound - dropdown from discovered SFX
    ui.horizontal(|ui| {
        ui.label("Collision Sound:");
        let mut sound = edit
            .definition
            .audio
            .collision_sound
            .clone()
            .unwrap_or_default();
        if render_sfx_dropdown(ui, "collision_sound", &mut sound, available_sfx, &mut edit.dirty) {
            edit.definition.audio.collision_sound = if sound.is_empty() { None } else { Some(sound) };
        }
    });

    // Hearing radius
    ui.horizontal(|ui| {
        ui.label("Hearing Radius:");
        let mut radius = edit.definition.audio.hearing_radius as i32;
        if ui
            .add(egui::DragValue::new(&mut radius).range(0..=1024))
            .changed()
        {
            edit.definition.audio.hearing_radius = radius.max(0) as u32;
            edit.mark_dirty();
        }
    });

    // Footstep distance
    ui.horizontal(|ui| {
        ui.label("Footstep Distance:");
        if ui
            .add(
                egui::DragValue::new(&mut edit.definition.audio.footstep_trigger_distance)
                    .speed(0.1),
            )
            .changed()
        {
            edit.mark_dirty();
        }
    });
}
