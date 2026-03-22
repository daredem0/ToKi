use super::*;
use crate::project::apply_project_settings_draft;

impl InspectorSystem {
    pub(super) fn render_project_settings_panel(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        project: Option<&mut Project>,
        _config: Option<&EditorConfig>,
    ) {
        let Some(project) = project else {
            ui.heading("Project");
            ui.separator();
            ui.label("No project open.");
            ui.label("Open or create a project to edit project-wide settings.");
            return;
        };

        ui.heading("Project");
        ui.separator();

        let mut draft = ProjectSettingsDraft::from_project(project);
        let mut changed = false;

        ui.collapsing("General", |ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                changed |= ui.text_edit_singleline(&mut draft.name).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Version:");
                changed |= ui.text_edit_singleline(&mut draft.version).changed();
            });
            ui.label("Description:");
            changed |= ui
                .add(
                    egui::TextEdit::multiline(&mut draft.description)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                )
                .changed();
        });

        ui.separator();
        ui.collapsing("Display", |ui| {
            ui.horizontal(|ui| {
                ui.label("Resolution Width:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.resolution_width)
                            .speed(1.0)
                            .range(1..=1920),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Resolution Height:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.resolution_height)
                            .speed(1.0)
                            .range(1..=1080),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Zoom:");
                let zoom_float = draft.zoom_percent as f32 / 100.0;
                let mut zoom_display = zoom_float;
                if ui
                    .add(
                        egui::DragValue::new(&mut zoom_display)
                            .speed(0.1)
                            .range(0.1..=10.0)
                            .suffix("x"),
                    )
                    .changed()
                {
                    draft.zoom_percent = (zoom_display * 100.0).round() as u32;
                    changed = true;
                }
            });
            changed |= ui
                .checkbox(
                    &mut draft.show_entity_health_bars,
                    "Show Entity Health Bars",
                )
                .changed();

            ui.separator();
            ui.label("Frame Rate");
            changed |= ui.checkbox(&mut draft.vsync, "VSync").changed();

            ui.add_enabled_ui(!draft.vsync, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Target FPS:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut draft.target_fps)
                                .speed(1.0)
                                .range(0..=240),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
                ui.label("Set to 0 for unlimited frame rate.");
            });

            ui.separator();
            ui.label("Game Logic Timing");
            ui.horizontal(|ui| {
                ui.label("Timing Mode:");
                let current_label = match draft.timing_mode {
                    toki_core::TimingMode::Fixed => "Fixed (60 FPS)",
                    toki_core::TimingMode::Delta => "Delta",
                };
                egui::ComboBox::from_id_salt("timing_mode")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut draft.timing_mode,
                                toki_core::TimingMode::Fixed,
                                "Fixed (60 FPS)",
                            )
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .selectable_value(
                                &mut draft.timing_mode,
                                toki_core::TimingMode::Delta,
                                "Delta",
                            )
                            .changed()
                        {
                            changed = true;
                        }
                    });
            });
            ui.label("Fixed: Deterministic, 60 ticks/sec. Delta: Scales with frame time.");
        });

        ui.separator();
        ui.collapsing("Runtime", |ui| {
            ui.horizontal(|ui| {
                ui.label("Splash Duration (ms):");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.splash_duration_ms)
                            .speed(25.0)
                            .range(0..=u64::MAX),
                    )
                    .changed();
            });
        });

        ui.separator();
        ui.collapsing("Audio", |ui| {
            ui.label("Channel loudness is global for the whole project.");
            ui.horizontal(|ui| {
                ui.label("Master:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.master_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Music:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.music_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Movement:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.movement_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Collision:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.collision_mix_percent, 0..=100)
                            .suffix("%")
                            .show_value(true),
                    )
                    .changed();
            });
        });

        ui.separator();
        ui.collapsing("Asset Paths", |ui| {
            ui.label("These are currently fixed conventions in the editor/runtime.");
            ui.horizontal(|ui| {
                ui.label("Sprites:");
                ui.monospace(&project.metadata.assets.sprites);
            });
            ui.horizontal(|ui| {
                ui.label("Tilemaps:");
                ui.monospace(&project.metadata.assets.tilemaps);
            });
            ui.horizontal(|ui| {
                ui.label("Audio:");
                ui.monospace(&project.metadata.assets.audio);
            });
        });

        ui.separator();
        ui.collapsing("Metadata", |ui| {
            ui.horizontal(|ui| {
                ui.label("Created:");
                ui.monospace(project.metadata.project.created.to_rfc3339());
            });
            ui.horizontal(|ui| {
                ui.label("Modified:");
                ui.monospace(project.metadata.project.modified.to_rfc3339());
            });
            ui.horizontal(|ui| {
                ui.label("Current Editor Version:");
                ui.monospace(env!("TOKI_VERSION"));
            });
            ui.horizontal(|ui| {
                ui.label("Project Created With:");
                ui.monospace(&project.metadata.project.toki_editor_version);
            });
        });

        if changed && apply_project_settings_draft(project, &draft) {
            ui_state.set_title(&project.name);
        }
    }

    #[cfg(test)]
    pub(super) fn apply_project_settings_draft(
        project: &mut Project,
        draft: &ProjectSettingsDraft,
    ) -> bool {
        apply_project_settings_draft(project, draft)
    }
}
