use crate::fonts::resolve_preview_font_family;
use crate::project::{Project, ProjectAssets};
use crate::ui::editor_ui::{sync_ui_layout_registry, EditorUI, UiCanvasInteraction};
use crate::ui::ui_event_registry::{
    render_optional_ui_event_id_editor, validate_ui_event_reference,
};
use egui::{
    text::{LayoutJob, TextFormat},
    Color32, CursorIcon, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use std::collections::{BTreeSet, HashMap};
use toki_core::expression::Expression;
use toki_core::flags::{FlagValue, GameFlags};
use toki_core::rules::TriggerContext;
use toki_core::text::{TextAnchor, TextSlant, TextWeight};
use toki_core::ui::{
    transform_logical_ui_composition_with_transform, transform_logical_ui_rect_with_transform,
    ui_presentation_transform, UiBlock, UiComposition, UiRect,
};
use toki_core::ui_layout::{
    UiAnchor, UiBinding, UiBindingContext, UiLayoutAsset, UiLayoutEngine, UiProgressBinding,
    UiSpacing, UiTextSegment, UiTextTemplate, UiTheme, UiTypography, UiWidgetFrame,
    UiWidgetKind, UiWidgetNode,
};
use toki_core::value_path::{ValuePath, ValuePathContext};

const PREVIEW_MIN_HEIGHT: f32 = 320.0;
const UI_SNAP_GRID: f32 = 4.0;
const UI_EDGE_MARGIN: f32 = 8.0;

const ZERO_SPACING: UiSpacing = UiSpacing {
    left: 0,
    top: 0,
    right: 0,
    bottom: 0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetKindChoice {
    Label,
    Button,
    ProgressBar,
    Slider,
    GridContainer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetPreset {
    Label,
    HealthBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetPositionPreset {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

pub(crate) fn render_ui_editor(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    project_assets: Option<&mut ProjectAssets>,
    project: Option<&mut Project>,
) {
    let Some(project_assets) = project_assets else {
        ui.label("Open a project to author UI layouts.");
        return;
    };

    sync_ui_layout_registry(ui_state, project_assets);
    let declared_flags = project
        .as_ref()
        .map(|project| project.metadata.runtime.flags.declarations.as_slice())
        .unwrap_or(&[]);
    let declared_ui_events = project
        .as_ref()
        .map(|project| project.metadata.runtime.ui.event_declarations.as_slice())
        .unwrap_or(&[]);
    render_ui_editor_main(
        ui,
        ui_state,
        project_assets,
        declared_flags,
        declared_ui_events,
        ui_preview_viewport_size(project.as_deref()),
    );
}

pub(crate) fn render_ui_editor_inspector_panel(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    project_assets: &mut ProjectAssets,
    project: Option<&Project>,
) {
    if ui_state.ui_editor_context().ui.draft.is_none() {
        ui.heading("UI Editor");
        ui.separator();
        ui.label("Select or create a UI layout.");
        return;
    }

    let declared_flags = project
        .map(|project| project.metadata.runtime.flags.declarations.as_slice())
        .unwrap_or(&[]);
    let declared_ui_events = project
        .map(|project| project.metadata.runtime.ui.event_declarations.as_slice())
        .unwrap_or(&[]);
    let preview_viewport_size = ui_preview_viewport_size(project);
    let selected_widget_id = ui_state.ui_editor_context().ui.selected_widget_id.clone();
    let mut next_selected_widget_id = selected_widget_id.clone();
    let mut dirty = false;
    let font_choices = ui_editor_font_choices(ui_state);

    ui.heading("UI Editor");
    ui.separator();
    {
        let layout = ui_state
            .ui_editor_context_mut()
            .ui
            .draft
            .as_mut()
            .expect("checked above");
        ui.collapsing("Widget Tree", |ui| {
            let mut remove_widget_id = None::<String>;
            render_widget_tree(
                ui,
                &layout.root,
                selected_widget_id.as_deref(),
                0,
                &mut next_selected_widget_id,
                &mut remove_widget_id,
            );
            if let Some(widget_id) = remove_widget_id {
                match remove_widget_by_id(&mut layout.root, &widget_id) {
                    Some(parent_id) => {
                        next_selected_widget_id = Some(parent_id);
                        dirty = true;
                    }
                    None => {
                        next_selected_widget_id = Some("root".to_string());
                    }
                }
            }
        });
        ui.separator();
        let inspector_selected_widget_id = next_selected_widget_id.clone();
        dirty |= render_selected_widget_inspector(
            ui,
            layout,
            inspector_selected_widget_id.as_deref(),
            &UiEditorAuthoringContext {
                declared_flags,
                declared_ui_events,
                font_choices: &font_choices,
                preview_viewport_size,
            },
            &mut next_selected_widget_id,
        );
        ui.separator();
        let issues = validate_ui_layout(layout, declared_flags, declared_ui_events);
        ui.collapsing("Validation", |ui| {
            for issue in &issues {
                ui.colored_label(Color32::from_rgb(255, 210, 80), issue);
            }
            if issues.is_empty() {
                ui.label("No validation issues.");
            }
        });
    }

    if next_selected_widget_id != selected_widget_id {
        if let Some(widget_id) = next_selected_widget_id {
            ui_state.ui_editor_context_mut().ui.select_widget(widget_id);
        } else {
            ui_state.ui_editor_context_mut().ui.selected_widget_id = None;
        }
    }
    if dirty {
        ui_state.ui_editor_context_mut().ui.dirty = true;
    }

    if let Some(status) = &ui_state.ui_editor_context().ui.status_message {
        ui.separator();
        ui.small(status);
    }

    let _ = project_assets;
}

fn render_ui_editor_main(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    project_assets: &mut ProjectAssets,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
    preview_viewport_size: glam::Vec2,
) {
    let Some(mut layout) = ui_state.ui_editor_context_mut().ui.draft.take() else {
        ui.label("No UI layout selected.");
        return;
    };
    handle_ui_editor_shortcuts(ui, ui_state, &mut layout, preview_viewport_size);

    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                ui_state.ui_editor_context().ui.dirty,
                egui::Button::new("Save UI Layout"),
            )
            .clicked()
        {
            let issues = validate_ui_layout(&layout, declared_flags, declared_ui_events);
            if !issues.is_empty() {
                ui_state.ui_editor_context_mut().ui.status_message = Some(format!(
                    "Cannot save UI layout with {} validation issue(s)",
                    issues.len()
                ));
            } else if let Err(error) = project_assets.save_ui_layout(&layout) {
                ui_state.ui_editor_context_mut().ui.status_message =
                    Some(format!("Failed to save UI layout '{}': {error}", layout.id));
            } else {
                let editor_state = &mut ui_state.ui_editor_context_mut().ui;
                editor_state.selected_layout_id = Some(layout.id.to_string());
                editor_state.loaded_layout_id = Some(layout.id.to_string());
                editor_state.dirty = false;
                editor_state.status_message = Some("UI layout saved".to_string());
                sync_ui_layout_registry(ui_state, project_assets);
            }
        }

        if let Some(status) = &ui_state.ui_editor_context().ui.status_message {
            ui.label(status);
        }
    });

    let mut dirty = false;
    ui.separator();
    dirty |= render_ui_layout_settings(ui, ui_state, &mut layout);
    ui.separator();
    dirty |= render_ui_toolbar(ui, ui_state, &mut layout);
    ui.separator();
    dirty |= render_ui_layout_canvas(
        ui,
        ui_state,
        &mut layout,
        declared_flags,
        preview_viewport_size,
    );

    if dirty {
        ui_state.ui_editor_context_mut().ui.dirty = true;
    }
    ui_state
        .ui_editor_context_mut()
        .ui
        .persist_active_view_into_layout();
    ui_state.ui_editor_context_mut().ui.draft = Some(layout);
}

fn handle_ui_editor_shortcuts(
    ui: &Ui,
    ui_state: &mut EditorUI,
    layout: &mut UiLayoutAsset,
    preview_viewport_size: glam::Vec2,
) {
    if ui.ctx().wants_keyboard_input() {
        return;
    }
    let command_pressed = ui.input(|input| input.modifiers.command);
    let delete_pressed = ui.input(|input| input.key_pressed(Key::Delete));
    let duplicate_pressed = command_pressed && ui.input(|input| input.key_pressed(Key::D));
    let focus_pressed =
        ui.input(|input| input.key_pressed(Key::F) || input.key_pressed(Key::Space));
    let reset_view_pressed = ui.input(|input| input.key_pressed(Key::Num0));
    let escape_pressed = ui.input(|input| input.key_pressed(Key::Escape));

    if delete_pressed {
        let selected_widget_id = ui_state.ui_editor_context().ui.selected_widget_id.clone();
        if let Some(widget_id) = selected_widget_id {
            match remove_widget_by_id(&mut layout.root, &widget_id) {
                Some(parent_id) => {
                    let editor_state = &mut ui_state.ui_editor_context_mut().ui;
                    editor_state.selected_widget_id = Some(parent_id);
                    editor_state.status_message =
                        Some(format!("Removed widget '{widget_id}' from layout."));
                    editor_state.dirty = true;
                }
                None => {
                    ui_state.ui_editor_context_mut().ui.status_message =
                        Some("Root widget cannot be removed.".to_string());
                }
            }
        }
    }

    if duplicate_pressed {
        let selected_widget_id = ui_state.ui_editor_context().ui.selected_widget_id.clone();
        if let Some(widget_id) = selected_widget_id {
            if let Some(new_widget_id) = duplicate_widget(&mut layout.root, &widget_id) {
                let editor_state = &mut ui_state.ui_editor_context_mut().ui;
                editor_state.selected_widget_id = Some(new_widget_id.clone());
                editor_state.status_message = Some(format!(
                    "Duplicated widget '{widget_id}' to '{new_widget_id}'."
                ));
                editor_state.dirty = true;
            }
        }
    }

    if focus_pressed {
        if let Some(selected_widget_id) = ui_state.ui_editor_context().ui.selected_widget_id.clone()
        {
            if let Some(frame) =
                compute_preview(&layout.root, declared_flags_stub(), preview_viewport_size)
                    .frames
                    .into_iter()
                    .find(|frame| frame.widget_id.as_str() == selected_widget_id)
            {
                let editor_state = &mut ui_state.ui_editor_context_mut().ui;
                editor_state.pan = [
                    40.0 - frame.rect.x * editor_state.zoom,
                    40.0 - frame.rect.y * editor_state.zoom,
                ];
                editor_state.persist_active_view_into_layout();
            }
        }
    }

    if reset_view_pressed {
        let editor_state = &mut ui_state.ui_editor_context_mut().ui;
        editor_state.zoom = 1.0;
        editor_state.pan = [12.0, 12.0];
        editor_state.persist_active_view_into_layout();
    }

    if escape_pressed {
        ui_state.ui_editor_context_mut().ui.selected_widget_id = None;
    }
}

fn render_ui_layout_settings(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    layout: &mut UiLayoutAsset,
) -> bool {
    let mut changed = false;
    ui.columns(2, |columns| {
        columns[0].horizontal(|ui| {
            ui.label("Id:");
            let mut id_value = layout.id.to_string();
            if ui.text_edit_singleline(&mut id_value).changed() {
                layout.id = id_value.into();
                ui_state.ui_editor_context_mut().ui.selected_layout_id =
                    Some(layout.id.to_string());
                changed = true;
            }
        });
        columns[0].horizontal(|ui| {
            ui.label("Title:");
            changed |= ui.text_edit_singleline(&mut layout.title).changed();
        });
        columns[1].horizontal(|ui| {
            ui.label("Startup Visible:");
            changed |= ui.checkbox(&mut layout.startup_visible, "").changed();
        });
        columns[1].horizontal(|ui| {
            ui.label("Z Order:");
            changed |= ui
                .add(egui::DragValue::new(&mut layout.z_order).speed(1.0))
                .changed();
        });
    });
    changed
}

fn render_ui_toolbar(ui: &mut Ui, ui_state: &mut EditorUI, layout: &mut UiLayoutAsset) -> bool {
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.strong("Quick Add:");
        for preset in [WidgetPreset::Label, WidgetPreset::HealthBar] {
            if ui
                .button(format!("+ {}", widget_preset_label(preset)))
                .clicked()
            {
                changed |= insert_new_widget_under_parent(
                    ui_state,
                    layout,
                    "root",
                    create_widget_preset(preset, layout),
                    &format!("Added {} preset.", widget_preset_label(preset)),
                );
            }
        }

        ui.separator();
        ui.strong("Advanced:");
        for kind in [
            WidgetKindChoice::Button,
            WidgetKindChoice::ProgressBar,
            WidgetKindChoice::Slider,
            WidgetKindChoice::GridContainer,
        ] {
            if ui
                .button(format!("+ {}", widget_kind_choice_label(kind)))
                .clicked()
            {
                changed |= insert_new_widget_under_parent(
                    ui_state,
                    layout,
                    &preferred_parent_id(
                        layout,
                        ui_state
                            .ui_editor_context()
                            .ui
                            .selected_widget_id
                            .as_deref(),
                    ),
                    create_widget(kind, layout),
                    &format!("Added {} widget.", widget_kind_choice_label(kind)),
                );
            }
        }
    });
    changed
}

