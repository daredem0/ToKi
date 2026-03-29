use super::*;

pub(super) fn render_display_section(
    ui_state: &mut EditorUI,
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.collapsing("Display", |ui| {
        for issue in validate_project_settings_draft(draft) {
            ui.colored_label(egui::Color32::YELLOW, issue.message);
        }

        changed |= render_resolution_controls(ui, draft);
        changed |= render_viewport_controls(ui, draft);
        changed |= render_world_rendering_controls(ui, draft);
        ui.separator();
        changed |= render_post_process_controls(ui_state, ui, draft);
        ui.separator();
        changed |= render_frame_rate_controls(ui, draft);
        ui.separator();
        changed |= render_timing_controls(ui, draft);
    });
    changed
}

fn render_resolution_controls(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
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
        let mut zoom_display = draft.zoom_percent as f32 / 100.0;
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
    changed
}

fn render_viewport_controls(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Viewport Mode:");
        egui::ComboBox::from_id_salt("project_runtime_viewport_mode")
            .selected_text(draft.viewport_mode.label())
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(
                        &mut draft.viewport_mode,
                        ProjectViewportModeDraft::AspectFit,
                        ProjectViewportModeDraft::AspectFit.label(),
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.viewport_mode,
                        ProjectViewportModeDraft::IntegerScale,
                        ProjectViewportModeDraft::IntegerScale.label(),
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.viewport_mode,
                        ProjectViewportModeDraft::WindowFill,
                        ProjectViewportModeDraft::WindowFill.label(),
                    )
                    .changed();
            });
    });

    match draft.viewport_mode {
        ProjectViewportModeDraft::AspectFit => {
            ui.horizontal(|ui| {
                ui.label("Fit Percent:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.viewport_aspect_fit_percent)
                            .speed(1.0)
                            .range(0..=500),
                    )
                    .changed();
                ui.label("%");
            });
        }
        ProjectViewportModeDraft::IntegerScale => {
            ui.horizontal(|ui| {
                ui.label("Scale Factor:");
                let mut use_auto = matches!(
                    draft.viewport_integer_scale_factor,
                    toki_core::project_runtime::IntegerScaleFactor::Auto
                );
                if ui.checkbox(&mut use_auto, "Auto").changed() {
                    draft.viewport_integer_scale_factor = if use_auto {
                        toki_core::project_runtime::IntegerScaleFactor::Auto
                    } else {
                        toki_core::project_runtime::IntegerScaleFactor::Fixed(1)
                    };
                    changed = true;
                }
                if let toki_core::project_runtime::IntegerScaleFactor::Fixed(value) =
                    &mut draft.viewport_integer_scale_factor
                {
                    changed |= ui
                        .add(
                            egui::DragValue::new(value)
                                .speed(1.0)
                                .range(0..=32)
                                .suffix("x"),
                        )
                        .changed();
                }
            });
        }
        ProjectViewportModeDraft::WindowFill => {
            ui.colored_label(
                egui::Color32::YELLOW,
                "Changes visible world area with window size.",
            );
            ui.horizontal(|ui| {
                ui.label("Zoom Percent:");
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut draft.viewport_window_fill_zoom_percent)
                            .speed(1.0)
                            .range(0..=400),
                    )
                    .changed();
                ui.label("%");
            });
        }
    }

    changed
}

fn render_world_rendering_controls(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    changed |= ui
        .checkbox(
            &mut draft.show_entity_health_bars,
            "Show Entity Health Bars",
        )
        .changed();
    changed |= ui
        .checkbox(&mut draft.show_ground_shadows, "Show Ground Shadows")
        .changed();
    changed
}

