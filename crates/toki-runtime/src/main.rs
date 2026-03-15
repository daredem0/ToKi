use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use toki_runtime::{
    run_minimal_window, run_minimal_window_with_options, RuntimeAudioMixOptions,
    RuntimeLaunchOptions,
};

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
struct RuntimeConfig {
    version: u32,
    #[allow(dead_code)]
    bundle_name: Option<String>,
    pack: Option<RuntimeConfigPack>,
    startup: Option<RuntimeConfigStartup>,
    splash: Option<RuntimeConfigSplash>,
    audio: Option<RuntimeConfigAudio>,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
struct RuntimeConfigPack {
    path: String,
    enabled: bool,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
struct RuntimeConfigStartup {
    scene: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
struct RuntimeConfigSplash {
    duration_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Deserialize, PartialEq, Eq)]
struct RuntimeConfigAudio {
    master_percent: Option<u8>,
    music_percent: Option<u8>,
    movement_percent: Option<u8>,
    collision_percent: Option<u8>,
}

fn main() -> Result<()> {
    let mut env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
    for directive in ["cosmic_text=info", "glyphon=info"] {
        if let Ok(parsed) = directive.parse() {
            env_filter = env_filter.add_directive(parsed);
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_file(false)
        .with_line_number(true)
        .init();

    tracing::info!("Starting ToKi runtime {}", env!("TOKI_VERSION"));

    let mut launch_options = parse_launch_options(std::env::args().skip(1).collect());
    if launch_options.project_path.is_none() {
        launch_options = apply_runtime_config_if_present(launch_options);
    }
    if launch_options.project_path.is_none() && launch_options.pack_path.is_none() {
        launch_options = auto_detect_project_launch_options(launch_options);
    }
    launch_options = apply_project_audio_mix_from_project_file_if_present(launch_options);
    if let Some(project_path) = &launch_options.project_path {
        if let Err(error) = std::env::set_current_dir(project_path) {
            tracing::warn!(
                "Failed to set runtime current dir to '{}': {}",
                project_path.display(),
                error
            );
        }
    }

    let run_result = if launch_options != RuntimeLaunchOptions::default() {
        run_minimal_window_with_options(launch_options)
    } else {
        run_minimal_window()
    };

    if let Err(e) = run_result {
        tracing::error!("Fatal error: {e:?}");
    }
    Ok(())
}

fn parse_launch_options(args: Vec<String>) -> RuntimeLaunchOptions {
    let mut launch_options = RuntimeLaunchOptions::default();
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--project" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.project_path = Some(PathBuf::from(value));
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--project' without value");
            }
            "--scene" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.scene_name = Some(value.clone());
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--scene' without value");
            }
            "--map" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.map_name = Some(value.clone());
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--map' without value");
            }
            "--splash-duration-ms" => {
                if let Some(value) = option_value(&args, index + 1) {
                    match value.parse::<u64>() {
                        Ok(duration_ms) => {
                            launch_options.splash.duration_ms = duration_ms;
                            index += 2;
                            continue;
                        }
                        Err(error) => {
                            tracing::warn!(
                                "Ignoring '--splash-duration-ms' invalid value '{}': {}",
                                value,
                                error
                            );
                        }
                    }
                } else {
                    tracing::warn!("Ignoring '--splash-duration-ms' without value");
                }
            }
            "--splash-hide-branding" => {
                launch_options.splash.show_branding = false;
                index += 1;
                continue;
            }
            "--pack" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.pack_path = Some(PathBuf::from(value));
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--pack' without value");
            }
            unknown => {
                tracing::warn!("Ignoring unknown runtime argument '{}'", unknown);
            }
        }

        index += 1;
    }

    launch_options
}

fn apply_runtime_config_if_present(
    mut launch_options: RuntimeLaunchOptions,
) -> RuntimeLaunchOptions {
    let Some((config, config_dir)) = load_runtime_config() else {
        return launch_options;
    };
    apply_runtime_config(&mut launch_options, config, &config_dir);
    launch_options
}

