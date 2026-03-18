use crate::menu::MenuSettings;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RuntimeDisplaySettings {
    #[serde(default)]
    pub show_entity_health_bars: bool,
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
}

pub const fn default_runtime_splash_duration_ms() -> u64 {
    3000
}

pub const fn default_runtime_audio_mix_percent() -> u8 {
    100
}

#[cfg(test)]
#[path = "project_runtime_tests.rs"]
mod tests;
