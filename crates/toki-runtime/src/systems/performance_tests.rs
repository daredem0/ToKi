
use super::PerformanceMonitor;
use std::time::{Duration, Instant};

#[test]
fn toggle_hud_display_flips_state() {
    let mut monitor = PerformanceMonitor::new();
    assert!(monitor.is_hud_display_enabled());
    monitor.toggle_hud_display();
    assert!(!monitor.is_hud_display_enabled());
    monitor.toggle_hud_display();
    assert!(monitor.is_hud_display_enabled());
}

#[test]
fn toggle_console_display_flips_state() {
    let mut monitor = PerformanceMonitor::new();
    assert!(monitor.is_console_display_enabled());
    monitor.toggle_console_display();
    assert!(!monitor.is_console_display_enabled());
    monitor.toggle_console_display();
    assert!(monitor.is_console_display_enabled());
}

#[test]
fn recording_timings_does_not_panic() {
    let mut monitor = PerformanceMonitor::new();
    let now = Instant::now();

    monitor.record_frame_interval(now);
    monitor.record_tick_time(Duration::from_millis(2));
    monitor.record_performance_breakdown(
        Duration::from_millis(3),
        Duration::from_millis(4),
        Duration::from_millis(8),
    );
    monitor.print_stats_if_needed();
}

#[test]
fn update_collection_keeps_last_sixty_samples() {
    let mut samples = Vec::new();
    for millis in 0..65u64 {
        PerformanceMonitor::update_collection(&mut samples, Duration::from_millis(millis));
    }

    assert_eq!(samples.len(), 60);
    assert_eq!(samples.first(), Some(&Duration::from_millis(5)));
    assert_eq!(samples.last(), Some(&Duration::from_millis(64)));
}

#[test]
fn print_stats_updates_last_print_after_interval() {
    let mut monitor = PerformanceMonitor::new();
    monitor.frame_times.push(Duration::from_millis(16));
    monitor.last_fps_print = Instant::now() - Duration::from_secs(2);
    let before = monitor.last_fps_print;

    monitor.print_stats_if_needed();

    assert!(monitor.last_fps_print > before);
}

#[test]
fn print_stats_is_skipped_when_disabled() {
    let mut monitor = PerformanceMonitor::new();
    monitor.frame_times.push(Duration::from_millis(16));
    monitor.show_console_stats = false;
    monitor.last_fps_print = Instant::now() - Duration::from_secs(2);
    let before = monitor.last_fps_print;

    monitor.print_stats_if_needed();

    assert_eq!(monitor.last_fps_print, before);
}

#[test]
fn stats_line_is_available_with_data_and_enabled() {
    let mut monitor = PerformanceMonitor::new();
    monitor.frame_times.push(Duration::from_millis(16));
    monitor.tick_times.push(Duration::from_millis(2));
    monitor.draw_times.push(Duration::from_millis(3));
    monitor.cpu_work_times.push(Duration::from_millis(4));
    monitor.total_frame_times.push(Duration::from_millis(8));

    let line = monitor
        .stats_line()
        .expect("stats line should exist for populated monitor");
    assert!(line.contains("FPS:"));
    assert!(line.contains("Frame:"));
    assert!(line.contains("Tick:"));
    assert!(line.contains("Draw:"));
    assert!(line.contains("CPU:"));
    assert!(line.contains("Overhead:"));
}

#[test]
fn stats_line_is_none_when_display_disabled() {
    let mut monitor = PerformanceMonitor::new();
    monitor.frame_times.push(Duration::from_millis(16));
    monitor.show_hud_stats = false;
    assert!(monitor.stats_line().is_none());
}
