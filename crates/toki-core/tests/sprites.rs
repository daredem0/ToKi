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