fn apply_runtime_config(
    launch_options: &mut RuntimeLaunchOptions,
    config: RuntimeConfig,
    config_dir: &std::path::Path,
) {
    if launch_options.project_path.is_none() {
        launch_options.project_path = Some(config_dir.to_path_buf());
    }
    if launch_options.scene_name.is_none() {
        launch_options.scene_name = config.startup.and_then(|startup| startup.scene);
    }
    if launch_options.pack_path.is_none() {
        if let Some(pack) = config.pack {
            if pack.enabled {
                launch_options.pack_path = Some(config_dir.join(pack.path));
            }
        }
    }
    if let Some(splash) = config.splash {
        if let Some(duration_ms) = splash.duration_ms {
            launch_options.splash.duration_ms = duration_ms;
        }
    }
    if let Some(audio) = config.audio {
        if let Some(master_percent) = audio.master_percent {
            launch_options.audio_mix.master_percent = master_percent.min(100);
        }
        if let Some(music_percent) = audio.music_percent {
            launch_options.audio_mix.music_percent = music_percent.min(100);
        }
        if let Some(movement_percent) = audio.movement_percent {
            launch_options.audio_mix.movement_percent = movement_percent.min(100);
        }
        if let Some(collision_percent) = audio.collision_percent {
            launch_options.audio_mix.collision_percent = collision_percent.min(100);
        }
    }
}

fn load_runtime_config() -> Option<(RuntimeConfig, PathBuf)> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("runtime_config.json"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("runtime_config.json"));
    }
    load_runtime_config_from_candidates(&candidates)
}

fn load_runtime_config_from_candidates(candidates: &[PathBuf]) -> Option<(RuntimeConfig, PathBuf)> {
    for path in candidates {
        if !path.exists() {
            continue;
        }
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<RuntimeConfig>(&content) {
                Ok(config) => {
                    let dir = path.parent().map(std::path::Path::to_path_buf)?;
                    return Some((config, dir));
                }
                Err(error) => {
                    tracing::warn!(
                        "Failed to parse runtime config '{}': {}",
                        path.display(),
                        error
                    );
                }
            },
            Err(error) => {
                tracing::warn!(
                    "Failed to read runtime config '{}': {}",
                    path.display(),
                    error
                );
            }
        }
    }
    None
}

fn auto_detect_project_launch_options(
    mut launch_options: RuntimeLaunchOptions,
) -> RuntimeLaunchOptions {
    let Ok(current_dir) = std::env::current_dir() else {
        return launch_options;
    };
    if !current_dir.join("project.toml").exists() {
        return launch_options;
    }

    launch_options.project_path = Some(current_dir.clone());

    if launch_options.scene_name.is_none() {
        launch_options.scene_name = detect_first_scene_name(&current_dir);
    }

    launch_options
}

#[derive(Debug, serde::Deserialize, Default)]
struct ProjectRuntimeMetadata {
    #[serde(default)]
    runtime: ProjectRuntimeSettings,
}

#[derive(Debug, serde::Deserialize, Default)]
struct ProjectRuntimeSettings {
    #[serde(default)]
    audio: ProjectRuntimeAudioSettings,
}

#[derive(Debug, serde::Deserialize, Default)]
struct ProjectRuntimeAudioSettings {
    #[serde(default = "default_project_audio_percent")]
    master_percent: u8,
    #[serde(default = "default_project_audio_percent")]
    music_percent: u8,
    #[serde(default = "default_project_audio_percent")]
    movement_percent: u8,
    #[serde(default = "default_project_audio_percent")]
    collision_percent: u8,
}

fn default_project_audio_percent() -> u8 {
    100
}

fn apply_project_audio_mix_from_project_file_if_present(
    mut launch_options: RuntimeLaunchOptions,
) -> RuntimeLaunchOptions {
    if launch_options.audio_mix != RuntimeAudioMixOptions::default() {
        return launch_options;
    }

    let Some(project_path) = launch_options.project_path.as_ref() else {
        return launch_options;
    };
    let project_file = project_path.join("project.toml");
    let Ok(content) = std::fs::read_to_string(&project_file) else {
        return launch_options;
    };
    let Ok(metadata) = toml::from_str::<ProjectRuntimeMetadata>(&content) else {
        tracing::warn!(
            "Failed to parse project runtime settings from '{}'",
            project_file.display()
        );
        return launch_options;
    };

    launch_options.audio_mix.music_percent = metadata.runtime.audio.music_percent.min(100);
    launch_options.audio_mix.master_percent = metadata.runtime.audio.master_percent.min(100);
    launch_options.audio_mix.movement_percent = metadata.runtime.audio.movement_percent.min(100);
    launch_options.audio_mix.collision_percent = metadata.runtime.audio.collision_percent.min(100);
    launch_options
}

