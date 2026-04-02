use super::InspectorSystem;
use crate::config::EditorConfig;
use crate::project::{Project, ProjectAssets};
use crate::scene::SceneViewport;
use crate::ui::editor_ui::{EditorUI, PlacementKind, ToolboxTab};
use crate::ui::hierarchy::collect_entity_definitions_for_toolbox;
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
        render_mode_and_cancel(ui_state, ui, &mut viewport);
        ui.separator();

        render_tab_strip(ui_state, ui);
        ui.separator();

        let project_path = resolve_project_path(project_assets.as_ref(), project.as_ref(), config);
        let Some(project_path) = project_path else {
            ui.label("Open a project to access placement tools.");
            return;
        };

        let tab = ui_state.scene_viewport_context().toolbox.selected_tab;
        match tab {
            ToolboxTab::Decorations => {
                render_decoration_tab(ui_state, ui, ctx, &project_path, &mut viewport);
            }
            ToolboxTab::Creatures | ToolboxTab::Humans | ToolboxTab::Items => {
                render_entity_definition_tab(ui_state, ui, project_assets, tab, &mut viewport);
            }
        }
    }
}

fn render_mode_and_cancel(
    ui_state: &mut EditorUI,
    ui: &mut egui::Ui,
    viewport: &mut Option<&mut SceneViewport>,
) {
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
}

fn render_tab_strip(ui_state: &mut EditorUI, ui: &mut egui::Ui) {
    let current = ui_state.scene_viewport_context().toolbox.selected_tab;
    let mut selected = current;
    ui.horizontal(|ui| {
        for tab in ToolboxTab::ALL {
            ui.selectable_value(&mut selected, *tab, tab.label());
        }
    });
    if selected != current {
        ui_state.scene_viewport_context_mut().toolbox.selected_tab = selected;
    }
}

fn resolve_project_path(
    project_assets: Option<&&mut ProjectAssets>,
    project: Option<&&mut Project>,
    config: Option<&EditorConfig>,
) -> Option<std::path::PathBuf> {
    project_assets
        .map(|a| a.project_path.clone())
        .or_else(|| project.map(|p| p.path.clone()))
        .or_else(|| config.and_then(|c| c.current_project_path().cloned()))
}

// ---------- Entity definition tab ----------

fn render_entity_definition_tab(
    ui_state: &mut EditorUI,
    ui: &mut egui::Ui,
    project_assets: Option<&mut ProjectAssets>,
    tab: ToolboxTab,
    viewport: &mut Option<&mut SceneViewport>,
) {
    let definitions = collect_entity_definitions_for_toolbox(ui_state, project_assets);
    let names = match definitions.get(&tab) {
        Some(names) if !names.is_empty() => names,
        _ => {
            ui.label(format!("No {} definitions found.", tab.label().to_lowercase()));
            return;
        }
    };

    ui.label(format!("{} ({})", tab.label(), names.len()));
    ui.separator();

    egui::ScrollArea::vertical()
        .max_height(400.0)
        .show(ui, |ui| {
            for definition in names {
                let label = if definition.display_name == definition.name {
                    definition.name.as_str().to_string()
                } else {
                    format!("{} ({})", definition.display_name, definition.name)
                };
                if ui.selectable_label(false, label).clicked() {
                    enter_definition_placement_mode(ui_state, &definition.name);
                    if let Some(viewport) = viewport.as_mut() {
                        viewport.mark_dirty();
                    }
                }
            }
        });

    ui.separator();
    ui.small("Click an entity to enter placement mode. Press Enter or Escape to exit.");
}

fn enter_definition_placement_mode(ui_state: &mut EditorUI, name: &str) {
    ui_state
        .scene_viewport_context_mut()
        .placement
        .enter_placement_mode(name.to_string());
}

// ---------- Decoration tab ----------

