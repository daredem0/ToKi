use anyhow::Result;
use std::sync::Arc;
use winit::window::Window;

/// Handles WGPU window rendering and egui integration
pub struct WindowRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    egui_renderer: egui_wgpu::Renderer,
}

impl WindowRenderer {
    /// Initialize the renderer with a window
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        // Initialize wgpu
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(Arc::clone(&window))?;
        
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }).await?;
        
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
        ).await?;
        
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);
        
        // Initialize egui renderer
        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);
        
        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            egui_renderer,
        })
    }
    
    /// Handle window resize
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }
    
    /// Get reference to WGPU device (for viewport initialization)
    #[allow(dead_code)] // Will be used for advanced WGPU integration
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    
    /// Get reference to WGPU queue (for viewport initialization)
    #[allow(dead_code)] // Will be used for advanced WGPU integration
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    
    /// Get current surface format (for viewport initialization)
    #[allow(dead_code)] // Will be used for advanced WGPU integration
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }
    
    /// Render a frame with the given egui output and context
    pub fn render(&mut self, _window: &Window, egui_output: egui::FullOutput, egui_ctx: &egui::Context) -> Result<()> {
        // Get surface texture
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                // Recreate surface on error
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(());
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to get surface texture: {e}")),
        };
        
        let surface_view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Editor Render Encoder"),
        });
        
        // Clear screen with dark background
        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        
        // Render egui if we have shapes to render
        if !egui_output.shapes.is_empty() {
            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.surface_config.width, self.surface_config.height],
                pixels_per_point: egui_output.pixels_per_point,
            };
            
            // Tessellate egui shapes using the context
            let clipped_primitives = egui_ctx.tessellate(egui_output.shapes, egui_output.pixels_per_point);
            
            // Update egui textures
            for (id, image_delta) in &egui_output.textures_delta.set {
                self.egui_renderer.update_texture(&self.device, &self.queue, *id, image_delta);
            }
            
            // Update buffers
            self.egui_renderer.update_buffers(&self.device, &self.queue, &mut encoder, &clipped_primitives, &screen_descriptor);
            
            // Create render pass and render egui
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            let mut render_pass_static = render_pass.forget_lifetime();
            self.egui_renderer.render(&mut render_pass_static, &clipped_primitives, &screen_descriptor);
        }
        
        // Free egui textures
        for id in &egui_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        
        // Submit and present
        self.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
        
        Ok(())
    }
}