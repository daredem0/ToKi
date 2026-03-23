use crate::project_runtime::{
    PostProcessMode, ProjectPreset, ProjectRuntimeMetadata, QuantizeStrategy, RuntimeConfigFile,
    RuntimeDisplaySettings, RuntimePostProcessSettings, RuntimeSettings,
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
    assert!(!settings.scene_persistence);
    assert!(!settings.display.show_entity_health_bars);
    assert!(settings.display.show_ground_shadows);
    assert_eq!(settings.display.indexed_palette_override, None);
    assert_eq!(settings.display.post_process.mode, PostProcessMode::None);
    assert_eq!(
        settings.display.post_process.quantize_strategy,
        QuantizeStrategy::Luminance
    );
    assert_eq!(settings.display.post_process.brightness_percent, 0);
    assert_eq!(settings.display.post_process.saturation_percent, 100);
    assert_eq!(settings.display.post_process.quantize_palette_id, "gray");
    assert_eq!(settings.display.post_process.vignette_strength_percent, 60);
    assert_eq!(settings.display.resolution_width, 160);
    assert_eq!(settings.display.resolution_height, 144);
    assert_eq!(settings.menu.pause_root_screen_id, "pause_menu");
}

#[test]
fn runtime_display_settings_default_enable_ground_shadows() {
    let display = RuntimeDisplaySettings::default();

    assert!(display.show_ground_shadows);
}

#[test]
fn project_runtime_metadata_defaults_runtime_section() {
    let metadata: ProjectRuntimeMetadata =
        toml::from_str("").expect("empty metadata should deserialize");

    assert_eq!(metadata.runtime, RuntimeSettings::default());
}

#[test]
fn runtime_metadata_supports_global_indexed_palette_override() {
    let metadata: ProjectRuntimeMetadata = toml::from_str(
        r#"
        [runtime.display]
        indexed_palette_override = "gb_swamp"
        "#,
    )
    .expect("runtime metadata should deserialize");

    assert_eq!(
        metadata.runtime.display.indexed_palette_override.as_deref(),
        Some("gb_swamp")
    );
}

#[test]
fn runtime_metadata_supports_post_process_settings() {
    let metadata: ProjectRuntimeMetadata = toml::from_str(
        r#"
        [runtime]
        scene_persistence = true

        [runtime.display.post_process]
        mode = "brightness_saturation"
        quantize_strategy = "rgb_distance"
        tint_color = [12, 34, 56, 255]
        tint_strength_percent = 70
        brightness_percent = 15
        saturation_percent = 140
        quantize_palette_id = "poison"
        gb_contrast_percent = 18
        vignette_strength_percent = 72
        "#,
    )
    .expect("runtime metadata should deserialize");

    assert_eq!(
        metadata.runtime.display.post_process.mode,
        PostProcessMode::BrightnessSaturation
    );
    assert_eq!(
        metadata.runtime.display.post_process.quantize_strategy,
        QuantizeStrategy::RgbDistance
    );
    assert_eq!(metadata.runtime.display.post_process.tint_color, [12, 34, 56, 255]);
    assert_eq!(metadata.runtime.display.post_process.tint_strength_percent, 70);
    assert_eq!(metadata.runtime.display.post_process.brightness_percent, 15);
    assert_eq!(metadata.runtime.display.post_process.saturation_percent, 140);
    assert_eq!(metadata.runtime.display.post_process.quantize_palette_id, "poison");
    assert_eq!(metadata.runtime.display.post_process.gb_contrast_percent, 18);
    assert_eq!(metadata.runtime.display.post_process.vignette_strength_percent, 72);
    assert!(metadata.runtime.scene_persistence);
}

#[test]
fn runtime_post_process_settings_resolve_quantize_palette_from_registry() {
    let settings = RuntimePostProcessSettings {
        mode: PostProcessMode::Quantize4,
        quantize_strategy: QuantizeStrategy::Luminance,
        tint_color: [0, 0, 0, 255],
        tint_strength_percent: 10,
        brightness_percent: 0,
        saturation_percent: 100,
        quantize_palette_id: "night".to_string(),
        gb_contrast_percent: 0,
        vignette_strength_percent: 60,
    };

    let resolved = settings.resolve(&std::collections::BTreeMap::new());

    assert_eq!(resolved.mode, PostProcessMode::Quantize4);
    assert_eq!(resolved.quantize_palette.colors[0], [0x10, 0x18, 0x2B, 0xFF]);
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
