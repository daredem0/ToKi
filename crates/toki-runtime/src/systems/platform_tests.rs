
use super::PlatformSystem;

#[test]
fn new_system_has_no_window() {
    let system = PlatformSystem::new();

    assert!(!system.has_window());
    assert!(system.window().is_none());
    assert!(system.window_for_gpu().is_none());
    assert!(system.inner_size().is_none());
    assert!(system.scale_factor().is_none());
}

#[test]
fn no_window_operations_are_noops() {
    let system = PlatformSystem::new();

    system.request_redraw();
    system.pre_present_notify();

    assert!(!system.has_window());
}
