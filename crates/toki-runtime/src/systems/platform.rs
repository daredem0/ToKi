use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// Platform system that manages window lifecycle and platform-specific coordination.
///
/// Handles window creation, management, and provides clean APIs for platform operations
/// while abstracting away winit-specific details from the main application logic.
#[derive(Debug)]
pub struct PlatformSystem {
    window: Option<Arc<Window>>,
}

impl Default for PlatformSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformSystem {
    /// Create a new PlatformSystem with no window initially
    pub fn new() -> Self {
        Self { window: None }
    }

    /// Initialize the window with the given event loop
    pub fn initialize_window(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize default window attributes
        let window_attributes =
            WindowAttributes::default().with_inner_size(LogicalSize::new(160.0, 144.0));

        // Attempt to create a window with the given attributes
        let raw_window = event_loop.create_window(window_attributes).unwrap();
        let window = Arc::new(raw_window);

        self.window = Some(window);
    }

    /// Get reference to the window (if available)
    pub fn window(&self) -> Option<&Arc<Window>> {
        self.window.as_ref()
    }

    /// Get window for GPU state initialization (consumes the reference)
    pub fn window_for_gpu(&self) -> Option<Arc<Window>> {
        self.window.clone()
    }

    /// Request a redraw on the window
    pub fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    /// Get the window's inner size
    pub fn inner_size(&self) -> Option<winit::dpi::PhysicalSize<u32>> {
        self.window.as_ref().map(|w| w.inner_size())
    }

    /// Get the window's scale factor
    pub fn scale_factor(&self) -> Option<f64> {
        self.window.as_ref().map(|w| w.scale_factor())
    }

    /// Prepare for presentation (call pre_present_notify)
    pub fn pre_present_notify(&self) {
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
    }

    /// Check if window is available
    pub fn has_window(&self) -> bool {
        self.window.is_some()
    }
}

#[cfg(test)]
#[path = "platform_tests.rs"]
mod tests;
