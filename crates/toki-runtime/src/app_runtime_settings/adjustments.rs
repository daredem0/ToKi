use std::collections::BTreeSet;

use toki_core::palette::builtin_palettes;
use toki_core::project_runtime::{PostProcessMode, QuantizeStrategy};

use super::{App, GraphicsSettingKey, RuntimeMenuOverlay};

pub(super) fn available_palette_ids(resources: &crate::systems::ResourceManager) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for id in builtin_palettes().into_keys() {
        ids.insert(id);
    }
    for id in resources.project_palettes().keys() {
        ids.insert(id.clone());
    }
    ids.into_iter().collect()
}

pub(super) fn adjust_percent(value: &mut u8, delta: i16) {
    let next = *value as i16 + delta;
    *value = next.clamp(0, 100) as u8;
}

pub(super) fn adjust_channel(value: &mut u8, delta: i16) {
    let next = *value as i16 + delta;
    *value = next.clamp(0, 255) as u8;
}

pub(super) fn cycle_target_fps(current: u32, direction: i32) -> u32 {
    const TARGET_FPS_OPTIONS: [u32; 7] = [0, 30, 45, 60, 90, 120, 144];
    let current_index = TARGET_FPS_OPTIONS
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(3) as i32;
    let next_index =
        (current_index + direction).rem_euclid(TARGET_FPS_OPTIONS.len() as i32) as usize;
    TARGET_FPS_OPTIONS[next_index]
}

pub(super) fn target_fps_to_slider(fps: u32) -> u8 {
    match fps {
        0 => 0,
        30 => 17,
        45 => 31,
        60 => 42,
        90 => 63,
        120 => 83,
        144 => 100,
        _ => 42,
    }
}

pub(super) fn slider_to_target_fps(percent: u8) -> u32 {
    const TARGET_FPS_OPTIONS: [u32; 7] = [0, 30, 45, 60, 90, 120, 144];
    let index = (((percent as f32 / 100.0) * (TARGET_FPS_OPTIONS.len() as f32 - 1.0)).round()
        as usize)
        .min(TARGET_FPS_OPTIONS.len() - 1);
    TARGET_FPS_OPTIONS[index]
}

pub(super) fn fps_label(fps: u32) -> String {
    if fps == 0 {
        "Unlimited".to_string()
    } else {
        fps.to_string()
    }
}

pub(super) fn on_off_label(value: bool) -> &'static str {
    if value {
        "On"
    } else {
        "Off"
    }
}

pub(super) fn channel_to_percent(value: u8) -> u8 {
    ((value as u16 * 100) / 255) as u8
}

pub(super) fn percent_to_channel(value: u8) -> u8 {
    ((value as u16 * 255) / 100) as u8
}

pub(super) fn cycle_post_process_mode(mode: PostProcessMode, direction: i32) -> PostProcessMode {
    let modes = [
        PostProcessMode::None,
        PostProcessMode::Tint,
        PostProcessMode::BrightnessSaturation,
        PostProcessMode::Quantize4,
        PostProcessMode::OrderedDitherQuantize,
        PostProcessMode::GbPalette,
        PostProcessMode::Vignette,
    ];
    let current_index = modes
        .iter()
        .position(|candidate| *candidate == mode)
        .unwrap_or(0) as i32;
    let next_index = (current_index + direction).rem_euclid(modes.len() as i32) as usize;
    modes[next_index]
}

pub(super) fn cycle_string(current: &mut String, options: &[String], direction: i32) {
    let current_index = options
        .iter()
        .position(|option| option == current)
        .unwrap_or(0) as i32;
    let next_index = (current_index + direction).rem_euclid(options.len() as i32) as usize;
    *current = options[next_index].clone();
}

pub(super) fn post_process_mode_label(mode: PostProcessMode) -> &'static str {
    match mode {
        PostProcessMode::None => "None",
        PostProcessMode::Tint => "Tint",
        PostProcessMode::BrightnessSaturation => "Bright/Sat",
        PostProcessMode::Quantize4 => "Quantize 4",
        PostProcessMode::OrderedDitherQuantize => "Dither Quantize",
        PostProcessMode::GbPalette => "GB Preset",
        PostProcessMode::Vignette => "Vignette",
    }
}

