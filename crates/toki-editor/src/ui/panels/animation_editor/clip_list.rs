//! Clip list panel.

use super::separators;
use crate::ui::EditorUI;

pub fn render_clip_list(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let available_height = ui.available_height();
    let available_width = ui.available_width();

    // Calculate heights based on ratio
    let header_height = 30.0; // Space for "Clips" heading
    let button_height = 30.0; // Space for "+ New Clip" button
    let bottom_section_min = 60.0; // Minimum height for default state section
    let separator_height = 8.0;

    let content_height = available_height - header_height - button_height - separator_height;
    let clip_list_height = (content_height * ui_state.animation.clip_list_ratio)
        .max(50.0)
        .min(content_height - bottom_section_min);
    let bottom_height = content_height - clip_list_height;

    ui.heading("Clips");

    // Add new clip button
    if ui.button("+ New Clip").clicked() {
        ui_state.animation.show_new_clip_dialog = true;
    }

    ui.add_space(4.0);

    // Collect clip info to avoid borrow issues
    let clip_info: Vec<_> = ui_state
        .animation
        .authoring
        .clips
        .iter()
        .enumerate()
        .map(|(idx, clip)| {
            let is_selected = ui_state.animation.authoring.selected_clip_index == Some(idx);
            let is_default = clip.state == ui_state.animation.authoring.default_state;
            (
                idx,
                clip.state.clone(),
                clip.frames.len(),
                is_selected,
                is_default,
            )
        })
        .collect();

    let mut select_index: Option<usize> = None;
    let mut delete_index: Option<usize> = None;

    egui::ScrollArea::vertical()
        .id_salt("anim_clip_list")
        .auto_shrink([false, false])
        .max_height(clip_list_height)
        .show(ui, |ui| {
            for (idx, state, frame_count, is_selected, is_default) in &clip_info {
                ui.horizontal(|ui| {
                    let label = if *is_default {
                        format!("{} ({}) *", state, frame_count)
                    } else {
                        format!("{} ({})", state, frame_count)
                    };

                    if ui.selectable_label(*is_selected, &label).clicked() {
                        select_index = Some(*idx);
                    }

                    if ui.small_button("x").on_hover_text("Delete").clicked() {
                        delete_index = Some(*idx);
                    }
                });
            }
        });

    // Apply deferred actions
    if let Some(idx) = select_index {
        ui_state.animation.authoring.select_clip(idx);
        ui_state.animation.preview.stop();
    }

    if let Some(idx) = delete_index {
        ui_state.animation.authoring.delete_clip(idx);
    }

    // Draggable separator between clip list and default state
    let sep_response = separators::render_horizontal_separator(ui, available_width);
    if sep_response.dragged() {
        let delta_ratio = sep_response.drag_delta().y / content_height;
        ui_state.animation.clip_list_ratio =
            (ui_state.animation.clip_list_ratio + delta_ratio).clamp(0.2, 0.9);
    }

    // Default state selector
    ui.allocate_ui_with_layout(
        egui::vec2(available_width, bottom_height),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            if !ui_state.animation.authoring.clips.is_empty() {
                ui.label("Default State:");
                let clip_states: Vec<String> = ui_state
                    .animation
                    .authoring
                    .clips
                    .iter()
                    .map(|c| c.state.clone())
                    .collect();

                let mut default_state = ui_state.animation.authoring.default_state.clone();
                egui::ComboBox::from_id_salt("anim_default_state")
                    .selected_text(&default_state)
                    .show_ui(ui, |ui| {
                        for state in &clip_states {
                            if ui
                                .selectable_value(&mut default_state, state.clone(), state)
                                .changed()
                            {
                                ui_state.animation.authoring.default_state = default_state.clone();
                                ui_state.animation.authoring.dirty = true;
                            }
                        }
                    });
            }
        },
    );
}
