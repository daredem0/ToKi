use super::{Project, ProjectMetadata};
use std::path::PathBuf;
use toki_core::project_runtime::RuntimeSettings;

#[test]
fn project_metadata_deserialization_defaults_runtime_settings() {
    let toml = r#"
[project]
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

[editor]
recent_files = []
"#;

    let metadata: ProjectMetadata =
        toml::from_str(toml).expect("metadata without runtime section should deserialize");
    assert_eq!(metadata.runtime.splash.duration_ms, 3000);
    assert_eq!(metadata.runtime.audio.master_percent, 100);
    assert_eq!(metadata.runtime.audio.music_percent, 100);
    assert_eq!(metadata.runtime.audio.movement_percent, 100);
    assert_eq!(metadata.runtime.audio.collision_percent, 100);
    assert!(!metadata.runtime.display.show_entity_health_bars);
}

#[test]
fn runtime_settings_default_to_community_splash_duration() {
    let runtime = RuntimeSettings::default();
    assert_eq!(runtime.splash.duration_ms, 3000);
    assert_eq!(runtime.audio.master_percent, 100);
    assert_eq!(runtime.audio.music_percent, 100);
    assert_eq!(runtime.audio.movement_percent, 100);
    assert_eq!(runtime.audio.collision_percent, 100);
    assert!(!runtime.display.show_entity_health_bars);
    assert_eq!(runtime.menu.pause_root_screen_id, "pause_menu");
    assert!(runtime.menu.gate_gameplay_when_open);
    assert_eq!(runtime.menu.screens.len(), 2);
}

#[test]
fn project_metadata_deserialization_reads_runtime_audio_mix_settings() {
    let toml = r#"
[project]
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
master_percent = 85
music_percent = 70
movement_percent = 55
collision_percent = 40

[runtime.display]
show_entity_health_bars = true

[runtime.menu]
pause_root_screen_id = "pause_menu"
gate_gameplay_when_open = true

[[runtime.menu.screens]]
id = "pause_menu"
title = "Paused"

[[runtime.menu.screens.items]]
kind = "button"
text = "Resume"

[runtime.menu.screens.items.action]
kind = "close_menu"
"#;

    let metadata: ProjectMetadata =
        toml::from_str(toml).expect("metadata with runtime audio should deserialize");
    assert_eq!(metadata.runtime.audio.master_percent, 85);
    assert_eq!(metadata.runtime.audio.music_percent, 70);
    assert_eq!(metadata.runtime.audio.movement_percent, 55);
    assert_eq!(metadata.runtime.audio.collision_percent, 40);
    assert!(metadata.runtime.display.show_entity_health_bars);
    assert_eq!(metadata.runtime.menu.pause_root_screen_id, "pause_menu");
    assert_eq!(metadata.runtime.menu.screens.len(), 1);
}

#[test]
fn new_project_uses_derived_editor_version() {
    let project = Project::new("Demo".to_string(), PathBuf::from("/tmp/Demo"));
    assert_eq!(
        project.metadata.project.toki_editor_version,
        env!("TOKI_VERSION")
    );
}

#[test]
fn audio_config_accessor_reduces_message_chain() {
    let mut project = Project::new("Demo".to_string(), PathBuf::from("/tmp/Demo"));
    project.metadata.runtime.audio.master_percent = 75;
    project.metadata.runtime.audio.music_percent = 60;

    // Use accessor instead of deep navigation
    let audio = project.audio_config();
    assert_eq!(audio.master_percent, 75);
    assert_eq!(audio.music_percent, 60);
}

#[test]
fn audio_config_mut_accessor_allows_modification() {
    let mut project = Project::new("Demo".to_string(), PathBuf::from("/tmp/Demo"));

    // Use mutable accessor instead of deep navigation
    project.audio_config_mut().master_percent = 50;
    project.audio_config_mut().music_percent = 40;

    assert_eq!(project.metadata.runtime.audio.master_percent, 50);
    assert_eq!(project.metadata.runtime.audio.music_percent, 40);
}
