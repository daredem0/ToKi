//! Simple winit window example.
// winit imports
use winit::window::Window; // Window: window handle; Attributes: window config; ID: unique per window

// wgpu imports
use wgpu::Surface; // Represents the drawing surface (your window's framebuffer)
use wgpu::SurfaceConfiguration; // Configuration for how to draw to the surface (format, vsync, etc.)

use std::sync::Arc;
// Local modules

use crate::texture::GpuTexture;

pub fn create_texture_bindgroup(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_bind_group_layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    texture_file: std::path::PathBuf,
    texture_label: Option<&str>,
) -> wgpu::BindGroup {
    // Convert path to string with proper error handling
    let texture_path_str = texture_file.as_path().to_str().unwrap_or_else(|| {
        tracing::error!(
            "Failed to convert texture path to string: {:?}",
            texture_file
        );
        panic!("Invalid texture path encoding: {:?}", texture_file);
    });

    if texture_path_str.is_empty() {
        tracing::debug!(
            "Loading default texture (no path provided) for label: {:?}",
            texture_label
        );
    } else {
        tracing::debug!("Loading texture from: {}", texture_path_str);
    }

    let texture = GpuTexture::from_file(device, queue, texture_path_str, texture_label)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load texture from '{}': {}", texture_path_str, e);
            tracing::error!("Texture label: {:?}", texture_label);
            tracing::error!("Make sure the texture file exists and is a valid image format");
            panic!("Texture loading failed for '{}': {}", texture_path_str, e);
        });

    tracing::debug!("Creating bind group for texture: {:?}", texture_label);
    let bind_group = create_bind_group(device, texture_bind_group_layout, &texture, uniform_buffer);

    if texture_path_str.is_empty() {
        tracing::debug!(
            "Successfully created texture bind group with default texture for: {:?}",
            texture_label
        );
    } else {
        tracing::debug!(
            "Successfully created texture bind group for: {}",
            texture_path_str
        );
    }

    bind_group
}
pub fn create_device_and_surface(
    window: Arc<Window>,
) -> (
    wgpu::Device,
    wgpu::Queue,
    Surface<'static>,
    SurfaceConfiguration,
) {
    pollster::block_on(create_device_and_surface_async(window))
}

/// Async version of WGPU setup for better integration with modern async runtimes
pub async fn create_device_and_surface_async(
    window: Arc<Window>,
) -> (
    wgpu::Device,
    wgpu::Queue,
    Surface<'static>,
    SurfaceConfiguration,
) {
    // Create wgpu instance with better defaults
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        flags: wgpu::InstanceFlags::default(),
        ..Default::default()
    });

    // Get window size before creating surface
    let size = window.inner_size();

    // Create the surface of the window
    let surface = instance
        .create_surface(window)
        .expect("Failed to create surface");

    // Get a GPU adapter with proper error handling
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("No suitable GPU adapters found on the system!");

    // Request GPU device and command queue with proper features
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::default(),
            label: Some("Toki Device"),
        })
        .await
        .expect("Failed to create device");

    // Configure surface with VSync and proper format selection
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    // Choose VSync for frame rate limiting and lower CPU usage
    let present_mode = surface_caps
        .present_modes
        .iter()
        .find(|&&mode| mode == wgpu::PresentMode::Fifo) // VSync (60 FPS cap)
        .or_else(|| {
            surface_caps
                .present_modes
                .iter()
                .find(|&&mode| mode == wgpu::PresentMode::FifoRelaxed)
        }) // Adaptive VSync
        .copied()
        .unwrap_or(surface_caps.present_modes[0]); // Fallback to first available

    tracing::info!(
        "Using present mode: {:?} (available: {:?})",
        present_mode,
        surface_caps.present_modes
    );

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &config);
    (device, queue, surface, config)
}

pub fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Sprite Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprite.wgsl").into()),
    })
}

pub fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
    })
}

pub fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture: &GpuTexture,
    uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Texture Bind Group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&texture.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    })
}
