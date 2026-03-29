use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{App, RuntimeAudioMixOptions, RuntimeDisplayOptions, RuntimeLaunchOptions};

const RUNTIME_SETTINGS_VERSION: u32 = 1;
const MAX_RUNTIME_SETTINGS_FILE_SIZE: u64 = 256 * 1024;
const RUNTIME_SETTINGS_FILE_NAME: &str = "runtime_config.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct RuntimeSettingsSnapshot {
    version: u32,
    audio_mix: RuntimeAudioMixOptions,
    display: RuntimeDisplayOptions,
}

impl RuntimeSettingsSnapshot {
    fn capture(launch_options: &RuntimeLaunchOptions) -> Self {
        Self {
            version: RUNTIME_SETTINGS_VERSION,
            audio_mix: launch_options.audio_mix.clone(),
            display: launch_options.display.clone(),
        }
    }
}

fn runtime_settings_file_path(save_root: impl AsRef<Path>) -> PathBuf {
    save_root.as_ref().join(RUNTIME_SETTINGS_FILE_NAME)
}

fn save_runtime_settings_snapshot(
    launch_options: &RuntimeLaunchOptions,
    save_root: impl AsRef<Path>,
) -> anyhow::Result<PathBuf> {
    let path = runtime_settings_file_path(save_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&RuntimeSettingsSnapshot::capture(launch_options))?;
    fs::write(&path, json)?;
    Ok(path)
}

fn load_runtime_settings_snapshot(
    save_root: impl AsRef<Path>,
) -> anyhow::Result<Option<RuntimeSettingsSnapshot>> {
    let path = runtime_settings_file_path(save_root);
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path)?;
    if metadata.len() > MAX_RUNTIME_SETTINGS_FILE_SIZE {
        anyhow::bail!(
            "runtime settings file is too large: '{}' ({} bytes, max {})",
            path.display(),
            metadata.len(),
            MAX_RUNTIME_SETTINGS_FILE_SIZE
        );
    }
    let json = fs::read_to_string(&path)?;
    let data: RuntimeSettingsSnapshot = serde_json::from_str(&json)?;
    if data.version != RUNTIME_SETTINGS_VERSION {
        anyhow::bail!(
            "unsupported runtime settings file version {}; expected {}",
            data.version,
            RUNTIME_SETTINGS_VERSION
        );
    }
    Ok(Some(data))
}

fn apply_runtime_settings_snapshot(
    launch_options: &mut RuntimeLaunchOptions,
    snapshot: RuntimeSettingsSnapshot,
) {
    launch_options.audio_mix = snapshot.audio_mix;
    launch_options.display = snapshot.display;
}

