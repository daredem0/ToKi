use super::*;

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
