use toki_core::project_runtime::PostProcessMode;

use super::adjustments::{
    channel_to_percent, fps_label, on_off_label, post_process_mode_label,
    quantize_strategy_label, target_fps_to_slider,
};
use super::App;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeMenuOverlay {
    Audio { selected_index: usize },
    Display { selected_index: usize },
    Graphics { selected_index: usize },
}

impl RuntimeMenuOverlay {
    pub(crate) fn audio() -> Self {
        Self::Audio { selected_index: 0 }
    }

    pub(crate) fn graphics() -> Self {
        Self::Graphics { selected_index: 0 }
    }

    pub(crate) fn display() -> Self {
        Self::Display { selected_index: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeOverlayEntry {
    pub(crate) label: String,
    pub(crate) value_text: String,
    pub(crate) slider_percent: Option<u8>,
    pub(crate) selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphicsSettingKey {
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
    pub(super) fn audio_overlay_entries(&self, selected_index: usize) -> Vec<RuntimeOverlayEntry> {
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

    pub(super) fn graphics_overlay_entries(
        &self,
        selected_index: usize,
    ) -> Vec<RuntimeOverlayEntry> {
        self.graphics_entries_with_keys(selected_index)
            .into_iter()
            .map(|(_, entry)| entry)
            .collect()
    }

    pub(super) fn graphics_entries_with_keys(
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
}