impl App {
    pub(super) fn load_persisted_runtime_settings_from_root(
        launch_options: &mut RuntimeLaunchOptions,
        save_root: impl AsRef<Path>,
    ) -> anyhow::Result<Option<PathBuf>> {
        let save_root = save_root.as_ref();
        let path = runtime_settings_file_path(save_root);
        if let Some(snapshot) = load_runtime_settings_snapshot(save_root)? {
            apply_runtime_settings_snapshot(launch_options, snapshot);
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    pub(super) fn apply_persisted_runtime_settings_from_disk(
        launch_options: &mut RuntimeLaunchOptions,
    ) {
        let save_root = Self::resolve_save_root_for_launch_options(launch_options);
        match Self::load_persisted_runtime_settings_from_root(launch_options, &save_root) {
            Ok(Some(path)) => {
                tracing::info!(
                    "Loaded persisted runtime settings from '{}'",
                    path.display()
                );
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    "Failed to load persisted runtime settings from '{}': {}",
                    runtime_settings_file_path(save_root).display(),
                    error
                );
            }
        }
    }

    pub(super) fn persist_runtime_settings(&self) -> anyhow::Result<PathBuf> {
        let save_root = self.resolve_save_root();
        save_runtime_settings_snapshot(&self.launch_options, save_root)
    }

    pub(super) fn persist_runtime_settings_if_possible(&self) {
        if let Err(error) = self.persist_runtime_settings() {
            tracing::warn!("Failed to persist runtime settings: {}", error);
        }
    }

    pub(super) fn apply_live_audio_mix_settings(&mut self) {
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
}

#[cfg(test)]
mod tests {
    use super::{
        apply_runtime_settings_snapshot, load_runtime_settings_snapshot, runtime_settings_file_path,
        save_runtime_settings_snapshot, RuntimeSettingsSnapshot,
    };
    use crate::app::{App, RuntimeLaunchOptions, RuntimeDisplayOptions};
    use toki_core::project_runtime::{IntegerScaleFactor, RuntimeViewportMode};

    #[test]
    fn runtime_settings_file_path_uses_global_runtime_config_name() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = runtime_settings_file_path(temp.path());
        assert_eq!(path, temp.path().join("runtime_config.json"));
    }

    #[test]
    fn runtime_settings_snapshot_round_trips_audio_and_display_overrides() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mut options = RuntimeLaunchOptions::default();
        options.audio_mix.master_percent = 83;
        options.audio_mix.music_percent = 61;
        options.display.vsync = false;
        options.display.target_fps = 144;
        options.display.viewport = RuntimeViewportMode::IntegerScale {
            factor: IntegerScaleFactor::Fixed(4),
        };

        let path = save_runtime_settings_snapshot(&options, temp.path()).expect("save snapshot");
        let loaded = load_runtime_settings_snapshot(temp.path())
            .expect("load snapshot")
            .expect("snapshot present");

        assert_eq!(path, temp.path().join("runtime_config.json"));
        assert_eq!(loaded.audio_mix, options.audio_mix);
        assert_eq!(loaded.display, options.display);
    }

    #[test]
    fn missing_runtime_settings_snapshot_is_treated_as_optional() {
        let temp = tempfile::tempdir().expect("temp dir");
        assert!(
            load_runtime_settings_snapshot(temp.path())
                .expect("load optional snapshot")
                .is_none()
        );
    }

    #[test]
    fn applying_runtime_settings_snapshot_overwrites_audio_and_display_options() {
        let mut options = RuntimeLaunchOptions::default();
        let snapshot = RuntimeSettingsSnapshot {
            version: 1,
            audio_mix: crate::app::RuntimeAudioMixOptions {
                master_percent: 70,
                music_percent: 55,
                movement_percent: 40,
                collision_percent: 25,
            },
            display: RuntimeDisplayOptions {
                viewport: RuntimeViewportMode::WindowFill { zoom_percent: 150 },
                vsync: false,
                target_fps: 120,
                ..RuntimeDisplayOptions::default()
            },
        };

        apply_runtime_settings_snapshot(&mut options, snapshot);

        assert_eq!(options.audio_mix.master_percent, 70);
        assert_eq!(options.audio_mix.music_percent, 55);
        assert_eq!(
            options.display.viewport,
            RuntimeViewportMode::WindowFill { zoom_percent: 150 }
        );
        assert!(!options.display.vsync);
        assert_eq!(options.display.target_fps, 120);
    }

    #[test]
    fn loading_persisted_runtime_settings_from_root_updates_launch_options() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mut original = RuntimeLaunchOptions::default();
        original.audio_mix.master_percent = 66;
        original.display.viewport = RuntimeViewportMode::IntegerScale {
            factor: IntegerScaleFactor::Fixed(3),
        };
        save_runtime_settings_snapshot(&original, temp.path()).expect("save snapshot");

        let mut loaded = RuntimeLaunchOptions::default();
        let path = App::load_persisted_runtime_settings_from_root(&mut loaded, temp.path())
            .expect("load persisted settings");

        assert_eq!(path, Some(temp.path().join("runtime_config.json")));
        assert_eq!(loaded.audio_mix, original.audio_mix);
        assert_eq!(loaded.display, original.display);
    }
}
