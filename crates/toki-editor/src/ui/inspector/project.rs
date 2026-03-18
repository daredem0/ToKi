use super::*;

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

        if changed && Self::apply_project_settings_draft(project, &draft) {
            ui_state.set_title(&project.name);
        }
    }

    pub(super) fn apply_project_settings_draft(
        project: &mut Project,
        draft: &ProjectSettingsDraft,
    ) -> bool {
        let trimmed_name = draft.name.trim();
        let trimmed_version = draft.version.trim();

        let mut changed = false;
        if !trimmed_name.is_empty() && project.metadata.project.name != trimmed_name {
            project.metadata.project.name = trimmed_name.to_string();
            project.name = trimmed_name.to_string();
            changed = true;
        }
        if !trimmed_version.is_empty() && project.metadata.project.version != trimmed_version {
            project.metadata.project.version = trimmed_version.to_string();
            changed = true;
        }
        if project.metadata.project.description != draft.description {
            project.metadata.project.description = draft.description.clone();
            changed = true;
        }
        if project.metadata.runtime.splash.duration_ms != draft.splash_duration_ms {
            project.metadata.runtime.splash.duration_ms = draft.splash_duration_ms;
            changed = true;
        }
        if project.metadata.runtime.display.show_entity_health_bars != draft.show_entity_health_bars
        {
            project.metadata.runtime.display.show_entity_health_bars =
                draft.show_entity_health_bars;
            changed = true;
        }
        if project.metadata.runtime.display.resolution_width != draft.resolution_width {
            project.metadata.runtime.display.resolution_width = draft.resolution_width;
            changed = true;
        }
        if project.metadata.runtime.display.resolution_height != draft.resolution_height {
            project.metadata.runtime.display.resolution_height = draft.resolution_height;
            changed = true;
        }
        if project.metadata.runtime.display.zoom_percent != draft.zoom_percent {
            project.metadata.runtime.display.zoom_percent = draft.zoom_percent;
            changed = true;
        }
        if project.metadata.runtime.display.vsync != draft.vsync {
            project.metadata.runtime.display.vsync = draft.vsync;
            changed = true;
        }
        if project.metadata.runtime.display.target_fps != draft.target_fps {
            project.metadata.runtime.display.target_fps = draft.target_fps;
            changed = true;
        }
        if project.audio_config().master_percent != draft.master_mix_percent {
            project.audio_config_mut().master_percent = draft.master_mix_percent;
            changed = true;
        }
        if project.audio_config().music_percent != draft.music_mix_percent {
            project.audio_config_mut().music_percent = draft.music_mix_percent;
            changed = true;
        }
        if project.audio_config().movement_percent != draft.movement_mix_percent {
            project.audio_config_mut().movement_percent = draft.movement_mix_percent;
            changed = true;
        }
        if project.audio_config().collision_percent != draft.collision_mix_percent {
            project.audio_config_mut().collision_percent = draft.collision_mix_percent;
            changed = true;
        }

        if changed {
            project.metadata.project.modified = Utc::now();
            project.is_dirty = true;
        }

        changed
    }
}
