use std::time::{Duration, Instant};

/// Controls visual frame rate limiting, separate from game tick rate.
///
/// The frame limiter calculates how long to wait before rendering the next frame.
/// It does NOT perform the actual waiting - that's the caller's responsibility.
/// This separation allows the limiter to be fully testable without real time.
#[derive(Debug, Clone)]
pub struct FrameLimiter {
    /// Target duration between frames. None means unlimited (vsync-driven).
    target_frame_duration: Option<Duration>,
    /// Instant when the last frame started rendering.
    last_frame_instant: Option<Instant>,
}

impl Default for FrameLimiter {
    fn default() -> Self {
        Self::new_unlimited()
    }
}

impl FrameLimiter {
    /// Create a frame limiter with no limit (vsync-driven or as fast as possible).
    pub fn new_unlimited() -> Self {
        Self {
            target_frame_duration: None,
            last_frame_instant: None,
        }
    }

    /// Create a frame limiter targeting a specific FPS.
    /// If fps is 0, creates an unlimited limiter.
    pub fn new_with_target_fps(fps: u32) -> Self {
        if fps == 0 {
            return Self::new_unlimited();
        }

        // Calculate frame duration in nanoseconds for precision
        let frame_duration_ns = 1_000_000_000u64 / u64::from(fps);
        Self {
            target_frame_duration: Some(Duration::from_nanos(frame_duration_ns)),
            last_frame_instant: None,
        }
    }

    /// Returns true if this limiter has no frame rate limit.
    pub fn is_unlimited(&self) -> bool {
        self.target_frame_duration.is_none()
    }

    /// Returns the target frame duration, if any.
    pub fn target_frame_duration(&self) -> Option<Duration> {
        self.target_frame_duration
    }

    /// Calculate how long to wait before rendering the next frame.
    ///
    /// Pass the current instant. Returns `Duration::ZERO` if:
    /// - The limiter is unlimited
    /// - This is the first frame
    /// - Enough time has passed since the last frame
    ///
    /// Otherwise returns the remaining time to wait.
    pub fn calculate_wait_duration(&self, now: Instant) -> Duration {
        let Some(target) = self.target_frame_duration else {
            return Duration::ZERO;
        };

        let Some(last) = self.last_frame_instant else {
            return Duration::ZERO;
        };

        let elapsed = now.duration_since(last);
        target.saturating_sub(elapsed)
    }

    /// Record that a frame is starting at the given instant.
    /// Call this after waiting (if any) and before rendering.
    pub fn record_frame_start(&mut self, now: Instant) {
        self.last_frame_instant = Some(now);
    }

