//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::window::{Window, WindowAttributes, WindowId}; // Window: window handle; Attributes: window config; ID: unique per window

// wgpu imports
use wgpu::Device; // Abstraction over GPU hardware; used to create GPU resources (buffers, pipelines, etc.)
use wgpu::Queue; // Used to submit rendering commands to the GPU
use wgpu::Surface; // Represents the drawing surface (your window's framebuffer)
use wgpu::SurfaceConfiguration; // Configuration for how to draw to the surface (format, vsync, etc.)

use std::sync::Arc;
// Local modules
#[path = "util/fill.rs"]
mod fill; // fill_window()

mod errors; // Custom errors
use crate::errors::RenderError;

#[derive(Debug)]
struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
}

#[derive(Debug)]
struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            gpu: None,
        }
    }
}

impl GpuState {
    fn new(window: Arc<Window>) -> Self {
        // Create wgpu instance
        let instance = wgpu::Instance::default();

        // This has to happen before we set the surface. Once we create the surface,
        // we dont own the window anymore.
        let size = window.inner_size();
        // Create the surface of the window
        let surface = unsafe { instance.create_surface(window).unwrap() };

        // Get a GPU Abstraction. Important: This has to be async
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("No suitable GPU adapters found on the system!");

        // Now that we got the adapter, we can request the actual GPU device and command queue
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::default(),
            label: Some("Toki device"),
        }))
        .expect("Failed to create device");

        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo, //vsync
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };

        surface.configure(&device, &config);

        Self {
            surface,
            config,
            device,
            queue,
        }
    }
    fn draw(&mut self) {
        let output = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Drawing commands go here...
        }

        self.queue.submit(Some(encoder.finish()));
        self.device.poll(wgpu::PollType::Wait); // <-- This is crucial
        output.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize default window attributes
        let window_attributes = WindowAttributes::default();

        // Attempt to create a window with the given attributes
        // This has to be done before the GPU state is initialized to ensure
        // its lifetime is longer than that of GPU state
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // Now we can safely initialize GPU state
        let gpu = GpuState::new(Arc::clone(&window));
        window.request_redraw();
        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        println!("{event:?}");

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

                // Wayland needs something to actually be drawn to even show the window
                // so were just filling it up for now.
                //fill::fill_window(window);
                if let Some(gpu) = &mut self.gpu {
                    gpu.draw();
                }
            }
            // Ignore all other events
            _ => (),
        }
    }
}

/// Runs a minimal window using the winit library.
pub fn run_minimal_window() -> Result<(), RenderError> {
    let event_loop = EventLoop::new()?;

    // Create an instance of the App struct
    let mut app = App::new();

    // Run the application
    event_loop.run_app(&mut app)?;

    // Return Ok if the application was closed successfully
    Ok(())
}
