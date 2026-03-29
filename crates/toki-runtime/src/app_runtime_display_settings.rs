use toki_core::menu::MenuInput;
use toki_core::project_runtime::{IntegerScaleFactor, RuntimeViewportMode};

use super::app_runtime_settings::{RuntimeMenuOverlay, RuntimeOverlayEntry};
use super::App;

const DISPLAY_SETTING_STEP_PERCENT: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisplaySettingKey {
    ViewportMode,
    AspectFitPercent,
    IntegerScaleFactor,
    WindowFillZoomPercent,
    Back,
}

impl App {
    pub(super) fn apply_live_display_settings(&mut self) {
        self.rendering.set_desired_resolution(
            self.launch_options.display.resolution_width,
            self.launch_options.display.resolution_height,
        );
        self.rendering
            .set_viewport_mode(self.launch_options.display.viewport);
        self.camera_system.camera_mut().zoom = self.launch_options.display.zoom_factor().max(0.1);
        let world_bounds = self.current_world_bounds();
        self.sync_runtime_viewport_to_window(world_bounds);
        self.refresh_tilemap_vertices_for_current_camera();
        self.platform.request_redraw();
        self.persist_runtime_settings_if_possible();
    }

    pub(super) fn display_overlay_entries(
        &self,
        selected_index: usize,
    ) -> Vec<RuntimeOverlayEntry> {
        self.display_entries_with_keys(selected_index)
            .into_iter()
            .map(|(_, entry)| entry)
            .collect()
    }

    fn display_entries_with_keys(
        &self,
        selected_index: usize,
    ) -> Vec<(DisplaySettingKey, RuntimeOverlayEntry)> {
        let viewport = self.launch_options.display.viewport;
        let mut entries = vec![(
            DisplaySettingKey::ViewportMode,
            RuntimeOverlayEntry {
                label: "Viewport Mode".to_string(),
                value_text: viewport_mode_label(viewport).to_string(),
                slider_percent: None,
                selected: false,
            },
        )];

        match viewport {
            RuntimeViewportMode::AspectFit { fit_percent } => {
                entries.push((
                    DisplaySettingKey::AspectFitPercent,
                    RuntimeOverlayEntry {
                        label: "Fit Percent".to_string(),
                        value_text: format!("{fit_percent}%"),
                        slider_percent: Some(fit_percent.min(100) as u8),
                        selected: false,
                    },
                ));
            }
            RuntimeViewportMode::IntegerScale { factor } => {
                entries.push((
                    DisplaySettingKey::IntegerScaleFactor,
                    RuntimeOverlayEntry {
                        label: "Scale Factor".to_string(),
                        value_text: integer_scale_factor_label(factor),
                        slider_percent: None,
                        selected: false,
                    },
                ));
            }
            RuntimeViewportMode::WindowFill { zoom_percent } => {
                entries.push((
                    DisplaySettingKey::WindowFillZoomPercent,
                    RuntimeOverlayEntry {
                        label: "Zoom Percent".to_string(),
                        value_text: format!("{zoom_percent}%"),
                        slider_percent: Some(window_fill_zoom_to_slider(zoom_percent)),
                        selected: false,
                    },
                ));
            }
        }

        entries.push((
            DisplaySettingKey::Back,
            RuntimeOverlayEntry {
                label: "Back".to_string(),
                value_text: "Close".to_string(),
                slider_percent: None,
                selected: false,
            },
        ));

        let selected_index = selected_index.min(entries.len().saturating_sub(1));
        if let Some((_, entry)) = entries.get_mut(selected_index) {
            entry.selected = true;
        }
        entries
    }

    pub(super) fn handle_display_overlay_input(&mut self, input: MenuInput) -> bool {
        let mut selected_index = match self.runtime_overlay.as_ref() {
            Some(RuntimeMenuOverlay::Display { selected_index }) => *selected_index,
            _ => return false,
        };
        let entry_count = self.display_entries_with_keys(selected_index).len();
        selected_index = selected_index.min(entry_count.saturating_sub(1));

        let next_selected = match input {
            MenuInput::Up => Some(selected_index.saturating_sub(1)),
            MenuInput::Down => Some((selected_index + 1).min(entry_count.saturating_sub(1))),
            _ => None,
        };
        if let Some(next_selected) = next_selected {
            let RuntimeMenuOverlay::Display { selected_index } = self
                .runtime_overlay
                .as_mut()
                .expect("display overlay must be active")
            else {
                return false;
            };
            *selected_index = next_selected;
            return false;
        }

        match input {
            MenuInput::Left => self.adjust_display_setting(selected_index, -1),
            MenuInput::Right | MenuInput::Confirm => {
                let selected_key = self.display_entries_with_keys(selected_index)[selected_index].0;
                if selected_key == DisplaySettingKey::Back && matches!(input, MenuInput::Confirm) {
                    return true;
                }
                self.adjust_display_setting(selected_index, 1);
            }
            MenuInput::Back => return true,
            MenuInput::Up | MenuInput::Down => {}
        }
        false
    }

