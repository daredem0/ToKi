use super::InspectorSystem;
use crate::config::EditorConfig;
use crate::project::{Project, ProjectAssets};
use crate::scene::SceneViewport;
use crate::ui::editor_ui::{EditorUI, PlacementKind};
use crate::ui::object_sheet_browser::{
    build_decoration_placement_draft, ensure_object_sheet_preview_texture,
    render_object_gallery_item, resolve_object_sheet_browser_source, sync_selected_object_name,
    sync_selected_sheet_name,
};

impl InspectorSystem {
    pub(crate) fn render_scene_viewport_toolbox(
        ui_state: &mut EditorUI,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        project: Option<&mut Project>,
        project_assets: Option<&mut ProjectAssets>,
        config: Option<&EditorConfig>,
        mut viewport: Option<&mut SceneViewport>,
    ) {
        ui.heading("Toolbox");
        ui.separator();

        if let Some(mode_label) = ui_state.scene_viewport_context().placement.mode_label() {
            ui.label(format!("Active Mode: {mode_label}"));
        } else {
            ui.label("Active Mode: Select");
        }

        if ui_state.scene_viewport_context().placement.is_in_placement_mode()
            && ui.button("Cancel Placement").clicked()
        {
            ui_state
                .scene_viewport_context_mut()
                .placement
                .exit_placement_mode();
            if let Some(viewport) = viewport.as_mut() {
                viewport.mark_dirty();
            }
        }

        let project_path = project_assets
            .as_ref()
            .map(|assets| assets.project_path.clone())
            .or_else(|| project.as_ref().map(|project| project.path.clone()))
            .or_else(|| config.and_then(|config| config.current_project_path().cloned()));
        let Some(project_path) = project_path else {
            ui.separator();
            ui.label("Open a project to access decoration placement tools.");
            return;
        };

        let current_selected_sheet = ui_state
            .scene_viewport_context()
            .toolbox
            .selected_object_sheet
            .as_deref();
        let Some(source) = resolve_object_sheet_browser_source(&project_path, current_selected_sheet)
        else {
            ui.separator();
            ui.label("No object sheets found in assets/sprites.");
            return;
        };

        {
            let toolbox = &mut ui_state.scene_viewport_context_mut().toolbox;
            sync_selected_sheet_name(&mut toolbox.selected_object_sheet, &source.sheet_names);
            sync_selected_object_name(&mut toolbox.selected_object_name, &source.object_names);
        }

        let mut selected_sheet = ui_state
            .scene_viewport_context()
            .toolbox
            .selected_object_sheet
            .clone()
            .unwrap_or_else(|| source.selected_sheet_name.clone());
        let mut selected_object = ui_state
            .scene_viewport_context()
            .toolbox
            .selected_object_name
            .clone()
            .unwrap_or_else(|| source.object_names.first().cloned().unwrap_or_default());
        let mut selection_changed = false;

        ui.separator();
        ui.label("Object Sheet");
        egui::ComboBox::from_id_salt("scene_toolbox_object_sheet")
            .selected_text(selected_sheet.as_str())
            .show_ui(ui, |ui| {
                for sheet_name in &source.sheet_names {
                    selection_changed |= ui
                        .selectable_value(&mut selected_sheet, sheet_name.clone(), sheet_name)
                        .changed();
                }
            });

        let active_source =
            resolve_object_sheet_browser_source(&project_path, Some(selected_sheet.as_str()))
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
            selection_changed = true;
        }

        ui.label("Object");
        egui::ComboBox::from_id_salt("scene_toolbox_object_name")
            .selected_text(selected_object.as_str())
            .show_ui(ui, |ui| {
                for object_name in &active_source.object_names {
                    selection_changed |= ui
                        .selectable_value(
                            &mut selected_object,
                            object_name.clone(),
                            object_name,
                        )
                        .changed();
                }
            });

