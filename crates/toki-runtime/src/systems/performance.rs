use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceStats {
    pub fps: f64,
    pub frame_ms: f64,
    pub tick_ms: f64,
    pub draw_ms: f64,
    pub cpu_ms: f64,
    pub overhead_ms: f64,
}

impl PerformanceStats {
    pub fn format_line(&self) -> String {
        format!(
            "FPS: {:.1} | Frame: {:.2}ms | Tick: {:.2}ms | Draw: {:.2}ms | CPU: {:.2}ms | Overhead: {:.2}ms",
            self.fps,
            self.frame_ms,
            self.tick_ms,
            self.draw_ms,
            self.cpu_ms,
            self.overhead_ms
        )
    }
}

/// Performance monitoring system that tracks frame timing and displays statistics.
///
/// Uses Option B approach: App provides the timing measurements, PerformanceMonitor
/// stores the data and handles display logic.
#[derive(Debug)]
pub struct PerformanceMonitor {
    // Data storage for rolling statistics
    frame_times: Vec<Duration>,
    tick_times: Vec<Duration>,
    draw_times: Vec<Duration>,
    cpu_work_times: Vec<Duration>,
    total_frame_times: Vec<Duration>,

    // Display state
    last_fps_print: Instant,
    last_frame_time: Instant,
    show_console_stats: bool,
    show_hud_stats: bool,
}

impl PerformanceMonitor {
    /// Create a new PerformanceMonitor with stats enabled by default
    pub fn new() -> Self {
        Self {
            frame_times: Vec::new(),
            tick_times: Vec::new(),
            draw_times: Vec::new(),
            cpu_work_times: Vec::new(),
            total_frame_times: Vec::new(),
            last_fps_print: Instant::now(),
            last_frame_time: Instant::now(),
            show_console_stats: true,
            show_hud_stats: true,
        }
    }

    /// Toggle the in-window HUD display of performance statistics
    pub fn toggle_hud_display(&mut self) {
        self.show_hud_stats = !self.show_hud_stats;
        println!(
            "FPS HUD display: {}",
            if self.show_hud_stats { "ON" } else { "OFF" }
        );
    }

    /// Toggle console printing of performance statistics
    pub fn toggle_console_display(&mut self) {
        self.show_console_stats = !self.show_console_stats;
        println!(
            "FPS console logging: {}",
            if self.show_console_stats { "ON" } else { "OFF" }
        );
    }

    /// Check if HUD performance display is enabled
    pub fn is_hud_display_enabled(&self) -> bool {
        self.show_hud_stats
    }

    /// Check if console performance logging is enabled
    pub fn is_console_display_enabled(&self) -> bool {
        self.show_console_stats
    }

    /// Record the time interval between frames
    pub fn record_frame_interval(&mut self, now: Instant) {
        let frame_time = now.duration_since(self.last_frame_time);
        self.last_frame_time = now;

        Self::update_collection(&mut self.frame_times, frame_time);

        // Check if we should print stats
        self.print_stats_if_needed();
    }

    /// Record the time taken for a game logic tick
    pub fn record_tick_time(&mut self, tick_time: Duration) {
        Self::update_collection(&mut self.tick_times, tick_time);
    }

    /// Record performance breakdown for a frame
    pub fn record_performance_breakdown(
        &mut self,
        cpu_time: Duration,
        draw_time: Duration,
        total_time: Duration,
    ) {
        Self::update_collection(&mut self.cpu_work_times, cpu_time);
        Self::update_collection(&mut self.draw_times, draw_time);
        Self::update_collection(&mut self.total_frame_times, total_time);
    }

    /// Print statistics if enough time has passed and display is enabled
    pub fn print_stats_if_needed(&mut self) {
        if !self.show_console_stats {
            return;
        }

        let now = Instant::now();
        if now.duration_since(self.last_fps_print) >= Duration::from_secs(1) {
            self.print_performance_stats();
            self.last_fps_print = now;
        }
    }

    /// Update a collection with a new value, maintaining a rolling window
    fn update_collection(collection: &mut Vec<Duration>, new_value: Duration) {
        collection.push(new_value);

        const MAX_SAMPLES: usize = 60;
        if collection.len() > MAX_SAMPLES {
            collection.remove(0);
        }
    }

    /// Print comprehensive performance statistics
    fn print_performance_stats(&self) {
        if let Some(stats) = self.current_stats() {
            println!("{}", stats.format_line());
        }
    }

    pub fn current_stats(&self) -> Option<PerformanceStats> {
        if self.frame_times.is_empty() {
            return None;
        }

        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time / self.frame_times.len() as u32;
        let fps = if avg_frame_time.as_nanos() > 0 {
            1_000_000_000.0 / avg_frame_time.as_nanos() as f64
        } else {
            0.0
        };

        let avg_tick_time = if !self.tick_times.is_empty() {
            let total: Duration = self.tick_times.iter().sum();
            (total / self.tick_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        let avg_draw_time = if !self.draw_times.is_empty() {
            let total: Duration = self.draw_times.iter().sum();
            (total / self.draw_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        let avg_cpu_time = if !self.cpu_work_times.is_empty() {
            let total: Duration = self.cpu_work_times.iter().sum();
            (total / self.cpu_work_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        let avg_total_frame = if !self.total_frame_times.is_empty() {
            let total: Duration = self.total_frame_times.iter().sum();
            (total / self.total_frame_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        let overhead = (avg_total_frame - avg_cpu_time - avg_draw_time).max(0.0);

        Some(PerformanceStats {
            fps,
            frame_ms: avg_total_frame,
            tick_ms: avg_tick_time,
            draw_ms: avg_draw_time,
            cpu_ms: avg_cpu_time,
            overhead_ms: overhead,
        })
    }

    pub fn stats_line(&self) -> Option<String> {
        if !self.show_hud_stats {
            return None;
        }
        self.current_stats().map(|stats| stats.format_line())
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "performance_tests.rs"]
mod tests;