pub(super) fn quantize_strategy_label(strategy: QuantizeStrategy) -> &'static str {
    match strategy {
        QuantizeStrategy::Luminance => "Luminance",
        QuantizeStrategy::RgbDistance => "RGB Distance",
    }
}

pub(super) fn cycle_quantize_strategy(
    strategy: QuantizeStrategy,
    direction: i32,
) -> QuantizeStrategy {
    let strategies = [QuantizeStrategy::Luminance, QuantizeStrategy::RgbDistance];
    let current_index = strategies
        .iter()
        .position(|candidate| *candidate == strategy)
        .unwrap_or(0) as i32;
    let next_index = (current_index + direction).rem_euclid(strategies.len() as i32) as usize;
    strategies[next_index]
}

impl App {
    pub(super) fn adjust_audio_setting(&mut self, selected_index: usize, delta: i16) {
        match selected_index {
            0 => adjust_percent(&mut self.launch_options.audio_mix.master_percent, delta),
            1 => adjust_percent(&mut self.launch_options.audio_mix.music_percent, delta),
            2 => adjust_percent(&mut self.launch_options.audio_mix.movement_percent, delta),
            3 => adjust_percent(&mut self.launch_options.audio_mix.collision_percent, delta),
            _ => return,
        }
        self.apply_live_audio_mix_settings();
        self.persist_runtime_settings_if_possible();
    }

    pub(super) fn adjust_graphics_setting(&mut self, selected_index: usize, direction: i32) {
        let entries = self.graphics_entries_with_keys(selected_index);
        let Some((selected_key, _)) = entries.get(selected_index) else {
            return;
        };

        match *selected_key {
            GraphicsSettingKey::Vsync => {
                self.launch_options.display.vsync = !self.launch_options.display.vsync;
                self.rebuild_frame_timing_settings();
            }
            GraphicsSettingKey::TargetFps => {
                self.launch_options.display.target_fps =
                    cycle_target_fps(self.launch_options.display.target_fps, direction);
                if !self.launch_options.display.vsync {
                    self.rebuild_frame_timing_settings();
                }
            }
            GraphicsSettingKey::PostProcessMode => {
                self.launch_options.display.post_process.mode = cycle_post_process_mode(
                    self.launch_options.display.post_process.mode,
                    direction,
                );
            }
            GraphicsSettingKey::QuantizeStrategy => {
                self.launch_options.display.post_process.quantize_strategy =
                    cycle_quantize_strategy(
                        self.launch_options.display.post_process.quantize_strategy,
                        direction,
                    );
            }
            GraphicsSettingKey::Brightness => {
                let next = self.launch_options.display.post_process.brightness_percent
                    + super::BRIGHTNESS_STEP_PERCENT * direction as i16;
                self.launch_options.display.post_process.brightness_percent = next.clamp(-100, 100);
            }
            GraphicsSettingKey::Saturation => {
                let next = self.launch_options.display.post_process.saturation_percent as i16
                    + super::SATURATION_STEP_PERCENT * direction as i16;
                self.launch_options.display.post_process.saturation_percent =
                    next.clamp(0, 200) as u8;
            }
            GraphicsSettingKey::TintStrength => adjust_percent(
                &mut self
                    .launch_options
                    .display
                    .post_process
                    .tint_strength_percent,
                (super::SETTING_STEP_PERCENT as i16) * direction as i16,
            ),
            GraphicsSettingKey::TintRed => adjust_channel(
                &mut self.launch_options.display.post_process.tint_color[0],
                super::TINT_CHANNEL_STEP as i16 * direction as i16,
            ),
            GraphicsSettingKey::TintGreen => adjust_channel(
                &mut self.launch_options.display.post_process.tint_color[1],
                super::TINT_CHANNEL_STEP as i16 * direction as i16,
            ),
            GraphicsSettingKey::TintBlue => adjust_channel(
                &mut self.launch_options.display.post_process.tint_color[2],
                super::TINT_CHANNEL_STEP as i16 * direction as i16,
            ),
            GraphicsSettingKey::QuantizePalette => {
                let palette_ids = available_palette_ids(&self.resources);
                if !palette_ids.is_empty() {
                    cycle_string(
                        &mut self.launch_options.display.post_process.quantize_palette_id,
                        &palette_ids,
                        direction,
                    );
                }
            }
            GraphicsSettingKey::GbContrast => {
                let next = self.launch_options.display.post_process.gb_contrast_percent
                    + super::GB_CONTRAST_STEP * direction as i16;
                self.launch_options.display.post_process.gb_contrast_percent =
                    next.clamp(-100, 100);
            }
            GraphicsSettingKey::VignetteStrength => adjust_percent(
                &mut self
                    .launch_options
                    .display
                    .post_process
                    .vignette_strength_percent,
                (super::SETTING_STEP_PERCENT as i16) * direction as i16,
            ),
            GraphicsSettingKey::Back => return,
        }

        self.rendering
            .set_post_process_settings(self.resolved_post_process_settings());
        self.persist_runtime_settings_if_possible();
    }