fn render_ui_layout_canvas(
    ui: &mut Ui,
    ui_state: &mut EditorUI,
    layout: &mut UiLayoutAsset,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    preview_viewport_size: glam::Vec2,
) -> bool {
    let mut changed = false;
    let desired_size = egui::vec2(
        ui.available_width(),
        ui.available_height().max(PREVIEW_MIN_HEIGHT),
    );
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let (mut zoom, mut pan, selected_widget_id, canvas_interaction) = {
        let editor_state = &ui_state.ui_editor_context().ui;
        (
            editor_state.zoom,
            editor_state.pan,
            editor_state.selected_widget_id.clone(),
            editor_state.canvas_interaction.clone(),
        )
    };

    painter.rect_filled(rect, 8.0, Color32::from_rgb(16, 22, 28));
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(1.0, Color32::from_gray(70)),
        StrokeKind::Inside,
    );

    if response.hovered() {
        if !ui.ctx().wants_keyboard_input() {
            if ui.input(|input| input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals)) {
                zoom = (zoom * 1.1).clamp(0.35, 3.0);
                changed = true;
            }
            if ui.input(|input| input.key_pressed(Key::Minus)) {
                zoom = (zoom / 1.1).clamp(0.35, 3.0);
                changed = true;
            }
        }
        let scroll_delta = ui.input(|input| input.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            let factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
            zoom = (zoom * factor).clamp(0.35, 3.0);
            changed = true;
        }
    }

    let preview = compute_preview(&layout.root, declared_flags, preview_viewport_size);
    let origin = Vec2::new(rect.left() + pan[0], rect.top() + pan[1]);
    let viewport_rect = Rect::from_min_size(
        Pos2::new(origin.x, origin.y),
        Vec2::new(
            preview_viewport_size.x * zoom,
            preview_viewport_size.y * zoom,
        ),
    );

    painter.rect_filled(viewport_rect, 4.0, Color32::from_rgb(10, 12, 18));
    painter.rect_stroke(
        viewport_rect,
        4.0,
        Stroke::new(1.0, Color32::from_gray(100)),
        StrokeKind::Inside,
    );
    paint_ui_composition(
        &painter,
        &preview.composition,
        glam::vec2(origin.x, origin.y),
        zoom,
        preview_viewport_size,
        &ui_editor_font_choices(ui_state),
    );

    let pointer_pos = response
        .interact_pointer_pos()
        .or_else(|| response.hover_pos());
    let drag_start_pos = ui
        .input(|input| input.pointer.press_origin())
        .or(pointer_pos);
    let hovered_widget_id = pointer_pos.and_then(|pointer_pos| {
        topmost_widget_id_at_point(
            &preview.frames,
            origin,
            zoom,
            preview_viewport_size,
            pointer_pos,
        )
    });
    let hovered_resize_widget_id = drag_start_pos.and_then(|pointer_pos| {
        hovered_widget_id.as_deref().and_then(|widget_id| {
            resize_handle_widget_id_at_point(
                &preview.frames,
                origin,
                zoom,
                preview_viewport_size,
                pointer_pos,
                widget_id,
            )
        })
    });
    let mut next_canvas_interaction = canvas_interaction;
    let mut next_selected_widget_id = selected_widget_id.clone();
    if hovered_resize_widget_id.is_some() {
        ui.output_mut(|output| output.cursor_icon = CursorIcon::ResizeNwSe);
    } else if hovered_widget_id.is_some() {
        ui.output_mut(|output| output.cursor_icon = CursorIcon::Grab);
    }
    for frame in preview.frames.iter().rev() {
        let screen_rect = scaled_rect_for_viewport(frame.rect, origin, zoom, preview_viewport_size);
        let is_selected = selected_widget_id.as_deref() == Some(frame.widget_id.as_str());
        let is_hovered = hovered_widget_id.as_deref() == Some(frame.widget_id.as_str());
        let stroke_color = if is_selected {
            Color32::from_rgb(255, 210, 90)
        } else if is_hovered {
            Color32::from_rgb(120, 190, 255)
        } else {
            Color32::from_rgba_unmultiplied(200, 200, 200, 110)
        };
        painter.rect_stroke(
            screen_rect,
            2.0,
            Stroke::new(
                if is_selected || is_hovered { 2.0 } else { 1.0 },
                stroke_color,
            ),
            StrokeKind::Inside,
        );

        if (is_selected || next_selected_widget_id.as_deref() == Some(frame.widget_id.as_str()))
            && frame.widget_id.as_str() != "root"
        {
            let handle_rect = resize_handle_rect(screen_rect);
            let handle_color =
                if hovered_resize_widget_id.as_deref() == Some(frame.widget_id.as_str()) {
                    Color32::from_rgb(255, 240, 140)
                } else {
                    Color32::from_rgb(255, 210, 90)
                };
            painter.rect_filled(handle_rect, 1.0, handle_color);
        }
    }

    let resize_target_widget_id = drag_start_pos.and_then(|pointer_pos| {
        selected_widget_id
            .as_deref()
            .and_then(|widget_id| {
                resize_handle_widget_id_at_point(
                    &preview.frames,
                    origin,
                    zoom,
                    preview_viewport_size,
                    pointer_pos,
                    widget_id,
                )
            })
            .or_else(|| {
                topmost_widget_id_at_point(
                    &preview.frames,
                    origin,
                    zoom,
                    preview_viewport_size,
                    pointer_pos,
                )
                    .as_deref()
                    .and_then(|widget_id| {
                        resize_handle_widget_id_at_point(
                            &preview.frames,
                            origin,
                            zoom,
                            preview_viewport_size,
                            pointer_pos,
                            widget_id,
                        )
                    })
            })
    });

    if response.clicked() {
        next_selected_widget_id = hovered_widget_id.clone();
    }

    if response.drag_started() && next_canvas_interaction.is_none() {
        if let Some(widget_id) = resize_target_widget_id {
            next_selected_widget_id = Some(widget_id.clone());
            let start_size = find_widget(&layout.root, &widget_id)
                .map(|widget| widget.layout.size)
                .unwrap_or([16.0, 16.0]);
            let press_origin = drag_start_pos.unwrap_or(Pos2::new(rect.left(), rect.top()));
            next_canvas_interaction = Some(UiCanvasInteraction::ResizeWidget {
                widget_id,
                press_origin: [press_origin.x, press_origin.y],
                start_size,
            });
        } else if let Some(widget_id) = drag_start_pos
            .and_then(|pointer_pos| {
                topmost_widget_id_at_point(
                    &preview.frames,
                    origin,
                    zoom,
                    preview_viewport_size,
                    pointer_pos,
                )
            })
            .or(hovered_widget_id)
        {
            next_selected_widget_id = Some(widget_id.clone());
            let start_offset = find_widget(&layout.root, &widget_id)
                .map(|widget| widget.layout.offset)
                .unwrap_or([0.0, 0.0]);
            let press_origin = drag_start_pos.unwrap_or(Pos2::new(rect.left(), rect.top()));
            next_canvas_interaction = Some(UiCanvasInteraction::MoveWidget {
                widget_id,
                press_origin: [press_origin.x, press_origin.y],
                start_offset,
            });
        } else {
            next_canvas_interaction = Some(UiCanvasInteraction::Pan);
        }
    }

    let pointer_down = ui.ctx().input(|input| input.pointer.primary_down());
    let delta = ui.ctx().input(|input| input.pointer.delta());
    let current_pointer_pos = ui.ctx().input(|input| input.pointer.interact_pos());
    match next_canvas_interaction.as_ref() {
        Some(UiCanvasInteraction::Pan) if pointer_down && response.dragged() => {
            pan[0] += delta.x;
            pan[1] += delta.y;
            changed = true;
        }
        Some(UiCanvasInteraction::MoveWidget {
            widget_id,
            press_origin,
            start_offset,
        }) if pointer_down => {
            let root_level_widget = is_root_level_widget(layout, widget_id);
            if let (Some(widget), Some(pointer_pos)) = (
                find_widget_mut(&mut layout.root, widget_id),
                current_pointer_pos,
            ) {
                let total_delta = [
                    (pointer_pos.x - press_origin[0]) / zoom,
                    (pointer_pos.y - press_origin[1]) / zoom,
                ];
                widget.layout.offset[0] = snap_to_grid(start_offset[0] + total_delta[0]);
                widget.layout.offset[1] = snap_to_grid(start_offset[1] + total_delta[1]);
                if root_level_widget {
                    clamp_widget_layout_to_viewport(widget, preview_viewport_size);
                }
                changed = true;
            }
        }
        Some(UiCanvasInteraction::ResizeWidget {
            widget_id,
            press_origin,
            start_size,
        }) if pointer_down => {
            let root_level_widget = is_root_level_widget(layout, widget_id);
            if let (Some(widget), Some(pointer_pos)) = (
                find_widget_mut(&mut layout.root, widget_id),
                current_pointer_pos,
            ) {
                let total_delta = [
                    (pointer_pos.x - press_origin[0]) / zoom,
                    (pointer_pos.y - press_origin[1]) / zoom,
                ];
                let min_size = minimum_widget_size(widget);
                widget.layout.size[0] =
                    snap_to_grid((start_size[0] + total_delta[0]).max(min_size[0]));
                widget.layout.size[1] =
                    snap_to_grid((start_size[1] + total_delta[1]).max(min_size[1]));
                if root_level_widget {
                    clamp_widget_layout_to_viewport(widget, preview_viewport_size);
                }
                changed = true;
            }
        }
        _ => {}
    }

    if !pointer_down {
        next_canvas_interaction = None;
    }

    if response.double_clicked() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let mut widget = create_widget_preset(WidgetPreset::Label, layout);
            let widget_id = widget.id.to_string();
            widget.layout.offset = [
                snap_to_grid((pointer_pos.x - rect.left() - pan[0]) / zoom),
                snap_to_grid((pointer_pos.y - rect.top() - pan[1]) / zoom),
            ];
            if insert_child_widget(&mut layout.root, "root", widget) {
                next_selected_widget_id = Some(widget_id);
                changed = true;
            }
        }
    }

    let editor_state = &mut ui_state.ui_editor_context_mut().ui;
    editor_state.zoom = zoom;
    editor_state.pan = pan;
    editor_state.canvas_interaction = next_canvas_interaction;
    if next_selected_widget_id != editor_state.selected_widget_id {
        editor_state.selected_widget_id = next_selected_widget_id;
        editor_state.view_dirty = true;
    }

    changed
}

