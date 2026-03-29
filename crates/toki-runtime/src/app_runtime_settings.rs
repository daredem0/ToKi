use std::collections::BTreeSet;

use toki_core::menu::{
    build_menu_layout, compose_menu_ui, menu_hex_color_rgba, MenuAppearance, MenuEntryLayout,
    MenuInput, MenuLayout, MenuView, MenuViewEntry,
};
use toki_core::palette::builtin_palettes;
use toki_core::project_runtime::{PostProcessMode, QuantizeStrategy};
use toki_core::ui::{UiBlock, UiComposition, UiRect};

use super::App;
use crate::systems::FrameLimiter;

const SETTING_STEP_PERCENT: u8 = 5;
const TINT_CHANNEL_STEP: u8 = 16;
const GB_CONTRAST_STEP: i16 = 5;
const BRIGHTNESS_STEP_PERCENT: i16 = 5;
const SATURATION_STEP_PERCENT: i16 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RuntimeMenuOverlay {
    Audio { selected_index: usize },
    Graphics { selected_index: usize },
}

impl RuntimeMenuOverlay {
    pub(super) fn audio() -> Self {
        Self::Audio { selected_index: 0 }
    }

    pub(super) fn graphics() -> Self {
        Self::Graphics { selected_index: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeOverlayEntry {
    label: String,
    value_text: String,
    slider_percent: Option<u8>,
    selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct RuntimeOverlayPresentation {
    layout: MenuLayout,
    entries: Vec<RuntimeOverlayEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeOverlayHitTarget {
    Entry(usize),
    Slider { entry_index: usize, percent: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphicsSettingKey {
    Vsync,
    TargetFps,
    PostProcessMode,
    QuantizeStrategy,
    Brightness,
    Saturation,
    TintStrength,
    TintRed,
    TintGreen,
    TintBlue,
    QuantizePalette,
    GbContrast,
    VignetteStrength,
    Back,
}

impl App {
    fn rebuild_frame_timing_settings(&mut self) {
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

    fn audio_overlay_entries(&self, selected_index: usize) -> Vec<RuntimeOverlayEntry> {
        vec![
            RuntimeOverlayEntry {
                label: "Master".to_string(),
                value_text: format!("{}%", self.launch_options.audio_mix.master_percent),
                slider_percent: Some(self.launch_options.audio_mix.master_percent),
                selected: selected_index == 0,
            },
            RuntimeOverlayEntry {
                label: "Music".to_string(),
                value_text: format!("{}%", self.launch_options.audio_mix.music_percent),
                slider_percent: Some(self.launch_options.audio_mix.music_percent),
                selected: selected_index == 1,
            },
            RuntimeOverlayEntry {
                label: "Movement".to_string(),
                value_text: format!("{}%", self.launch_options.audio_mix.movement_percent),
                slider_percent: Some(self.launch_options.audio_mix.movement_percent),
                selected: selected_index == 2,
            },
            RuntimeOverlayEntry {
                label: "Collision".to_string(),
                value_text: format!("{}%", self.launch_options.audio_mix.collision_percent),
                slider_percent: Some(self.launch_options.audio_mix.collision_percent),
                selected: selected_index == 3,
            },
            RuntimeOverlayEntry {
                label: "Back".to_string(),
                value_text: "Close".to_string(),
                slider_percent: None,
                selected: selected_index == 4,
            },
        ]
    }

    fn graphics_overlay_entries(&self, selected_index: usize) -> Vec<RuntimeOverlayEntry> {
        self.graphics_entries_with_keys(selected_index)
            .into_iter()
            .map(|(_, entry)| entry)
            .collect()
    }

    fn graphics_entries_with_keys(
        &self,
        selected_index: usize,
    ) -> Vec<(GraphicsSettingKey, RuntimeOverlayEntry)> {
        let post = &self.launch_options.display.post_process;
        let mut entries = vec![
            (
                GraphicsSettingKey::Vsync,
                RuntimeOverlayEntry {
                    label: "VSync".to_string(),
                    value_text: on_off_label(self.launch_options.display.vsync).to_string(),
                    slider_percent: None,
                    selected: false,
                },
            ),
            (
                GraphicsSettingKey::TargetFps,
                RuntimeOverlayEntry {
                    label: "Target FPS".to_string(),
                    value_text: fps_label(self.launch_options.display.target_fps).to_string(),
                    slider_percent: Some(target_fps_to_slider(
                        self.launch_options.display.target_fps,
                    )),
                    selected: false,
                },
            ),
            (
                GraphicsSettingKey::PostProcessMode,
                RuntimeOverlayEntry {
                    label: "Post Process".to_string(),
                    value_text: post_process_mode_label(post.mode).to_string(),
                    slider_percent: None,
                    selected: false,
                },
            ),
        ];

        match post.mode {
            PostProcessMode::None => {}
            PostProcessMode::Tint => {
                entries.extend([
                    (
                        GraphicsSettingKey::TintStrength,
                        RuntimeOverlayEntry {
                            label: "Tint Strength".to_string(),
                            value_text: format!("{}%", post.tint_strength_percent),
                            slider_percent: Some(post.tint_strength_percent),
                            selected: false,
                        },
                    ),
                    (
                        GraphicsSettingKey::TintRed,
                        RuntimeOverlayEntry {
                            label: "Tint Red".to_string(),
                            value_text: post.tint_color[0].to_string(),
                            slider_percent: Some(channel_to_percent(post.tint_color[0])),
                            selected: false,
                        },
                    ),
                    (
                        GraphicsSettingKey::TintGreen,
                        RuntimeOverlayEntry {
                            label: "Tint Green".to_string(),
                            value_text: post.tint_color[1].to_string(),
                            slider_percent: Some(channel_to_percent(post.tint_color[1])),
                            selected: false,
                        },
                    ),
                    (
                        GraphicsSettingKey::TintBlue,
                        RuntimeOverlayEntry {
                            label: "Tint Blue".to_string(),
                            value_text: post.tint_color[2].to_string(),
                            slider_percent: Some(channel_to_percent(post.tint_color[2])),
                            selected: false,
                        },
                    ),
                ]);
            }
            PostProcessMode::BrightnessSaturation => {
                entries.extend([
                    (
                        GraphicsSettingKey::Brightness,
                        RuntimeOverlayEntry {
                            label: "Brightness".to_string(),
                            value_text: format!("{}%", post.brightness_percent),
                            slider_percent: Some(((post.brightness_percent + 100) / 2) as u8),
                            selected: false,
                        },
                    ),
                    (
                        GraphicsSettingKey::Saturation,
                        RuntimeOverlayEntry {
                            label: "Saturation".to_string(),
                            value_text: format!("{}%", post.saturation_percent),
                            slider_percent: Some(post.saturation_percent.min(200) / 2),
                            selected: false,
                        },
                    ),
                ]);
            }
            PostProcessMode::Quantize4 => {
                entries.extend([
                    (
                        GraphicsSettingKey::QuantizeStrategy,
                        RuntimeOverlayEntry {
                            label: "Quantize Strategy".to_string(),
                            value_text: quantize_strategy_label(post.quantize_strategy).to_string(),
                            slider_percent: None,
                            selected: false,
                        },
                    ),
                    (
                        GraphicsSettingKey::QuantizePalette,
                        RuntimeOverlayEntry {
                            label: "Quantize Palette".to_string(),
                            value_text: post.quantize_palette_id.clone(),
                            slider_percent: None,
                            selected: false,
                        },
                    ),
                ]);
            }
            PostProcessMode::OrderedDitherQuantize => {
                entries.push((
                    GraphicsSettingKey::QuantizePalette,
                    RuntimeOverlayEntry {
                        label: "Quantize Palette".to_string(),
                        value_text: post.quantize_palette_id.clone(),
                        slider_percent: None,
                        selected: false,
                    },
                ));
            }
            PostProcessMode::GbPalette => {
                entries.extend([
                    (
                        GraphicsSettingKey::QuantizeStrategy,
                        RuntimeOverlayEntry {
                            label: "Quantize Strategy".to_string(),
                            value_text: quantize_strategy_label(post.quantize_strategy).to_string(),
                            slider_percent: None,
                            selected: false,
                        },
                    ),
                    (
                        GraphicsSettingKey::GbContrast,
                        RuntimeOverlayEntry {
                            label: "GB Contrast".to_string(),
                            value_text: format!("{}%", post.gb_contrast_percent),
                            slider_percent: Some(((post.gb_contrast_percent + 100) / 2) as u8),
                            selected: false,
                        },
                    ),
                ]);
            }
            PostProcessMode::Vignette => {
                entries.push((
                    GraphicsSettingKey::VignetteStrength,
                    RuntimeOverlayEntry {
                        label: "Vignette Strength".to_string(),
                        value_text: format!("{}%", post.vignette_strength_percent),
                        slider_percent: Some(post.vignette_strength_percent),
                        selected: false,
                    },
                ));
            }
        }

        entries.push((
            GraphicsSettingKey::Back,
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

    fn handle_audio_overlay_input(&mut self, input: MenuInput) -> bool {
        let selected_index = match self.runtime_overlay.as_ref() {
            Some(RuntimeMenuOverlay::Audio { selected_index }) => *selected_index,
            _ => return false,
        };

        let next_selected = match input {
            MenuInput::Up => Some(selected_index.saturating_sub(1)),
            MenuInput::Down => Some((selected_index + 1).min(4)),
            _ => None,
        };
        if let Some(next_selected) = next_selected {
            let RuntimeMenuOverlay::Audio { selected_index } = self
                .runtime_overlay
                .as_mut()
                .expect("audio overlay must be active")
            else {
                return false;
            };
            *selected_index = next_selected;
            return false;
        }

        match input {
            MenuInput::Left => {
                self.adjust_audio_setting(selected_index, -(SETTING_STEP_PERCENT as i16))
            }
            MenuInput::Right => {
                self.adjust_audio_setting(selected_index, SETTING_STEP_PERCENT as i16)
            }
            MenuInput::Confirm => {
                if selected_index == 4 {
                    return true;
                }
            }
            MenuInput::Back => return true,
            MenuInput::Up | MenuInput::Down => {}
        }
        false
    }

    fn handle_graphics_overlay_input(&mut self, input: MenuInput) -> bool {
        let mut selected_index = match self.runtime_overlay.as_ref() {
            Some(RuntimeMenuOverlay::Graphics { selected_index }) => *selected_index,
            _ => return false,
        };
        let entry_count = self.graphics_entries_with_keys(selected_index).len();
        selected_index = selected_index.min(entry_count.saturating_sub(1));

        let next_selected = match input {
            MenuInput::Up => Some(selected_index.saturating_sub(1)),
            MenuInput::Down => Some((selected_index + 1).min(entry_count.saturating_sub(1))),
            _ => None,
        };
        if let Some(next_selected) = next_selected {
            let RuntimeMenuOverlay::Graphics { selected_index } = self
                .runtime_overlay
                .as_mut()
                .expect("graphics overlay must be active")
            else {
                return false;
            };
            *selected_index = next_selected;
            return false;
        }

        match input {
            MenuInput::Left => self.adjust_graphics_setting(selected_index, -1),
            MenuInput::Right | MenuInput::Confirm => {
                let selected_key =
                    self.graphics_entries_with_keys(selected_index)[selected_index].0;
                if selected_key == GraphicsSettingKey::Back && matches!(input, MenuInput::Confirm) {
                    return true;
                }
                self.adjust_graphics_setting(selected_index, 1);
            }
            MenuInput::Back => return true,
            MenuInput::Up | MenuInput::Down => {}
        }
        false
    }

    fn adjust_audio_setting(&mut self, selected_index: usize, delta: i16) {
        match selected_index {
            0 => adjust_percent(&mut self.launch_options.audio_mix.master_percent, delta),
            1 => adjust_percent(&mut self.launch_options.audio_mix.music_percent, delta),
            2 => adjust_percent(&mut self.launch_options.audio_mix.movement_percent, delta),
            3 => adjust_percent(&mut self.launch_options.audio_mix.collision_percent, delta),
            _ => return,
        }
        self.audio_system
            .set_master_volume_percent(self.launch_options.audio_mix.master_percent);
        self.audio_system
            .set_channel_volume_percent("music", self.launch_options.audio_mix.music_percent);
        self.audio_system
            .set_channel_volume_percent("music_a", self.launch_options.audio_mix.music_percent);
        self.audio_system
            .set_channel_volume_percent("music_b", self.launch_options.audio_mix.music_percent);
        self.audio_system
            .set_channel_volume_percent("movement", self.launch_options.audio_mix.movement_percent);
        self.audio_system.set_channel_volume_percent(
            "collision",
            self.launch_options.audio_mix.collision_percent,
        );
    }

    fn adjust_graphics_setting(&mut self, selected_index: usize, direction: i32) {
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
                    + BRIGHTNESS_STEP_PERCENT * direction as i16;
                self.launch_options.display.post_process.brightness_percent = next.clamp(-100, 100);
            }
            GraphicsSettingKey::Saturation => {
                let next = self.launch_options.display.post_process.saturation_percent as i16
                    + SATURATION_STEP_PERCENT * direction as i16;
                self.launch_options.display.post_process.saturation_percent =
                    next.clamp(0, 200) as u8;
            }
            GraphicsSettingKey::TintStrength => adjust_percent(
                &mut self
                    .launch_options
                    .display
                    .post_process
                    .tint_strength_percent,
                (SETTING_STEP_PERCENT as i16) * direction as i16,
            ),
            GraphicsSettingKey::TintRed => adjust_channel(
                &mut self.launch_options.display.post_process.tint_color[0],
                TINT_CHANNEL_STEP as i16 * direction as i16,
            ),
            GraphicsSettingKey::TintGreen => adjust_channel(
                &mut self.launch_options.display.post_process.tint_color[1],
                TINT_CHANNEL_STEP as i16 * direction as i16,
            ),
            GraphicsSettingKey::TintBlue => adjust_channel(
                &mut self.launch_options.display.post_process.tint_color[2],
                TINT_CHANNEL_STEP as i16 * direction as i16,
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
                    + GB_CONTRAST_STEP * direction as i16;
                self.launch_options.display.post_process.gb_contrast_percent =
                    next.clamp(-100, 100);
            }
            GraphicsSettingKey::VignetteStrength => adjust_percent(
                &mut self
                    .launch_options
                    .display
                    .post_process
                    .vignette_strength_percent,
                (SETTING_STEP_PERCENT as i16) * direction as i16,
            ),
            GraphicsSettingKey::Back => return,
        }

        self.rendering
            .set_post_process_settings(self.resolved_post_process_settings());
    }

    fn runtime_overlay_presentation(
        &self,
        appearance: &MenuAppearance,
        viewport: glam::Vec2,
    ) -> Option<RuntimeOverlayPresentation> {
        let overlay = self.runtime_overlay.clone()?;
        let (title, entries) = match overlay {
            RuntimeMenuOverlay::Audio { selected_index } => (
                "Audio Settings".to_string(),
                self.audio_overlay_entries(selected_index),
            ),
            RuntimeMenuOverlay::Graphics { selected_index } => (
                "Graphics Settings".to_string(),
                self.graphics_overlay_entries(selected_index),
            ),
        };

        let view = MenuView {
            screen_id: "__runtime_settings__".to_string(),
            title,
            title_border_style_override: None,
            entries: entries
                .iter()
                .map(|entry| MenuViewEntry {
                    text: format!("{}: {}", entry.label, entry.value_text),
                    selected: entry.selected,
                    selectable: true,
                    border_style_override: None,
                })
                .collect(),
        };

        Some(RuntimeOverlayPresentation {
            layout: build_menu_layout(&view, appearance, viewport),
            entries,
        })
    }

    fn select_runtime_overlay_entry(&mut self, entry_index: usize) {
        match self.runtime_overlay.as_mut() {
            Some(RuntimeMenuOverlay::Audio { selected_index })
            | Some(RuntimeMenuOverlay::Graphics { selected_index }) => {
                *selected_index = entry_index;
            }
            None => {}
        }
    }

    fn set_runtime_overlay_slider_percent(&mut self, entry_index: usize, percent: u8) {
        match self.runtime_overlay {
            Some(RuntimeMenuOverlay::Audio { .. }) => {
                self.set_audio_slider_percent(entry_index, percent)
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
    }
}

fn compose_runtime_settings_ui(
    layout_entries: &[MenuEntryLayout],
    overlay_entries: &[RuntimeOverlayEntry],
    layout: &toki_core::menu::MenuLayout,
    appearance: &MenuAppearance,
) -> UiComposition {
    let mut composition = compose_menu_ui(layout, appearance);
    let accent =
        menu_hex_color_rgba(&appearance.border_color_hex).unwrap_or([0.49, 1.0, 0.49, 1.0]);
    let track = [0.12, 0.18, 0.12, 0.85];

    for (layout_entry, overlay_entry) in layout_entries.iter().zip(overlay_entries.iter()) {
        let Some(slider_percent) = overlay_entry.slider_percent else {
            continue;
        };
        let track_x = layout_entry.rect.x + layout_entry.rect.width * 0.56;
        let track_width = layout_entry.rect.width * 0.28;
        let track_y = layout_entry.rect.y + layout_entry.rect.height - 7.0;
        let track_height = 3.0;
        composition.push(UiBlock {
            rect: toki_core::ui::UiRect {
                x: track_x,
                y: track_y,
                width: track_width,
                height: track_height,
            },
            fill_color: Some(track),
            border_color: None,
            border_thickness: 0.0,
            text: None,
        });
        composition.push(UiBlock {
            rect: toki_core::ui::UiRect {
                x: track_x,
                y: track_y,
                width: track_width * (slider_percent.min(100) as f32 / 100.0),
                height: track_height,
            },
            fill_color: Some(accent),
            border_color: None,
            border_thickness: 0.0,
            text: None,
        });
    }

    composition
}

fn runtime_overlay_hit_target_at_position(
    layout_entries: &[MenuEntryLayout],
    overlay_entries: &[RuntimeOverlayEntry],
    position: glam::Vec2,
) -> Option<RuntimeOverlayHitTarget> {
    for (entry_index, (layout_entry, overlay_entry)) in layout_entries
        .iter()
        .zip(overlay_entries.iter())
        .enumerate()
    {
        if let Some(slider_rect) = runtime_overlay_slider_rect(layout_entry, overlay_entry) {
            if rect_contains(slider_rect, position) {
                return Some(RuntimeOverlayHitTarget::Slider {
                    entry_index,
                    percent: slider_percent_from_position(slider_rect, position.x),
                });
            }
        }
        if rect_contains(layout_entry.rect, position) {
            return Some(RuntimeOverlayHitTarget::Entry(entry_index));
        }
    }
    None
}

fn runtime_overlay_slider_rect(
    layout_entry: &MenuEntryLayout,
    overlay_entry: &RuntimeOverlayEntry,
) -> Option<UiRect> {
    overlay_entry.slider_percent?;
    Some(UiRect {
        x: layout_entry.rect.x + layout_entry.rect.width * 0.56,
        y: layout_entry.rect.y + layout_entry.rect.height - 10.0,
        width: layout_entry.rect.width * 0.28,
        height: 8.0,
    })
}

fn rect_contains(rect: UiRect, position: glam::Vec2) -> bool {
    position.x >= rect.x
        && position.x <= rect.x + rect.width
        && position.y >= rect.y
        && position.y <= rect.y + rect.height
}

fn slider_percent_from_position(rect: UiRect, x: f32) -> u8 {
    (((x - rect.x) / rect.width.max(1.0)).clamp(0.0, 1.0) * 100.0).round() as u8
}

fn available_palette_ids(resources: &crate::systems::ResourceManager) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for id in builtin_palettes().into_keys() {
        ids.insert(id);
    }
    for id in resources.project_palettes().keys() {
        ids.insert(id.clone());
    }
    ids.into_iter().collect()
}

fn adjust_percent(value: &mut u8, delta: i16) {
    let next = *value as i16 + delta;
    *value = next.clamp(0, 100) as u8;
}

fn adjust_channel(value: &mut u8, delta: i16) {
    let next = *value as i16 + delta;
    *value = next.clamp(0, 255) as u8;
}

fn cycle_target_fps(current: u32, direction: i32) -> u32 {
    const TARGET_FPS_OPTIONS: [u32; 7] = [0, 30, 45, 60, 90, 120, 144];
    let current_index = TARGET_FPS_OPTIONS
        .iter()
        .position(|candidate| *candidate == current)
        .unwrap_or(3) as i32;
    let next_index =
        (current_index + direction).rem_euclid(TARGET_FPS_OPTIONS.len() as i32) as usize;
    TARGET_FPS_OPTIONS[next_index]
}

fn target_fps_to_slider(fps: u32) -> u8 {
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

fn slider_to_target_fps(percent: u8) -> u32 {
    const TARGET_FPS_OPTIONS: [u32; 7] = [0, 30, 45, 60, 90, 120, 144];
    let index = (((percent as f32 / 100.0) * (TARGET_FPS_OPTIONS.len() as f32 - 1.0)).round()
        as usize)
        .min(TARGET_FPS_OPTIONS.len() - 1);
    TARGET_FPS_OPTIONS[index]
}

fn fps_label(fps: u32) -> String {
    if fps == 0 {
        "Unlimited".to_string()
    } else {
        fps.to_string()
    }
}

fn on_off_label(value: bool) -> &'static str {
    if value {
        "On"
    } else {
        "Off"
    }
}

fn channel_to_percent(value: u8) -> u8 {
    ((value as u16 * 100) / 255) as u8
}

fn percent_to_channel(value: u8) -> u8 {
    ((value as u16 * 255) / 100) as u8
}

fn cycle_post_process_mode(mode: PostProcessMode, direction: i32) -> PostProcessMode {
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

fn cycle_string(current: &mut String, options: &[String], direction: i32) {
    let current_index = options
        .iter()
        .position(|option| option == current)
        .unwrap_or(0) as i32;
    let next_index = (current_index + direction).rem_euclid(options.len() as i32) as usize;
    *current = options[next_index].clone();
}

fn post_process_mode_label(mode: PostProcessMode) -> &'static str {
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

fn quantize_strategy_label(strategy: QuantizeStrategy) -> &'static str {
    match strategy {
        QuantizeStrategy::Luminance => "Luminance",
        QuantizeStrategy::RgbDistance => "RGB Distance",
    }
}

fn cycle_quantize_strategy(strategy: QuantizeStrategy, direction: i32) -> QuantizeStrategy {
    let strategies = [QuantizeStrategy::Luminance, QuantizeStrategy::RgbDistance];
    let current_index = strategies
        .iter()
        .position(|candidate| *candidate == strategy)
        .unwrap_or(0) as i32;
    let next_index = (current_index + direction).rem_euclid(strategies.len() as i32) as usize;
    strategies[next_index]
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
