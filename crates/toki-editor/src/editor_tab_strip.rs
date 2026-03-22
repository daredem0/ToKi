use egui::{scroll_area::ScrollBarVisibility, Align, Button, FontId, Layout, ScrollArea, Ui};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditorTabStripState {
    pub horizontal_offset: f32,
    pub last_selected_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorTabSpec<T> {
    pub value: T,
    pub label: &'static str,
}

impl EditorTabStripState {
    fn max_offset(total_content_width: f32, viewport_width: f32) -> f32 {
        (total_content_width - viewport_width).max(0.0)
    }

    fn clamp_offset(&mut self, total_content_width: f32, viewport_width: f32) {
        self.horizontal_offset = self
            .horizontal_offset
            .clamp(0.0, Self::max_offset(total_content_width, viewport_width));
    }

    fn scroll_by(&mut self, delta: f32, total_content_width: f32, viewport_width: f32) {
        self.horizontal_offset += delta;
        self.clamp_offset(total_content_width, viewport_width);
    }

    fn ensure_range_visible(
        &mut self,
        range_start: f32,
        range_end: f32,
        total_content_width: f32,
        viewport_width: f32,
    ) {
        if range_start < self.horizontal_offset {
            self.horizontal_offset = range_start;
        } else if range_end > self.horizontal_offset + viewport_width {
            self.horizontal_offset = range_end - viewport_width;
        }
        self.clamp_offset(total_content_width, viewport_width);
    }

    fn update_selection_visibility(
        &mut self,
        selected_index: Option<usize>,
        ranges: &[(f32, f32)],
        total_content_width: f32,
        viewport_width: f32,
    ) {
        self.clamp_offset(total_content_width, viewport_width);

        if selected_index != self.last_selected_index {
            if let Some(index) = selected_index {
                let (start, end) = ranges[index];
                self.ensure_range_visible(start, end, total_content_width, viewport_width);
            }
            self.last_selected_index = selected_index;
        }
    }

    fn button_scroll_step(viewport_width: f32) -> f32 {
        (viewport_width * 0.75).max(96.0)
    }

    fn wheel_scroll_delta(ui: &Ui) -> f32 {
        ui.input(|input| {
            let delta = input.smooth_scroll_delta;
            if delta.x.abs() > delta.y.abs() {
                -delta.x
            } else {
                -delta.y
            }
        })
    }
}

fn measure_tab_width(ui: &Ui, label: &str) -> f32 {
    let font_id = FontId::proportional(ui.style().text_styles[&egui::TextStyle::Button].size);
    let text_size = ui
        .painter()
        .layout_no_wrap(label.to_string(), font_id, ui.visuals().text_color())
        .size();
    text_size.x + ui.spacing().button_padding.x * 2.0 + 8.0
}

fn tab_ranges(widths: &[f32], item_spacing: f32) -> Vec<(f32, f32)> {
    let mut ranges = Vec::with_capacity(widths.len());
    let mut x = 0.0;
    for width in widths {
        ranges.push((x, x + *width));
        x += *width + item_spacing;
    }
    ranges
}

fn total_content_width(widths: &[f32], item_spacing: f32) -> f32 {
    if widths.is_empty() {
        0.0
    } else {
        widths.iter().sum::<f32>() + item_spacing * (widths.len().saturating_sub(1) as f32)
    }
}

pub fn render_tab_strip<T: Copy + PartialEq>(
    ui: &mut Ui,
    id_source: impl std::hash::Hash,
    state: &mut EditorTabStripState,
    selected: &mut T,
    items: &[EditorTabSpec<T>],
) {
    let item_spacing = ui.spacing().item_spacing.x;
    let widths = items
        .iter()
        .map(|item| measure_tab_width(ui, item.label))
        .collect::<Vec<_>>();
    let total_width = total_content_width(&widths, item_spacing);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let arrow_button_width = ui.spacing().interact_size.y;
        let overflow = total_width > available_width;
        let viewport_width = if overflow {
            (available_width - arrow_button_width * 2.0 - item_spacing * 2.0).max(1.0)
        } else {
            available_width
        };

        let ranges = tab_ranges(&widths, item_spacing);
        let selected_index = items.iter().position(|item| item.value == *selected);
        state.update_selection_visibility(selected_index, &ranges, total_width, viewport_width);

        if overflow {
            let step = EditorTabStripState::button_scroll_step(viewport_width);
            let can_scroll_left = state.horizontal_offset > 0.0;

            if ui.add_enabled(can_scroll_left, Button::new("<")).clicked() {
                state.scroll_by(-step, total_width, viewport_width);
                ui.ctx().request_repaint();
            }
        }

        let scroll_output = ui
            .allocate_ui_with_layout(
                egui::vec2(viewport_width, ui.spacing().interact_size.y),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ScrollArea::horizontal()
                        .id_salt(id_source)
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                        .horizontal_scroll_offset(state.horizontal_offset)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            // Force the tab row to keep its full natural width so horizontal
                            // scrolling has real overflow to operate on instead of shrinking
                            // content to the viewport width.
                            ui.set_min_width(total_width);
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                for item in items {
                                    ui.selectable_value(selected, item.value, item.label);
                                }
                            });
                        })
                },
            )
            .inner;

        state.horizontal_offset = scroll_output.state.offset.x;
        state.clamp_offset(total_width, scroll_output.inner_rect.width());

        if overflow {
            if ui.rect_contains_pointer(scroll_output.inner_rect) {
                let wheel_delta = EditorTabStripState::wheel_scroll_delta(ui);
                if wheel_delta != 0.0 {
                    state.scroll_by(wheel_delta, total_width, scroll_output.inner_rect.width());
                    ui.ctx().request_repaint();
                }
            }

            let step = EditorTabStripState::button_scroll_step(scroll_output.inner_rect.width());
            let can_scroll_right = state.horizontal_offset
                < EditorTabStripState::max_offset(total_width, scroll_output.inner_rect.width());
            if ui.add_enabled(can_scroll_right, Button::new(">")).clicked() {
                state.scroll_by(step, total_width, scroll_output.inner_rect.width());
                ui.ctx().request_repaint();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::EditorTabStripState;

    #[test]
    fn max_offset_clamps_to_zero_for_non_overflowing_content() {
        assert_eq!(EditorTabStripState::max_offset(100.0, 120.0), 0.0);
        assert_eq!(EditorTabStripState::max_offset(120.0, 120.0), 0.0);
    }

    #[test]
    fn scroll_by_clamps_to_valid_range() {
        let mut state = EditorTabStripState::default();
        state.scroll_by(80.0, 400.0, 120.0);
        assert_eq!(state.horizontal_offset, 80.0);

        state.scroll_by(500.0, 400.0, 120.0);
        assert_eq!(state.horizontal_offset, 280.0);

        state.scroll_by(-500.0, 400.0, 120.0);
        assert_eq!(state.horizontal_offset, 0.0);
    }

    #[test]
    fn ensure_range_visible_scrolls_left_when_selected_range_is_before_viewport() {
        let mut state = EditorTabStripState {
            horizontal_offset: 80.0,
            last_selected_index: None,
        };
        state.ensure_range_visible(20.0, 60.0, 400.0, 120.0);
        assert_eq!(state.horizontal_offset, 20.0);
    }

    #[test]
    fn ensure_range_visible_scrolls_right_when_selected_range_is_after_viewport() {
        let mut state = EditorTabStripState {
            horizontal_offset: 20.0,
            last_selected_index: None,
        };
        state.ensure_range_visible(130.0, 190.0, 400.0, 120.0);
        assert_eq!(state.horizontal_offset, 70.0);
    }

    #[test]
    fn button_scroll_step_has_reasonable_minimum() {
        assert_eq!(EditorTabStripState::button_scroll_step(40.0), 96.0);
        assert_eq!(EditorTabStripState::button_scroll_step(200.0), 150.0);
    }

    #[test]
    fn update_selection_visibility_only_forces_visibility_when_selection_changes() {
        let ranges = vec![(0.0, 80.0), (90.0, 170.0), (180.0, 260.0)];
        let mut state = EditorTabStripState {
            horizontal_offset: 120.0,
            last_selected_index: Some(0),
        };

        state.update_selection_visibility(Some(0), &ranges, 260.0, 120.0);
        assert_eq!(state.horizontal_offset, 120.0);

        state.update_selection_visibility(Some(2), &ranges, 260.0, 120.0);
        assert_eq!(state.horizontal_offset, 140.0);
    }
}
