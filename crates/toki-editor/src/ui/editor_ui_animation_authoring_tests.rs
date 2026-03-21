// Tests for animation authoring state and logic

use super::*;
use toki_core::entity::{AnimationClipDef, AnimationsDef};

// ============================================================================
// AuthoredFrame Tests
// ============================================================================

#[test]
fn authored_frame_new_creates_frame_without_duration() {
    let frame = AuthoredFrame::new(3, 5);
    assert_eq!(frame.position, [3, 5]);
    assert!(frame.duration_ms.is_none());
}

#[test]
fn authored_frame_with_duration_creates_frame_with_duration() {
    let frame = AuthoredFrame::with_duration(2, 4, 150.0);
    assert_eq!(frame.position, [2, 4]);
    assert_eq!(frame.duration_ms, Some(150.0));
}

// ============================================================================
// AuthoredClip Frame Manipulation Tests
// ============================================================================

#[test]
fn authored_clip_default_has_empty_frames() {
    let clip = AuthoredClip::default();
    assert!(clip.frames.is_empty());
    assert_eq!(clip.state, "idle");
    assert_eq!(clip.default_duration_ms, 100.0);
    assert_eq!(clip.loop_mode, "loop");
}

#[test]
fn authored_clip_new_creates_with_state() {
    let clip = AuthoredClip::new("walk_down");
    assert_eq!(clip.state, "walk_down");
    assert!(clip.frames.is_empty());
}

#[test]
fn authored_clip_add_frame_appends_to_sequence() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);
    clip.add_frame(1, 0);
    clip.add_frame(2, 0);

    assert_eq!(clip.frames.len(), 3);
    assert_eq!(clip.frames[0].position, [0, 0]);
    assert_eq!(clip.frames[1].position, [1, 0]);
    assert_eq!(clip.frames[2].position, [2, 0]);
}

#[test]
fn authored_clip_add_frame_with_duration() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame_with_duration(0, 0, 200.0);

    assert_eq!(clip.frames.len(), 1);
    assert_eq!(clip.frames[0].duration_ms, Some(200.0));
}

#[test]
fn authored_clip_remove_frame_removes_at_index() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);
    clip.add_frame(1, 0);
    clip.add_frame(2, 0);

    assert!(clip.remove_frame(1));
    assert_eq!(clip.frames.len(), 2);
    assert_eq!(clip.frames[0].position, [0, 0]);
    assert_eq!(clip.frames[1].position, [2, 0]);
}

#[test]
fn authored_clip_remove_frame_out_of_bounds_returns_false() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);

    assert!(!clip.remove_frame(5));
    assert_eq!(clip.frames.len(), 1);
}

#[test]
fn authored_clip_move_frame_reorders_forward() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);
    clip.add_frame(1, 0);
    clip.add_frame(2, 0);

    assert!(clip.move_frame(0, 2));
    assert_eq!(clip.frames[0].position, [1, 0]);
    assert_eq!(clip.frames[1].position, [2, 0]);
    assert_eq!(clip.frames[2].position, [0, 0]);
}

#[test]
fn authored_clip_move_frame_reorders_backward() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);
    clip.add_frame(1, 0);
    clip.add_frame(2, 0);

    assert!(clip.move_frame(2, 0));
    assert_eq!(clip.frames[0].position, [2, 0]);
    assert_eq!(clip.frames[1].position, [0, 0]);
    assert_eq!(clip.frames[2].position, [1, 0]);
}

#[test]
fn authored_clip_move_frame_same_index_returns_true() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);

    assert!(clip.move_frame(0, 0));
}

#[test]
fn authored_clip_move_frame_out_of_bounds_returns_false() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);

    assert!(!clip.move_frame(0, 5));
    assert!(!clip.move_frame(5, 0));
}

// ============================================================================
// AuthoredClip Duration Tests
// ============================================================================

#[test]
fn authored_clip_set_frame_duration() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);
    clip.add_frame(1, 0);

    assert!(clip.set_frame_duration(0, Some(200.0)));
    assert_eq!(clip.frames[0].duration_ms, Some(200.0));
    assert!(clip.frames[1].duration_ms.is_none());
}