    fn adjust_display_setting(&mut self, selected_index: usize, direction: i32) {
        let entries = self.display_entries_with_keys(selected_index);
        let Some((selected_key, _)) = entries.get(selected_index) else {
            return;
        };
        let Some(next_viewport) =
            adjusted_display_viewport(self.launch_options.display.viewport, *selected_key, direction)
        else {
            return;
        };
        self.launch_options.display.viewport = next_viewport;

        self.apply_live_display_settings();
    }

    pub(super) fn set_display_slider_percent(&mut self, entry_index: usize, percent: u8) {
        let entries = self.display_entries_with_keys(entry_index);
        let Some((selected_key, _)) = entries.get(entry_index) else {
            return;
        };
        let Some(next_viewport) =
            slider_adjusted_display_viewport(self.launch_options.display.viewport, *selected_key, percent)
        else {
            return;
        };
        self.launch_options.display.viewport = next_viewport;

        self.apply_live_display_settings();
    }
}

fn adjusted_display_viewport(
    viewport: RuntimeViewportMode,
    key: DisplaySettingKey,
    direction: i32,
) -> Option<RuntimeViewportMode> {
    match (key, viewport) {
        (DisplaySettingKey::ViewportMode, viewport) => Some(cycle_viewport_mode(viewport, direction)),
        (
            DisplaySettingKey::AspectFitPercent,
            RuntimeViewportMode::AspectFit { fit_percent },
        ) => Some(RuntimeViewportMode::AspectFit {
            fit_percent: ((fit_percent as i32)
                + DISPLAY_SETTING_STEP_PERCENT as i32 * direction)
                .clamp(1, 100) as u16,
        }),
        (
            DisplaySettingKey::IntegerScaleFactor,
            RuntimeViewportMode::IntegerScale { factor },
        ) => Some(RuntimeViewportMode::IntegerScale {
            factor: cycle_integer_scale_factor(factor, direction),
        }),
        (
            DisplaySettingKey::WindowFillZoomPercent,
            RuntimeViewportMode::WindowFill { zoom_percent },
        ) => Some(RuntimeViewportMode::WindowFill {
            zoom_percent: ((zoom_percent as i32) + 10 * direction).clamp(50, 200) as u16,
        }),
        (DisplaySettingKey::Back, _) => None,
        _ => None,
    }
}

fn slider_adjusted_display_viewport(
    viewport: RuntimeViewportMode,
    key: DisplaySettingKey,
    percent: u8,
) -> Option<RuntimeViewportMode> {
    match (key, viewport) {
        (
            DisplaySettingKey::AspectFitPercent,
            RuntimeViewportMode::AspectFit { .. },
        ) => Some(RuntimeViewportMode::AspectFit {
            fit_percent: percent.max(1) as u16,
        }),
        (
            DisplaySettingKey::WindowFillZoomPercent,
            RuntimeViewportMode::WindowFill { .. },
        ) => Some(RuntimeViewportMode::WindowFill {
            zoom_percent: slider_to_window_fill_zoom(percent),
        }),
        _ => None,
    }
}

fn viewport_mode_label(mode: RuntimeViewportMode) -> &'static str {
    match mode {
        RuntimeViewportMode::AspectFit { .. } => "Aspect Fit",
        RuntimeViewportMode::IntegerScale { .. } => "Integer Scale",
        RuntimeViewportMode::WindowFill { .. } => "Window Fill",
    }
}

fn cycle_viewport_mode(mode: RuntimeViewportMode, direction: i32) -> RuntimeViewportMode {
    let current_index = match mode {
        RuntimeViewportMode::AspectFit { .. } => 0,
        RuntimeViewportMode::IntegerScale { .. } => 1,
        RuntimeViewportMode::WindowFill { .. } => 2,
    };
    match (current_index + direction).rem_euclid(3) {
        0 => RuntimeViewportMode::AspectFit { fit_percent: 100 },
        1 => RuntimeViewportMode::IntegerScale {
            factor: IntegerScaleFactor::Auto,
        },
        _ => RuntimeViewportMode::WindowFill { zoom_percent: 100 },
    }
}

fn integer_scale_factor_label(factor: IntegerScaleFactor) -> String {
    match factor {
        IntegerScaleFactor::Auto => "Auto".to_string(),
        IntegerScaleFactor::Fixed(value) => format!("{value}x"),
    }
}

fn cycle_integer_scale_factor(factor: IntegerScaleFactor, direction: i32) -> IntegerScaleFactor {
    const OPTIONS: [IntegerScaleFactor; 9] = [
        IntegerScaleFactor::Auto,
        IntegerScaleFactor::Fixed(1),
        IntegerScaleFactor::Fixed(2),
        IntegerScaleFactor::Fixed(3),
        IntegerScaleFactor::Fixed(4),
        IntegerScaleFactor::Fixed(5),
        IntegerScaleFactor::Fixed(6),
        IntegerScaleFactor::Fixed(7),
        IntegerScaleFactor::Fixed(8),
    ];
    let current_index = OPTIONS
        .iter()
        .position(|candidate| *candidate == factor)
        .unwrap_or(0) as i32;
    OPTIONS[(current_index + direction).rem_euclid(OPTIONS.len() as i32) as usize]
}

