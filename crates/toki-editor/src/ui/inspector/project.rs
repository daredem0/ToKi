use super::*;
use crate::project::apply_project_settings_draft;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use toki_core::palette::{save_palette_asset_to_path, Palette4};
use toki_core::project_assets::load_project_palettes;

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
        let mut palette_files_changed = false;

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
            changed |= ui
                .checkbox(&mut draft.show_ground_shadows, "Show Ground Shadows")
                .changed();

            ui.separator();
            ui.label("Post Process");
            ui.horizontal(|ui| {
                ui.label("Mode:");
                let current_label = match draft.post_process_mode {
                    toki_core::project_runtime::PostProcessMode::None => "None",
                    toki_core::project_runtime::PostProcessMode::Tint => "Tint",
                    toki_core::project_runtime::PostProcessMode::Quantize4 => "Quantize 4",
                    toki_core::project_runtime::PostProcessMode::GbPalette => "GB Preset",
                };
                egui::ComboBox::from_id_salt("project_post_process_mode")
                    .selected_text(current_label)
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut draft.post_process_mode,
                                toki_core::project_runtime::PostProcessMode::None,
                                "None",
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut draft.post_process_mode,
                                toki_core::project_runtime::PostProcessMode::Tint,
                                "Tint",
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut draft.post_process_mode,
                                toki_core::project_runtime::PostProcessMode::Quantize4,
                                "Quantize 4",
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut draft.post_process_mode,
                                toki_core::project_runtime::PostProcessMode::GbPalette,
                                "GB Preset",
                            )
                            .changed();
                    });
            });

            ui.add_enabled_ui(
                matches!(
                    draft.post_process_mode,
                    toki_core::project_runtime::PostProcessMode::Quantize4
                        | toki_core::project_runtime::PostProcessMode::GbPalette
                ),
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Quantize Strategy:");
                        let current_label = match draft.post_process_quantize_strategy {
                            toki_core::project_runtime::QuantizeStrategy::Luminance => "Luminance",
                            toki_core::project_runtime::QuantizeStrategy::RgbDistance => {
                                "RGB Distance"
                            }
                        };
                        egui::ComboBox::from_id_salt("project_quantize_strategy")
                            .selected_text(current_label)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(
                                        &mut draft.post_process_quantize_strategy,
                                        toki_core::project_runtime::QuantizeStrategy::Luminance,
                                        "Luminance",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        &mut draft.post_process_quantize_strategy,
                                        toki_core::project_runtime::QuantizeStrategy::RgbDistance,
                                        "RGB Distance",
                                    )
                                    .changed();
                            });
                    });
                },
            );

            ui.horizontal(|ui| {
                ui.label("Tint Color:");
                let mut color32 = egui::Color32::from_rgba_unmultiplied(
                    draft.post_process_tint_color[0],
                    draft.post_process_tint_color[1],
                    draft.post_process_tint_color[2],
                    draft.post_process_tint_color[3],
                );
                if ui.color_edit_button_srgba(&mut color32).changed() {
                    draft.post_process_tint_color =
                        [color32.r(), color32.g(), color32.b(), color32.a()];
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Tint Strength:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.post_process_tint_strength_percent, 0..=100)
                            .suffix("%"),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                let mut palette_ids = ui_state
                    .project
                    .available_palettes
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>();
                palette_ids.sort();
                ui.label("Quantize Palette:");
                egui::ComboBox::from_id_salt("project_quantize_palette")
                    .selected_text(draft.post_process_quantize_palette_id.clone())
                    .show_ui(ui, |ui| {
                        for palette_id in &palette_ids {
                            changed |= ui
                                .selectable_value(
                                    &mut draft.post_process_quantize_palette_id,
                                    palette_id.clone(),
                                    palette_id,
                                )
                                .changed();
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("GB Contrast:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.post_process_gb_contrast_percent, -100..=100)
                            .suffix("%"),
                    )
                    .changed();
            });

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
        ui.collapsing("Palettes", |ui| {
            let mut palette_ids = ui_state
                .project
                .available_palettes
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            palette_ids.sort();

            ui.horizontal(|ui| {
                ui.label("Global Indexed Override:");
                egui::ComboBox::from_id_salt("project_indexed_palette_override")
                    .selected_text(
                        draft
                            .indexed_palette_override
                            .as_deref()
                            .unwrap_or("Atlas Default"),
                    )
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut draft.indexed_palette_override,
                                None,
                                "Atlas Default",
                            )
                            .changed();
                        for palette_id in &palette_ids {
                            changed |= ui
                                .selectable_value(
                                    &mut draft.indexed_palette_override,
                                    Some(palette_id.clone()),
                                    palette_id,
                                )
                                .changed();
                        }
                    });
            });

            ui.separator();
            ui.label("Built-in palettes are always available.");
            ui.separator();
            ui.label("Project Palette Files:");
            let mut project_palettes = load_project_palette_files(project);
            let mut remove_palette_id = None;
            let palette_ids = project_palettes.keys().cloned().collect::<Vec<_>>();
            for palette_id in palette_ids {
                let mut palette = project_palettes
                    .get(&palette_id)
                    .copied()
                    .unwrap_or(Palette4::new([[0, 0, 0, 255]; 4]));
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(&palette_id);
                        if ui.button("Remove").clicked() {
                            remove_palette_id = Some(palette_id.clone());
                        }
                    });
                    ui.horizontal(|ui| {
                        for color in &mut palette.colors {
                            let mut color32 = egui::Color32::from_rgba_unmultiplied(
                                color[0], color[1], color[2], color[3],
                            );
                            if ui.color_edit_button_srgba(&mut color32).changed() {
                                *color = [
                                    color32.r(),
                                    color32.g(),
                                    color32.b(),
                                    color32.a(),
                                ];
                                palette_files_changed = true;
                            }
                        }
                    });
                });
                if project_palettes.get(&palette_id).copied() != Some(palette) {
                    match save_project_palette_file(project, &palette_id, palette) {
                        Ok(()) => {
                            project_palettes.insert(palette_id.clone(), palette);
                            palette_files_changed = true;
                        }
                        Err(error) => {
                            tracing::warn!(
                                "Failed to save project palette '{}' in '{}': {}",
                                palette_id,
                                project.path.display(),
                                error
                            );
                        }
                    }
                }
            }
            if let Some(remove_palette_id) = remove_palette_id {
                match remove_project_palette_file(project, &remove_palette_id) {
                    Ok(()) => {
                        if draft.indexed_palette_override.as_deref()
                            == Some(remove_palette_id.as_str())
                        {
                            draft.indexed_palette_override = None;
                            changed = true;
                        }
                        palette_files_changed = true;
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Failed to remove project palette '{}' in '{}': {}",
                            remove_palette_id,
                            project.path.display(),
                            error
                        );
                    }
                }
            }

            if ui.button("Add Project Palette").clicked() {
                let mut index = project_palettes.len() + 1;
                let palette_id = loop {
                    let candidate = format!("custom_palette_{index}");
                    if !ui_state.project.available_palettes.contains_key(&candidate) {
                        break candidate;
                    }
                    index += 1;
                };
                match save_project_palette_file(
                    project,
                    &palette_id,
                    Palette4::new([
                        [0x00, 0x00, 0x00, 0xFF],
                        [0x55, 0x55, 0x55, 0xFF],
                        [0xAA, 0xAA, 0xAA, 0xFF],
                        [0xFF, 0xFF, 0xFF, 0xFF],
                    ]),
                ) {
                    Ok(()) => {
                        palette_files_changed = true;
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Failed to create project palette '{}' in '{}': {}",
                            palette_id,
                            project.path.display(),
                            error
                        );
                    }
                }
            }
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
            ui_state
                .project
                .set_available_palettes(&load_project_palette_files(project));
        } else if palette_files_changed {
            ui_state
                .project
                .set_available_palettes(&load_project_palette_files(project));
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

fn load_project_palette_files(project: &Project) -> BTreeMap<String, Palette4> {
    load_project_palettes(&project.path).unwrap_or_else(|error| {
        tracing::warn!(
            "Failed to load project palettes from '{}': {}",
            project.path.display(),
            error
        );
        BTreeMap::new()
    })
}

fn project_palette_file_path(project: &Project, palette_id: &str) -> PathBuf {
    project
        .path
        .join("palettes")
        .join(format!("{palette_id}.json"))
}

fn save_project_palette_file(
    project: &Project,
    palette_id: &str,
    palette: Palette4,
) -> anyhow::Result<()> {
    let path = project_palette_file_path(project, palette_id);
    save_palette_asset_to_path(&path, palette).map_err(anyhow::Error::from)
}

fn remove_project_palette_file(project: &Project, palette_id: &str) -> anyhow::Result<()> {
    let path = project_palette_file_path(project, palette_id);
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}
