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

        ui.separator();
        ui.label("Built-in palettes are always available.");
        ui.separator();
        ui.label("Project Palette Files:");

        let mut project_palettes = load_project_palette_files(project);
        let mut remove_palette_id = None;
        for palette_id in project_palettes.keys().cloned().collect::<Vec<_>>() {
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
                            *color = [color32.r(), color32.g(), color32.b(), color32.a()];
                            outcome.palette_files_changed = true;
                        }
                    }
                });
            });
            if project_palettes.get(&palette_id).copied() != Some(palette) {
                match save_project_palette_file(project, &palette_id, palette) {
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
                    if draft.indexed_palette_override.as_deref()
                        == Some(remove_palette_id.as_str())
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

        if ui.button("Add Project Palette").clicked() {
            let palette_id = next_custom_palette_id(ui_state, project_palettes.len() + 1);
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
    });
    outcome
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
