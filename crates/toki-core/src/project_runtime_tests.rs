use crate::project_runtime::{
    ProjectPreset, ProjectRuntimeMetadata, RuntimeConfigFile, RuntimeDisplaySettings,
    RuntimeSettings,
};

#[test]
fn project_preset_topdown_returns_gameboy_resolution() {
    let preset = ProjectPreset::Topdown;
    let (width, height) = preset.default_resolution();

    assert_eq!(width, 160);
    assert_eq!(height, 144);
}

#[test]
fn runtime_display_settings_defaults_to_topdown_resolution() {
    let display = RuntimeDisplaySettings::default();

    assert_eq!(display.resolution_width, 160);
    assert_eq!(display.resolution_height, 144);
}

#[test]
fn runtime_display_settings_custom_resolution_overrides_default() {
    let toml = r#"
resolution_width = 256
resolution_height = 224
"#;

    let display: RuntimeDisplaySettings =
        toml::from_str(toml).expect("custom resolution should deserialize");

    assert_eq!(display.resolution_width, 256);
    assert_eq!(display.resolution_height, 224);
}

#[test]
fn runtime_settings_defaults_match_engine_baseline() {
    let settings = RuntimeSettings::default();

    assert_eq!(settings.splash.duration_ms, 3000);
    assert_eq!(settings.audio.master_percent, 100);
    assert_eq!(settings.audio.music_percent, 100);
    assert_eq!(settings.audio.movement_percent, 100);
    assert_eq!(settings.audio.collision_percent, 100);
    assert!(!settings.display.show_entity_health_bars);
    assert_eq!(settings.display.resolution_width, 160);
    assert_eq!(settings.display.resolution_height, 144);
    assert_eq!(settings.menu.pause_root_screen_id, "pause_menu");
}

#[test]
fn project_runtime_metadata_defaults_runtime_section() {
    let metadata: ProjectRuntimeMetadata =
        toml::from_str("").expect("empty metadata should deserialize");

    assert_eq!(metadata.runtime, RuntimeSettings::default());
}

#[test]
fn runtime_config_file_roundtrips_optional_sections() {
    let config = RuntimeConfigFile {
        version: 1,
        bundle_name: Some("Demo".to_string()),
        pack: None,
        startup: None,
        splash: None,
        audio: None,
        display: None,
        menu: Some(Default::default()),
    };

    let json = serde_json::to_string(&config).expect("serialize");
    let decoded: RuntimeConfigFile = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(decoded, config);
}