#[test]
fn authored_clip_set_frame_duration_out_of_bounds_returns_false() {
    let mut clip = AuthoredClip::new("idle");
    assert!(!clip.set_frame_duration(0, Some(200.0)));
}

#[test]
fn authored_clip_effective_duration_uses_override() {
    let mut clip = AuthoredClip::new("idle");
    clip.default_duration_ms = 100.0;
    clip.add_frame_with_duration(0, 0, 250.0);
    clip.add_frame(1, 0);

    assert_eq!(clip.effective_duration(0), Some(250.0));
    assert_eq!(clip.effective_duration(1), Some(100.0));
}

#[test]
fn authored_clip_effective_duration_none_for_invalid_index() {
    let clip = AuthoredClip::new("idle");
    assert!(clip.effective_duration(0).is_none());
}

#[test]
fn authored_clip_has_per_frame_durations() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame(0, 0);
    clip.add_frame(1, 0);
    assert!(!clip.has_per_frame_durations());

    clip.set_frame_duration(0, Some(200.0));
    assert!(clip.has_per_frame_durations());
}

#[test]
fn authored_clip_clear_per_frame_durations() {
    let mut clip = AuthoredClip::new("idle");
    clip.add_frame_with_duration(0, 0, 200.0);
    clip.add_frame_with_duration(1, 0, 300.0);
    assert!(clip.has_per_frame_durations());

    clip.clear_per_frame_durations();
    assert!(!clip.has_per_frame_durations());
    assert!(clip.frames[0].duration_ms.is_none());
    assert!(clip.frames[1].duration_ms.is_none());
}

// ============================================================================
// AuthoredClip Conversion Tests
// ============================================================================

#[test]
fn authored_clip_to_clip_def_without_per_frame_durations() {
    let mut clip = AuthoredClip::new("walk_down");
    clip.default_duration_ms = 150.0;
    clip.loop_mode = "loop".to_string();
    clip.add_frame(0, 0);
    clip.add_frame(1, 0);
    clip.add_frame(2, 0);

    let def = clip.to_clip_def();
    assert_eq!(def.state, "walk_down");
    assert_eq!(def.frame_positions, Some(vec![[0, 0], [1, 0], [2, 0]]));
    assert!(def.frame_tiles.is_empty());
    assert_eq!(def.frame_duration_ms, 150.0);
    assert!(def.frame_durations_ms.is_none());
    assert_eq!(def.loop_mode, "loop");
}

#[test]
fn authored_clip_to_clip_def_with_per_frame_durations() {
    let mut clip = AuthoredClip::new("attack");
    clip.default_duration_ms = 100.0;
    clip.loop_mode = "once".to_string();
    clip.add_frame(0, 0);
    clip.add_frame_with_duration(1, 0, 200.0);
    clip.add_frame(2, 0);

    let def = clip.to_clip_def();
    assert_eq!(def.frame_durations_ms, Some(vec![100.0, 200.0, 100.0]));
}

#[test]
fn authored_clip_to_clip_def_empty_frames() {
    let clip = AuthoredClip::new("idle");
    let def = clip.to_clip_def();
    assert!(def.frame_positions.is_none());
}

#[test]
fn authored_clip_from_clip_def_position_based() {
    let def = AnimationClipDef {
        state: "walk_up".to_string(),
        frame_tiles: Vec::new(),
        frame_positions: Some(vec![[0, 1], [1, 1], [2, 1]]),
        frame_duration_ms: 180.0,
        frame_durations_ms: None,
        loop_mode: "loop".to_string(),
    };

    let clip = AuthoredClip::from_clip_def(&def);
    assert_eq!(clip.state, "walk_up");
    assert_eq!(clip.frames.len(), 3);
    assert_eq!(clip.frames[0].position, [0, 1]);
    assert_eq!(clip.frames[1].position, [1, 1]);
    assert_eq!(clip.frames[2].position, [2, 1]);
    assert_eq!(clip.default_duration_ms, 180.0);
    assert_eq!(clip.loop_mode, "loop");
}

