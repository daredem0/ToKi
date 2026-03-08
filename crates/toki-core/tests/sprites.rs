use toki_core::sprite::{Animation, Animator, Frame};

#[test]
fn animator_starts_at_frame_zero() {
    let animator = Animator::new();
    assert_eq!(animator.current_frame, 0);
    assert_eq!(animator.elapsed_ms, 0);
}

#[test]
fn animator_advances_frame_on_tick() {
    let anim = Animation {
        name: "bounce".to_string(),
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 100,
            },
            Frame {
                index: 1,
                duration_ms: 100,
            },
        ],
        looped: true,
    };
    let mut animator = Animator::new();
    animator.update(150, &anim);
    assert_eq!(animator.current_frame, 1);
}

#[test]
fn animator_loops_correctly() {
    let anim = Animation {
        name: "loop".to_string(),
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 100,
            },
            Frame {
                index: 1,
                duration_ms: 100,
            },
        ],
        looped: true,
    };
    let mut animator = Animator::new();
    animator.update(250, &anim);
    assert_eq!(animator.current_frame, 0); // should wrap
}

#[test]
fn animator_stops_at_last_frame_when_not_looping() {
    let anim = Animation {
        name: "once".to_string(),
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 100,
            },
            Frame {
                index: 1,
                duration_ms: 100,
            },
        ],
        looped: false,
    };
    let mut animator = Animator::new();
    animator.update(300, &anim);
    assert_eq!(animator.current_frame, 1); // capped at last frame
}

#[test]
fn animator_handles_zero_duration_frames() {
    let anim = Animation {
        name: "instant".to_string(),
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 0, // Zero duration
            },
            Frame {
                index: 1,
                duration_ms: 100,
            },
        ],
        looped: false,
    };
    let mut animator = Animator::new();
    animator.update(1, &anim); // Even 1ms should advance past zero-duration frame
    assert_eq!(animator.current_frame, 1);
}

#[test]
fn animator_handles_empty_animation() {
    let anim = Animation {
        name: "empty".to_string(),
        frames: vec![], // No frames
        looped: true,
    };
    let mut animator = Animator::new();
    animator.update(1000, &anim);
    // Should remain at frame 0 (default) when no frames exist
    assert_eq!(animator.current_frame, 0);
}

#[test]
fn animator_handles_single_frame_animation() {
    let anim = Animation {
        name: "single".to_string(),
        frames: vec![Frame {
            index: 42,
            duration_ms: 100,
        }],
        looped: true,
    };
    let mut animator = Animator::new();
    animator.update(150, &anim);
    // Should stay at the single frame
    assert_eq!(animator.current_frame, 0);
}

#[test]
fn animator_handles_large_time_updates() {
    let anim = Animation {
        name: "fast".to_string(),
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 10,
            },
            Frame {
                index: 1,
                duration_ms: 10,
            },
            Frame {
                index: 2,
                duration_ms: 10,
            },
        ],
        looped: true,
    };
    let mut animator = Animator::new();

    // Update with a very large time value
    animator.update(10000, &anim); // Much larger than total animation duration (30ms)

    // Should have looped many times and be at some valid frame
    assert!(animator.current_frame < 3);
}

#[test]
fn animator_multiple_updates_accumulate() {
    let anim = Animation {
        name: "multi".to_string(),
        frames: vec![
            Frame {
                index: 0,
                duration_ms: 100,
            },
            Frame {
                index: 1,
                duration_ms: 100,
            },
        ],
        looped: false,
    };
    let mut animator = Animator::new();

    animator.update(50, &anim);
    assert_eq!(animator.current_frame, 0);
    assert_eq!(animator.elapsed_ms, 50);

    animator.update(60, &anim); // Total: 110ms
    assert_eq!(animator.current_frame, 1);
    assert_eq!(animator.elapsed_ms, 10); // 110 - 100 (duration of first frame)
}

#[test]
fn animation_name_and_looped_properties() {
    let anim = Animation {
        name: "test_animation".to_string(),
        frames: vec![Frame {
            index: 0,
            duration_ms: 100,
        }],
        looped: true,
    };

    assert_eq!(anim.name, "test_animation");
    assert_eq!(anim.looped, true);
    assert_eq!(anim.frames.len(), 1);
}

#[test]
fn frame_properties() {
    let frame = Frame {
        index: 42,
        duration_ms: 1500,
    };

    assert_eq!(frame.index, 42);
    assert_eq!(frame.duration_ms, 1500);
}
