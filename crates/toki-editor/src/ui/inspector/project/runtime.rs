use super::*;
use crate::ui::ui_event_registry::validate_ui_event_registry;

pub(super) fn render_runtime_section(
    ui: &mut egui::Ui,
    project: &Project,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.collapsing("Runtime", |ui| {
        changed |= ui
            .checkbox(
                &mut draft.scene_persistence,
                "Persist Scene State Across Scene Changes",
            )
            .changed();

        ui.separator();
        ui.collapsing("Authored UI Events", |ui| {
            for issue in validate_ui_event_registry(&draft.ui_event_declarations) {
                ui.colored_label(egui::Color32::from_rgb(255, 120, 120), issue);
            }

            let mut remove_index = None;
            for (index, declaration) in draft.ui_event_declarations.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("UI Event {}", index + 1));
                        if ui.small_button("Delete").clicked() {
                            remove_index = Some(index);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Id:");
                        changed |= ui.text_edit_singleline(&mut declaration.id).changed();
                    });
                });
            }
            if let Some(index) = remove_index {
                draft.ui_event_declarations.remove(index);
                changed = true;
            }
            if ui.button("+ Add UI Event").clicked() {
                draft
                    .ui_event_declarations
                    .push(toki_core::project_runtime::ProjectUiEventDefinition {
                        id: String::new(),
                    });
                changed = true;
            }
        });
    });

    ui.collapsing("Audio", |ui| {
        ui.label("Channel loudness is global for the whole project.");
        changed |= render_mix_slider(ui, "Master:", &mut draft.master_mix_percent);
        changed |= render_mix_slider(ui, "Music:", &mut draft.music_mix_percent);
        changed |= render_mix_slider(ui, "Movement:", &mut draft.movement_mix_percent);
        changed |= render_mix_slider(ui, "Collision:", &mut draft.collision_mix_percent);
    });

    ui.collapsing("Asset Paths", |ui| {
        ui.label("These are currently fixed conventions in the editor/runtime.");
        render_asset_path(ui, "Sprites:", &project.metadata.assets.sprites);
        render_asset_path(ui, "Tilemaps:", &project.metadata.assets.tilemaps);
        render_asset_path(ui, "Audio:", &project.metadata.assets.audio);
    });

    changed
}

fn render_mix_slider(ui: &mut egui::Ui, label: &str, value: &mut u8) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed |= ui
            .add(
                egui::Slider::new(value, 0..=100)
                    .suffix("%")
                    .show_value(true),
            )
            .changed();
    });
    changed
}

fn render_asset_path(ui: &mut egui::Ui, label: &str, path: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.monospace(path);
    });
}
