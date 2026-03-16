
use super::{
    amplitude_to_decibels, classify_sfx_inventory, discover_ogg_assets, percent_to_decibels,
    spatial_attenuation,
};
use std::fs;

#[test]
fn discover_ogg_assets_returns_sorted_stems_and_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let audio_dir = temp.path().join("audio");
    fs::create_dir_all(&audio_dir).expect("audio dir");
    fs::write(audio_dir.join("z_sound.ogg"), "z").expect("z");
    fs::write(audio_dir.join("a_sound.ogg"), "a").expect("a");
    fs::write(audio_dir.join("readme.txt"), "ignore").expect("txt");

    let discovered = discover_ogg_assets(&audio_dir).expect("discover");
    assert_eq!(discovered.len(), 2);
    assert_eq!(discovered[0].0, "a_sound");
    assert_eq!(discovered[1].0, "z_sound");
    assert_eq!(
        discovered[0].1.file_name().and_then(|name| name.to_str()),
        Some("a_sound.ogg")
    );
    assert_eq!(
        discovered[1].1.file_name().and_then(|name| name.to_str()),
        Some("z_sound.ogg")
    );
}

#[test]
fn discover_ogg_assets_ignores_non_audio_extensions() {
    let temp = tempfile::tempdir().expect("temp dir");
    let audio_dir = temp.path().join("audio");
    fs::create_dir_all(&audio_dir).expect("audio dir");
    fs::write(audio_dir.join("sound.mp3"), "x").expect("mp3");
    fs::write(audio_dir.join("sound.wav"), "x").expect("wav");

    let discovered = discover_ogg_assets(&audio_dir).expect("discover");
    assert!(discovered.is_empty());
}

#[test]
fn classify_sfx_inventory_only_marks_hot_sounds_for_preload() {
    let discovered = vec![
        ("sfx_jump".to_string(), std::path::PathBuf::from("jump.ogg")),
        (
            "lavandia".to_string(),
            std::path::PathBuf::from("lavandia.ogg"),
        ),
        (
            "sfx_select".to_string(),
            std::path::PathBuf::from("select.ogg"),
        ),
    ];
    let preload_names = vec!["sfx_jump".to_string(), "sfx_select".to_string()];

    let inventory = classify_sfx_inventory(&discovered, &preload_names);
    assert_eq!(inventory.all_paths, discovered);
    assert_eq!(
        inventory.preloaded_names,
        vec!["sfx_jump".to_string(), "sfx_select".to_string()]
    );
}

#[test]
fn percent_to_decibels_maps_full_half_and_muted_values() {
    assert_eq!(percent_to_decibels(100), kira::Decibels::IDENTITY);
    assert_eq!(percent_to_decibels(0), kira::Decibels::SILENCE);
    assert!((percent_to_decibels(50).0 - (-6.0206)).abs() < 0.01);
}

#[test]
fn amplitude_to_decibels_matches_existing_music_baseline() {
    assert!((amplitude_to_decibels(0.3).0 - (-10.4576)).abs() < 0.02);
}

#[test]
fn amplitude_to_decibels_combines_master_and_channel_gain_multiplicatively() {
    assert!((amplitude_to_decibels(0.8 * 0.5).0 - (-7.9588)).abs() < 0.02);
}

#[test]
fn spatial_attenuation_is_full_at_listener_and_zero_outside_radius() {
    assert_eq!(
        spatial_attenuation(
            Some(glam::IVec2::new(0, 0)),
            Some(glam::IVec2::new(0, 0)),
            Some(100)
        ),
        Some(1.0)
    );
    assert_eq!(
        spatial_attenuation(
            Some(glam::IVec2::new(0, 0)),
            Some(glam::IVec2::new(100, 0)),
            Some(100)
        ),
        None
    );
}

#[test]
fn spatial_attenuation_falls_off_smoothly_inside_radius() {
    let attenuation = spatial_attenuation(
        Some(glam::IVec2::new(0, 0)),
        Some(glam::IVec2::new(50, 0)),
        Some(100),
    )
    .expect("attenuation should exist");
    assert!(attenuation > 0.0);
    assert!(attenuation < 1.0);
}