fn render_widget_tree(
    ui: &mut Ui,
    widget: &UiWidgetNode,
    selected_widget_id: Option<&str>,
    depth: usize,
    next_selected_widget_id: &mut Option<String>,
    remove_widget_id: &mut Option<String>,
) {
    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 12.0);
        let selected = selected_widget_id == Some(widget.id.as_str());
        let label = if widget.id.as_str() == "root" {
            "Viewport Root".to_string()
        } else {
            format!("{} ({})", widget.title, widget.id)
        };
        let response = ui.selectable_label(selected, label);
        if response.clicked() {
            *next_selected_widget_id = Some(widget.id.to_string());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widget.id.as_str() != "root"
                && ui
                    .small_button("🗑")
                    .on_hover_text("Delete widget")
                    .clicked()
            {
                *remove_widget_id = Some(widget.id.to_string());
            }
        });
    });
    for child in &widget.children {
        render_widget_tree(
            ui,
            child,
            selected_widget_id,
            depth + 1,
            next_selected_widget_id,
            remove_widget_id,
        );
    }
}

fn render_selected_widget_inspector(
    ui: &mut Ui,
    layout: &mut UiLayoutAsset,
    selected_widget_id: Option<&str>,
    authoring: &UiEditorAuthoringContext<'_>,
    next_selected_widget_id: &mut Option<String>,
) -> bool {
    let Some(selected_widget_id) = selected_widget_id else {
        ui.label("No widget selected.");
        return false;
    };
    let Some(widget) = find_widget_mut(&mut layout.root, selected_widget_id) else {
        ui.label("Selected widget no longer exists.");
        return false;
    };
    let widget_before = widget.clone();
    let mut changed = false;
    let is_root = widget.id.as_str() == "root";

    ui.heading("Selected Widget");
    ui.separator();
    if is_root {
        ui.small("Root acts as the logical viewport container. Root children are placed independently by anchor, offset, and size.");
        ui.separator();
    }
    ui.horizontal(|ui| {
        ui.label("Id:");
        if is_root {
            ui.monospace("root");
        } else {
            let mut widget_id = widget.id.to_string();
            if ui.text_edit_singleline(&mut widget_id).changed() {
                widget.id = widget_id.clone().into();
                *next_selected_widget_id = Some(widget_id);
                changed = true;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Title:");
        changed |= ui.text_edit_singleline(&mut widget.title).changed();
    });
    if !is_root {
        ui.horizontal(|ui| {
            ui.label("Kind:");
            let current_kind = widget_kind_choice(widget);
            let mut selected_kind = current_kind;
            egui::ComboBox::from_id_salt(("ui_widget_kind", widget.id.as_str()))
                .selected_text(widget_kind_choice_label(current_kind))
                .show_ui(ui, |ui| {
                    for candidate in [
                        WidgetKindChoice::Label,
                        WidgetKindChoice::ProgressBar,
                        WidgetKindChoice::GridContainer,
                    ] {
                        ui.selectable_value(
                            &mut selected_kind,
                            candidate,
                            widget_kind_choice_label(candidate),
                        );
                    }
                });
            if selected_kind != current_kind {
                widget.kind = default_widget_kind(selected_kind);
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            changed |= render_optional_ui_event_id_editor(
                ui,
                ("ui_widget_event", widget.id.as_str()),
                "Event Id:",
                &mut widget.event_id,
                authoring.declared_ui_events,
            );
        });
        ui.horizontal(|ui| {
            ui.label("Visible If:");
            changed |= render_optional_template_editor(
                ui,
                ("ui_widget_visible_if", widget.id.as_str()),
                &mut widget.visible_if,
                &bool_expression_choices(authoring.declared_flags),
                "None",
                "Custom",
            );
        });
        ui.horizontal(|ui| {
            ui.label("Enabled If:");
            changed |= render_optional_template_editor(
                ui,
                ("ui_widget_enabled_if", widget.id.as_str()),
                &mut widget.enabled_if,
                &bool_expression_choices(authoring.declared_flags),
                "None",
                "Custom",
            );
        });
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut widget.focusable, "Focusable").changed();
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Position:");
            for preset in [
                WidgetPositionPreset::TopLeft,
                WidgetPositionPreset::TopRight,
                WidgetPositionPreset::BottomLeft,
                WidgetPositionPreset::BottomRight,
                WidgetPositionPreset::Center,
            ] {
                if ui.button(widget_position_label(preset)).clicked() {
                    apply_widget_position_preset(widget, preset, authoring.preview_viewport_size);
                    changed = true;
                }
            }
        });
    }
    ui.separator();
    render_layout_spec_editor(ui, &mut widget.layout);
    ui.separator();
    render_typography_editor(ui, &mut widget.style.typography, authoring.font_choices);
    ui.separator();
    if is_root {
        ui.small(
            "Root has no per-widget content. Add or select a child widget to author HUD content.",
        );
    } else {
        render_widget_kind_editor(
            ui,
            widget,
            authoring.declared_flags,
            authoring.declared_ui_events,
        );
    }
    changed |= *widget != widget_before;
    changed
}

