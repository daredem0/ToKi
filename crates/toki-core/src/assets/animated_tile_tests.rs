use super::animated_tile::{AnimatedTileDef, TileLoopMode};

fn sample_def() -> AnimatedTileDef {
    AnimatedTileDef {
        frames: vec!["water_0".into(), "water_1".into(), "water_2".into()],
        frame_duration_ms: 200.0,
        frame_durations_ms: None,
        loop_mode: TileLoopMode::Loop,
    }
}

#[test]
fn frame_count_matches_frames_len() {
    assert_eq!(sample_def().frame_count(), 3);
}

#[test]
fn frame_duration_uses_uniform_when_no_overrides() {
    let def = sample_def();
    assert!((def.frame_duration_at(0) - 200.0).abs() < f32::EPSILON);
    assert!((def.frame_duration_at(1) - 200.0).abs() < f32::EPSILON);
    assert!((def.frame_duration_at(2) - 200.0).abs() < f32::EPSILON);
}

#[test]
fn frame_duration_uses_per_frame_overrides() {
    let def = AnimatedTileDef {
        frames: vec!["a".into(), "b".into(), "c".into()],
        frame_duration_ms: 100.0,
        frame_durations_ms: Some(vec![50.0, 150.0, 300.0]),
        loop_mode: TileLoopMode::Loop,
    };
    assert!((def.frame_duration_at(0) - 50.0).abs() < f32::EPSILON);
    assert!((def.frame_duration_at(1) - 150.0).abs() < f32::EPSILON);
    assert!((def.frame_duration_at(2) - 300.0).abs() < f32::EPSILON);
}

#[test]
fn frame_duration_falls_back_for_missing_override() {
    let def = AnimatedTileDef {
        frames: vec!["a".into(), "b".into(), "c".into()],
        frame_duration_ms: 100.0,
        frame_durations_ms: Some(vec![50.0]), // only 1 override for 3 frames
        loop_mode: TileLoopMode::Loop,
    };
    assert!((def.frame_duration_at(0) - 50.0).abs() < f32::EPSILON);
    assert!((def.frame_duration_at(1) - 100.0).abs() < f32::EPSILON); // fallback
}

#[test]
fn serde_round_trip() {
    let def = sample_def();
    let json = serde_json::to_string(&def).expect("serialize");
    let restored: AnimatedTileDef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.frame_count(), 3);
    assert_eq!(restored.loop_mode, TileLoopMode::Loop);
}

#[test]
fn ping_pong_mode_round_trips() {
    let def = AnimatedTileDef {
        frames: vec!["a".into(), "b".into()],
        frame_duration_ms: 100.0,
        frame_durations_ms: None,
        loop_mode: TileLoopMode::PingPong,
    };
    let json = serde_json::to_string(&def).expect("serialize");
    let restored: AnimatedTileDef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.loop_mode, TileLoopMode::PingPong);
}