fn render_decoration_tab(
    ui_state: &mut EditorUI,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    project_path: &std::path::Path,
    viewport: &mut Option<&mut SceneViewport>,
) {
    let current_selected_sheet = ui_state
        .scene_viewport_context()
        .toolbox
        .selected_object_sheet
        .as_deref();
    let Some(source) = resolve_object_sheet_browser_source(project_path, current_selected_sheet)
    else {
        ui.label("No object sheets found in assets/sprites.");
        return;
    };

    sync_toolbox_selections(ui_state, &source);
    let (mut selected_sheet, mut selected_object) = current_sheet_and_object(ui_state, &source);
    let mut selection_changed = false;

    render_sheet_selector(ui, &source, &mut selected_sheet, &mut selection_changed);

    let active_source =
        resolve_object_sheet_browser_source(project_path, Some(selected_sheet.as_str()))
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

    render_object_selector(ui, &active_source, &mut selected_object, &mut selection_changed);

    if selection_changed {
        apply_selection_change(ui_state, &selected_sheet, &selected_object, project_path, viewport);
    }

    let texture = load_preview_texture(ui_state, ctx, &active_source);
    let mut gallery_object_selected = false;
    render_gallery(
        ui,
        texture,
        &active_source,
        &mut selected_object,
        &mut gallery_object_selected,
    );

    sync_final_object_selection(ui_state, &selected_object);

    if gallery_object_selected {
        if let Some(draft) =
            build_decoration_placement_draft(project_path, &selected_sheet, &selected_object)
        {
            ui_state
                .scene_viewport_context_mut()
                .placement
                .enter_decoration_placement_mode(draft);
            if let Some(viewport) = viewport.as_mut() {
                viewport.mark_dirty();
            }
        }
    }
    ui.separator();
    ui.small(
        "Click an object in the palette to enter placement mode. Press Enter or Escape to exit.",
    );
}

fn sync_toolbox_selections(
    ui_state: &mut EditorUI,
    source: &crate::ui::object_sheet_browser::ObjectSheetBrowserSource,
) {
    let toolbox = &mut ui_state.scene_viewport_context_mut().toolbox;
    sync_selected_sheet_name(&mut toolbox.selected_object_sheet, &source.sheet_names);
    sync_selected_object_name(&mut toolbox.selected_object_name, &source.object_names);
}

fn current_sheet_and_object(
    ui_state: &EditorUI,
    source: &crate::ui::object_sheet_browser::ObjectSheetBrowserSource,
) -> (String, String) {
    let toolbox = &ui_state.scene_viewport_context().toolbox;
    let sheet = toolbox
        .selected_object_sheet
        .clone()
        .unwrap_or_else(|| source.selected_sheet_name.clone());
    let object = toolbox
        .selected_object_name
        .clone()
        .unwrap_or_else(|| source.object_names.first().cloned().unwrap_or_default());
    (sheet, object)
}

fn render_sheet_selector(
    ui: &mut egui::Ui,
    source: &crate::ui::object_sheet_browser::ObjectSheetBrowserSource,
    selected_sheet: &mut String,
    selection_changed: &mut bool,
) {
    ui.label("Object Sheet");
    egui::ComboBox::from_id_salt("scene_toolbox_object_sheet")
        .selected_text(selected_sheet.as_str())
        .show_ui(ui, |ui| {
            for sheet_name in &source.sheet_names {
                *selection_changed |= ui
                    .selectable_value(selected_sheet, sheet_name.clone(), sheet_name)
                    .changed();
            }
        });
}

fn render_object_selector(
    ui: &mut egui::Ui,
    source: &crate::ui::object_sheet_browser::ObjectSheetBrowserSource,
    selected_object: &mut String,
    selection_changed: &mut bool,
) {
    ui.label("Object");
    egui::ComboBox::from_id_salt("scene_toolbox_object_name")
        .selected_text(selected_object.as_str())
        .show_ui(ui, |ui| {
            for object_name in &source.object_names {
                *selection_changed |= ui
                    .selectable_value(selected_object, object_name.clone(), object_name)
                    .changed();
            }
        });
}