fn render_layout_spec_editor(ui: &mut Ui, layout: &mut toki_core::ui_layout::UiLayoutSpec) {
    ui.collapsing("Layout", |ui| {
        ui.horizontal(|ui| {
            ui.label("Anchor:");
            egui::ComboBox::from_id_salt(("ui_widget_anchor", layout as *const _ as usize))
                .selected_text(match layout.anchor {
                    UiAnchor::TopLeft => "TopLeft",
                    UiAnchor::TopRight => "TopRight",
                    UiAnchor::BottomLeft => "BottomLeft",
                    UiAnchor::BottomRight => "BottomRight",
                    UiAnchor::Center => "Center",
                    UiAnchor::Stretch => "Stretch",
                })
                .show_ui(ui, |ui| {
                    for (anchor, label) in [
                        (UiAnchor::TopLeft, "TopLeft"),
                        (UiAnchor::TopRight, "TopRight"),
                        (UiAnchor::BottomLeft, "BottomLeft"),
                        (UiAnchor::BottomRight, "BottomRight"),
                        (UiAnchor::Center, "Center"),
                        (UiAnchor::Stretch, "Stretch"),
                    ] {
                        ui.selectable_value(&mut layout.anchor, anchor, label);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Offset:");
            ui.add(egui::DragValue::new(&mut layout.offset[0]).speed(1.0));
            ui.add(egui::DragValue::new(&mut layout.offset[1]).speed(1.0));
        });
        ui.horizontal(|ui| {
            ui.label("Size:");
            ui.add(
                egui::DragValue::new(&mut layout.size[0])
                    .speed(1.0)
                    .range(1.0..=4096.0),
            );
            ui.add(
                egui::DragValue::new(&mut layout.size[1])
                    .speed(1.0)
                    .range(1.0..=4096.0),
            );
        });
    });
}

fn render_typography_editor(ui: &mut Ui, typography: &mut UiTypography, font_choices: &[String]) {
    ui.collapsing("Typography", |ui| {
        ui.horizontal(|ui| {
            ui.label("Font:");
            let mut selected = typography
                .font_family
                .clone()
                .unwrap_or_else(|| "Theme default".to_string());
            egui::ComboBox::from_id_salt(("ui_typography_font", typography as *const _ as usize))
                .selected_text(selected.clone())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut selected,
                        "Theme default".to_string(),
                        "Theme default",
                    );
                    for family in font_choices {
                        ui.selectable_value(&mut selected, family.clone(), family);
                    }
                });
            typography.font_family = if selected == "Theme default" {
                None
            } else {
                Some(selected)
            };
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            let mut font_size = typography.font_size_px.unwrap_or(8) as i32;
            if ui
                .add(egui::DragValue::new(&mut font_size).range(6..=128))
                .changed()
            {
                typography.font_size_px = Some(font_size.max(6) as u16);
            }
            if ui.small_button("Theme").clicked() {
                typography.font_size_px = None;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Weight:");
            let mut weight = typography.weight;
            egui::ComboBox::from_id_salt(("ui_typography_weight", typography as *const _ as usize))
                .selected_text(match weight {
                    None => "Theme",
                    Some(TextWeight::Normal) => "Normal",
                    Some(TextWeight::Bold) => "Bold",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut weight, None, "Theme");
                    ui.selectable_value(&mut weight, Some(TextWeight::Normal), "Normal");
                    ui.selectable_value(&mut weight, Some(TextWeight::Bold), "Bold");
                });
            typography.weight = weight;
        });

        ui.horizontal(|ui| {
            ui.label("Slant:");
            let mut slant = typography.slant;
            egui::ComboBox::from_id_salt(("ui_typography_slant", typography as *const _ as usize))
                .selected_text(match slant {
                    None => "Theme",
                    Some(TextSlant::Normal) => "Normal",
                    Some(TextSlant::Italic) => "Italic",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut slant, None, "Theme");
                    ui.selectable_value(&mut slant, Some(TextSlant::Normal), "Normal");
                    ui.selectable_value(&mut slant, Some(TextSlant::Italic), "Italic");
                });
            typography.slant = slant;
        });

        ui.horizontal(|ui| {
            ui.label("Anchor:");
            let mut anchor = typography.anchor;
            egui::ComboBox::from_id_salt(("ui_typography_anchor", typography as *const _ as usize))
                .selected_text(match anchor {
                    None => "Auto",
                    Some(TextAnchor::TopLeft) => "TopLeft",
                    Some(TextAnchor::TopCenter) => "TopCenter",
                    Some(TextAnchor::TopRight) => "TopRight",
                    Some(TextAnchor::CenterLeft) => "CenterLeft",
                    Some(TextAnchor::Center) => "Center",
                    Some(TextAnchor::CenterRight) => "CenterRight",
                    Some(TextAnchor::BottomLeft) => "BottomLeft",
                    Some(TextAnchor::BottomCenter) => "BottomCenter",
                    Some(TextAnchor::BottomRight) => "BottomRight",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut anchor, None, "Auto");
                    for (candidate, label) in [
                        (TextAnchor::TopLeft, "TopLeft"),
                        (TextAnchor::TopCenter, "TopCenter"),
                        (TextAnchor::TopRight, "TopRight"),
                        (TextAnchor::CenterLeft, "CenterLeft"),
                        (TextAnchor::Center, "Center"),
                        (TextAnchor::CenterRight, "CenterRight"),
                        (TextAnchor::BottomLeft, "BottomLeft"),
                        (TextAnchor::BottomCenter, "BottomCenter"),
                        (TextAnchor::BottomRight, "BottomRight"),
                    ] {
                        ui.selectable_value(&mut anchor, Some(candidate), label);
                    }
                });
            typography.anchor = anchor;
        });
    });
}

fn render_widget_kind_editor(
    ui: &mut Ui,
    widget: &mut UiWidgetNode,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
) {
    match &mut widget.kind {
        UiWidgetKind::Label { content } => render_text_template_editor(ui, content, declared_flags),
        UiWidgetKind::Button { label } => render_text_template_editor(ui, label, declared_flags),
        UiWidgetKind::ProgressBar { value } => {
            render_progress_binding_editor(ui, value, declared_flags)
        }
        UiWidgetKind::Slider {
            value,
            step_percent,
            show_value,
        } => {
            render_progress_binding_editor(ui, value, declared_flags);
            ui.horizontal(|ui| {
                ui.label("Step %:");
                let mut slider_step = *step_percent as i32;
                if ui
                    .add(egui::DragValue::new(&mut slider_step).range(0..=100))
                    .changed()
                {
                    *step_percent = slider_step.clamp(0, 100) as u8;
                }
            });
            ui.checkbox(show_value, "Show Value");
        }
        UiWidgetKind::GridContainer { columns, spacing } => {
            ui.horizontal(|ui| {
                ui.label("Columns:");
                let mut value = *columns as i32;
                if ui
                    .add(egui::DragValue::new(&mut value).range(1..=16))
                    .changed()
                {
                    *columns = value.max(1) as u16;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Spacing:");
                let mut horizontal = spacing.left as i32;
                let mut vertical = spacing.top as i32;
                if ui
                    .add(egui::DragValue::new(&mut horizontal).range(0..=256))
                    .changed()
                {
                    spacing.left = horizontal.max(0) as u16;
                    spacing.right = 0;
                }
                if ui
                    .add(egui::DragValue::new(&mut vertical).range(0..=256))
                    .changed()
                {
                    spacing.top = vertical.max(0) as u16;
                    spacing.bottom = 0;
                }
            });
        }
    }

    let issues = validate_widget(widget, declared_flags, declared_ui_events);
    for issue in issues {
        ui.colored_label(Color32::from_rgb(255, 210, 80), issue);
    }
}

fn render_text_template_editor(
    ui: &mut Ui,
    template: &mut UiTextTemplate,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) {
    if template.segments.is_empty() {
        template.segments.push(UiTextSegment::Literal {
            text: "Label".to_string(),
        });
    }
    ui.collapsing("Text Content", |ui| {
        let segment_count = template.segments.len();
        let mut remove_index = None::<usize>;
        for (index, segment) in template.segments.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Segment {}", index + 1));
                    if ui.small_button("Remove").clicked() && segment_count > 1 {
                        remove_index = Some(index);
                    }
                });
                match segment {
                    UiTextSegment::Literal { text } => {
                        ui.text_edit_singleline(text);
                    }
                    UiTextSegment::Binding { binding } => {
                        render_binding_editor(
                            ui,
                            binding,
                            ("ui_text_binding", index),
                            declared_flags,
                        );
                    }
                }
                let mut kind = match segment {
                    UiTextSegment::Literal { .. } => 0,
                    UiTextSegment::Binding { .. } => 1,
                };
                egui::ComboBox::from_id_salt(("ui_text_segment_kind", index))
                    .selected_text(if kind == 0 { "Literal" } else { "Binding" })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut kind, 0, "Literal");
                        ui.selectable_value(&mut kind, 1, "Binding");
                    });
                let is_literal = matches!(segment, UiTextSegment::Literal { .. });
                if kind == 0 && !is_literal {
                    *segment = UiTextSegment::Literal {
                        text: String::new(),
                    };
                } else if kind == 1 && is_literal {
                    *segment = UiTextSegment::Binding {
                        binding: UiBinding::ValuePath {
                            path: "flags.example".to_string(),
                            key: None,
                        },
                    };
                }
            });
        }
        if let Some(remove_index) = remove_index {
            template.segments.remove(remove_index);
        }
        if ui.button("+ Add Segment").clicked() {
            template.segments.push(UiTextSegment::Literal {
                text: "Text".to_string(),
            });
        }
    });
}

fn render_progress_binding_editor(
    ui: &mut Ui,
    binding: &mut UiProgressBinding,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) {
    ui.collapsing("Progress Binding", |ui| {
        let mut mode = match binding {
            UiProgressBinding::CurrentMax { .. } => 0,
            UiProgressBinding::Percent { .. } => 1,
        };
        egui::ComboBox::from_id_salt(("ui_progress_mode", binding as *const _ as usize))
            .selected_text(if mode == 0 { "Current/Max" } else { "Percent" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, 0, "Current/Max");
                ui.selectable_value(&mut mode, 1, "Percent");
            });
        if mode == 0 && matches!(binding, UiProgressBinding::Percent { .. }) {
            *binding = UiProgressBinding::CurrentMax {
                current: UiBinding::ValuePath {
                    path: "player.health".to_string(),
                    key: None,
                },
                max: UiBinding::ValuePath {
                    path: "player.max_health".to_string(),
                    key: None,
                },
            };
        } else if mode == 1 && matches!(binding, UiProgressBinding::CurrentMax { .. }) {
            *binding = UiProgressBinding::Percent {
                percent: UiBinding::Expression {
                    expression: "100".to_string(),
                    key: None,
                },
            };
        }
        match binding {
            UiProgressBinding::CurrentMax { current, max } => {
                ui.label("Current");
                render_binding_editor(ui, current, ("ui_progress_current", 0), declared_flags);
                ui.label("Max");
                render_binding_editor(ui, max, ("ui_progress_max", 0), declared_flags);
            }
            UiProgressBinding::Percent { percent } => {
                ui.label("Percent");
                render_binding_editor(ui, percent, ("ui_progress_percent", 0), declared_flags);
            }
        }
    });
}

fn render_binding_editor(
    ui: &mut Ui,
    binding: &mut UiBinding,
    id_salt: impl std::hash::Hash + Copy,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) {
    let mut mode = match binding {
        UiBinding::ValuePath { .. } => 0,
        UiBinding::Expression { .. } => 1,
    };
    egui::ComboBox::from_id_salt(("ui_binding_mode", id_salt))
        .selected_text(if mode == 0 { "ValuePath" } else { "Expression" })
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut mode, 0, "ValuePath");
            ui.selectable_value(&mut mode, 1, "Expression");
        });
    if mode == 0 && matches!(binding, UiBinding::Expression { .. }) {
        *binding = UiBinding::ValuePath {
            path: "flags.example".to_string(),
            key: None,
        };
    } else if mode == 1 && matches!(binding, UiBinding::ValuePath { .. }) {
        *binding = UiBinding::Expression {
            expression: "1".to_string(),
            key: None,
        };
    }
    match binding {
        UiBinding::ValuePath { path, key } => {
            render_value_path_picker(ui, ("ui_binding_path", id_salt), path, declared_flags);
            ui.horizontal(|ui| {
                ui.label("Override Key:");
                let mut key_value = key.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut key_value).changed() {
                    *key = if key_value.trim().is_empty() {
                        None
                    } else {
                        Some(key_value)
                    };
                }
            });
        }
        UiBinding::Expression { expression, key } => {
            render_expression_template_editor(
                ui,
                ("ui_binding_expression", id_salt),
                expression,
                &numeric_expression_choices(declared_flags),
                "Custom",
            );
            ui.horizontal(|ui| {
                ui.label("Override Key:");
                let mut key_value = key.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut key_value).changed() {
                    *key = if key_value.trim().is_empty() {
                        None
                    } else {
                        Some(key_value)
                    };
                }
            });
        }
    }
}