fn detect_first_scene_name(project_path: &std::path::Path) -> Option<String> {
    let scenes_dir = project_path.join("scenes");
    let mut scene_file_stems = std::fs::read_dir(scenes_dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    scene_file_stems.sort();
    scene_file_stems.into_iter().next()
}

fn option_value(args: &[String], value_index: usize) -> Option<&String> {
    let value = args.get(value_index)?;
    if value.starts_with("--") {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_project_audio_mix_from_project_file_if_present, apply_runtime_config,
        auto_detect_project_launch_options, detect_first_scene_name,
        load_runtime_config_from_candidates, option_value, parse_launch_options, RuntimeConfig,
        RuntimeConfigAudio, RuntimeConfigPack, RuntimeConfigSplash, RuntimeConfigStartup,
    };
    use std::path::PathBuf;
    use toki_runtime::{RuntimeAudioMixOptions, RuntimeLaunchOptions};

    #[test]
    fn parse_launch_options_reads_project_scene_and_map() {
        let options = parse_launch_options(vec![
            "--project".to_string(),
            "/tmp/project".to_string(),
            "--scene".to_string(),
            "Main Scene".to_string(),
            "--map".to_string(),
            "map_01".to_string(),
        ]);

        assert_eq!(options.project_path, Some(PathBuf::from("/tmp/project")));
        assert_eq!(options.scene_name.as_deref(), Some("Main Scene"));
        assert_eq!(options.map_name.as_deref(), Some("map_01"));
        assert_eq!(options.splash.duration_ms, 3000);
        assert!(options.splash.show_branding);
        assert!(options.pack_path.is_none());
        assert_eq!(options.audio_mix, RuntimeAudioMixOptions::default());
    }

    #[test]
    fn parse_launch_options_ignores_missing_values() {
        let options = parse_launch_options(vec![
            "--project".to_string(),
            "--scene".to_string(),
            "--map".to_string(),
        ]);

        assert!(options.project_path.is_none());
        assert!(options.scene_name.is_none());
        assert!(options.map_name.is_none());
        assert!(options.pack_path.is_none());
    }

    #[test]
    fn parse_launch_options_ignores_unknown_flags() {
        let options = parse_launch_options(vec![
            "--unknown".to_string(),
            "value".to_string(),
            "--project".to_string(),
            "/tmp/project".to_string(),
        ]);

        assert_eq!(options.project_path, Some(PathBuf::from("/tmp/project")));
        assert!(options.scene_name.is_none());
        assert!(options.map_name.is_none());
        assert!(options.splash.show_branding);
        assert!(options.pack_path.is_none());
    }

    #[test]
    fn parse_launch_options_reads_splash_flags() {
        let options = parse_launch_options(vec![
            "--splash-duration-ms".to_string(),
            "2750".to_string(),
            "--splash-hide-branding".to_string(),
        ]);

        assert_eq!(options.splash.duration_ms, 2750);
        assert!(!options.splash.show_branding);
    }

    #[test]
    fn parse_launch_options_reads_pack_flag() {
        let options =
            parse_launch_options(vec!["--pack".to_string(), "/tmp/game.toki.pak".to_string()]);
        assert_eq!(options.pack_path, Some(PathBuf::from("/tmp/game.toki.pak")));
    }

    #[test]
    fn parse_launch_options_ignores_invalid_splash_duration() {
        let options = parse_launch_options(vec![
            "--splash-duration-ms".to_string(),
            "not-a-number".to_string(),
        ]);

        assert_eq!(options.splash.duration_ms, 3000);
    }

    #[test]
    fn option_value_rejects_next_flag() {
        let args = vec!["--project".to_string(), "--scene".to_string()];
        assert_eq!(option_value(&args, 1), None);
    }

    #[test]
    fn apply_runtime_config_if_present_populates_pack_and_startup_scene() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mut options = RuntimeLaunchOptions::default();
        apply_runtime_config(
            &mut options,
            RuntimeConfig {
                version: 1,
                bundle_name: Some("Demo".to_string()),
                pack: Some(RuntimeConfigPack {
                    path: "game.toki.pak".to_string(),
                    enabled: true,
                }),
                startup: Some(RuntimeConfigStartup {
                    scene: Some("Main Scene".to_string()),
                }),
                splash: Some(RuntimeConfigSplash {
                    duration_ms: Some(3200),
                }),
                audio: Some(RuntimeConfigAudio {
                    master_percent: Some(85),
                    music_percent: Some(65),
                    movement_percent: Some(45),
                    collision_percent: Some(25),
                }),
            },
            temp.path(),
        );

        assert_eq!(options.project_path, Some(temp.path().to_path_buf()));
        assert_eq!(options.pack_path, Some(temp.path().join("game.toki.pak")));
        assert_eq!(options.scene_name.as_deref(), Some("Main Scene"));
        assert_eq!(options.splash.duration_ms, 3200);
        assert_eq!(options.audio_mix.master_percent, 85);
        assert_eq!(options.audio_mix.music_percent, 65);
        assert_eq!(options.audio_mix.movement_percent, 45);
        assert_eq!(options.audio_mix.collision_percent, 25);
    }

    #[test]
    fn apply_runtime_config_keeps_existing_paths_and_scene_but_updates_splash_duration() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mut options = RuntimeLaunchOptions::default();
        options.project_path = Some(PathBuf::from("/cli/project"));
        options.pack_path = Some(PathBuf::from("/cli/game.toki.pak"));
        options.scene_name = Some("CLI Scene".to_string());
        options.splash.duration_ms = 2500;
        options.splash.show_branding = false;

        apply_runtime_config(
            &mut options,
            RuntimeConfig {
                version: 1,
                bundle_name: Some("Demo".to_string()),
                pack: Some(RuntimeConfigPack {
                    path: "game.toki.pak".to_string(),
                    enabled: true,
                }),
                startup: Some(RuntimeConfigStartup {
                    scene: Some("Config Scene".to_string()),
                }),
                splash: Some(RuntimeConfigSplash {
                    duration_ms: Some(3200),
                }),
                audio: Some(RuntimeConfigAudio {
                    master_percent: Some(95),
                    music_percent: Some(80),
                    movement_percent: Some(60),
                    collision_percent: Some(40),
                }),
            },
            temp.path(),
        );

        assert_eq!(options.project_path, Some(PathBuf::from("/cli/project")));
        assert_eq!(options.pack_path, Some(PathBuf::from("/cli/game.toki.pak")));
        assert_eq!(options.scene_name.as_deref(), Some("CLI Scene"));
        assert_eq!(options.splash.duration_ms, 3200);
        assert!(!options.splash.show_branding);
        assert_eq!(options.audio_mix.master_percent, 95);
        assert_eq!(options.audio_mix.music_percent, 80);
        assert_eq!(options.audio_mix.movement_percent, 60);
        assert_eq!(options.audio_mix.collision_percent, 40);
    }

    #[test]
    fn apply_runtime_config_ignores_disabled_pack_entry() {
        let temp = tempfile::tempdir().expect("temp dir");
        let mut options = RuntimeLaunchOptions::default();
        apply_runtime_config(
            &mut options,
            RuntimeConfig {
                version: 1,
                bundle_name: Some("Demo".to_string()),
                pack: Some(RuntimeConfigPack {
                    path: "game.toki.pak".to_string(),
                    enabled: false,
                }),
                startup: Some(RuntimeConfigStartup {
                    scene: Some("Main Scene".to_string()),
                }),
                splash: None,
                audio: None,
            },
            temp.path(),
        );

        assert_eq!(options.project_path, Some(temp.path().to_path_buf()));
        assert!(options.pack_path.is_none());
        assert_eq!(options.scene_name.as_deref(), Some("Main Scene"));
    }

    #[test]
    fn load_runtime_config_returns_none_without_file() {
        let temp = tempfile::tempdir().expect("temp dir");
        let cfg = load_runtime_config_from_candidates(&[temp.path().join("runtime_config.json")]);
        assert!(cfg.is_none());
    }

    #[test]
    fn load_runtime_config_skips_invalid_candidate_and_uses_next() {
        let first = tempfile::tempdir().expect("first temp dir");
        let second = tempfile::tempdir().expect("second temp dir");
        let first_cfg = first.path().join("runtime_config.json");
        let second_cfg = second.path().join("runtime_config.json");
        std::fs::write(&first_cfg, "{ invalid json ").expect("first config");
        std::fs::write(
            &second_cfg,
            r#"{
  "version": 1,
  "pack": { "path": "game.toki.pak", "enabled": true },
  "startup": { "scene": "Main Scene" },
  "splash": { "duration_ms": 3000 },
  "audio": { "master_percent": 88, "music_percent": 70, "movement_percent": 55, "collision_percent": 40 }
}"#,
        )
        .expect("second config");

        let loaded = load_runtime_config_from_candidates(&[first_cfg, second_cfg.clone()])
            .expect("second candidate should load");
        assert_eq!(loaded.1, second.path().to_path_buf());
        assert_eq!(
            loaded.0.pack,
            Some(RuntimeConfigPack {
                path: "game.toki.pak".to_string(),
                enabled: true
            })
        );
        assert_eq!(
            loaded.0.audio,
            Some(RuntimeConfigAudio {
                master_percent: Some(88),
                music_percent: Some(70),
                movement_percent: Some(55),
                collision_percent: Some(40),
            })
        );
    }

    #[test]
    fn detect_first_scene_name_reads_sorted_scene_stem() {
        let dir = tempfile::tempdir().expect("temp dir");
        let scenes = dir.path().join("scenes");
        std::fs::create_dir_all(&scenes).expect("scenes dir");
        std::fs::write(scenes.join("z_scene.json"), "{}").expect("z scene");
        std::fs::write(scenes.join("a_scene.json"), "{}").expect("a scene");
        std::fs::write(scenes.join("notes.txt"), "ignored").expect("notes");

        let detected = detect_first_scene_name(dir.path());
        assert_eq!(detected.as_deref(), Some("a_scene"));
    }

    #[test]
    fn auto_detect_project_launch_options_sets_project_and_scene_when_bundle_layout_exists() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("project.toml"), "[project]\nname='Demo'\n")
            .expect("project");
        let scenes = dir.path().join("scenes");
        std::fs::create_dir_all(&scenes).expect("scenes dir");
        std::fs::write(scenes.join("main.json"), "{}").expect("scene");

        let original_dir =
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        std::env::set_current_dir(dir.path()).expect("set cwd");
        let detected = auto_detect_project_launch_options(RuntimeLaunchOptions::default());
        std::env::set_current_dir(original_dir).expect("restore cwd");

        assert_eq!(detected.project_path, Some(dir.path().to_path_buf()));
        assert_eq!(detected.scene_name.as_deref(), Some("main"));
    }

    #[test]
    fn apply_project_audio_mix_from_project_file_reads_runtime_audio_settings() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("project.toml"),
            r#"[project]