        if selection_changed {
            {
                let scene_context = ui_state.scene_viewport_context_mut();
                scene_context.toolbox.selected_object_sheet = Some(selected_sheet.clone());
                scene_context.toolbox.selected_object_name = Some(selected_object.clone());
                scene_context.placement.preview_cached_frame = None;
            }
            if matches!(
                ui_state.scene_viewport_context().placement.kind,
                Some(PlacementKind::Decoration(_))
            ) {
                if let Some(draft) = build_decoration_placement_draft(
                    &project_path,
                    &selected_sheet,
                    &selected_object,
                ) {
                    ui_state
                        .scene_viewport_context_mut()
                        .placement
                        .kind = Some(PlacementKind::Decoration(draft));
                }
            }
            if let Some(viewport) = viewport.as_mut() {
                viewport.mark_dirty();
            }
        }

        let texture = {
            let scene_context = ui_state.scene_viewport_context_mut();
            ensure_object_sheet_preview_texture(
                &mut scene_context.toolbox.preview_image_path,
                &mut scene_context.toolbox.preview_texture,
                ctx,
                &active_source.texture_path,
            )
        };
        if let Some(texture) = texture {
            if let Some(texture_size) = active_source.object_sheet.image_size() {
                ui.separator();
                ui.label("Preview");
                if let Some(object_name) = active_source
                    .object_names
                    .iter()
                    .find(|name| *name == &selected_object)
                {
                    ui.horizontal(|ui| {
                        render_object_gallery_item(
                            ui,
                            texture.id(),
                            texture_size,
                            &active_source.object_sheet,
                            object_name,
                            true,
                            72.0,
                        );
                        ui.label(object_name.as_str());
                    });
                }

                ui.separator();
                ui.label("Object Palette");
                render_object_gallery_grid(
                    ui,
                    texture.id(),
                    texture_size,
                    &active_source.object_sheet,
                    &active_source.object_names,
                    &mut selected_object,
                );
            }
        }

        if ui_state
            .scene_viewport_context()
            .toolbox
            .selected_object_name
            .as_deref()
            != Some(selected_object.as_str())
        {
            let scene_context = ui_state.scene_viewport_context_mut();
            scene_context.toolbox.selected_object_name = Some(selected_object.clone());
            scene_context.placement.preview_cached_frame = None;
        }

        ui.separator();
        let can_place = build_decoration_placement_draft(
            &project_path,
            &selected_sheet,
            &selected_object,
        )
        .is_some();
        if ui
            .add_enabled(can_place, egui::Button::new("Place Object"))
            .clicked()
        {
            if let Some(draft) = build_decoration_placement_draft(
                &project_path,
                &selected_sheet,
                &selected_object,
            ) {
                ui_state
                    .scene_viewport_context_mut()
                    .placement
                    .enter_decoration_placement_mode(draft);
                if let Some(viewport) = viewport.as_mut() {
                    viewport.mark_dirty();
                }
            }
        }
        ui.small("Place decorations directly in the scene viewport.");
    }
}

fn render_object_gallery_grid(
    ui: &mut egui::Ui,
    texture_id: egui::TextureId,
    texture_size: glam::UVec2,
    object_sheet: &toki_core::assets::object_sheet::ObjectSheetMeta,
    object_names: &[String],
    selected_object_name: &mut String,
) {
    const COLUMNS: usize = 4;
    const SLOT_SIZE: f32 = 64.0;

    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            egui::Grid::new("scene_toolbox_object_gallery")
                .num_columns(COLUMNS)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (index, object_name) in object_names.iter().enumerate() {
                        ui.vertical(|ui| {
                            let is_selected = selected_object_name.as_str() == object_name.as_str();
                            if render_object_gallery_item(
                                ui,
                                texture_id,
                                texture_size,
                                object_sheet,
                                object_name,
                                is_selected,
                                SLOT_SIZE,
                            )
                            .clicked()
                            {
                                *selected_object_name = object_name.clone();
                            }
                            ui.add_sized(
                                [SLOT_SIZE, 16.0],
                                egui::Label::new(egui::RichText::new(object_name.as_str()).small())
                                    .truncate(),
                            );
                        });

                        if (index + 1) % COLUMNS == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
}