fn render_post_process_controls(
    ui_state: &mut EditorUI,
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.label("Post Process");
    ui.horizontal(|ui| {
        ui.label("Mode:");
        let current_label = match draft.post_process_mode {
            toki_core::project_runtime::PostProcessMode::None => "None",
            toki_core::project_runtime::PostProcessMode::Tint => "Tint",
            toki_core::project_runtime::PostProcessMode::BrightnessSaturation => {
                "Brightness + Saturation"
            }
            toki_core::project_runtime::PostProcessMode::Quantize4 => "Quantize 4",
            toki_core::project_runtime::PostProcessMode::OrderedDitherQuantize => {
                "Ordered Dither Quantize"
            }
            toki_core::project_runtime::PostProcessMode::GbPalette => "GB Preset",
            toki_core::project_runtime::PostProcessMode::Vignette => "Vignette",
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
                        toki_core::project_runtime::PostProcessMode::BrightnessSaturation,
                        "Brightness + Saturation",
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
                        toki_core::project_runtime::PostProcessMode::OrderedDitherQuantize,
                        "Ordered Dither Quantize",
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.post_process_mode,
                        toki_core::project_runtime::PostProcessMode::GbPalette,
                        "GB Preset",
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.post_process_mode,
                        toki_core::project_runtime::PostProcessMode::Vignette,
                        "Vignette",
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
                    toki_core::project_runtime::QuantizeStrategy::RgbDistance => "RGB Distance",
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

    ui.add_enabled_ui(
        matches!(
            draft.post_process_mode,
            toki_core::project_runtime::PostProcessMode::Tint
        ),
        |ui| {
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
        },
    );

    ui.add_enabled_ui(
        matches!(
            draft.post_process_mode,
            toki_core::project_runtime::PostProcessMode::BrightnessSaturation
        ),
        |ui| {
            ui.horizontal(|ui| {
                ui.label("Brightness:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.post_process_brightness_percent, -100..=100)
                            .suffix("%"),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Saturation:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.post_process_saturation_percent, 0..=200)
                            .suffix("%"),
                    )
                    .changed();
            });
        },
    );

    ui.add_enabled_ui(
        matches!(
            draft.post_process_mode,
            toki_core::project_runtime::PostProcessMode::Quantize4
                | toki_core::project_runtime::PostProcessMode::OrderedDitherQuantize
        ),
        |ui| {
            let mut palette_ids = ui_state
                .project
                .available_palettes
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            palette_ids.sort();
            ui.horizontal(|ui| {
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
        },
    );

    ui.add_enabled_ui(
        matches!(
            draft.post_process_mode,
            toki_core::project_runtime::PostProcessMode::GbPalette
        ),
        |ui| {
            ui.horizontal(|ui| {
                ui.label("GB Contrast:");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut draft.post_process_gb_contrast_percent, -100..=100)
                            .suffix("%"),
                    )
                    .changed();
            });
        },
    );

    ui.add_enabled_ui(
        matches!(
            draft.post_process_mode,
            toki_core::project_runtime::PostProcessMode::Vignette
        ),
        |ui| {
            ui.horizontal(|ui| {
                ui.label("Vignette Strength:");
                changed |= ui
                    .add(
                        egui::Slider::new(
                            &mut draft.post_process_vignette_strength_percent,
                            0..=100,
                        )
                        .suffix("%"),
                    )
                    .changed();
            });
        },
    );

    changed
}

fn render_frame_rate_controls(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
    ui.label("Frame Rate");
    changed |= ui.checkbox(&mut draft.vsync, "VSync").changed();
    ui.add_enabled_ui(!draft.vsync, |ui| {
        ui.horizontal(|ui| {
            ui.label("Target FPS:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut draft.target_fps)
                        .speed(1.0)
                        .range(0..=240),
                )
                .changed();
        });
        ui.label("Set to 0 for unlimited frame rate.");
    });
    changed
}

fn render_timing_controls(
    ui: &mut egui::Ui,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> bool {
    let mut changed = false;
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
                changed |= ui
                    .selectable_value(
                        &mut draft.timing_mode,
                        toki_core::TimingMode::Fixed,
                        "Fixed (60 FPS)",
                    )
                    .changed();
                changed |= ui
                    .selectable_value(
                        &mut draft.timing_mode,
                        toki_core::TimingMode::Delta,
                        "Delta",
                    )
                    .changed();
            });
    });
    ui.label("Fixed: Deterministic, 60 ticks/sec. Delta: Scales with frame time.");
    changed
}
