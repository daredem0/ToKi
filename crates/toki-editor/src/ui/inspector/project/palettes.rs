use super::*;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct PaletteSectionOutcome {
    pub(super) changed: bool,
    pub(super) palette_files_changed: bool,
}

pub(super) fn render_palettes_section(
    ui_state: &mut EditorUI,
    ui: &mut egui::Ui,
    project: &Project,
    draft: &mut crate::project::ProjectSettingsDraft,
) -> PaletteSectionOutcome {
    let mut outcome = PaletteSectionOutcome::default();
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
                    outcome.changed |= ui
                        .selectable_value(
                            &mut draft.indexed_palette_override,
                            None,
                            "Atlas Default",
                        )
                        .changed();
                    for palette_id in &palette_ids {
                        outcome.changed |= ui
                            .selectable_value(
                                &mut draft.indexed_palette_override,
                                Some(palette_id.clone()),
                                palette_id,
                            )
                            .changed();
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Size Mismatch:");
            egui::ComboBox::from_id_salt("palette_mismatch_strategy")
                .selected_text(match draft.palette_mismatch_strategy {
                    PaletteMismatchStrategy::Lenient => "Lenient",
                    PaletteMismatchStrategy::Interpolate => "Interpolate",
                })
                .show_ui(ui, |ui| {
                    outcome.changed |= ui
                        .selectable_value(
                            &mut draft.palette_mismatch_strategy,
                            PaletteMismatchStrategy::Lenient,
                            "Lenient",
                        )
                        .on_hover_text("Use palette as-is; unmapped shades keep canonical gray.")
                        .changed();
                    outcome.changed |= ui
                        .selectable_value(
                            &mut draft.palette_mismatch_strategy,
                            PaletteMismatchStrategy::Interpolate,
                            "Interpolate",
                        )
                        .on_hover_text(
                            "Stretch palette to target size by interpolating the color ramp.",
                        )
                        .changed();
                });
        });

        ui.separator();
        ui.label("Built-in palettes are always available.");
        ui.separator();
        ui.label("Project Palette Files:");

        let mut project_palettes = load_project_palette_files(project);
        let mut remove_palette_id = None;
        for palette_id in project_palettes.keys().cloned().collect::<Vec<_>>() {
            let mut palette = project_palettes
                .get(&palette_id)
                .cloned()
                .unwrap_or_else(|| Palette::grayscale(PaletteSize::Pal4));
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} ({})", &palette_id, palette.size()));
                    if ui.button("Remove").clicked() {
                        remove_palette_id = Some(palette_id.clone());
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for color in palette.colors_mut() {
                        let mut color32 = egui::Color32::from_rgba_unmultiplied(
                            color[0], color[1], color[2], color[3],
                        );
                        if ui.color_edit_button_srgba(&mut color32).changed() {
                            *color = [color32.r(), color32.g(), color32.b(), color32.a()];
                            outcome.palette_files_changed = true;
                        }
                    }
                });
            });
            if project_palettes.get(&palette_id) != Some(&palette) {
                match save_project_palette_file(project, &palette_id, &palette) {
                    Ok(()) => {
                        project_palettes.insert(palette_id.clone(), palette);
                        outcome.palette_files_changed = true;
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
                    if draft.indexed_palette_override.as_deref() == Some(remove_palette_id.as_str())
                    {
                        draft.indexed_palette_override = None;
                        outcome.changed = true;
                    }
                    outcome.palette_files_changed = true;
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

        ui.horizontal(|ui| {
            render_add_palette_controls(ui, ui_state, project, &project_palettes, &mut outcome);
        });
    });
    outcome
}

const PALETTE_SIZE_OPTIONS: [PaletteSize; 6] = [
    PaletteSize::Pal4,
    PaletteSize::Pal8,
    PaletteSize::Pal16,
    PaletteSize::Pal32,
    PaletteSize::Pal64,
    PaletteSize::Pal256,
];

fn render_add_palette_controls(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    project: &Project,
    project_palettes: &BTreeMap<String, Palette>,
    outcome: &mut PaletteSectionOutcome,
) {
    let selected_size = ui.data_mut(|data| {
        *data.get_temp_mut_or_insert_with(egui::Id::new("new_palette_size"), || 0usize)
    });
    let palette_size = PALETTE_SIZE_OPTIONS[selected_size.min(PALETTE_SIZE_OPTIONS.len() - 1)];

    egui::ComboBox::from_id_salt("new_palette_size_combo")
        .selected_text(format!("{} colors", palette_size))
        .width(90.0)
        .show_ui(ui, |ui| {
            for (i, size) in PALETTE_SIZE_OPTIONS.iter().enumerate() {
                let label = format!("{} colors", size);
                if ui.selectable_label(selected_size == i, label).clicked() {
                    ui.data_mut(|data| {
                        data.insert_temp(egui::Id::new("new_palette_size"), i);
                    });
                }
            }
        });

    if ui.button("Add Project Palette").clicked() {
        let palette_id = next_custom_palette_id(ui_state, project_palettes.len() + 1);
        let new_palette = Palette::grayscale(palette_size);
        match save_project_palette_file(project, &palette_id, &new_palette) {
            Ok(()) => {
                outcome.palette_files_changed = true;
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
}

fn next_custom_palette_id(ui_state: &EditorUI, starting_index: usize) -> String {
    let mut index = starting_index;
    loop {
        let candidate = format!("custom_palette_{index}");
        if !ui_state.project.available_palettes.contains_key(&candidate) {
            return candidate;
        }
        index += 1;
    }
}