fn apply_selection_change(
    ui_state: &mut EditorUI,
    selected_sheet: &str,
    selected_object: &str,
    project_path: &std::path::Path,
    viewport: &mut Option<&mut SceneViewport>,
) {
    let scene_context = ui_state.scene_viewport_context_mut();
    scene_context.toolbox.selected_object_sheet = Some(selected_sheet.to_string());
    scene_context.toolbox.selected_object_name = Some(selected_object.to_string());
    scene_context.placement.preview_cached_frame = None;

    if matches!(
        ui_state.scene_viewport_context().placement.kind,
        Some(PlacementKind::Decoration(_))
    ) {
        if let Some(draft) =
            build_decoration_placement_draft(project_path, selected_sheet, selected_object)
        {
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

fn load_preview_texture(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    source: &crate::ui::object_sheet_browser::ObjectSheetBrowserSource,
) -> Option<egui::TextureHandle> {
    let scene_context = ui_state.scene_viewport_context_mut();
    ensure_object_sheet_preview_texture(
        &mut scene_context.toolbox.preview_image_path,
        &mut scene_context.toolbox.preview_texture,
        ctx,
        &source.texture_path,
    )
}

fn render_gallery(
    ui: &mut egui::Ui,
    texture: Option<egui::TextureHandle>,
    source: &crate::ui::object_sheet_browser::ObjectSheetBrowserSource,
    selected_object: &mut String,
    gallery_object_selected: &mut bool,
) {
    let Some(texture) = texture else { return };
    let Some(texture_size) = source.object_sheet.image_size() else {
        return;
    };

    ui.separator();
    ui.label("Preview");
    if let Some(object_name) = source
        .object_names
        .iter()
        .find(|name| name.as_str() == selected_object.as_str())
    {
        ui.horizontal(|ui| {
            render_object_gallery_item(
                ui,
                texture.id(),
                texture_size,
                &source.object_sheet,
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
        &source.object_sheet,
        &source.object_names,
        selected_object,
        gallery_object_selected,
    );
}

fn sync_final_object_selection(ui_state: &mut EditorUI, selected_object: &str) {
    if ui_state
        .scene_viewport_context()
        .toolbox
        .selected_object_name
        .as_deref()
        != Some(selected_object)
    {
        let scene_context = ui_state.scene_viewport_context_mut();
        scene_context.toolbox.selected_object_name = Some(selected_object.to_string());
        scene_context.placement.preview_cached_frame = None;
    }
}

fn render_object_gallery_grid(
    ui: &mut egui::Ui,
    texture_id: egui::TextureId,
    texture_size: glam::UVec2,
    object_sheet: &toki_core::assets::object_sheet::ObjectSheetMeta,
    object_names: &[String],
    selected_object_name: &mut String,
    gallery_object_selected: &mut bool,
) {
    const COLUMNS: usize = 4;

    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            egui::Grid::new("scene_toolbox_object_gallery")
                .num_columns(COLUMNS)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (index, object_name) in object_names.iter().enumerate() {
                        render_gallery_cell(
                            ui,
                            texture_id,
                            texture_size,
                            object_sheet,
                            object_name,
                            selected_object_name,
                            gallery_object_selected,
                        );
                        if (index + 1) % COLUMNS == 0 {
                            ui.end_row();
                        }
                    }
                });
        });
}

fn render_gallery_cell(
    ui: &mut egui::Ui,
    texture_id: egui::TextureId,
    texture_size: glam::UVec2,
    object_sheet: &toki_core::assets::object_sheet::ObjectSheetMeta,
    object_name: &str,
    selected_object_name: &mut String,
    gallery_object_selected: &mut bool,
) {
    const SLOT_SIZE: f32 = 64.0;
    ui.vertical(|ui| {
        let is_selected = selected_object_name.as_str() == object_name;
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
            *selected_object_name = object_name.to_string();
            *gallery_object_selected = true;
        }
        ui.add_sized(
            [SLOT_SIZE, 16.0],
            egui::Label::new(egui::RichText::new(object_name).small()).truncate(),
        );
    });
}
