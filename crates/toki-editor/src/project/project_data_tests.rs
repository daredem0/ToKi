
use super::{Project, ProjectMetadata, RuntimeSettings};
use std::path::PathBuf;

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
"#;

    let metadata: ProjectMetadata =
        toml::from_str(toml).expect("metadata with runtime audio should deserialize");
    assert_eq!(metadata.runtime.audio.master_percent, 85);
    assert_eq!(metadata.runtime.audio.music_percent, 70);
    assert_eq!(metadata.runtime.audio.movement_percent, 55);
    assert_eq!(metadata.runtime.audio.collision_percent, 40);
    assert!(metadata.runtime.display.show_entity_health_bars);
}

#[test]
fn new_project_uses_derived_editor_version() {
    let project = Project::new("Demo".to_string(), PathBuf::from("/tmp/Demo"));
    assert_eq!(
        project.metadata.project.toki_editor_version,
        env!("TOKI_VERSION")
    );
}