#[test]
fn authored_clip_from_clip_def_with_per_frame_durations() {
    let def = AnimationClipDef {
        state: "attack".to_string(),
        frame_tiles: Vec::new(),
        frame_positions: Some(vec![[0, 0], [1, 0]]),
        frame_duration_ms: 100.0,
        frame_durations_ms: Some(vec![100.0, 250.0]),
        loop_mode: "once".to_string(),
    };

    let clip = AuthoredClip::from_clip_def(&def);
    assert!(clip.frames[0].duration_ms.is_none()); // 100.0 == default, so None
    assert_eq!(clip.frames[1].duration_ms, Some(250.0));
}

#[test]
fn authored_clip_roundtrip_conversion() {
    let mut original = AuthoredClip::new("walk_down");
    original.default_duration_ms = 150.0;
    original.loop_mode = "ping_pong".to_string();
    original.add_frame(0, 0);
    original.add_frame_with_duration(1, 0, 200.0);
    original.add_frame(2, 0);

    let def = original.to_clip_def();
    let restored = AuthoredClip::from_clip_def(&def);

    assert_eq!(restored.state, original.state);
    assert_eq!(restored.frames.len(), original.frames.len());
    assert_eq!(restored.default_duration_ms, original.default_duration_ms);
    assert_eq!(restored.loop_mode, original.loop_mode);
    for (a, b) in restored.frames.iter().zip(original.frames.iter()) {
        assert_eq!(a.position, b.position);
    }
}

// ============================================================================
// AnimationAuthoringState Clip Management Tests
// ============================================================================

#[test]
fn animation_authoring_state_default() {
    let state = AnimationAuthoringState::new();
    assert!(state.clips.is_empty());
    assert!(state.selected_clip_index.is_none());
    assert!(state.selected_frame_index.is_none());
    assert!(state.atlas_name.is_empty());
    assert!(state.default_state.is_empty());
    assert!(!state.dirty);
}

#[test]
fn animation_authoring_state_create_clip() {
    let mut state = AnimationAuthoringState::new();
    let index = state.create_clip("idle_down");

    assert_eq!(index, 0);
    assert_eq!(state.clips.len(), 1);
    assert_eq!(state.clips[0].state, "idle_down");
    assert_eq!(state.selected_clip_index, Some(0));
    assert!(state.dirty);
}

#[test]
fn animation_authoring_state_create_multiple_clips() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle_down");
    state.create_clip("walk_down");
    state.create_clip("attack_down");

    assert_eq!(state.clips.len(), 3);
    assert_eq!(state.selected_clip_index, Some(2)); // Last created
}

#[test]
fn animation_authoring_state_delete_clip() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.create_clip("walk");
    state.create_clip("attack");

    assert!(state.delete_clip(1));
    assert_eq!(state.clips.len(), 2);
    assert_eq!(state.clips[0].state, "idle");
    assert_eq!(state.clips[1].state, "attack");
}

#[test]
fn animation_authoring_state_delete_clip_adjusts_selection() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.create_clip("walk");
    state.selected_clip_index = Some(1);

    state.delete_clip(1);
    assert_eq!(state.selected_clip_index, Some(0)); // Adjusted to last valid
}

#[test]
fn animation_authoring_state_delete_clip_before_selection() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.create_clip("walk");
    state.create_clip("attack");
    state.selected_clip_index = Some(2);

    state.delete_clip(0);
    assert_eq!(state.selected_clip_index, Some(1)); // Shifted down
}

#[test]
fn animation_authoring_state_delete_last_clip_clears_selection() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.delete_clip(0);

    assert!(state.clips.is_empty());
    assert!(state.selected_clip_index.is_none());
}

#[test]
fn animation_authoring_state_delete_clip_out_of_bounds() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");

    assert!(!state.delete_clip(5));
    assert_eq!(state.clips.len(), 1);
}

#[test]
fn animation_authoring_state_select_clip() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.create_clip("walk");
    state.selected_clip_index = None;

    assert!(state.select_clip(0));
    assert_eq!(state.selected_clip_index, Some(0));
}