fn render_value_path_picker(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + Copy,
    path: &mut String,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) {
    let choices = value_path_choices(declared_flags);
    ui.horizontal(|ui| {
        ui.label("Path:");
        let current_choice = if choices.iter().any(|choice| choice == path) {
            path.clone()
        } else {
            "Custom".to_string()
        };
        let mut selected = current_choice;
        egui::ComboBox::from_id_salt(("ui_value_path_choice", id_salt))
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                for choice in &choices {
                    ui.selectable_value(&mut selected, choice.clone(), choice);
                }
                ui.selectable_value(&mut selected, "Custom".to_string(), "Custom");
            });
        if selected != "Custom" && selected != *path {
            *path = selected;
        } else if selected == "Custom" && choices.iter().any(|choice| choice == path) {
            path.clear();
        }
    });
    if !choices.iter().any(|choice| choice == path) {
        ui.horizontal(|ui| {
            ui.label("Custom:");
            ui.text_edit_singleline(path);
        });
    }
}

fn render_expression_template_editor(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + Copy,
    expression: &mut String,
    templates: &[String],
    custom_label: &str,
) {
    ui.horizontal(|ui| {
        ui.label("Expression:");
        let current_choice = if templates.iter().any(|choice| choice == expression) {
            expression.clone()
        } else {
            custom_label.to_string()
        };
        let mut selected = current_choice;
        egui::ComboBox::from_id_salt(("ui_expression_template", id_salt))
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                for choice in templates {
                    ui.selectable_value(&mut selected, choice.clone(), choice);
                }
                ui.selectable_value(&mut selected, custom_label.to_string(), custom_label);
            });
        if selected != custom_label && selected != *expression {
            *expression = selected;
        } else if selected == custom_label && templates.iter().any(|choice| choice == expression) {
            expression.clear();
        }
    });
    if !templates.iter().any(|choice| choice == expression) {
        ui.horizontal(|ui| {
            ui.label("Custom:");
            ui.text_edit_singleline(expression);
        });
    }
}

fn render_optional_template_editor(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + Copy,
    value: &mut Option<String>,
    templates: &[String],
    none_label: &str,
    custom_label: &str,
) -> bool {
    let before = value.clone();
    let current_text = value.clone().unwrap_or_default();
    let current_choice = if value.is_none() {
        none_label.to_string()
    } else if templates.iter().any(|choice| choice == &current_text) {
        current_text.clone()
    } else {
        custom_label.to_string()
    };
    let mut selected = current_choice;
    egui::ComboBox::from_id_salt(("ui_optional_template", id_salt))
        .selected_text(selected.clone())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, none_label.to_string(), none_label);
            for choice in templates {
                ui.selectable_value(&mut selected, choice.clone(), choice);
            }
            ui.selectable_value(&mut selected, custom_label.to_string(), custom_label);
        });
    if selected == none_label {
        *value = None;
    } else if selected != custom_label {
        *value = Some(selected);
    } else if value
        .as_ref()
        .is_none_or(|expr| templates.iter().any(|choice| choice == expr))
    {
        *value = Some(String::new());
    }
    if value
        .as_ref()
        .is_some_and(|expr| !templates.iter().any(|choice| choice == expr))
    {
        let mut custom_value = current_text;
        ui.horizontal(|ui| {
            ui.label("Custom:");
            if ui.text_edit_singleline(&mut custom_value).changed() {
                *value = if custom_value.trim().is_empty() {
                    None
                } else {
                    Some(custom_value.trim().to_string())
                };
            }
        });
    }
    *value != before
}

fn value_path_choices(
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) -> Vec<String> {
    let mut choices = vec![
        "player.health".to_string(),
        "player.max_health".to_string(),
        "player.active".to_string(),
        "player.kind".to_string(),
        "self.health".to_string(),
        "self.max_health".to_string(),
        "self.active".to_string(),
        "self.kind".to_string(),
        "target.health".to_string(),
        "target.max_health".to_string(),
        "target.active".to_string(),
        "target.kind".to_string(),
    ];
    choices.extend(
        declared_flags
            .iter()
            .map(|flag| format!("flags.{}", flag.id))
            .collect::<Vec<_>>(),
    );
    choices.sort();
    choices.dedup();
    choices
}

fn bool_expression_choices(
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) -> Vec<String> {
    let mut choices = vec![
        "player.active".to_string(),
        "self.active".to_string(),
        "target.active".to_string(),
        "player.health > 0".to_string(),
        "self.health > 0".to_string(),
        "target.health > 0".to_string(),
    ];
    choices.extend(
        declared_flags
            .iter()
            .filter_map(|flag| match flag.default_value {
                FlagValue::Bool(_) => Some(format!("flags.{}", flag.id)),
                _ => None,
            }),
    );
    choices.extend(
        declared_flags
            .iter()
            .filter_map(|flag| match flag.default_value {
                FlagValue::Bool(_) => Some(format!("!flags.{}", flag.id)),
                _ => None,
            }),
    );
    choices.sort();
    choices.dedup();
    choices
}

fn numeric_expression_choices(
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) -> Vec<String> {
    let mut choices = vec![
        "0".to_string(),
        "1".to_string(),
        "10".to_string(),
        "25".to_string(),
        "50".to_string(),
        "100".to_string(),
        "player.health".to_string(),
        "player.max_health".to_string(),
        "self.health".to_string(),
        "target.health".to_string(),
        "min(player.health, player.max_health)".to_string(),
    ];
    choices.extend(
        declared_flags
            .iter()
            .filter_map(|flag| match flag.default_value {
                FlagValue::Int(_) => Some(format!("flags.{}", flag.id)),
                _ => None,
            }),
    );
    choices.sort();
    choices.dedup();
    choices
}

struct PreviewOutput {
    composition: UiComposition,
    frames: Vec<UiWidgetFrame>,
}

struct UiEditorAuthoringContext<'a> {
    declared_flags: &'a [toki_core::project_runtime::ProjectFlagDefinition],
    declared_ui_events: &'a [toki_core::project_runtime::ProjectUiEventDefinition],
    font_choices: &'a [String],
    preview_viewport_size: glam::Vec2,
}

fn compute_preview(
    root: &UiWidgetNode,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    preview_viewport_size: glam::Vec2,
) -> PreviewOutput {
    let entity_manager = toki_core::entity::EntityManager::default();
    let flags = GameFlags::default();
    let trigger = TriggerContext::default();
    let overrides = HashMap::<String, FlagValue>::new();
    let layout = UiLayoutAsset {
        id: "preview".into(),
        title: "Preview".to_string(),
        startup_visible: true,
        z_order: 0,
        root: root.clone(),
    };
    let output = UiLayoutEngine::compose(
        &layout,
        &UiTheme::default(),
        preview_viewport_size,
        UiBindingContext {
            value_paths: ValuePathContext {
                entity_manager: &entity_manager,
                game_flags: &flags,
                player_id: None,
                trigger_context: &trigger,
            },
            binding_overrides: &overrides,
            declared_flags,
        },
        None,
    );
    PreviewOutput {
        composition: output.composition,
        frames: output.widget_frames,
    }
}

fn validate_ui_layout(
    layout: &UiLayoutAsset,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
) -> Vec<String> {
    let mut issues = Vec::new();
    if layout.id.as_str().trim().is_empty() {
        issues.push("Layout id must not be empty".to_string());
    }
    let mut seen = BTreeSet::<String>::new();
    validate_widget_recursive(
        &layout.root,
        declared_flags,
        declared_ui_events,
        &mut seen,
        &mut issues,
    );
    issues
}

fn validate_widget_recursive(
    widget: &UiWidgetNode,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
    seen: &mut BTreeSet<String>,
    issues: &mut Vec<String>,
) {
    if widget.id.as_str().trim().is_empty() {
        issues.push("Widget id must not be empty".to_string());
    } else if !seen.insert(widget.id.to_string()) {
        issues.push(format!("Duplicate widget id '{}'", widget.id));
    }
    issues.extend(validate_widget(widget, declared_flags, declared_ui_events));
    for child in &widget.children {
        validate_widget_recursive(child, declared_flags, declared_ui_events, seen, issues);
    }
}

fn validate_widget(
    widget: &UiWidgetNode,
    _declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    declared_ui_events: &[toki_core::project_runtime::ProjectUiEventDefinition],
) -> Vec<String> {
    let mut issues = Vec::new();
    if let Some(event_id) = widget.event_id.as_deref() {
        if let Some(issue) = validate_ui_event_reference(event_id, declared_ui_events, "Widget event")
        {
            issues.push(issue);
        }
    }
    for (label, gate) in [
        ("visible_if", widget.visible_if.as_deref()),
        ("enabled_if", widget.enabled_if.as_deref()),
    ] {
        if let Some(expression) = gate {
            if let Err(error) = Expression::parse(expression) {
                issues.push(format!(
                    "{} on '{}' is invalid: {}",
                    label, widget.id, error
                ));
            }
        }
    }
    match &widget.kind {
        UiWidgetKind::Label { content } => {
            for segment in &content.segments {
                if let UiTextSegment::Binding { binding } = segment {
                    validate_binding(binding, &mut issues, &widget.id);
                }
            }
        }
        UiWidgetKind::Button { label } => {
            for segment in &label.segments {
                if let UiTextSegment::Binding { binding } = segment {
                    validate_binding(binding, &mut issues, &widget.id);
                }
            }
        }
        UiWidgetKind::ProgressBar { value } => match value {
            UiProgressBinding::CurrentMax { current, max } => {
                validate_binding(current, &mut issues, &widget.id);
                validate_binding(max, &mut issues, &widget.id);
            }
            UiProgressBinding::Percent { percent } => {
                validate_binding(percent, &mut issues, &widget.id);
            }
        },
        UiWidgetKind::Slider { value, .. } => match value {
            UiProgressBinding::CurrentMax { current, max } => {
                validate_binding(current, &mut issues, &widget.id);
                validate_binding(max, &mut issues, &widget.id);
            }
            UiProgressBinding::Percent { percent } => {
                validate_binding(percent, &mut issues, &widget.id);
            }
        },
        UiWidgetKind::GridContainer { columns, .. } => {
            if *columns == 0 {
                issues.push(format!(
                    "GridContainer '{}' must have at least one column",
                    widget.id
                ));
            }
        }
    }
    issues
}

