use super::{
    apply_project_runtime_settings_from_project_file_if_present, apply_runtime_config,
    auto_detect_project_launch_options, detect_first_scene_name,
    load_runtime_config_from_candidates, option_value, parse_launch_options,
};
use std::path::PathBuf;
use toki_core::menu::{MenuItemDefinition, MenuScreenDefinition, MenuSettings, UiAction};
use toki_core::project_runtime::{
    RuntimeConfigAudio, RuntimeConfigDisplay, RuntimeConfigFile, RuntimeConfigPack,
    RuntimeConfigSplash, RuntimeConfigStartup,
};
use toki_runtime::{RuntimeAudioMixOptions, RuntimeDisplayOptions, RuntimeLaunchOptions};

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
    assert_eq!(options.display, RuntimeDisplayOptions::default());
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
    let configured_menu = MenuSettings {
        pause_root_screen_id: "custom_pause".to_string(),
        gate_gameplay_when_open: false,
        appearance: Default::default(),
        screens: vec![MenuScreenDefinition {
            id: "custom_pause".to_string(),
            title: "Custom Pause".to_string(),
            title_border_style_override: None,
            items: vec![MenuItemDefinition::Button {
                text: "Resume".to_string(),
                border_style_override: None,
                action: UiAction::CloseUi,
            }],
        }],
        dialogs: vec![],
    };
    apply_runtime_config(
        &mut options,
        RuntimeConfigFile {
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
            display: Some(RuntimeConfigDisplay {
                show_entity_health_bars: Some(true),
                resolution_width: None,
                resolution_height: None,
            }),
            menu: Some(configured_menu.clone()),
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
    assert!(options.display.show_entity_health_bars);
    assert_eq!(options.menu, configured_menu);
}

#[test]
fn apply_runtime_config_keeps_existing_paths_and_scene_but_updates_splash_duration() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut options = RuntimeLaunchOptions {
        project_path: Some(PathBuf::from("/cli/project")),
        pack_path: Some(PathBuf::from("/cli/game.toki.pak")),
        scene_name: Some("CLI Scene".to_string()),
        splash: toki_runtime::app::RuntimeSplashOptions {
            duration_ms: 2500,
            show_branding: false,
        },
        ..RuntimeLaunchOptions::default()
    };

    apply_runtime_config(
        &mut options,
        RuntimeConfigFile {
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
            display: Some(RuntimeConfigDisplay {
                show_entity_health_bars: Some(true),
                resolution_width: None,
                resolution_height: None,
            }),
            menu: Some(MenuSettings {
                pause_root_screen_id: "override".to_string(),
                gate_gameplay_when_open: false,
                appearance: Default::default(),
                screens: vec![],
                dialogs: vec![],
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
    assert!(options.display.show_entity_health_bars);
    assert_eq!(options.menu.pause_root_screen_id, "override");
}

#[test]
fn apply_runtime_config_ignores_disabled_pack_entry() {
    let temp = tempfile::tempdir().expect("temp dir");
    let mut options = RuntimeLaunchOptions::default();
    apply_runtime_config(
        &mut options,
        RuntimeConfigFile {
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
            display: None,
            menu: None,
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
  "audio": { "master_percent": 88, "music_percent": 70, "movement_percent": 55, "collision_percent": 40 },
  "menu": { "pause_root_screen_id": "pause_menu", "gate_gameplay_when_open": true, "screens": [] }
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
    assert_eq!(
        loaded.0.menu,
        Some(MenuSettings {
            pause_root_screen_id: "pause_menu".to_string(),
            gate_gameplay_when_open: true,
            appearance: Default::default(),
            screens: vec![],
            dialogs: vec![],
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
    std::fs::write(dir.path().join("project.toml"), "[project]\nname='Demo'\n").expect("project");
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
fn apply_project_runtime_settings_from_project_file_reads_audio_and_display_settings() {
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

[runtime.display]
show_entity_health_bars = true

[runtime.menu]
pause_root_screen_id = "custom_pause"
gate_gameplay_when_open = false

[[runtime.menu.screens]]
id = "custom_pause"
title = "Custom Pause"

[[runtime.menu.screens.items]]
kind = "button"
text = "Resume"

[runtime.menu.screens.items.action]
kind = "close_menu"
"#,
    )
    .expect("project");

    let options = RuntimeLaunchOptions {
        project_path: Some(dir.path().to_path_buf()),
        ..RuntimeLaunchOptions::default()
    };
    let updated = apply_project_runtime_settings_from_project_file_if_present(options);

    assert_eq!(updated.audio_mix.master_percent, 91);
    assert_eq!(updated.audio_mix.music_percent, 72);
    assert_eq!(updated.audio_mix.movement_percent, 58);
    assert_eq!(updated.audio_mix.collision_percent, 31);
    assert!(updated.display.show_entity_health_bars);
    assert_eq!(updated.menu.pause_root_screen_id, "custom_pause");
    assert!(!updated.menu.gate_gameplay_when_open);
    assert_eq!(updated.menu.screens.len(), 1);
}

#[test]
fn apply_project_runtime_settings_do_not_override_existing_launch_audio_mix() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(
            dir.path().join("project.toml"),
            "[runtime.audio]\nmusic_percent=10\nmovement_percent=20\ncollision_percent=30\n[runtime.display]\nshow_entity_health_bars=true\n",
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
        display: RuntimeDisplayOptions {
            show_entity_health_bars: false,
            resolution_width: 160,
            resolution_height: 144,
        },
        menu: MenuSettings {
            pause_root_screen_id: "cli_pause".to_string(),
            gate_gameplay_when_open: false,
            appearance: Default::default(),
            screens: vec![],
            dialogs: vec![],
        },
        ..RuntimeLaunchOptions::default()
    };
    let updated = apply_project_runtime_settings_from_project_file_if_present(options);

    assert_eq!(updated.audio_mix.master_percent, 95);
    assert_eq!(updated.audio_mix.music_percent, 90);
    assert_eq!(updated.audio_mix.movement_percent, 80);
    assert_eq!(updated.audio_mix.collision_percent, 70);
    assert!(updated.display.show_entity_health_bars);
    assert_eq!(updated.menu.pause_root_screen_id, "cli_pause");
}
