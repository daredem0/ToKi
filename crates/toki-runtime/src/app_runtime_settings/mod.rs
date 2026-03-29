mod adjustments;
mod input;
mod overlay;
mod presentation;

use toki_core::menu::{MenuAppearance, MenuInput};

use super::App;
use crate::systems::FrameLimiter;

#[cfg(test)]
use adjustments::{
    adjust_channel, adjust_percent, channel_to_percent, cycle_post_process_mode,
    cycle_quantize_strategy, cycle_string, cycle_target_fps, fps_label, on_off_label,
    quantize_strategy_label, slider_to_target_fps, target_fps_to_slider,
};
pub(crate) use overlay::{GraphicsSettingKey, RuntimeMenuOverlay, RuntimeOverlayEntry};
use presentation::{
    compose_runtime_settings_ui, runtime_overlay_hit_target_at_position,
    runtime_overlay_slider_rect, slider_percent_from_position, RuntimeOverlayHitTarget,
};
#[cfg(test)]
use presentation::rect_contains;

const SETTING_STEP_PERCENT: u8 = 5;
const TINT_CHANNEL_STEP: u8 = 16;
const GB_CONTRAST_STEP: i16 = 5;
const BRIGHTNESS_STEP_PERCENT: i16 = 5;
const SATURATION_STEP_PERCENT: i16 = 10;

impl App {
    pub(super) fn rebuild_frame_timing_settings(&mut self) {
        self.frame_limiter = if self.launch_options.display.vsync {
            FrameLimiter::new_unlimited()
        } else {
            FrameLimiter::new_with_target_fps(self.launch_options.display.target_fps)
        };
        self.rendering.set_vsync(self.launch_options.display.vsync);
    }

    pub(super) fn handle_runtime_overlay_input(&mut self, input: MenuInput) -> bool {
        let Some(overlay) = self.runtime_overlay.clone() else {
            return false;
        };

        let should_close = match overlay {
            RuntimeMenuOverlay::Audio { .. } => self.handle_audio_overlay_input(input),
            RuntimeMenuOverlay::Display { .. } => self.handle_display_overlay_input(input),
            RuntimeMenuOverlay::Graphics { .. } => self.handle_graphics_overlay_input(input),
        };

        if should_close {
            self.runtime_overlay = None;
            self.runtime_overlay_slider_drag = None;
        }
        true
    }

    pub(super) fn render_runtime_settings_overlay(
        &mut self,
        appearance: &MenuAppearance,
        viewport: glam::Vec2,
    ) -> bool {
        let Some(presentation) = self.runtime_overlay_presentation(appearance, viewport) else {
            return false;
        };
        let composition = compose_runtime_settings_ui(
            &presentation.layout.entries,
            &presentation.entries,
            &presentation.layout,
            appearance,
        );
        self.rendering.render_viewport_ui_composition(&composition);
        true
    }