fn validate_binding(
    binding: &UiBinding,
    issues: &mut Vec<String>,
    widget_id: &toki_core::UiWidgetId,
) {
    match binding {
        UiBinding::ValuePath { path, .. } => {
            if let Err(error) = ValuePath::parse(path) {
                issues.push(format!(
                    "Widget '{}' has invalid value path '{}': {}",
                    widget_id, path, error
                ));
            }
        }
        UiBinding::Expression { expression, .. } => {
            if let Err(error) = Expression::parse(expression) {
                issues.push(format!(
                    "Widget '{}' has invalid expression '{}': {}",
                    widget_id, expression, error
                ));
            }
        }
    }
}

fn widget_kind_choice(widget: &UiWidgetNode) -> WidgetKindChoice {
    match widget.kind {
        UiWidgetKind::Label { .. } => WidgetKindChoice::Label,
        UiWidgetKind::Button { .. } => WidgetKindChoice::Button,
        UiWidgetKind::ProgressBar { .. } => WidgetKindChoice::ProgressBar,
        UiWidgetKind::Slider { .. } => WidgetKindChoice::Slider,
        UiWidgetKind::GridContainer { .. } => WidgetKindChoice::GridContainer,
    }
}

fn widget_kind_choice_label(kind: WidgetKindChoice) -> &'static str {
    match kind {
        WidgetKindChoice::Label => "Label",
        WidgetKindChoice::Button => "Button",
        WidgetKindChoice::ProgressBar => "ProgressBar",
        WidgetKindChoice::Slider => "Slider",
        WidgetKindChoice::GridContainer => "GridContainer",
    }
}

fn widget_preset_label(preset: WidgetPreset) -> &'static str {
    match preset {
        WidgetPreset::Label => "Label",
        WidgetPreset::HealthBar => "Health Bar",
    }
}

fn widget_position_label(preset: WidgetPositionPreset) -> &'static str {
    match preset {
        WidgetPositionPreset::TopLeft => "Top Left",
        WidgetPositionPreset::TopRight => "Top Right",
        WidgetPositionPreset::BottomLeft => "Bottom Left",
        WidgetPositionPreset::BottomRight => "Bottom Right",
        WidgetPositionPreset::Center => "Center",
    }
}

fn default_widget_kind(kind: WidgetKindChoice) -> UiWidgetKind {
    match kind {
        WidgetKindChoice::Label => UiWidgetKind::Label {
            content: UiTextTemplate {
                segments: vec![UiTextSegment::Literal {
                    text: "Label".to_string(),
                }],
            },
        },
        WidgetKindChoice::Button => UiWidgetKind::Button {
            label: UiTextTemplate {
                segments: vec![UiTextSegment::Literal {
                    text: "Button".to_string(),
                }],
            },
        },
        WidgetKindChoice::ProgressBar => UiWidgetKind::ProgressBar {
            value: UiProgressBinding::CurrentMax {
                current: UiBinding::ValuePath {
                    path: "player.health".to_string(),
                    key: None,
                },
                max: UiBinding::Expression {
                    expression: "100".to_string(),
                    key: None,
                },
            },
        },
        WidgetKindChoice::Slider => UiWidgetKind::Slider {
            value: UiProgressBinding::Percent {
                percent: UiBinding::Expression {
                    expression: "50".to_string(),
                    key: None,
                },
            },
            step_percent: 5,
            show_value: true,
        },
        WidgetKindChoice::GridContainer => UiWidgetKind::GridContainer {
            columns: 2,
            spacing: UiSpacing::default(),
        },
    }
}

fn create_widget(kind: WidgetKindChoice, layout: &UiLayoutAsset) -> UiWidgetNode {
    let id = unique_widget_id(&layout.root, kind);
    UiWidgetNode {
        id: id.clone().into(),
        title: id.clone(),
        layout: ui_layout_spec_for_kind(kind),
        style: hud_widget_style(kind),
        event_id: None,
        focusable: false,
        visible_if: None,
        enabled_if: None,
        kind: default_widget_kind(kind),
        children: Vec::new(),
    }
}

fn create_widget_preset(preset: WidgetPreset, layout: &UiLayoutAsset) -> UiWidgetNode {
    match preset {
        WidgetPreset::Label => create_widget(WidgetKindChoice::Label, layout),
        WidgetPreset::HealthBar => {
            let mut widget = create_widget(WidgetKindChoice::ProgressBar, layout);
            widget.title = "Health Bar".to_string();
            widget.layout.size = [72.0, 12.0];
            widget.layout.offset = [8.0, 28.0];
            widget.layout.margin = ZERO_SPACING;
            widget.layout.padding = ZERO_SPACING;
            widget.kind = UiWidgetKind::ProgressBar {
                value: UiProgressBinding::CurrentMax {
                    current: UiBinding::ValuePath {
                        path: "player.health".to_string(),
                        key: None,
                    },
                    max: UiBinding::ValuePath {
                        path: "player.max_health".to_string(),
                        key: None,
                    },
                },
            };
            widget
        }
    }
}

fn ui_layout_spec_for_kind(kind: WidgetKindChoice) -> toki_core::ui_layout::UiLayoutSpec {
    toki_core::ui_layout::UiLayoutSpec {
        size: match kind {
            WidgetKindChoice::Label => [96.0, 18.0],
            WidgetKindChoice::Button => [96.0, 22.0],
            WidgetKindChoice::ProgressBar => [80.0, 12.0],
            WidgetKindChoice::Slider => [96.0, 18.0],
            WidgetKindChoice::GridContainer => [96.0, 48.0],
        },
        offset: [UI_EDGE_MARGIN, UI_EDGE_MARGIN],
        margin: ZERO_SPACING,
        padding: ZERO_SPACING,
        ..Default::default()
    }
}

fn hud_widget_style(kind: WidgetKindChoice) -> toki_core::ui_layout::UiWidgetStyle {
    let font_size_px = match kind {
        WidgetKindChoice::Label => Some(8),
        WidgetKindChoice::Button => Some(8),
        WidgetKindChoice::ProgressBar => Some(8),
        WidgetKindChoice::Slider => Some(8),
        WidgetKindChoice::GridContainer => None,
    };
    toki_core::ui_layout::UiWidgetStyle {
        typography: UiTypography {
            font_size_px,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn apply_widget_position_preset(
    widget: &mut UiWidgetNode,
    preset: WidgetPositionPreset,
    preview_viewport_size: glam::Vec2,
) {
    let size = widget.layout.size;
    match preset {
        WidgetPositionPreset::TopLeft => {
            widget.layout.anchor = UiAnchor::TopLeft;
            widget.layout.offset = [UI_EDGE_MARGIN, UI_EDGE_MARGIN];
        }
        WidgetPositionPreset::TopRight => {
            widget.layout.anchor = UiAnchor::TopRight;
            widget.layout.offset = [-UI_EDGE_MARGIN, UI_EDGE_MARGIN];
        }
        WidgetPositionPreset::BottomLeft => {
            widget.layout.anchor = UiAnchor::BottomLeft;
            widget.layout.offset = [UI_EDGE_MARGIN, -UI_EDGE_MARGIN];
        }
        WidgetPositionPreset::BottomRight => {
            widget.layout.anchor = UiAnchor::BottomRight;
            widget.layout.offset = [-UI_EDGE_MARGIN, -UI_EDGE_MARGIN];
        }
        WidgetPositionPreset::Center => {
            widget.layout.anchor = UiAnchor::Center;
            widget.layout.offset = [0.0, 0.0];
            widget.layout.size[0] = widget.layout.size[0].min(preview_viewport_size.x.max(1.0));
            widget.layout.size[1] = widget.layout.size[1].min(preview_viewport_size.y.max(1.0));
        }
    }
    widget.layout.offset[0] = snap_to_grid(widget.layout.offset[0]);
    widget.layout.offset[1] = snap_to_grid(widget.layout.offset[1]);
    let min_size = minimum_widget_size(widget);
    widget.layout.size[0] = snap_to_grid(size[0]).max(min_size[0]);
    widget.layout.size[1] = snap_to_grid(size[1]).max(min_size[1]);
    clamp_widget_layout_to_viewport(widget, preview_viewport_size);
}

fn insert_new_widget_under_parent(
    ui_state: &mut EditorUI,
    layout: &mut UiLayoutAsset,
    parent_id: &str,
    widget: UiWidgetNode,
    status_message: &str,
) -> bool {
    let widget_id = widget.id.to_string();
    if insert_child_widget(&mut layout.root, parent_id, widget) {
        let editor_state = &mut ui_state.ui_editor_context_mut().ui;
        editor_state.select_widget(widget_id);
        editor_state.status_message = Some(status_message.to_string());
        true
    } else {
        false
    }
}

fn minimum_widget_size(widget: &UiWidgetNode) -> [f32; 2] {
    match widget.kind {
        UiWidgetKind::Label { .. } => [48.0, 16.0],
        UiWidgetKind::Button { .. } => [56.0, 18.0],
        UiWidgetKind::ProgressBar { .. } => [48.0, 12.0],
        UiWidgetKind::Slider { .. } => [64.0, 16.0],
        UiWidgetKind::GridContainer { .. } => [56.0, 24.0],
    }
}

fn is_root_level_widget(layout: &UiLayoutAsset, widget_id: &str) -> bool {
    find_parent_id(&layout.root, widget_id).as_deref() == Some("root")
}

fn clamp_widget_layout_to_viewport(widget: &mut UiWidgetNode, viewport_size: glam::Vec2) {
    let min_size = minimum_widget_size(widget);
    let max_width = viewport_size.x.max(min_size[0]);
    let max_height = viewport_size.y.max(min_size[1]);
    widget.layout.size[0] = widget.layout.size[0].clamp(min_size[0], max_width);
    widget.layout.size[1] = widget.layout.size[1].clamp(min_size[1], max_height);

    let max_x = (viewport_size.x - widget.layout.size[0]).max(0.0);
    let max_y = (viewport_size.y - widget.layout.size[1]).max(0.0);
    match widget.layout.anchor {
        UiAnchor::TopLeft => {
            widget.layout.offset[0] = widget.layout.offset[0].clamp(0.0, max_x);
            widget.layout.offset[1] = widget.layout.offset[1].clamp(0.0, max_y);
        }
        UiAnchor::TopRight => {
            widget.layout.offset[0] = widget.layout.offset[0].clamp(-max_x, 0.0);
            widget.layout.offset[1] = widget.layout.offset[1].clamp(0.0, max_y);
        }
        UiAnchor::BottomLeft => {
            widget.layout.offset[0] = widget.layout.offset[0].clamp(0.0, max_x);
            widget.layout.offset[1] = widget.layout.offset[1].clamp(-max_y, 0.0);
        }
        UiAnchor::BottomRight => {
            widget.layout.offset[0] = widget.layout.offset[0].clamp(-max_x, 0.0);
            widget.layout.offset[1] = widget.layout.offset[1].clamp(-max_y, 0.0);
        }
        UiAnchor::Center => {
            widget.layout.offset[0] = widget.layout.offset[0].clamp(-max_x * 0.5, max_x * 0.5);
            widget.layout.offset[1] = widget.layout.offset[1].clamp(-max_y * 0.5, max_y * 0.5);
        }
        UiAnchor::Stretch => {
            widget.layout.offset[0] = widget.layout.offset[0].clamp(0.0, viewport_size.x * 0.5);
            widget.layout.offset[1] = widget.layout.offset[1].clamp(0.0, viewport_size.y * 0.5);
        }
    }

    widget.layout.offset[0] = snap_to_grid(widget.layout.offset[0]);
    widget.layout.offset[1] = snap_to_grid(widget.layout.offset[1]);
    widget.layout.size[0] = snap_to_grid(widget.layout.size[0]).max(min_size[0]);
    widget.layout.size[1] = snap_to_grid(widget.layout.size[1]).max(min_size[1]);
}

fn preferred_parent_id(layout: &UiLayoutAsset, selected_widget_id: Option<&str>) -> String {
    selected_widget_id
        .and_then(|widget_id| {
            find_widget(&layout.root, widget_id).and_then(|widget| {
                if can_have_children(widget) {
                    Some(widget.id.to_string())
                } else {
                    find_parent_id(&layout.root, widget_id)
                }
            })
        })
        .unwrap_or_else(|| "root".to_string())
}

fn snap_to_grid(value: f32) -> f32 {
    (value / UI_SNAP_GRID).round() * UI_SNAP_GRID
}

fn can_have_children(widget: &UiWidgetNode) -> bool {
    matches!(widget.kind, UiWidgetKind::GridContainer { .. })
}

fn unique_widget_id(root: &UiWidgetNode, kind: WidgetKindChoice) -> String {
    let prefix = match kind {
        WidgetKindChoice::Label => "label",
        WidgetKindChoice::Button => "button",
        WidgetKindChoice::ProgressBar => "progress",
        WidgetKindChoice::Slider => "slider",
        WidgetKindChoice::GridContainer => "grid",
    };
    let existing = collect_widget_ids(root);
    let mut index = 1usize;
    loop {
        let candidate = format!("{prefix}_{index}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
        index += 1;
    }
}

fn collect_widget_ids(root: &UiWidgetNode) -> BTreeSet<&str> {
    let mut ids = BTreeSet::new();
    collect_widget_ids_recursive(root, &mut ids);
    ids
}

fn collect_widget_ids_recursive<'a>(widget: &'a UiWidgetNode, ids: &mut BTreeSet<&'a str>) {
    ids.insert(widget.id.as_str());
    for child in &widget.children {
        collect_widget_ids_recursive(child, ids);
    }
}

fn find_widget<'a>(widget: &'a UiWidgetNode, widget_id: &str) -> Option<&'a UiWidgetNode> {
    if widget.id.as_str() == widget_id {
        return Some(widget);
    }
    widget
        .children
        .iter()
        .find_map(|child| find_widget(child, widget_id))
}

fn find_widget_mut<'a>(
    widget: &'a mut UiWidgetNode,
    widget_id: &str,
) -> Option<&'a mut UiWidgetNode> {
    if widget.id.as_str() == widget_id {
        return Some(widget);
    }
    for child in &mut widget.children {
        if let Some(found) = find_widget_mut(child, widget_id) {
            return Some(found);
        }
    }
    None
}

