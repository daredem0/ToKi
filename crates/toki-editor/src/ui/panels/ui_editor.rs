use crate::project::{Project, ProjectAssets};
use crate::ui::editor_ui::{sync_ui_layout_registry, EditorUI};
use crate::fonts::resolve_preview_font_family;
use egui::{
    text::{LayoutJob, TextFormat},
    Color32, FontId, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use std::collections::{BTreeSet, HashMap};
use toki_core::expression::Expression;
use toki_core::flags::{FlagValue, GameFlags};
use toki_core::rules::TriggerContext;
use toki_core::text::{TextAnchor, TextSlant, TextWeight};
use toki_core::ui::{UiBlock, UiComposition, UiRect};
use toki_core::ui_layout::{
    UiAnchor, UiBinding, UiBindingContext, UiCollectionBinding, UiCollectionRowTemplate,
    UiCollectionTextSegment, UiLayoutAsset, UiLayoutEngine, UiProgressBinding, UiSpacing,
    UiTextSegment, UiTextTemplate, UiTheme, UiTypography, UiWidgetFrame, UiWidgetKind,
    UiWidgetNode,
};
use toki_core::value_path::{ValuePath, ValuePathContext};

const PREVIEW_MIN_HEIGHT: f32 = 320.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetKindChoice {
    Label,
    Image,
    ProgressBar,
    GridContainer,
    ScrollList,
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
    render_ui_editor_main(
        ui,
        ui_state,
        project_assets,
        declared_flags,
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
            render_widget_tree(
                ui,
                &layout.root,
                selected_widget_id.as_deref(),
                0,
                &mut next_selected_widget_id,
            );
        });
        ui.separator();
        let inspector_selected_widget_id = next_selected_widget_id.clone();
        dirty |= render_selected_widget_inspector(
            ui,
            layout,
            inspector_selected_widget_id.as_deref(),
            declared_flags,
            &font_choices,
            &mut next_selected_widget_id,
        );
        ui.separator();
        let issues = validate_ui_layout(layout, declared_flags);
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
            let issues = validate_ui_layout(&layout, declared_flags);
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
    ui_state.ui_editor_context_mut().ui.persist_active_view_into_layout();
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
                editor_state.status_message =
                    Some(format!("Duplicated widget '{widget_id}' to '{new_widget_id}'."));
                editor_state.dirty = true;
            }
        }
    }

    if focus_pressed {
        if let Some(selected_widget_id) = ui_state.ui_editor_context().ui.selected_widget_id.clone()
        {
            if let Some(frame) = compute_preview(&layout.root, declared_flags_stub(), preview_viewport_size)
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
        editor_state.pan = [16.0, 16.0];
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
                ui_state.ui_editor_context_mut().ui.selected_layout_id = Some(layout.id.to_string());
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
        for kind in [
            WidgetKindChoice::Label,
            WidgetKindChoice::Image,
            WidgetKindChoice::ProgressBar,
            WidgetKindChoice::GridContainer,
            WidgetKindChoice::ScrollList,
        ] {
            if ui.button(format!("+ {}", widget_kind_choice_label(kind))).clicked() {
                let selected_widget_id = ui_state.ui_editor_context().ui.selected_widget_id.clone();
                let parent_id = selected_widget_id
                    .as_deref()
                    .and_then(|widget_id| {
                        find_widget(&layout.root, widget_id).and_then(|widget| {
                            if can_have_children(widget) {
                                Some(widget.id.to_string())
                            } else {
                                find_parent_id(&layout.root, widget_id)
                            }
                        })
                    })
                    .unwrap_or_else(|| "root".to_string());
                let widget = create_widget(kind, layout);
                let widget_id = widget.id.to_string();
                if insert_child_widget(&mut layout.root, &parent_id, widget) {
                    ui_state.ui_editor_context_mut().ui.select_widget(widget_id);
                    ui_state.ui_editor_context_mut().ui.status_message =
                        Some(format!("Added {} widget.", widget_kind_choice_label(kind)));
                    changed = true;
                }
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
    let desired_size = egui::vec2(ui.available_width(), ui.available_height().max(PREVIEW_MIN_HEIGHT));
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    let (mut zoom, mut pan, selected_widget_id) = {
        let editor_state = &ui_state.ui_editor_context().ui;
        (
            editor_state.zoom,
            editor_state.pan,
            editor_state.selected_widget_id.clone(),
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
    let origin = Vec2::new(
        rect.left() + pan[0],
        rect.top() + pan[1],
    );
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
        origin,
        zoom,
        &ui_editor_font_choices(ui_state),
    );

    let mut widget_dragged = false;
    let mut next_selected_widget_id = selected_widget_id.clone();
    for frame in preview.frames.iter().rev() {
        let screen_rect = scaled_rect(frame.rect, origin, zoom);
        let response = ui.interact(
            screen_rect,
            ui.id().with(("ui_widget_frame", frame.widget_id.as_str())),
            Sense::click_and_drag(),
        );
        let is_selected = selected_widget_id.as_deref() == Some(frame.widget_id.as_str());
        let stroke_color = if is_selected {
            Color32::from_rgb(255, 210, 90)
        } else {
            Color32::from_rgba_unmultiplied(200, 200, 200, 110)
        };
        painter.rect_stroke(
            screen_rect,
            2.0,
            Stroke::new(if is_selected { 2.0 } else { 1.0 }, stroke_color),
            StrokeKind::Inside,
        );

        if response.clicked() {
            next_selected_widget_id = Some(frame.widget_id.to_string());
        }

        if response.dragged() && frame.widget_id.as_str() != "root" {
            widget_dragged = true;
            if let Some(widget) = find_widget_mut(&mut layout.root, frame.widget_id.as_str()) {
                let delta = ui.ctx().input(|input| input.pointer.delta());
                widget.layout.offset[0] += delta.x / zoom;
                widget.layout.offset[1] += delta.y / zoom;
                changed = true;
            }
        }

        if is_selected && frame.widget_id.as_str() != "root" {
            let handle_rect = Rect::from_min_size(
                screen_rect.right_bottom() - Vec2::splat(10.0),
                Vec2::splat(10.0),
            );
            painter.rect_filled(handle_rect, 1.0, Color32::from_rgb(255, 210, 90));
            let handle_response = ui.interact(
                handle_rect,
                ui.id().with(("ui_widget_resize", frame.widget_id.as_str())),
                Sense::drag(),
            );
            if handle_response.dragged() {
                let delta = ui.ctx().input(|input| input.pointer.delta());
                if let Some(widget) = find_widget_mut(&mut layout.root, frame.widget_id.as_str()) {
                    widget.layout.size[0] = (widget.layout.size[0] + delta.x / zoom)
                        .max(16.0);
                    widget.layout.size[1] = (widget.layout.size[1] + delta.y / zoom)
                        .max(16.0);
                    changed = true;
                }
            }
        }
    }

    if response.dragged() && !widget_dragged {
        let delta = ui.ctx().input(|input| input.pointer.delta());
        pan[0] += delta.x;
        pan[1] += delta.y;
        changed = true;
    }

    if response.double_clicked() {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let widget = create_widget(WidgetKindChoice::Label, layout);
            let widget_id = widget.id.to_string();
            let mut widget = widget;
            widget.layout.offset = [
                (pointer_pos.x - rect.left() - pan[0]) / zoom,
                (pointer_pos.y - rect.top() - pan[1]) / zoom,
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
) {
    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 12.0);
        let selected = selected_widget_id == Some(widget.id.as_str());
        if ui
            .selectable_label(selected, format!("{} ({})", widget.title, widget.id))
            .clicked()
        {
            *next_selected_widget_id = Some(widget.id.to_string());
        }
    });
    for child in &widget.children {
        render_widget_tree(
            ui,
            child,
            selected_widget_id,
            depth + 1,
            next_selected_widget_id,
        );
    }
}

fn render_selected_widget_inspector(
    ui: &mut Ui,
    layout: &mut UiLayoutAsset,
    selected_widget_id: Option<&str>,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    font_choices: &[String],
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

    ui.heading("Selected Widget");
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Id:");
        let mut widget_id = widget.id.to_string();
        if ui.text_edit_singleline(&mut widget_id).changed() {
            widget.id = widget_id.clone().into();
            *next_selected_widget_id = Some(widget_id);
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Title:");
        changed |= ui.text_edit_singleline(&mut widget.title).changed();
    });
    ui.horizontal(|ui| {
        ui.label("Kind:");
        let current_kind = widget_kind_choice(widget);
        let mut selected_kind = current_kind;
        egui::ComboBox::from_id_salt(("ui_widget_kind", widget.id.as_str()))
            .selected_text(widget_kind_choice_label(current_kind))
            .show_ui(ui, |ui| {
                for candidate in [
                    WidgetKindChoice::Label,
                    WidgetKindChoice::Image,
                    WidgetKindChoice::ProgressBar,
                    WidgetKindChoice::GridContainer,
                    WidgetKindChoice::ScrollList,
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
        ui.label("Event Id:");
        let mut event_id = widget.event_id.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut event_id).changed() {
            widget.event_id = if event_id.trim().is_empty() {
                None
            } else {
                Some(event_id)
            };
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Visible If:");
        let mut visible_if = widget.visible_if.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut visible_if).changed() {
            widget.visible_if = if visible_if.trim().is_empty() {
                None
            } else {
                Some(visible_if)
            };
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Enabled If:");
        let mut enabled_if = widget.enabled_if.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut enabled_if).changed() {
            widget.enabled_if = if enabled_if.trim().is_empty() {
                None
            } else {
                Some(enabled_if)
            };
            changed = true;
        }
    });
    ui.horizontal(|ui| {
        changed |= ui.checkbox(&mut widget.focusable, "Focusable").changed();
    });
    ui.separator();
    render_layout_spec_editor(ui, &mut widget.layout);
    ui.separator();
    render_typography_editor(ui, &mut widget.style.typography, font_choices);
    ui.separator();
    render_widget_kind_editor(ui, widget, declared_flags);
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
            ui.add(egui::DragValue::new(&mut layout.size[0]).speed(1.0).range(1.0..=4096.0));
            ui.add(egui::DragValue::new(&mut layout.size[1]).speed(1.0).range(1.0..=4096.0));
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
            let mut font_size = typography.font_size_px.unwrap_or(14) as i32;
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
) {
    match &mut widget.kind {
        UiWidgetKind::Label { content } => render_text_template_editor(ui, content),
        UiWidgetKind::Image { image_id } => {
            ui.horizontal(|ui| {
                ui.label("Image Id:");
                ui.text_edit_singleline(image_id);
            });
        }
        UiWidgetKind::ProgressBar { value } => render_progress_binding_editor(ui, value),
        UiWidgetKind::GridContainer { columns, spacing } => {
            ui.horizontal(|ui| {
                ui.label("Columns:");
                let mut value = *columns as i32;
                if ui.add(egui::DragValue::new(&mut value).range(1..=16)).changed() {
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
        UiWidgetKind::ScrollList {
            collection,
            row_height,
            row_spacing,
            row_template,
        } => {
            ui.horizontal(|ui| {
                ui.label("Collection:");
                egui::ComboBox::from_id_salt(("ui_scroll_collection", widget.id.as_str()))
                    .selected_text(match collection {
                        UiCollectionBinding::PlayerInventory => "PlayerInventory",
                        UiCollectionBinding::DeclaredFlags => "DeclaredFlags",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            collection,
                            UiCollectionBinding::PlayerInventory,
                            "PlayerInventory",
                        );
                        ui.selectable_value(
                            collection,
                            UiCollectionBinding::DeclaredFlags,
                            "DeclaredFlags",
                        );
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Row Height:");
                let mut value = *row_height as i32;
                if ui.add(egui::DragValue::new(&mut value).range(12..=256)).changed() {
                    *row_height = value.max(12) as u16;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Row Spacing:");
                let mut value = *row_spacing as i32;
                if ui.add(egui::DragValue::new(&mut value).range(0..=64)).changed() {
                    *row_spacing = value.max(0) as u16;
                }
            });
            render_collection_row_template_editor(ui, row_template);
        }
    }

    let issues = validate_widget(widget, declared_flags);
    for issue in issues {
        ui.colored_label(Color32::from_rgb(255, 210, 80), issue);
    }
}

fn render_text_template_editor(ui: &mut Ui, template: &mut UiTextTemplate) {
    if template.segments.is_empty() {
        template
            .segments
            .push(UiTextSegment::Literal {
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
                        render_binding_editor(ui, binding, ("ui_text_binding", index));
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

fn render_progress_binding_editor(ui: &mut Ui, binding: &mut UiProgressBinding) {
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
                render_binding_editor(ui, current, ("ui_progress_current", 0));
                ui.label("Max");
                render_binding_editor(ui, max, ("ui_progress_max", 0));
            }
            UiProgressBinding::Percent { percent } => {
                ui.label("Percent");
                render_binding_editor(ui, percent, ("ui_progress_percent", 0));
            }
        }
    });
}

fn render_collection_row_template_editor(ui: &mut Ui, template: &mut UiCollectionRowTemplate) {
    if template.segments.is_empty() {
        template.segments = vec![
            UiCollectionTextSegment::ItemId,
            UiCollectionTextSegment::Literal {
                text: " x".to_string(),
            },
            UiCollectionTextSegment::ItemCount,
        ];
    }
    ui.collapsing("Row Template", |ui| {
        for (index, segment) in template.segments.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("Row Segment {}", index + 1));
                let mut kind = match segment {
                    UiCollectionTextSegment::Literal { .. } => 0,
                    UiCollectionTextSegment::ItemId => 1,
                    UiCollectionTextSegment::ItemCount => 2,
                    UiCollectionTextSegment::ItemValue => 3,
                };
                egui::ComboBox::from_id_salt(("ui_row_segment_kind", index))
                    .selected_text(match kind {
                        0 => "Literal",
                        1 => "ItemId",
                        2 => "ItemCount",
                        _ => "ItemValue",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut kind, 0, "Literal");
                        ui.selectable_value(&mut kind, 1, "ItemId");
                        ui.selectable_value(&mut kind, 2, "ItemCount");
                        ui.selectable_value(&mut kind, 3, "ItemValue");
                    });
                if kind == 0 {
                    match segment {
                        UiCollectionTextSegment::Literal { text } => {
                            ui.text_edit_singleline(text);
                        }
                        _ => {
                            *segment = UiCollectionTextSegment::Literal {
                                text: String::new(),
                            }
                        }
                    }
                } else {
                    *segment = match kind {
                        1 => UiCollectionTextSegment::ItemId,
                        2 => UiCollectionTextSegment::ItemCount,
                        _ => UiCollectionTextSegment::ItemValue,
                    };
                }
            });
        }
        if ui.button("+ Add Row Segment").clicked() {
            template.segments.push(UiCollectionTextSegment::Literal {
                text: String::new(),
            });
        }
    });
}

fn render_binding_editor(ui: &mut Ui, binding: &mut UiBinding, id_salt: impl std::hash::Hash) {
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
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.text_edit_singleline(path);
            });
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
            ui.horizontal(|ui| {
                ui.label("Expression:");
                ui.text_edit_singleline(expression);
            });
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

struct PreviewOutput {
    composition: UiComposition,
    frames: Vec<UiWidgetFrame>,
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
) -> Vec<String> {
    let mut issues = Vec::new();
    if layout.id.as_str().trim().is_empty() {
        issues.push("Layout id must not be empty".to_string());
    }
    let mut seen = BTreeSet::<String>::new();
    validate_widget_recursive(&layout.root, declared_flags, &mut seen, &mut issues);
    issues
}

fn validate_widget_recursive(
    widget: &UiWidgetNode,
    declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
    seen: &mut BTreeSet<String>,
    issues: &mut Vec<String>,
) {
    if widget.id.as_str().trim().is_empty() {
        issues.push("Widget id must not be empty".to_string());
    } else if !seen.insert(widget.id.to_string()) {
        issues.push(format!("Duplicate widget id '{}'", widget.id));
    }
    issues.extend(validate_widget(widget, declared_flags));
    for child in &widget.children {
        validate_widget_recursive(child, declared_flags, seen, issues);
    }
}

fn validate_widget(
    widget: &UiWidgetNode,
    _declared_flags: &[toki_core::project_runtime::ProjectFlagDefinition],
) -> Vec<String> {
    let mut issues = Vec::new();
    for (label, gate) in [
        ("visible_if", widget.visible_if.as_deref()),
        ("enabled_if", widget.enabled_if.as_deref()),
    ] {
        if let Some(expression) = gate {
            if let Err(error) = Expression::parse(expression) {
                issues.push(format!("{} on '{}' is invalid: {}", label, widget.id, error));
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
        UiWidgetKind::Image { image_id } => {
            if image_id.trim().is_empty() {
                issues.push(format!("Image widget '{}' requires a non-empty image id", widget.id));
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
        UiWidgetKind::GridContainer { columns, .. } => {
            if *columns == 0 {
                issues.push(format!("GridContainer '{}' must have at least one column", widget.id));
            }
        }
        UiWidgetKind::ScrollList { row_height, .. } => {
            if *row_height == 0 {
                issues.push(format!("ScrollList '{}' row height must be positive", widget.id));
            }
        }
    }
    issues
}

fn validate_binding(binding: &UiBinding, issues: &mut Vec<String>, widget_id: &toki_core::UiWidgetId) {
    match binding {
        UiBinding::ValuePath { path, .. } => {
            if let Err(error) = ValuePath::parse(path) {
                issues.push(format!("Widget '{}' has invalid value path '{}': {}", widget_id, path, error));
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
        UiWidgetKind::Image { .. } => WidgetKindChoice::Image,
        UiWidgetKind::ProgressBar { .. } => WidgetKindChoice::ProgressBar,
        UiWidgetKind::GridContainer { .. } => WidgetKindChoice::GridContainer,
        UiWidgetKind::ScrollList { .. } => WidgetKindChoice::ScrollList,
    }
}

fn widget_kind_choice_label(kind: WidgetKindChoice) -> &'static str {
    match kind {
        WidgetKindChoice::Label => "Label",
        WidgetKindChoice::Image => "Image",
        WidgetKindChoice::ProgressBar => "ProgressBar",
        WidgetKindChoice::GridContainer => "GridContainer",
        WidgetKindChoice::ScrollList => "ScrollList",
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
        WidgetKindChoice::Image => UiWidgetKind::Image {
            image_id: "image_id".to_string(),
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
        WidgetKindChoice::GridContainer => UiWidgetKind::GridContainer {
            columns: 2,
            spacing: UiSpacing::default(),
        },
        WidgetKindChoice::ScrollList => UiWidgetKind::ScrollList {
            collection: UiCollectionBinding::PlayerInventory,
            row_height: 24,
            row_spacing: 4,
            row_template: UiCollectionRowTemplate {
                segments: vec![
                    UiCollectionTextSegment::ItemId,
                    UiCollectionTextSegment::Literal {
                        text: " x".to_string(),
                    },
                    UiCollectionTextSegment::ItemCount,
                ],
            },
        },
    }
}

fn create_widget(kind: WidgetKindChoice, layout: &UiLayoutAsset) -> UiWidgetNode {
    let id = unique_widget_id(&layout.root, kind);
    UiWidgetNode {
        id: id.clone().into(),
        title: id.clone(),
        layout: UiLayoutSpecForKind(kind),
        style: Default::default(),
        event_id: None,
        focusable: false,
        visible_if: None,
        enabled_if: None,
        kind: default_widget_kind(kind),
        children: Vec::new(),
    }
}

#[allow(non_snake_case)]
fn UiLayoutSpecForKind(kind: WidgetKindChoice) -> toki_core::ui_layout::UiLayoutSpec {
    toki_core::ui_layout::UiLayoutSpec {
        size: match kind {
            WidgetKindChoice::Label => [160.0, 28.0],
            WidgetKindChoice::Image => [96.0, 96.0],
            WidgetKindChoice::ProgressBar => [180.0, 24.0],
            WidgetKindChoice::GridContainer => [220.0, 120.0],
            WidgetKindChoice::ScrollList => [220.0, 128.0],
        },
        offset: [24.0, 24.0],
        ..Default::default()
    }
}

fn can_have_children(widget: &UiWidgetNode) -> bool {
    matches!(widget.kind, UiWidgetKind::GridContainer { .. })
}

fn unique_widget_id(root: &UiWidgetNode, kind: WidgetKindChoice) -> String {
    let prefix = match kind {
        WidgetKindChoice::Label => "label",
        WidgetKindChoice::Image => "image",
        WidgetKindChoice::ProgressBar => "progress",
        WidgetKindChoice::GridContainer => "grid",
        WidgetKindChoice::ScrollList => "list",
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
    origin: Vec2,
    scale: f32,
    available_fonts: &[String],
) {
    for block in &composition.blocks {
        paint_ui_block(painter, block, origin, scale, available_fonts);
    }
}

fn paint_ui_block(
    painter: &egui::Painter,
    block: &UiBlock,
    origin: Vec2,
    scale: f32,
    available_fonts: &[String],
) {
    let rect = scaled_rect(block.rect, origin, scale);
    if let Some(fill) = block.fill_color {
        painter.rect_filled(rect, 0.0, rgba_to_color32(fill));
    }
    if let Some(border) = block.border_color {
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(block.border_thickness.max(1.0) * scale.max(1.0), rgba_to_color32(border)),
            StrokeKind::Outside,
        );
    }
    if let Some(text) = &block.text {
        let font_family = resolve_preview_font_family(&text.style.font_family, available_fonts);
        let mut format = TextFormat::simple(
            FontId::new(text.style.size_px * scale, font_family),
            rgba_to_color32(text.style.color),
        );
        format.italics = matches!(text.style.slant, TextSlant::Italic);
        let galley = painter.layout_job(LayoutJob::single_section(text.content.clone(), format));
        let anchor_position = Pos2::new(
            origin.x + text.position.x * scale,
            origin.y + text.position.y * scale,
        );
        let galley_pos = align_galley_top_left(anchor_position, galley.size(), text.anchor);
        painter.galley(galley_pos, galley.clone(), rgba_to_color32(text.style.color));
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

fn scaled_rect(rect: UiRect, origin: Vec2, scale: f32) -> Rect {
    Rect::from_min_size(
        Pos2::new(origin.x + rect.x * scale, origin.y + rect.y * scale),
        Vec2::new(rect.width * scale, rect.height * scale),
    )
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