    pub(super) fn set_runtime_overlay_slider_percent(&mut self, entry_index: usize, percent: u8) {
        match self.runtime_overlay {
            Some(RuntimeMenuOverlay::Audio { .. }) => {
                self.set_audio_slider_percent(entry_index, percent)
            }
            Some(RuntimeMenuOverlay::Display { .. }) => {
                self.set_display_slider_percent(entry_index, percent)
            }
            Some(RuntimeMenuOverlay::Graphics { .. }) => {
                self.set_graphics_slider_percent(entry_index, percent)
            }
            None => {}
        }
    }

    fn set_audio_slider_percent(&mut self, entry_index: usize, percent: u8) {
        match entry_index {
            0 => self.launch_options.audio_mix.master_percent = percent.min(100),
            1 => self.launch_options.audio_mix.music_percent = percent.min(100),
            2 => self.launch_options.audio_mix.movement_percent = percent.min(100),
            3 => self.launch_options.audio_mix.collision_percent = percent.min(100),
            _ => return,
        }
        self.adjust_audio_setting(entry_index, 0);
    }

    fn set_graphics_slider_percent(&mut self, entry_index: usize, percent: u8) {
        let entries = self.graphics_entries_with_keys(entry_index);
        let Some((selected_key, _)) = entries.get(entry_index) else {
            return;
        };

        match *selected_key {
            GraphicsSettingKey::TargetFps => {
                self.launch_options.display.target_fps = slider_to_target_fps(percent);
                if !self.launch_options.display.vsync {
                    self.rebuild_frame_timing_settings();
                }
            }
            GraphicsSettingKey::Brightness => {
                self.launch_options.display.post_process.brightness_percent =
                    (percent as i16 * 2 - 100).clamp(-100, 100);
            }
            GraphicsSettingKey::Saturation => {
                self.launch_options.display.post_process.saturation_percent =
                    (percent as u16 * 2).min(200) as u8;
            }
            GraphicsSettingKey::TintStrength => {
                self.launch_options
                    .display
                    .post_process
                    .tint_strength_percent = percent.min(100);
            }
            GraphicsSettingKey::TintRed => {
                self.launch_options.display.post_process.tint_color[0] =
                    percent_to_channel(percent);
            }
            GraphicsSettingKey::TintGreen => {
                self.launch_options.display.post_process.tint_color[1] =
                    percent_to_channel(percent);
            }
            GraphicsSettingKey::TintBlue => {
                self.launch_options.display.post_process.tint_color[2] =
                    percent_to_channel(percent);
            }
            GraphicsSettingKey::GbContrast => {
                self.launch_options.display.post_process.gb_contrast_percent =
                    (percent as i16 * 2 - 100).clamp(-100, 100);
            }
            GraphicsSettingKey::VignetteStrength => {
                self.launch_options
                    .display
                    .post_process
                    .vignette_strength_percent = percent.min(100);
            }
            GraphicsSettingKey::Vsync
            | GraphicsSettingKey::PostProcessMode
            | GraphicsSettingKey::QuantizeStrategy
            | GraphicsSettingKey::QuantizePalette
            | GraphicsSettingKey::Back => return,
        }

        self.rendering
            .set_post_process_settings(self.resolved_post_process_settings());
        self.persist_runtime_settings_if_possible();
    }
}