fn window_fill_zoom_to_slider(percent: u16) -> u8 {
    (((percent.clamp(50, 200) - 50) as f32 / 150.0) * 100.0).round() as u8
}

fn slider_to_window_fill_zoom(percent: u8) -> u16 {
    (50.0 + 150.0 * (percent as f32 / 100.0)).round() as u16
}

#[cfg(test)]
mod tests {
    use super::{
        adjusted_display_viewport, cycle_integer_scale_factor, cycle_viewport_mode,
        integer_scale_factor_label, slider_adjusted_display_viewport, slider_to_window_fill_zoom,
        viewport_mode_label, window_fill_zoom_to_slider, DisplaySettingKey,
    };
    use toki_core::project_runtime::{IntegerScaleFactor, RuntimeViewportMode};

    #[test]
    fn viewport_mode_cycle_wraps_in_both_directions() {
        assert_eq!(
            cycle_viewport_mode(RuntimeViewportMode::AspectFit { fit_percent: 100 }, -1),
            RuntimeViewportMode::WindowFill { zoom_percent: 100 }
        );
        assert_eq!(
            cycle_viewport_mode(RuntimeViewportMode::WindowFill { zoom_percent: 100 }, 1),
            RuntimeViewportMode::AspectFit { fit_percent: 100 }
        );
    }

    #[test]
    fn integer_scale_factor_cycle_wraps_in_both_directions() {
        assert_eq!(
            cycle_integer_scale_factor(IntegerScaleFactor::Auto, -1),
            IntegerScaleFactor::Fixed(8)
        );
        assert_eq!(
            cycle_integer_scale_factor(IntegerScaleFactor::Fixed(8), 1),
            IntegerScaleFactor::Auto
        );
    }

    #[test]
    fn viewport_labels_are_human_readable() {
        assert_eq!(
            viewport_mode_label(RuntimeViewportMode::AspectFit { fit_percent: 100 }),
            "Aspect Fit"
        );
        assert_eq!(integer_scale_factor_label(IntegerScaleFactor::Auto), "Auto");
        assert_eq!(
            integer_scale_factor_label(IntegerScaleFactor::Fixed(3)),
            "3x"
        );
    }

    #[test]
    fn window_fill_zoom_slider_round_trips_common_values() {
        assert_eq!(window_fill_zoom_to_slider(50), 0);
        assert_eq!(slider_to_window_fill_zoom(0), 50);
        assert_eq!(slider_to_window_fill_zoom(100), 200);
    }

    #[test]
    fn display_overlay_adjustment_cycles_viewport_mode() {
        assert_eq!(
            adjusted_display_viewport(
                RuntimeViewportMode::IntegerScale {
                    factor: IntegerScaleFactor::Auto,
                },
                DisplaySettingKey::ViewportMode,
                -1,
            ),
            Some(RuntimeViewportMode::AspectFit { fit_percent: 100 })
        );
        assert_eq!(
            adjusted_display_viewport(
                RuntimeViewportMode::WindowFill { zoom_percent: 100 },
                DisplaySettingKey::ViewportMode,
                1,
            ),
            Some(RuntimeViewportMode::AspectFit { fit_percent: 100 })
        );
    }

    #[test]
    fn display_overlay_adjustment_updates_mode_specific_values() {
        assert_eq!(
            adjusted_display_viewport(
                RuntimeViewportMode::AspectFit { fit_percent: 100 },
                DisplaySettingKey::AspectFitPercent,
                -1,
            ),
            Some(RuntimeViewportMode::AspectFit { fit_percent: 95 })
        );
        assert!(
            matches!(
                adjusted_display_viewport(
                    RuntimeViewportMode::IntegerScale {
                        factor: IntegerScaleFactor::Auto,
                    },
                    DisplaySettingKey::IntegerScaleFactor,
                    1,
                ),
                Some(RuntimeViewportMode::IntegerScale {
                    factor: IntegerScaleFactor::Fixed(1),
                })
            )
        );
        assert_eq!(
            adjusted_display_viewport(
                RuntimeViewportMode::WindowFill { zoom_percent: 100 },
                DisplaySettingKey::WindowFillZoomPercent,
                1,
            ),
            Some(RuntimeViewportMode::WindowFill { zoom_percent: 110 })
        );
    }

    #[test]
    fn display_overlay_slider_updates_supported_viewport_values() {
        assert_eq!(
            slider_adjusted_display_viewport(
                RuntimeViewportMode::AspectFit { fit_percent: 100 },
                DisplaySettingKey::AspectFitPercent,
                42,
            ),
            Some(RuntimeViewportMode::AspectFit { fit_percent: 42 })
        );
        assert_eq!(
            slider_adjusted_display_viewport(
                RuntimeViewportMode::WindowFill { zoom_percent: 100 },
                DisplaySettingKey::WindowFillZoomPercent,
                100,
            ),
            Some(RuntimeViewportMode::WindowFill { zoom_percent: 200 })
        );
    }
}