fn find_parent_id(widget: &UiWidgetNode, widget_id: &str) -> Option<String> {
    for child in &widget.children {
        if child.id.as_str() == widget_id {
            return Some(widget.id.to_string());
        }
        if let Some(found) = find_parent_id(child, widget_id) {
            return Some(found);
        }
    }
    None
}

fn insert_child_widget(root: &mut UiWidgetNode, parent_id: &str, child: UiWidgetNode) -> bool {
    if let Some(parent) = find_widget_mut(root, parent_id) {
        parent.children.push(child);
        return true;
    }
    false
}

fn remove_widget_by_id(root: &mut UiWidgetNode, widget_id: &str) -> Option<String> {
    let position = root
        .children
        .iter()
        .position(|child| child.id.as_str() == widget_id);
    if let Some(position) = position {
        root.children.remove(position);
        return Some(root.id.to_string());
    }
    for child in &mut root.children {
        if let Some(parent_id) = remove_widget_by_id(child, widget_id) {
            return Some(parent_id);
        }
    }
    None
}

fn duplicate_widget(root: &mut UiWidgetNode, widget_id: &str) -> Option<String> {
    duplicate_widget_in_parent(root, widget_id)
}

fn duplicate_widget_in_parent(parent: &mut UiWidgetNode, widget_id: &str) -> Option<String> {
    if let Some(index) = parent
        .children
        .iter()
        .position(|child| child.id.as_str() == widget_id)
    {
        let existing = collect_widget_ids(parent);
        let mut clone = parent.children[index].clone();
        clone.id = make_duplicate_id(&existing, clone.id.as_str()).into();
        clone.title = format!("{} Copy", clone.title);
        let new_id = clone.id.to_string();
        parent.children.insert(index + 1, clone);
        return Some(new_id);
    }
    for child in &mut parent.children {
        if let Some(result) = duplicate_widget_in_parent(child, widget_id) {
            return Some(result);
        }
    }
    None
}

fn make_duplicate_id(existing: &BTreeSet<&str>, source_id: &str) -> String {
    let mut index = 1usize;
    loop {
        let candidate = format!("{source_id}_copy_{index}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
        index += 1;
    }
}

fn paint_ui_composition(
    painter: &egui::Painter,
    composition: &UiComposition,
    origin: glam::Vec2,
    scale: f32,
    logical_viewport_size: glam::Vec2,
    available_fonts: &[String],
) {
    let transformed = transform_logical_ui_composition_with_transform(
        composition,
        ui_presentation_transform(origin, scale, logical_viewport_size, logical_viewport_size * scale),
    );
    for block in &transformed.blocks {
        paint_ui_block(painter, block, available_fonts);
    }
}

fn paint_ui_block(painter: &egui::Painter, block: &UiBlock, available_fonts: &[String]) {
    let rect = Rect::from_min_size(
        Pos2::new(block.rect.x, block.rect.y),
        Vec2::new(block.rect.width, block.rect.height),
    );
    if let Some(fill) = block.fill_color {
        painter.rect_filled(rect, 0.0, rgba_to_color32(fill));
    }
    if let Some(border) = block.border_color {
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(block.border_thickness.max(1.0), rgba_to_color32(border)),
            StrokeKind::Outside,
        );
    }
    if let Some(text) = &block.text {
        let font_family = resolve_preview_font_family(&text.style.font_family, available_fonts);
        let mut format = TextFormat::simple(
            FontId::new(text.style.size_px, font_family),
            rgba_to_color32(text.style.color),
        );
        format.italics = matches!(text.style.slant, TextSlant::Italic);
        let mut job = LayoutJob::default();
        job.wrap.max_width = text.max_width.unwrap_or(f32::INFINITY);
        job.append(&text.content, 0.0, format);
        let galley = painter.layout_job(job);
        let anchor_position = Pos2::new(text.position.x, text.position.y);
        let galley_pos = align_galley_top_left(anchor_position, galley.size(), text.anchor);
        painter.galley(
            galley_pos,
            galley.clone(),
            rgba_to_color32(text.style.color),
        );
        if matches!(text.style.weight, TextWeight::Bold) {
            painter.galley(
                Pos2::new(galley_pos.x + 0.75, galley_pos.y),
                galley,
                rgba_to_color32(text.style.color),
            );
        }
    }
}

fn align_galley_top_left(
    anchor_position: Pos2,
    size: Vec2,
    anchor: toki_core::text::TextAnchor,
) -> Pos2 {
    let x = match anchor {
        toki_core::text::TextAnchor::TopLeft
        | toki_core::text::TextAnchor::CenterLeft
        | toki_core::text::TextAnchor::BottomLeft => anchor_position.x,
        toki_core::text::TextAnchor::TopCenter
        | toki_core::text::TextAnchor::Center
        | toki_core::text::TextAnchor::BottomCenter => anchor_position.x - size.x * 0.5,
        toki_core::text::TextAnchor::TopRight
        | toki_core::text::TextAnchor::CenterRight
        | toki_core::text::TextAnchor::BottomRight => anchor_position.x - size.x,
    };
    let y = match anchor {
        toki_core::text::TextAnchor::TopLeft
        | toki_core::text::TextAnchor::TopCenter
        | toki_core::text::TextAnchor::TopRight => anchor_position.y,
        toki_core::text::TextAnchor::CenterLeft
        | toki_core::text::TextAnchor::Center
        | toki_core::text::TextAnchor::CenterRight => anchor_position.y - size.y * 0.5,
        toki_core::text::TextAnchor::BottomLeft
        | toki_core::text::TextAnchor::BottomCenter
        | toki_core::text::TextAnchor::BottomRight => anchor_position.y - size.y,
    };
    Pos2::new(x, y)
}

fn ui_editor_font_choices(ui_state: &EditorUI) -> Vec<String> {
    if ui_state.menu_preview_font_families.is_empty() {
        vec!["Sans".to_string(), "Serif".to_string(), "Mono".to_string()]
    } else {
        ui_state.menu_preview_font_families.clone()
    }
}

fn ui_preview_viewport_size(project: Option<&Project>) -> glam::Vec2 {
    project
        .map(|project| {
            glam::vec2(
                project.metadata.runtime.display.resolution_width.max(1) as f32,
                project.metadata.runtime.display.resolution_height.max(1) as f32,
            )
        })
        .unwrap_or_else(|| {
            glam::vec2(
                toki_core::project_runtime::default_resolution_width() as f32,
                toki_core::project_runtime::default_resolution_height() as f32,
            )
        })
}

