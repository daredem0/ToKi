use std::time::{Duration, Instant};

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
    show_fps_stats: bool,
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
            show_fps_stats: true,
        }
    }
    
    /// Toggle the display of performance statistics
    pub fn toggle_display(&mut self) {
        self.show_fps_stats = !self.show_fps_stats;
        println!(
            "FPS stats display: {}",
            if self.show_fps_stats { "ON" } else { "OFF" }
        );
    }
    
    /// Check if performance display is enabled
    pub fn is_display_enabled(&self) -> bool {
        self.show_fps_stats
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
    pub fn record_performance_breakdown(&mut self, cpu_time: Duration, draw_time: Duration, total_time: Duration) {
        Self::update_collection(&mut self.cpu_work_times, cpu_time);
        Self::update_collection(&mut self.draw_times, draw_time);
        Self::update_collection(&mut self.total_frame_times, total_time);
    }
    
    /// Print statistics if enough time has passed and display is enabled
    pub fn print_stats_if_needed(&mut self) {
        if !self.show_fps_stats {
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
        if self.frame_times.is_empty() {
            return;
        }

        // Calculate FPS from frame intervals
        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time / self.frame_times.len() as u32;
        let fps = if avg_frame_time.as_nanos() > 0 {
            1_000_000_000.0 / avg_frame_time.as_nanos() as f64
        } else {
            0.0
        };

        // Calculate average tick time (game logic)
        let avg_tick_time = if !self.tick_times.is_empty() {
            let total: Duration = self.tick_times.iter().sum();
            (total / self.tick_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate average draw time (GPU rendering)
        let avg_draw_time = if !self.draw_times.is_empty() {
            let total: Duration = self.draw_times.iter().sum();
            (total / self.draw_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate average CPU work time (frame preparation)
        let avg_cpu_time = if !self.cpu_work_times.is_empty() {
            let total: Duration = self.cpu_work_times.iter().sum();
            (total / self.cpu_work_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate average total frame time
        let avg_total_frame = if !self.total_frame_times.is_empty() {
            let total: Duration = self.total_frame_times.iter().sum();
            (total / self.total_frame_times.len() as u32).as_secs_f64() * 1000.0
        } else {
            0.0
        };

        // Calculate overhead (total - cpu - draw)
        let overhead = avg_total_frame - avg_cpu_time - avg_draw_time;

        // Print comprehensive performance breakdown
        println!(
            "FPS: {:.1} | Frame: {:.2}ms | Tick: {:.2}ms | Draw: {:.2}ms | CPU: {:.2}ms | Overhead: {:.2}ms",
            fps,
            avg_total_frame,
            avg_tick_time,
            avg_draw_time,
            avg_cpu_time,
            overhead.max(0.0) // Don't show negative overhead
        );
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}