use toki_core::TimingSystem;
use std::time::Duration;

#[test]
fn timing_system_new_has_correct_defaults() {
    let timing = TimingSystem::new();
    
    // Should have 60 FPS timestep (16.666... ms)
    let expected_timestep = Duration::from_nanos(16_666_667);
    assert_eq!(timing.timestep(), expected_timestep);
    
    // Should start with zero accumulator
    assert_eq!(timing.accumulator(), Duration::ZERO);
}

#[test]
fn timing_system_with_custom_timestep() {
    let custom_timestep = Duration::from_millis(10);
    let timing = TimingSystem::with_timestep(custom_timestep);
    
    assert_eq!(timing.timestep(), custom_timestep);
    assert_eq!(timing.accumulator(), Duration::ZERO);
}

#[test]
fn timing_system_reset_clears_accumulator() {
    let mut timing = TimingSystem::new();
    
    // Simulate some time passing and accumulation
    std::thread::sleep(Duration::from_millis(1));
    let _ = timing.should_tick(); // This will update accumulator
    
    // Accumulator should have some value now
    assert!(timing.accumulator() > Duration::ZERO);
    
    // Reset should clear it
    timing.reset();
    assert_eq!(timing.accumulator(), Duration::ZERO);
}

#[test]
fn timing_system_should_tick_with_short_timestep() {
    let short_timestep = Duration::from_millis(1); // Very short for testing
    let mut timing = TimingSystem::with_timestep(short_timestep);
    
    // Wait a bit to accumulate time
    std::thread::sleep(Duration::from_millis(5));
    
    // Should indicate that a tick is needed
    assert!(timing.should_tick());
}

#[test]
fn timing_system_consume_timestep_reduces_accumulator() {
    let short_timestep = Duration::from_millis(1);
    let mut timing = TimingSystem::with_timestep(short_timestep);
    
    // Wait to accumulate time
    std::thread::sleep(Duration::from_millis(5));
    timing.should_tick(); // Update accumulator
    
    let initial_accumulator = timing.accumulator();
    assert!(initial_accumulator >= short_timestep);
    
    // Consume one timestep
    timing.consume_timestep();
    
    // Accumulator should be reduced by timestep amount
    let after_consume = timing.accumulator();
    assert!(after_consume < initial_accumulator);
    assert_eq!(after_consume, initial_accumulator - short_timestep);
}

#[test]
fn timing_system_multiple_consume_timestep() {
    let short_timestep = Duration::from_millis(1);
    let mut timing = TimingSystem::with_timestep(short_timestep);
    
    // Wait longer to accumulate multiple timesteps worth
    std::thread::sleep(Duration::from_millis(10));
    timing.should_tick(); // Update accumulator
    
    let initial_accumulator = timing.accumulator();
    assert!(initial_accumulator >= short_timestep * 5); // Should have accumulated plenty
    
    // Consume multiple timesteps
    let mut count = 0;
    while timing.accumulator() >= short_timestep && count < 20 {
        timing.consume_timestep();
        count += 1;
    }
    
    // Should have consumed several timesteps
    assert!(count >= 5);
    assert!(timing.accumulator() < short_timestep);
}

#[test]
fn timing_system_iterator_yields_correct_count() {
    let short_timestep = Duration::from_millis(1);
    let mut timing = TimingSystem::with_timestep(short_timestep);
    
    // Wait to accumulate multiple timesteps
    std::thread::sleep(Duration::from_millis(5));
    
    // Count iterations
    let mut count = 0;
    for _ in timing.update() {
        count += 1;
        // Safety valve
        if count > 20 {
            break;
        }
    }
    
    // Should have yielded multiple iterations
    assert!(count >= 3);
    
    // After iterator, accumulator should be less than one timestep
    assert!(timing.accumulator() < short_timestep);
}

#[test]
fn timing_system_iterator_drop_behavior() {
    let short_timestep = Duration::from_millis(1);
    let mut timing = TimingSystem::with_timestep(short_timestep);
    
    // Wait to accumulate time
    std::thread::sleep(Duration::from_millis(5));
    
    // Create iterator but don't consume it fully
    {
        let mut iter = timing.update();
        let _ = iter.next(); // Take only one iteration
        // Iterator is dropped here
    }
    
    // After drop, accumulator should be empty (drop consumes remaining)
    assert!(timing.accumulator() < short_timestep);
}

#[test]
fn timing_system_no_tick_when_insufficient_time() {
    let long_timestep = Duration::from_secs(1); // Very long timestep
    let mut timing = TimingSystem::with_timestep(long_timestep);
    
    // Short wait - not enough for a full timestep
    std::thread::sleep(Duration::from_millis(10));
    
    // Should not indicate tick needed
    assert!(!timing.should_tick());
    
    // Iterator should yield nothing
    let count: usize = timing.update().count();
    assert_eq!(count, 0);
}

#[test]
fn timing_system_accumulator_tracking() {
    let timestep = Duration::from_millis(50);
    let mut timing = TimingSystem::with_timestep(timestep);
    
    // Initially zero
    assert_eq!(timing.accumulator(), Duration::ZERO);
    
    // After checking should_tick, accumulator should have some value
    timing.should_tick();
    let first_check = timing.accumulator();
    assert!(first_check > Duration::ZERO);
    
    // Another check should increase it further
    std::thread::sleep(Duration::from_millis(1));
    timing.should_tick();
    let second_check = timing.accumulator();
    assert!(second_check >= first_check);
}