#[test]
fn animation_authoring_state_select_clip_out_of_bounds() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");

    assert!(!state.select_clip(5));
}

#[test]
fn animation_authoring_state_selected_clip() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle_down");
    state.selected_clip_index = Some(0);

    let clip = state.selected_clip();
    assert!(clip.is_some());
    assert_eq!(clip.unwrap().state, "idle_down");
}

#[test]
fn animation_authoring_state_selected_clip_none_when_no_selection() {
    let state = AnimationAuthoringState::new();
    assert!(state.selected_clip().is_none());
}

// ============================================================================
// AnimationAuthoringState Frame Manipulation Tests
// ============================================================================

#[test]
fn animation_authoring_state_add_frame_to_selected() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);

    assert!(state.add_frame_to_selected(0, 0));
    assert!(state.add_frame_to_selected(1, 0));

    assert_eq!(state.clips[0].frames.len(), 2);
    assert!(state.dirty);
}

#[test]
fn animation_authoring_state_add_frame_no_selection_returns_false() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = None;

    assert!(!state.add_frame_to_selected(0, 0));
}

#[test]
fn animation_authoring_state_remove_selected_frame() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);
    state.add_frame_to_selected(2, 0);
    state.selected_frame_index = Some(1);

    assert!(state.remove_selected_frame());
    assert_eq!(state.clips[0].frames.len(), 2);
    assert_eq!(state.selected_frame_index, Some(1)); // Adjusted
}

#[test]
fn animation_authoring_state_remove_last_frame_clears_selection() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.selected_frame_index = Some(0);

    assert!(state.remove_selected_frame());
    assert!(state.selected_frame_index.is_none());
}

#[test]
fn animation_authoring_state_remove_frame_no_selection_returns_false() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.selected_frame_index = None;

    assert!(!state.remove_selected_frame());
}

// ============================================================================
// AnimationAuthoringState Navigation Tests
// ============================================================================

#[test]
fn animation_authoring_state_select_next_frame() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);
    state.add_frame_to_selected(2, 0);
    state.selected_frame_index = Some(0);

    assert!(state.select_next_frame());
    assert_eq!(state.selected_frame_index, Some(1));

    assert!(state.select_next_frame());
    assert_eq!(state.selected_frame_index, Some(2));

    // At end, should stay at last
    assert!(state.select_next_frame());
    assert_eq!(state.selected_frame_index, Some(2));
}

#[test]
fn animation_authoring_state_select_prev_frame() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);
    state.add_frame_to_selected(2, 0);
    state.selected_frame_index = Some(2);

    assert!(state.select_prev_frame());
    assert_eq!(state.selected_frame_index, Some(1));

    assert!(state.select_prev_frame());
    assert_eq!(state.selected_frame_index, Some(0));

    // At start, should stay at first
    assert!(state.select_prev_frame());
    assert_eq!(state.selected_frame_index, Some(0));
}

#[test]
fn animation_authoring_state_move_selected_frame_up() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);
    state.add_frame_to_selected(2, 0);
    state.selected_frame_index = Some(2);

    assert!(state.move_selected_frame_up());
    assert_eq!(state.clips[0].frames[1].position, [2, 0]);
    assert_eq!(state.clips[0].frames[2].position, [1, 0]);
    assert_eq!(state.selected_frame_index, Some(1)); // Selection follows
}

#[test]
fn animation_authoring_state_move_selected_frame_up_at_start() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);
    state.selected_frame_index = Some(0);

    assert!(!state.move_selected_frame_up());
}

#[test]
fn animation_authoring_state_move_selected_frame_down() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);
    state.add_frame_to_selected(2, 0);
    state.selected_frame_index = Some(0);

    assert!(state.move_selected_frame_down());
    assert_eq!(state.clips[0].frames[0].position, [1, 0]);
    assert_eq!(state.clips[0].frames[1].position, [0, 0]);
    assert_eq!(state.selected_frame_index, Some(1)); // Selection follows
}