    /// Convenience method: calculate wait, then record frame start.
    /// Returns the wait duration that should be slept before rendering.
    pub fn next_frame(&mut self, now: Instant) -> Duration {
        let wait = self.calculate_wait_duration(now);
        // Record the target time, not the actual time, to avoid drift
        if let (Some(target), Some(last)) = (self.target_frame_duration, self.last_frame_instant) {
            // If we're on schedule or behind, advance by one frame duration
            let elapsed = now.duration_since(last);
            if elapsed >= target {
                // Behind schedule - snap to now to avoid accumulating debt
                self.last_frame_instant = Some(now);
            } else {
                // On schedule - advance by exact frame duration to prevent drift
                self.last_frame_instant = Some(last + target);
            }
        } else {
            self.last_frame_instant = Some(now);
        }
        wait
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlimited_limiter_returns_zero_wait() {
        let limiter = FrameLimiter::new_unlimited();
        let now = Instant::now();
        assert_eq!(limiter.calculate_wait_duration(now), Duration::ZERO);
    }

    #[test]
    fn unlimited_limiter_is_unlimited() {
        let limiter = FrameLimiter::new_unlimited();
        assert!(limiter.is_unlimited());
        assert!(limiter.target_frame_duration().is_none());
    }

    #[test]
    fn target_fps_creates_limited_limiter() {
        let limiter = FrameLimiter::new_with_target_fps(60);
        assert!(!limiter.is_unlimited());

        // 60 FPS = 16.666...ms per frame
        let target = limiter.target_frame_duration().unwrap();
        assert!(target.as_nanos() > 16_000_000);
        assert!(target.as_nanos() < 17_000_000);
    }

    #[test]
    fn target_fps_zero_creates_unlimited() {
        let limiter = FrameLimiter::new_with_target_fps(0);
        assert!(limiter.is_unlimited());
    }

    #[test]
    fn first_frame_returns_zero_wait() {
        let limiter = FrameLimiter::new_with_target_fps(30);
        let now = Instant::now();
        assert_eq!(limiter.calculate_wait_duration(now), Duration::ZERO);
    }

    #[test]
    fn frame_within_budget_returns_remaining_time() {
        let mut limiter = FrameLimiter::new_with_target_fps(30); // ~33.33ms per frame
        let start = Instant::now();
        limiter.record_frame_start(start);

        // Simulate 10ms elapsed
        let after_10ms = start + Duration::from_millis(10);
        let wait = limiter.calculate_wait_duration(after_10ms);

        // Should wait ~23.33ms more (33.33 - 10 = 23.33)
        assert!(
            wait > Duration::from_millis(20),
            "wait {:?} should be > 20ms",
            wait
        );
        assert!(
            wait < Duration::from_millis(25),
            "wait {:?} should be < 25ms",
            wait
        );
    }

    #[test]
    fn frame_past_budget_returns_zero() {
        let mut limiter = FrameLimiter::new_with_target_fps(30); // ~33.33ms per frame
        let start = Instant::now();
        limiter.record_frame_start(start);

        // Simulate 50ms elapsed (past the ~33.33ms budget)
        let after_50ms = start + Duration::from_millis(50);
        let wait = limiter.calculate_wait_duration(after_50ms);

        assert_eq!(wait, Duration::ZERO);
    }

    #[test]
    fn exactly_on_budget_returns_zero() {
        let mut limiter = FrameLimiter::new_with_target_fps(60);
        let start = Instant::now();
        limiter.record_frame_start(start);

        // Get the exact target duration and simulate that much time
        let target = limiter.target_frame_duration().unwrap();
        let on_time = start + target;
        let wait = limiter.calculate_wait_duration(on_time);

        assert_eq!(wait, Duration::ZERO);
    }

    #[test]
    fn next_frame_advances_time_correctly_when_on_schedule() {
        let mut limiter = FrameLimiter::new_with_target_fps(60);
        let target = limiter.target_frame_duration().unwrap();

        // First frame
        let t0 = Instant::now();
        let wait0 = limiter.next_frame(t0);
        assert_eq!(wait0, Duration::ZERO, "first frame should not wait");

        // Second frame - called exactly on time
        let t1 = t0 + target;
        let wait1 = limiter.next_frame(t1);
        assert_eq!(wait1, Duration::ZERO, "on-time frame should not wait");

        // Third frame - called 5ms early
        let t2 = t1 + target - Duration::from_millis(5);
        let wait2 = limiter.next_frame(t2);
        assert!(
            wait2 > Duration::from_millis(4),
            "early frame should wait ~5ms, got {:?}",
            wait2
        );
    }

    #[test]
    fn next_frame_snaps_to_now_when_behind_schedule() {
        let mut limiter = FrameLimiter::new_with_target_fps(60);
        let _target = limiter.target_frame_duration().unwrap();

        // First frame
        let t0 = Instant::now();
        limiter.next_frame(t0);

        // Second frame - called 100ms late (way behind)
        let t1 = t0 + Duration::from_millis(100);
        let wait1 = limiter.next_frame(t1);
        assert_eq!(wait1, Duration::ZERO, "late frame should not wait");

        // Third frame - should be relative to t1, not t0
        let t2 = t1 + Duration::from_millis(5);
        let wait2 = limiter.next_frame(t2);
        assert!(
            wait2 > Duration::from_millis(10),
            "should wait based on t1, not t0, got {:?}",
            wait2
        );
    }

    #[test]
    fn various_fps_values_produce_correct_durations() {
        let test_cases = [
            (30, 33_333_333u64),  // 30 FPS = ~33.33ms
            (60, 16_666_666u64),  // 60 FPS = ~16.67ms
            (120, 8_333_333u64),  // 120 FPS = ~8.33ms
            (144, 6_944_444u64),  // 144 FPS = ~6.94ms
        ];

        for (fps, expected_ns) in test_cases {
            let limiter = FrameLimiter::new_with_target_fps(fps);
            let actual = limiter.target_frame_duration().unwrap().as_nanos() as u64;
            // Allow 1 nanosecond tolerance for integer division
            assert!(
                actual.abs_diff(expected_ns) <= 1,
                "FPS {} expected ~{}ns, got {}ns",
                fps,
                expected_ns,
                actual
            );
        }
    }
}
