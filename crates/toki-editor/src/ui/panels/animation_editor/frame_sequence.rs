//! Frame sequence panel.

use crate::ui::EditorUI;

pub fn render_frame_sequence(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.heading("Frames");

    let Some(clip_idx) = ui_state.animation.authoring.selected_clip_index else {
        ui.label("Select a clip");
        return;
    };

    if clip_idx >= ui_state.animation.authoring.clips.len() {
        return;
    };

    // Clip settings - get values first to avoid borrow issues
    let current_duration = ui_state.animation.authoring.clips[clip_idx].default_duration_ms;
    let current_loop_mode = ui_state.animation.authoring.clips[clip_idx]
        .loop_mode
        .clone();

    ui.horizontal(|ui| {
        ui.label("Duration (ms):");
    });
    let mut duration = current_duration;
    if ui
        .add(
            egui::DragValue::new(&mut duration)
                .speed(5.0)
                .range(10.0..=5000.0),
        )
        .changed()
    {
        ui_state.animation.authoring.clips[clip_idx].default_duration_ms = duration;
        ui_state.animation.authoring.dirty = true;
    }

    ui.horizontal(|ui| {
        ui.label("Loop Mode:");
    });
    let loop_modes = ["loop", "once", "ping_pong"];
    let mut loop_mode = current_loop_mode;
    let mut loop_mode_changed = false;
    egui::ComboBox::from_id_salt("loop_mode")
        .selected_text(&loop_mode)
        .show_ui(ui, |ui| {
            for mode in &loop_modes {
                if ui
                    .selectable_value(&mut loop_mode, mode.to_string(), *mode)
                    .changed()
                {
                    loop_mode_changed = true;
                }
            }
        });

    if loop_mode_changed {
        ui_state.animation.authoring.clips[clip_idx].loop_mode = loop_mode;
        ui_state.animation.authoring.dirty = true;
    }

    ui.separator();

    // Frame list - collect info first to avoid borrow issues
    let frame_info: Vec<_> = ui_state
        .animation
        .authoring
        .clips
        .get(clip_idx)
        .map(|clip| {
            clip.frames
                .iter()
                .enumerate()
                .map(|(idx, frame)| {
                    let is_selected =
                        ui_state.animation.authoring.selected_frame_index == Some(idx);
                    let is_preview = ui_state.animation.preview.current_frame() == idx;
                    (
                        idx,
                        frame.position,
                        frame.duration_ms,
                        is_selected,
                        is_preview,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let frame_count = frame_info.len();
    let mut select_frame: Option<usize> = None;
    let mut move_up: Option<usize> = None;
    let mut move_down: Option<usize> = None;
    let mut delete_frame: Option<usize> = None;

    egui::ScrollArea::vertical()
        .id_salt("anim_frame_sequence")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (idx, position, duration_override, is_selected, is_preview) in &frame_info {
                let label = format!(
                    "[{}, {}]{}{}",
                    position[0],
                    position[1],
                    duration_override
                        .map(|d| format!(" {:.0}ms", d))
                        .unwrap_or_default(),
                    if *is_preview { " <" } else { "" }
                );

                ui.horizontal(|ui| {
                    if ui.selectable_label(*is_selected, &label).clicked() {
                        select_frame = Some(*idx);
                    }

                    if *idx > 0 && ui.small_button("^").clicked() {
                        move_up = Some(*idx);
                    }
                    if *idx + 1 < frame_count && ui.small_button("v").clicked() {
                        move_down = Some(*idx);
                    }
                    if ui.small_button("x").clicked() {
                        delete_frame = Some(*idx);
                    }
                });
            }
        });

    // Apply deferred actions
    if let Some(idx) = select_frame {
        ui_state.animation.authoring.selected_frame_index = Some(idx);
        ui_state.animation.preview.go_to_frame(idx, frame_count);
    }

    if let Some(idx) = move_up {
        if let Some(clip) = ui_state.animation.authoring.clips.get_mut(clip_idx) {
            clip.move_frame(idx, idx - 1);
            if ui_state.animation.authoring.selected_frame_index == Some(idx) {
                ui_state.animation.authoring.selected_frame_index = Some(idx - 1);
            }
            ui_state.animation.authoring.dirty = true;
        }
    }

    if let Some(idx) = move_down {
        if let Some(clip) = ui_state.animation.authoring.clips.get_mut(clip_idx) {
            clip.move_frame(idx, idx + 1);
            if ui_state.animation.authoring.selected_frame_index == Some(idx) {
                ui_state.animation.authoring.selected_frame_index = Some(idx + 1);
            }
            ui_state.animation.authoring.dirty = true;
        }
    }

    if let Some(idx) = delete_frame {
        if let Some(clip) = ui_state.animation.authoring.clips.get_mut(clip_idx) {
            clip.remove_frame(idx);
            if clip.frames.is_empty() {
                ui_state.animation.authoring.selected_frame_index = None;
            } else if let Some(sel) = ui_state.animation.authoring.selected_frame_index {
                if sel >= clip.frames.len() {
                    ui_state.animation.authoring.selected_frame_index = Some(clip.frames.len() - 1);
                }
            }
            ui_state.animation.authoring.dirty = true;
        }
    }

    // Keyboard shortcuts
    let ctx = ui.ctx();
    if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
        ui_state.animation.authoring.remove_selected_frame();
    }
}
