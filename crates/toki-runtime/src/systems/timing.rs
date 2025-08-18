use std::time::{Duration, Instant};

/// Timing system that manages fixed timestep game loop timing.
/// 
/// Handles accumulator-based timing for consistent game logic updates
/// regardless of varying frame rates, implementing the classic "fix your timestep" pattern.
#[derive(Debug)]
pub struct TimingSystem {
    last_update: Instant,
    accumulator: Duration,
    timestep: Duration,
}

impl TimingSystem {
    /// Create a new TimingSystem with 60 FPS timestep
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            accumulator: Duration::ZERO,
            timestep: Duration::from_nanos(16_666_667), // ~16.67ms -> 60fps
        }
    }
    
    /// Create a TimingSystem with custom timestep
    pub fn with_timestep(timestep: Duration) -> Self {
        Self {
            last_update: Instant::now(),
            accumulator: Duration::ZERO,
            timestep,
        }
    }
    
    /// Update timing and return how many fixed timesteps should be processed
    /// 
    /// Call this once per frame in about_to_wait. The returned iterator
    /// yields one unit for each timestep that should be processed.
    pub fn update(&mut self) -> TimestepIterator<'_> {
        let now = Instant::now();
        let dt = now - self.last_update;
        self.last_update = now;
        
        self.accumulator += dt;
        
        TimestepIterator {
            accumulator: &mut self.accumulator,
            timestep: self.timestep,
        }
    }
    
    /// Get the fixed timestep duration
    pub fn timestep(&self) -> Duration {
        self.timestep
    }
    
    /// Get current accumulator value (for debugging)
    pub fn accumulator(&self) -> Duration {
        self.accumulator
    }
    
    /// Reset timing state (useful for pause/resume scenarios)
    pub fn reset(&mut self) {
        self.last_update = Instant::now();
        self.accumulator = Duration::ZERO;
    }
    
    /// Check if a timestep should be processed
    /// Call update() first to refresh the accumulator
    pub fn should_tick(&mut self) -> bool {
        self.update_accumulator();
        self.accumulator >= self.timestep
    }
    
    /// Consume one timestep from the accumulator
    pub fn consume_timestep(&mut self) {
        if self.accumulator >= self.timestep {
            self.accumulator -= self.timestep;
        }
    }
    
    /// Update the accumulator with elapsed time
    fn update_accumulator(&mut self) {
        let now = Instant::now();
        let dt = now - self.last_update;
        self.last_update = now;
        self.accumulator += dt;
    }
}

/// Iterator that yields timesteps to process
pub struct TimestepIterator<'a> {
    accumulator: &'a mut Duration,
    timestep: Duration,
}

impl<'a> Iterator for TimestepIterator<'a> {
    type Item = ();
    
    fn next(&mut self) -> Option<Self::Item> {
        if *self.accumulator >= self.timestep {
            *self.accumulator -= self.timestep;
            Some(())
        } else {
            None
        }
    }
}

impl<'a> Drop for TimestepIterator<'a> {
    fn drop(&mut self) {
        // Ensure we consume all remaining timesteps to prevent accumulation
        while self.next().is_some() {}
    }
}