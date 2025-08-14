//! Simple winit window example.
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[path = "util/fill.rs"]
mod fill;

mod errors;
use crate::errors::RenderError;

#[derive(Default, Debug)]
struct App {
    window: Option<Window>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize default window attributes
        let window_attributes = WindowAttributes::default();

        // Attempt to create a window with the given attributes
        self.window = match event_loop.create_window(window_attributes) {
            // If successful, store the window in self.window
            Ok(window) => Some(window),
            // If an error occurs, print the error, exit the event loop, and return
            Err(err) => {
                eprintln!("error creating window: {err}");
                event_loop.exit();
                return;
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        // Print a message describing the event that occurred
        println!("{event:?}");

        // Handle the event
        match event {
            // If the window was closed, stop the event loop
            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            }
            // If the window was resized, request a redraw
            WindowEvent::Resized(_) => {
                // Get the window from self.window
                let window = self.window.as_ref().expect("resize event without a window");

                // Request a redraw
                window.request_redraw();
            }
            // If the window needs to be redrawn, redraw it
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Get the window from self.window
                let window = self
                    .window
                    .as_ref()
                    .expect("redraw request without a window");

                // Notify that you're about to draw.
                // This is necessary for some platforms (like X11) to ensure that the window is
                // ready to be drawn to.
                window.pre_present_notify();

                // Draw.
                // This function is defined in the fill module and is responsible for drawing
                // something on the window.
                fill::fill_window(window);

                // For contiguous redraw loop you can request a redraw from here.
                // window.request_redraw();
            }
            // Ignore all other events
            _ => (),
        }
    }
}

/// Runs a minimal window using the winit library.
///
/// This function creates an EventLoop, which is the main entry point for
/// interacting with the winit library. It then creates an instance of the
/// App struct, which implements the ApplicationHandler trait. The
/// ApplicationHandler trait is a part of the winit library and is used to
/// define how the application should respond to events.
///
/// The run_app method is then called on the EventLoop, passing in the
/// instance of App as an argument. This method will block until the
/// application is closed.
///
/// The return value of this function is a Result. If the application is
/// closed successfully, the Result will contain an Ok value. If there is
/// an error, the Result will contain an Err value.
pub fn run_minimal_window() -> Result<(), RenderError> {
    // Create a new EventLoop
    let event_loop = EventLoop::new()?;

    // Create an instance of the App struct
    let mut app = App::default();

    // Run the application
    event_loop.run_app(&mut app)?;

    // Return Ok if the application was closed successfully
    Ok(())
}
