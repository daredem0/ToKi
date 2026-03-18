use crate::menu::MenuSettings;
use serde::{Deserialize, Serialize};

/// Project preset defining default settings for different game types.
///
/// Each preset provides sensible defaults for resolution and other settings.
/// Projects can override individual values while inheriting the preset defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProjectPreset {
    /// Top-down RPG style (Game Boy resolution: 160×144)
    #[default]
    Topdown,
    // Future presets:
    // Platformer (NES-like: 256×240)
    // Widescreen (16:9 pixel art: 320×180)
}

impl ProjectPreset {
    /// Returns the default resolution (width, height) for this preset.
    pub const fn default_resolution(&self) -> (u32, u32) {
        match self {
            ProjectPreset::Topdown => (160, 144),
            // Future: ProjectPreset::Platformer => (256, 240),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectRuntimeMetadata {
    #[serde(default)]
    pub runtime: RuntimeSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RuntimeSettings {
    #[serde(default)]
    pub splash: RuntimeSplashSettings,
    #[serde(default)]
    pub audio: RuntimeAudioMixSettings,
    #[serde(default)]
    pub display: RuntimeDisplaySettings,
    #[serde(default)]
    pub menu: MenuSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSplashSettings {
    #[serde(default = "default_runtime_splash_duration_ms")]
    pub duration_ms: u64,
}

impl Default for RuntimeSplashSettings {
    fn default() -> Self {
        Self {
            duration_ms: default_runtime_splash_duration_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeAudioMixSettings {
    #[serde(default = "default_runtime_audio_mix_percent")]
    pub master_percent: u8,
    #[serde(default = "default_runtime_audio_mix_percent")]
    pub music_percent: u8,
    #[serde(default = "default_runtime_audio_mix_percent")]
    pub movement_percent: u8,
    #[serde(default = "default_runtime_audio_mix_percent")]
    pub collision_percent: u8,
}

impl Default for RuntimeAudioMixSettings {
    fn default() -> Self {
        Self {
            master_percent: default_runtime_audio_mix_percent(),
            music_percent: default_runtime_audio_mix_percent(),
            movement_percent: default_runtime_audio_mix_percent(),
            collision_percent: default_runtime_audio_mix_percent(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeDisplaySettings {
    #[serde(default)]
    pub show_entity_health_bars: bool,
    /// Viewport width in pixels (defaults to preset resolution)
    #[serde(default = "default_resolution_width")]
    pub resolution_width: u32,
    /// Viewport height in pixels (defaults to preset resolution)
    #[serde(default = "default_resolution_height")]
    pub resolution_height: u32,
    /// Zoom level as percentage (100 = 1.0x, 200 = 2.0x, etc.)
    #[serde(default = "default_zoom_percent")]
    pub zoom_percent: u32,
}

impl Default for RuntimeDisplaySettings {
    fn default() -> Self {
        Self {
            show_entity_health_bars: false,
            resolution_width: default_resolution_width(),
            resolution_height: default_resolution_height(),
            zoom_percent: default_zoom_percent(),
        }
    }
}

impl RuntimeDisplaySettings {
    /// Returns the zoom level as a float (1.0 = 100%, 2.0 = 200%, etc.)
    pub fn zoom_factor(&self) -> f32 {
        self.zoom_percent as f32 / 100.0
    }
}

/// Default resolution width from Topdown preset (Game Boy: 160px)
pub const fn default_resolution_width() -> u32 {
    ProjectPreset::Topdown.default_resolution().0
}

/// Default resolution height from Topdown preset (Game Boy: 144px)
pub const fn default_resolution_height() -> u32 {
    ProjectPreset::Topdown.default_resolution().1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfigFile {
    pub version: u32,
    #[serde(default)]
    pub bundle_name: Option<String>,
    #[serde(default)]
    pub pack: Option<RuntimeConfigPack>,
    #[serde(default)]
    pub startup: Option<RuntimeConfigStartup>,
    #[serde(default)]
    pub splash: Option<RuntimeConfigSplash>,
    #[serde(default)]
    pub audio: Option<RuntimeConfigAudio>,
    #[serde(default)]
    pub display: Option<RuntimeConfigDisplay>,
    #[serde(default)]
    pub menu: Option<MenuSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfigPack {
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfigStartup {
    #[serde(default)]
    pub scene: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfigSplash {
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfigAudio {
    #[serde(default)]
    pub master_percent: Option<u8>,
    #[serde(default)]
    pub music_percent: Option<u8>,
    #[serde(default)]
    pub movement_percent: Option<u8>,
    #[serde(default)]
    pub collision_percent: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfigDisplay {
    #[serde(default)]
    pub show_entity_health_bars: Option<bool>,
    #[serde(default)]
    pub resolution_width: Option<u32>,
    #[serde(default)]
    pub resolution_height: Option<u32>,
    #[serde(default)]
    pub zoom_percent: Option<u32>,
}

pub const fn default_runtime_splash_duration_ms() -> u64 {
    3000
}

pub const fn default_runtime_audio_mix_percent() -> u8 {
    100
}

/// Default zoom level (100 = 1.0x, no zoom)
pub const fn default_zoom_percent() -> u32 {
    100
}

#[cfg(test)]
#[path = "project_runtime_tests.rs"]
mod tests;