fn scaled_rect_for_viewport(
    rect: UiRect,
    origin: Vec2,
    scale: f32,
    logical_viewport_size: glam::Vec2,
) -> Rect {
    let rect = transform_logical_ui_rect_with_transform(
        rect,
        ui_presentation_transform(
            glam::vec2(origin.x, origin.y),
            scale,
            logical_viewport_size,
            logical_viewport_size * scale,
        ),
    );
    Rect::from_min_size(
        Pos2::new(rect.x, rect.y),
        Vec2::new(rect.width, rect.height),
    )
}

fn resize_handle_rect(screen_rect: Rect) -> Rect {
    Rect::from_min_size(
        screen_rect.right_bottom() - Vec2::splat(16.0),
        Vec2::splat(16.0),
    )
}

fn topmost_widget_id_at_point(
    frames: &[UiWidgetFrame],
    origin: Vec2,
    scale: f32,
    logical_viewport_size: glam::Vec2,
    pointer_pos: Pos2,
) -> Option<String> {
    frames
        .iter()
        .rev()
        .filter(|frame| frame.widget_id.as_str() != "root")
        .find(|frame| {
            scaled_rect_for_viewport(frame.rect, origin, scale, logical_viewport_size)
                .contains(pointer_pos)
        })
        .map(|frame| frame.widget_id.to_string())
}

fn resize_handle_widget_id_at_point(
    frames: &[UiWidgetFrame],
    origin: Vec2,
    scale: f32,
    logical_viewport_size: glam::Vec2,
    pointer_pos: Pos2,
    widget_id: &str,
) -> Option<String> {
    frames
        .iter()
        .find(|frame| frame.widget_id.as_str() == widget_id)
        .and_then(|frame| {
            let screen_rect =
                scaled_rect_for_viewport(frame.rect, origin, scale, logical_viewport_size);
            resize_handle_rect(screen_rect)
                .contains(pointer_pos)
                .then(|| frame.widget_id.to_string())
        })
}

fn rgba_to_color32(rgba: [f32; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (rgba[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgba[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn declared_flags_stub() -> &'static [toki_core::project_runtime::ProjectFlagDefinition] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::{
        apply_widget_position_preset, bool_expression_choices, clamp_widget_layout_to_viewport,
        create_widget_preset,
        resize_handle_rect, resize_handle_widget_id_at_point, scaled_rect_for_viewport,
        snap_to_grid, topmost_widget_id_at_point, validate_widget, value_path_choices,
        WidgetPositionPreset, WidgetPreset,
    };
    use egui::{pos2, vec2};
    use toki_core::flags::FlagValue;
    use toki_core::project_runtime::ProjectFlagDefinition;
    use toki_core::ui::UiRect;
    use toki_core::ui_layout::{UiBinding, UiProgressBinding, UiWidgetFrame, UiWidgetKind, UiWidgetNode};

    #[test]
    fn child_widget_wins_hit_test_over_root() {
        let frames = vec![
            UiWidgetFrame {
                widget_id: "root".into(),
                rect: UiRect {
                    x: 0.0,
                    y: 0.0,
                    width: 160.0,
                    height: 144.0,
                },
                enabled: true,
            },
            UiWidgetFrame {
                widget_id: "child".into(),
                rect: UiRect {
                    x: 20.0,
                    y: 20.0,
                    width: 40.0,
                    height: 20.0,
                },
                enabled: true,
            },
        ];

        assert_eq!(
            topmost_widget_id_at_point(
                &frames,
                vec2(0.0, 0.0),
                1.0,
                glam::vec2(160.0, 144.0),
                pos2(30.0, 30.0),
            )
            .as_deref(),
            Some("child")
        );
        assert_eq!(
            topmost_widget_id_at_point(
                &frames,
                vec2(0.0, 0.0),
                1.0,
                glam::vec2(160.0, 144.0),
                pos2(4.0, 4.0),
            ),
            None
        );
    }

    #[test]
    fn resize_handle_hit_test_uses_lower_right_corner() {
        let frames = vec![UiWidgetFrame {
            widget_id: "child".into(),
            rect: UiRect {
                x: 20.0,
                y: 20.0,
                width: 40.0,
                height: 20.0,
            },
            enabled: true,
        }];
        let screen_rect =
            scaled_rect_for_viewport(frames[0].rect, vec2(0.0, 0.0), 1.0, glam::vec2(160.0, 144.0));
        let handle_rect = resize_handle_rect(screen_rect);
        let inside = handle_rect.center();

        assert_eq!(
            resize_handle_widget_id_at_point(
                &frames,
                vec2(0.0, 0.0),
                1.0,
                glam::vec2(160.0, 144.0),
                inside,
                "child",
            )
            .as_deref(),
            Some("child")
        );
        assert_eq!(
            resize_handle_widget_id_at_point(
                &frames,
                vec2(0.0, 0.0),
                1.0,
                glam::vec2(160.0, 144.0),
                pos2(25.0, 25.0),
                "child"
            ),
            None
        );
    }

    #[test]
    fn health_bar_preset_uses_player_health_bindings() {
        let layout = toki_core::ui_layout::UiLayoutAsset {
            root: UiWidgetNode::default(),
            ..Default::default()
        };
        let widget = create_widget_preset(WidgetPreset::HealthBar, &layout);
        match widget.kind {
            UiWidgetKind::ProgressBar {
                value: UiProgressBinding::CurrentMax { current, max },
            } => {
                assert_eq!(
                    current,
                    UiBinding::ValuePath {
                        path: "player.health".to_string(),
                        key: None,
                    }
                );
                assert_eq!(
                    max,
                    UiBinding::ValuePath {
                        path: "player.max_health".to_string(),
                        key: None,
                    }
                );
            }
            other => panic!("unexpected widget kind: {other:?}"),
        }
    }

    #[test]
    fn label_preset_creates_label_widget() {
        let layout = toki_core::ui_layout::UiLayoutAsset {
            root: UiWidgetNode::default(),
            ..Default::default()
        };
        let widget = create_widget_preset(WidgetPreset::Label, &layout);
        match widget.kind {
            UiWidgetKind::Label { content } => {
                assert_eq!(widget.title, "label_1");
                assert_eq!(content.segments.len(), 1);
            }
            other => panic!("unexpected widget kind: {other:?}"),
        }
    }

    #[test]
    fn position_preset_places_widget_in_requested_corner() {
        let mut widget = UiWidgetNode {
            id: "label".into(),
            layout: toki_core::ui_layout::UiLayoutSpec {
                size: [48.0, 12.0],
                ..Default::default()
            },
            ..Default::default()
        };
        apply_widget_position_preset(
            &mut widget,
            WidgetPositionPreset::BottomRight,
            glam::vec2(160.0, 144.0),
        );
        assert_eq!(
            widget.layout.anchor,
            toki_core::ui_layout::UiAnchor::BottomRight
        );
        assert_eq!(widget.layout.offset, [-8.0, -8.0]);
    }

    #[test]
    fn snap_to_grid_rounds_to_four_pixel_steps() {
        assert_eq!(snap_to_grid(1.9), 0.0);
        assert_eq!(snap_to_grid(5.9), 4.0);
        assert_eq!(snap_to_grid(6.1), 8.0);
    }

    #[test]
    fn value_path_choices_include_known_entity_paths_and_declared_flags() {
        let choices = value_path_choices(&[ProjectFlagDefinition {
            id: "coins".to_string(),
            default_value: FlagValue::Int(0),
        }]);
        assert!(choices.contains(&"player.health".to_string()));
        assert!(choices.contains(&"self.active".to_string()));
        assert!(choices.contains(&"flags.coins".to_string()));
    }

    #[test]
    fn bool_expression_choices_include_bool_flags_and_common_templates() {
        let choices = bool_expression_choices(&[
            ProjectFlagDefinition {
                id: "door_open".to_string(),
                default_value: FlagValue::Bool(false),
            },
            ProjectFlagDefinition {
                id: "coins".to_string(),
                default_value: FlagValue::Int(0),
            },
        ]);
        assert!(choices.contains(&"flags.door_open".to_string()));
        assert!(choices.contains(&"!flags.door_open".to_string()));
        assert!(choices.contains(&"player.active".to_string()));
        assert!(!choices.contains(&"flags.coins".to_string()));
    }

    #[test]
    fn validate_widget_flags_undeclared_ui_events() {
        let widget = UiWidgetNode {
            id: "button".into(),
            event_id: Some("missing_event".to_string()),
            ..Default::default()
        };

        let issues = validate_widget(
            &widget,
            &[],
            &[toki_core::project_runtime::ProjectUiEventDefinition {
                id: "open_inventory".to_string(),
            }],
        );

        assert!(issues
            .iter()
            .any(|issue| issue.contains("undeclared UI event 'missing_event'")));
    }

    #[test]
    fn clamp_widget_layout_to_viewport_keeps_top_left_widget_visible() {
        let mut widget = UiWidgetNode {
            id: "label".into(),
            kind: UiWidgetKind::Label {
                content: Default::default(),
            },
            layout: toki_core::ui_layout::UiLayoutSpec {
                anchor: toki_core::ui_layout::UiAnchor::TopLeft,
                offset: [200.0, 180.0],
                size: [120.0, 40.0],
                ..Default::default()
            },
            ..Default::default()
        };

        clamp_widget_layout_to_viewport(&mut widget, glam::vec2(160.0, 144.0));

        assert_eq!(widget.layout.offset, [40.0, 104.0]);
        assert_eq!(widget.layout.size, [120.0, 40.0]);
    }

    #[test]
    fn clamp_widget_layout_to_viewport_keeps_bottom_right_widget_visible() {
        let mut widget = UiWidgetNode {
            id: "grid".into(),
            kind: UiWidgetKind::GridContainer {
                columns: 2,
                spacing: Default::default(),
            },
            layout: toki_core::ui_layout::UiLayoutSpec {
                anchor: toki_core::ui_layout::UiAnchor::BottomRight,
                offset: [-120.0, -120.0],
                size: [72.0, 48.0],
                ..Default::default()
            },
            ..Default::default()
        };

        clamp_widget_layout_to_viewport(&mut widget, glam::vec2(160.0, 144.0));

        assert_eq!(widget.layout.offset, [-88.0, -96.0]);
    }
}