    pub(super) fn handle_runtime_overlay_pointer_hover(
        &mut self,
        position: glam::Vec2,
        viewport: glam::Vec2,
    ) -> bool {
        let appearance =
            self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);
        let Some(presentation) = self.runtime_overlay_presentation(&appearance, viewport) else {
            return false;
        };
        let Some(target) = runtime_overlay_hit_target_at_position(
            &presentation.layout.entries,
            &presentation.entries,
            position,
        ) else {
            return false;
        };
        self.select_runtime_overlay_entry(match target {
            RuntimeOverlayHitTarget::Entry(entry_index)
            | RuntimeOverlayHitTarget::Slider { entry_index, .. } => entry_index,
        });
        true
    }

    pub(super) fn handle_runtime_overlay_pointer_click(
        &mut self,
        position: glam::Vec2,
        viewport: glam::Vec2,
    ) -> bool {
        let appearance =
            self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);
        let Some(presentation) = self.runtime_overlay_presentation(&appearance, viewport) else {
            return false;
        };
        let Some(target) = runtime_overlay_hit_target_at_position(
            &presentation.layout.entries,
            &presentation.entries,
            position,
        ) else {
            return false;
        };

        match target {
            RuntimeOverlayHitTarget::Entry(entry_index) => {
                self.select_runtime_overlay_entry(entry_index);
                let should_close = match self.runtime_overlay.clone() {
                    Some(RuntimeMenuOverlay::Audio { .. }) => {
                        self.handle_audio_overlay_input(MenuInput::Confirm)
                    }
                    Some(RuntimeMenuOverlay::Display { .. }) => {
                        self.handle_display_overlay_input(MenuInput::Confirm)
                    }
                    Some(RuntimeMenuOverlay::Graphics { .. }) => {
                        self.handle_graphics_overlay_input(MenuInput::Confirm)
                    }
                    None => false,
                };
                if should_close {
                    self.runtime_overlay = None;
                }
            }
            RuntimeOverlayHitTarget::Slider {
                entry_index,
                percent,
            } => {
                self.select_runtime_overlay_entry(entry_index);
                self.runtime_overlay_slider_drag = Some(entry_index);
                self.set_runtime_overlay_slider_percent(entry_index, percent);
            }
        }

        true
    }

    pub(super) fn handle_runtime_overlay_pointer_drag(
        &mut self,
        position: glam::Vec2,
        viewport: glam::Vec2,
    ) -> bool {
        let Some(entry_index) = self.runtime_overlay_slider_drag else {
            return false;
        };
        let appearance =
            self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);
        let Some(presentation) = self.runtime_overlay_presentation(&appearance, viewport) else {
            self.runtime_overlay_slider_drag = None;
            return false;
        };
        let Some((layout_entry, overlay_entry)) = presentation
            .layout
            .entries
            .get(entry_index)
            .zip(presentation.entries.get(entry_index))
        else {
            self.runtime_overlay_slider_drag = None;
            return false;
        };
        let Some(slider_rect) = runtime_overlay_slider_rect(layout_entry, overlay_entry) else {
            self.runtime_overlay_slider_drag = None;
            return false;
        };
        self.select_runtime_overlay_entry(entry_index);
        self.set_runtime_overlay_slider_percent(
            entry_index,
            slider_percent_from_position(slider_rect, position.x),
        );
        true
    }

    pub(super) fn clear_runtime_overlay_pointer_drag(&mut self) {
        self.runtime_overlay_slider_drag = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        adjust_channel, adjust_percent, channel_to_percent, cycle_post_process_mode,
        cycle_quantize_strategy, cycle_string, cycle_target_fps, fps_label, on_off_label,
        quantize_strategy_label, rect_contains, runtime_overlay_hit_target_at_position,
        runtime_overlay_slider_rect, slider_percent_from_position, slider_to_target_fps,
        target_fps_to_slider, RuntimeMenuOverlay, RuntimeOverlayEntry, RuntimeOverlayHitTarget,
    };
    use toki_core::menu::{MenuBorderStyle, MenuEntryLayout};
    use toki_core::project_runtime::{PostProcessMode, QuantizeStrategy};
    use toki_core::ui::UiRect;

    #[test]
    fn percent_adjustment_clamps_to_zero_and_hundred() {
        let mut value = 3;
        adjust_percent(&mut value, -10);
        assert_eq!(value, 0);
        adjust_percent(&mut value, 250);
        assert_eq!(value, 100);
    }

    #[test]
    fn channel_adjustment_clamps_to_u8_bounds() {
        let mut value = 250;
        adjust_channel(&mut value, 20);
        assert_eq!(value, 255);
        adjust_channel(&mut value, -400);
        assert_eq!(value, 0);
    }

    #[test]
    fn channel_to_percent_handles_full_u8_range_without_overflow() {
        assert_eq!(channel_to_percent(0), 0);
        assert_eq!(channel_to_percent(255), 100);
        assert_eq!(channel_to_percent(128), 50);
    }

    #[test]
    fn mode_cycle_wraps_in_both_directions() {
        assert_eq!(
            cycle_post_process_mode(PostProcessMode::None, -1),
            PostProcessMode::Vignette
        );
        assert_eq!(
            cycle_post_process_mode(PostProcessMode::Vignette, 1),
            PostProcessMode::None
        );
    }

    #[test]
    fn quantize_strategy_cycle_wraps_in_both_directions() {
        assert_eq!(
            cycle_quantize_strategy(QuantizeStrategy::Luminance, -1),
            QuantizeStrategy::RgbDistance
        );
        assert_eq!(
            cycle_quantize_strategy(QuantizeStrategy::RgbDistance, 1),
            QuantizeStrategy::Luminance
        );
    }

    #[test]
    fn quantize_strategy_label_matches_ui_text() {
        assert_eq!(
            quantize_strategy_label(QuantizeStrategy::Luminance),
            "Luminance"
        );
        assert_eq!(
            quantize_strategy_label(QuantizeStrategy::RgbDistance),
            "RGB Distance"
        );
    }

    #[test]
    fn string_cycle_wraps() {
        let mut value = "b".to_string();
        cycle_string(
            &mut value,
            &["a".to_string(), "b".to_string(), "c".to_string()],
            1,
        );
        assert_eq!(value, "c");
        cycle_string(
            &mut value,
            &["a".to_string(), "b".to_string(), "c".to_string()],
            1,
        );
        assert_eq!(value, "a");
    }

    #[test]
    fn target_fps_cycle_wraps_in_both_directions() {
        assert_eq!(cycle_target_fps(0, -1), 144);
        assert_eq!(cycle_target_fps(144, 1), 0);
        assert_eq!(cycle_target_fps(60, 1), 90);
    }

    #[test]
    fn fps_label_formats_unlimited() {
        assert_eq!(fps_label(0), "Unlimited");
        assert_eq!(fps_label(60), "60");
    }

    #[test]
    fn target_fps_slider_maps_known_values() {
        assert_eq!(target_fps_to_slider(0), 0);
        assert_eq!(target_fps_to_slider(144), 100);
        assert_eq!(target_fps_to_slider(60), 42);
    }

    #[test]
    fn slider_to_target_fps_maps_extremes_and_midpoints() {
        assert_eq!(slider_to_target_fps(0), 0);
        assert_eq!(slider_to_target_fps(100), 144);
        assert_eq!(slider_to_target_fps(42), 60);
    }

    #[test]
    fn on_off_label_is_human_readable() {
        assert_eq!(on_off_label(true), "On");
        assert_eq!(on_off_label(false), "Off");
    }

    #[test]
    fn overlay_constructors_start_on_first_entry() {
        assert_eq!(
            RuntimeMenuOverlay::audio(),
            RuntimeMenuOverlay::Audio { selected_index: 0 }
        );
        assert_eq!(
            RuntimeMenuOverlay::graphics(),
            RuntimeMenuOverlay::Graphics { selected_index: 0 }
        );
        assert_eq!(
            RuntimeMenuOverlay::display(),
            RuntimeMenuOverlay::Display { selected_index: 0 }
        );
    }

    #[test]
    fn slider_rect_and_hit_target_detect_slider_before_entry_body() {
        let layout_entry = MenuEntryLayout {
            rect: UiRect {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 20.0,
            },
            text: "Master: 50%".to_string(),
            selected: true,
            selectable: true,
            border_style: MenuBorderStyle::Square,
        };
        let overlay_entry = RuntimeOverlayEntry {
            label: "Master".to_string(),
            value_text: "50%".to_string(),
            slider_percent: Some(50),
            selected: true,
        };
        let slider_rect =
            runtime_overlay_slider_rect(&layout_entry, &overlay_entry).expect("slider");
        assert!(rect_contains(
            slider_rect,
            glam::Vec2::new(slider_rect.x + 1.0, slider_rect.y + 1.0)
        ));
        assert_eq!(
            runtime_overlay_hit_target_at_position(
                &[layout_entry],
                &[overlay_entry],
                glam::Vec2::new(
                    slider_rect.x + slider_rect.width * 0.75,
                    slider_rect.y + 2.0
                ),
            ),
            Some(RuntimeOverlayHitTarget::Slider {
                entry_index: 0,
                percent: 75,
            })
        );
    }

    #[test]
    fn slider_percent_from_position_clamps_to_bounds() {
        let rect = UiRect {
            x: 20.0,
            y: 10.0,
            width: 80.0,
            height: 8.0,
        };
        assert_eq!(slider_percent_from_position(rect, 20.0), 0);
        assert_eq!(slider_percent_from_position(rect, 60.0), 50);
        assert_eq!(slider_percent_from_position(rect, 120.0), 100);
    }
}
