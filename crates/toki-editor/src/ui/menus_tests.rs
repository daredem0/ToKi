
use super::MenuSystem;

#[test]
fn busy_logo_animation_stays_within_expected_visual_bounds() {
    let sample_points = [0.0, 0.5, 1.0, 2.0, 4.0, 8.0];
    for sample in sample_points {
        let animation = MenuSystem::busy_logo_animation(sample);
        assert!(animation.bob_offset >= -2.1 && animation.bob_offset <= 2.1);
        assert!(animation.glow_alpha >= 28 && animation.glow_alpha <= 66);
        assert!(animation.glow_spread >= 3.0 && animation.glow_spread <= 5.6);
    }
}

#[test]
fn busy_logo_animation_changes_over_time() {
    let early = MenuSystem::busy_logo_animation(0.0);
    let later = MenuSystem::busy_logo_animation(1.0);
    assert_ne!(early, later);
}