#[test]
fn animation_authoring_state_move_selected_frame_down_at_end() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);
    state.selected_frame_index = Some(1);

    assert!(!state.move_selected_frame_down());
}

// ============================================================================
// AnimationAuthoringState Lookup Tests
// ============================================================================

#[test]
fn animation_authoring_state_find_clip_by_state() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle_down");
    state.create_clip("walk_down");
    state.create_clip("attack_down");

    assert_eq!(state.find_clip_by_state("walk_down"), Some(1));
    assert_eq!(state.find_clip_by_state("nonexistent"), None);
}

#[test]
fn animation_authoring_state_has_clip_for_state() {
    let mut state = AnimationAuthoringState::new();
    state.create_clip("idle_down");

    assert!(state.has_clip_for_state("idle_down"));
    assert!(!state.has_clip_for_state("walk_down"));
}

#[test]
fn animation_authoring_state_available_states() {
    let mut state = AnimationAuthoringState::new();
    let all_states = state.available_states();
    assert_eq!(all_states.len(), 15); // All states available

    state.create_clip("idle");
    state.create_clip("walk");
    let available = state.available_states();
    assert_eq!(available.len(), 13); // Two less
    assert!(!available.contains(&"idle"));
    assert!(!available.contains(&"walk"));
}

// ============================================================================
// AnimationAuthoringState Conversion Tests
// ============================================================================

#[test]
fn animation_authoring_state_from_animations_def() {
    let def = AnimationsDef {
        atlas_name: "players.json".to_string(),
        clips: vec![
            AnimationClipDef {
                state: "idle_down".to_string(),
                frame_tiles: Vec::new(),
                frame_positions: Some(vec![[0, 0]]),
                frame_duration_ms: 300.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            },
            AnimationClipDef {
                state: "walk_down".to_string(),
                frame_tiles: Vec::new(),
                frame_positions: Some(vec![[0, 0], [1, 0]]),
                frame_duration_ms: 180.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            },
        ],
        default_state: "idle_down".to_string(),
    };

    let state = AnimationAuthoringState::from_animations_def(&def);
    assert_eq!(state.atlas_name, "players.json");
    assert_eq!(state.default_state, "idle_down");
    assert_eq!(state.clips.len(), 2);
    assert_eq!(state.clips[0].state, "idle_down");
    assert_eq!(state.clips[1].state, "walk_down");
    assert!(!state.dirty);
}

#[test]
fn animation_authoring_state_to_animations_def() {
    let mut state = AnimationAuthoringState::new();
    state.atlas_name = "enemies.json".to_string();
    state.default_state = "idle".to_string();
    state.create_clip("idle");
    state.selected_clip_index = Some(0);
    state.add_frame_to_selected(0, 0);
    state.add_frame_to_selected(1, 0);

    let def = state.to_animations_def();
    assert_eq!(def.atlas_name, "enemies.json");
    assert_eq!(def.default_state, "idle");
    assert_eq!(def.clips.len(), 1);
    assert_eq!(def.clips[0].frame_positions, Some(vec![[0, 0], [1, 0]]));
}

#[test]
fn animation_authoring_state_roundtrip_conversion() {
    let original = AnimationsDef {
        atlas_name: "test.json".to_string(),
        clips: vec![
            AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: Vec::new(),
                frame_positions: Some(vec![[0, 0], [1, 0]]),
                frame_duration_ms: 200.0,
                frame_durations_ms: Some(vec![200.0, 300.0]),
                loop_mode: "ping_pong".to_string(),
            },
        ],
        default_state: "idle".to_string(),
    };

    let state = AnimationAuthoringState::from_animations_def(&original);
    let restored = state.to_animations_def();

    assert_eq!(restored.atlas_name, original.atlas_name);
    assert_eq!(restored.default_state, original.default_state);
    assert_eq!(restored.clips.len(), original.clips.len());
    assert_eq!(restored.clips[0].state, original.clips[0].state);
    assert_eq!(restored.clips[0].frame_positions, original.clips[0].frame_positions);
    assert_eq!(restored.clips[0].loop_mode, original.clips[0].loop_mode);
}
