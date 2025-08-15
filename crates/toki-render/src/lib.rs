//! Simple winit window example.
// winit imports
use winit::application::ApplicationHandler; // Trait that defines app lifecycle hooks (resumed, event handling, etc.)
use winit::dpi::LogicalSize;
use winit::event::WindowEvent; // Enum of possible window-related events (resize, input, close, etc.)
use winit::event_loop::{ActiveEventLoop, EventLoop}; // ActiveEventLoop is used inside lifecycle methods; EventLoop creates and runs the app
use winit::window::{self, Window, WindowAttributes, WindowId}; // Window: window handle; Attributes: window config; ID: unique per window

// wgpu imports
use wgpu::util::DeviceExt;
use wgpu::Device; // Abstraction over GPU hardware; used to create GPU resources (buffers, pipelines, etc.)
use wgpu::Queue; // Used to submit rendering commands to the GPU
use wgpu::Surface; // Represents the drawing surface (your window's framebuffer)
use wgpu::SurfaceConfiguration; // Configuration for how to draw to the surface (format, vsync, etc.)

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
// Local modules
#[path = "util/fill.rs"]
mod fill; // fill_window()

mod errors; // Custom errors
use crate::errors::RenderError;

mod vertex;
use crate::texture::GpuTexture;
use crate::vertex::Vertex;
mod texture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
}

#[derive(Debug)]
struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

#[derive(Debug)]
struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    sprite_position: glam::Vec2,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            gpu: None,
            sprite_position: glam::Vec2::new(32.0, 32.0),
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
        let surface = instance.create_surface(window).unwrap();

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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite.wgsl").into()),
        });

        // Gen transfomation matrix
        let ortho = glam::Mat4::orthographic_rh_gl(
            0.0, 160.0, // left to right
            144.0, 0.0, -1.0, 1.0,
        );

        // Move sprite to pixel position (e.g. 32, 32)
        let model = glam::Mat4::from_translation(glam::vec3(32.0, 32.0, 0.0));
        let mvp = ortho * model;
        let uniforms = Uniforms {
            mvp: mvp.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let slime_texture = GpuTexture::from_file(
            &device,
            &queue,
            "./assets/slime_sprite.png",
            Some("Slime Texture"),
        )
        .map_err(|e| {
            eprintln!("Failed to load slime texture: {e}");
            panic!();
        })
        .unwrap();

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&slime_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&slime_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

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

        let vertices: &[Vertex] = &[
            // Triangle 1
            Vertex {
                position: [0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [16.0, 0.0],
                tex_coords: [1.0, 0.0],
            },
            Vertex {
                position: [16.0, 16.0],
                tex_coords: [1.0, 1.0],
            },
            // Triangle 2
            Vertex {
                position: [0.0, 0.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [16.0, 16.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [0.0, 16.0],
                tex_coords: [0.0, 1.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            cache: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            surface,
            config,
            device,
            queue,
            vertex_buffer,
            render_pipeline,
            texture_bind_group,
            uniform_buffer,
        }
    }

    pub fn update_projection(&self, mvp: glam::Mat4) {
        let uniforms = Uniforms {
            mvp: mvp.to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
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
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        if let Err(e) = self.device.poll(wgpu::PollType::Wait) {
            eprintln!("Device poll failed: {e:?}");
        }
        output.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize default window attributes
        let window_attributes =
            WindowAttributes::default().with_inner_size(LogicalSize::new(160.0, 144.0));

        // Attempt to create a window with the given attributes
        // This has to be done before the GPU state is initialized to ensure
        // its lifetime is longer than that of GPU state
        let raw_window = event_loop.create_window(window_attributes).unwrap();
        let window = Arc::new(raw_window);

        // Now we can safely initialize GPU state
        let gpu = GpuState::new(Arc::clone(&window));
        window.request_redraw();
        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        println!("{event:?}");

        match event {
            // Handle keyboard inputs
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{KeyCode, PhysicalKey};
                // Movement speed in pixels per krey press
                let step = 2.0;
                let sprite_size = 16.0; // your sprite is 16×16 pixels
                let screen_width = 160.0;
                let screen_height = 144.0;
                if event.state.is_pressed() {
                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                            println!("Move forward");
                            self.sprite_position.y = (self.sprite_position.y - step).max(0.0);
                        }
                        PhysicalKey::Code(KeyCode::KeyA)
                        | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                            println!("Move left");
                            self.sprite_position.x = (self.sprite_position.x - step).max(0.0);
                        }
                        PhysicalKey::Code(KeyCode::KeyS)
                        | PhysicalKey::Code(KeyCode::ArrowDown) => {
                            println!("Move backward");
                            self.sprite_position.y =
                                (self.sprite_position.y + step).min(screen_height - sprite_size);
                        }
                        PhysicalKey::Code(KeyCode::KeyD)
                        | PhysicalKey::Code(KeyCode::ArrowRight) => {
                            println!("Move right");
                            self.sprite_position.x =
                                (self.sprite_position.x + step).min(screen_width - sprite_size);
                        }
                        // Ignore all other events
                        _ => (),
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }

            // If the window was closed, stop the event loop
            WindowEvent::CloseRequested => {
                println!("Close was requested; stopping");
                event_loop.exit();
            }
            // If the window was resized, request a redraw
            WindowEvent::Resized(_) => {
                // Get the window from self.window
                let window = self.window.as_ref().expect("resize event without a window");
                let size = window.inner_size();
                let aspect = size.width as f32 / size.height as f32;
                let desired_aspect = 160.0 / 144.0;
                let (view_width, view_height) = if aspect > desired_aspect {
                    let height = 144.0;
                    let width = height * aspect;
                    (width, height)
                } else {
                    let width = 160.0;
                    let height = width / aspect;
                    (width, height)
                };
                let projection =
                    glam::Mat4::orthographic_rh_gl(0.0, view_width, view_height, 0.0, -1.0, 1.0);
                if let Some(gpu) = &mut self.gpu {
                    gpu.update_projection(projection);
                }
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
                    let size = self
                        .window
                        .as_ref()
                        .expect("redraw request without a window")
                        .inner_size();
                    let aspect = size.width as f32 / size.height as f32;
                    let desired_aspect = 160.0 / 144.0;
                    let (view_width, view_height) = if aspect > desired_aspect {
                        let height = 144.0;
                        let width = height * aspect;
                        (width, height)
                    } else {
                        let width = 160.0;
                        let height = width / aspect;
                        (width, height)
                    };
                    let model = glam::Mat4::from_translation(self.sprite_position.extend(0.0));
                    let projection = glam::Mat4::orthographic_rh_gl(
                        0.0,
                        view_width,
                        view_height,
                        0.0,
                        -1.0,
                        1.0,
                    );
                    let mvp = projection * model;

                    gpu.update_projection(mvp);
                    println!("Redrawing projection");
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