name = "Demo"
version = "1.0.0"
created = "2026-01-01T00:00:00Z"
modified = "2026-01-01T00:00:00Z"
toki_editor_version = "0.0.14"
description = ""

[scenes]
main = "scenes/main.json"

[assets]
sprites = "assets/sprites/"
tilemaps = "assets/tilemaps/"
audio = "assets/audio/"

[runtime.audio]
master_percent = 91
music_percent = 72
movement_percent = 58
collision_percent = 31
"#,
        )
        .expect("project");

        let options = RuntimeLaunchOptions {
            project_path: Some(dir.path().to_path_buf()),
            ..RuntimeLaunchOptions::default()
        };
        let updated = apply_project_audio_mix_from_project_file_if_present(options);

        assert_eq!(updated.audio_mix.master_percent, 91);
        assert_eq!(updated.audio_mix.music_percent, 72);
        assert_eq!(updated.audio_mix.movement_percent, 58);
        assert_eq!(updated.audio_mix.collision_percent, 31);
    }

    #[test]
    fn apply_project_audio_mix_does_not_override_existing_launch_audio_mix() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(
            dir.path().join("project.toml"),
            "[runtime.audio]\nmusic_percent=10\nmovement_percent=20\ncollision_percent=30\n",
        )
        .expect("project");

        let options = RuntimeLaunchOptions {
            project_path: Some(dir.path().to_path_buf()),
            audio_mix: RuntimeAudioMixOptions {
                master_percent: 95,
                music_percent: 90,
                movement_percent: 80,
                collision_percent: 70,
            },
            ..RuntimeLaunchOptions::default()
        };
        let updated = apply_project_audio_mix_from_project_file_if_present(options);

        assert_eq!(updated.audio_mix.master_percent, 95);
        assert_eq!(updated.audio_mix.music_percent, 90);
        assert_eq!(updated.audio_mix.movement_percent, 80);
        assert_eq!(updated.audio_mix.collision_percent, 70);
    }
}
