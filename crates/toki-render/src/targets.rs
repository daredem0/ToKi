use crate::RenderError;
use std::sync::Arc;
use winit::window::Window;

/// Trait for different render targets (window surface, offscreen texture, etc.)
pub trait RenderTarget {
    /// Get the texture view to render to
    fn get_render_view(&mut self) -> Result<&wgpu::TextureView, RenderError>;
    
    /// Get the size of the render target
    fn size(&self) -> (u32, u32);
    
    /// Called before rendering starts - prepares the render target
    fn begin_frame(&mut self) -> Result<(), RenderError>;
    
    /// Called after rendering completes - presents or finalizes the target
    fn end_frame(&mut self) -> Result<(), RenderError>;
    
    /// Get the texture format
    fn format(&self) -> wgpu::TextureFormat;
    
    /// Handle resize events
    fn resize(&mut self, new_size: (u32, u32)) -> Result<(), RenderError>;
}

/// Render target for window surfaces (used by toki-runtime)
pub struct WindowTarget {
    surface: wgpu::Surface<'static>,
    current_texture: Option<wgpu::SurfaceTexture>,
    current_view: Option<wgpu::TextureView>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
}

impl WindowTarget {
    pub fn new(window: Arc<Window>, device: wgpu::Device, adapter: &wgpu::Adapter) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(Arc::clone(&window))
            .map_err(|e| RenderError::Other(format!("Failed to create surface: {}", e)))?;
        
        let surface_caps = surface.get_capabilities(adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &config);
        
        Ok(Self {
            surface,
            current_texture: None,
            current_view: None,
            config,
            device,
        })
    }
}

impl RenderTarget for WindowTarget {
    fn get_render_view(&mut self) -> Result<&wgpu::TextureView, RenderError> {
        self.current_view.as_ref().ok_or_else(|| {
            RenderError::Other("No current view available. Call begin_frame() first.".to_string())
        })
    }
    
    fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }
    
    fn begin_frame(&mut self) -> Result<(), RenderError> {
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                // Recreate surface on error
                self.surface.configure(&self.device, &self.config);
                return Err(RenderError::Other("Surface lost, reconfigured".to_string()));
            }
            Err(e) => return Err(RenderError::Other(format!("Failed to get surface texture: {}", e))),
        };
        
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        self.current_texture = Some(surface_texture);
        self.current_view = Some(view);
        
        Ok(())
    }
    
    fn end_frame(&mut self) -> Result<(), RenderError> {
        if let Some(texture) = self.current_texture.take() {
            texture.present();
        }
        self.current_view = None;
        Ok(())
    }
    
    fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
    
    fn resize(&mut self, new_size: (u32, u32)) -> Result<(), RenderError> {
        self.config.width = new_size.0.max(1);
        self.config.height = new_size.1.max(1);
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }
}

/// Render target for offscreen textures (used by toki-editor)
pub struct OffscreenTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    device: wgpu::Device,
    // For egui integration
    #[cfg(feature = "editor")]
    pub egui_texture_id: Option<egui::TextureId>,
}

impl OffscreenTarget {
    pub fn new(
        device: wgpu::Device,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> Result<Self, RenderError> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Render Target"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        Ok(Self {
            texture,
            view,
            size,
            format,
            device,
            #[cfg(feature = "editor")]
            egui_texture_id: None,
        })
    }
    
    /// Get the underlying texture for egui integration
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    
    /// Register this texture with egui renderer
    #[cfg(feature = "editor")]
    pub fn register_with_egui(&mut self, renderer: &mut egui_wgpu::Renderer) -> egui::TextureId {
        let texture_id = renderer.register_native_texture(
            &self.device,
            &self.view,
            wgpu::FilterMode::Linear,
        );
        self.egui_texture_id = Some(texture_id);
        texture_id
    }
}

impl RenderTarget for OffscreenTarget {
    fn get_render_view(&mut self) -> Result<&wgpu::TextureView, RenderError> {
        Ok(&self.view)
    }
    
    fn size(&self) -> (u32, u32) {
        self.size
    }
    
    fn begin_frame(&mut self) -> Result<(), RenderError> {
        // Nothing special needed for offscreen targets
        Ok(())
    }
    
    fn end_frame(&mut self) -> Result<(), RenderError> {
        // Nothing special needed for offscreen targets
        Ok(())
    }
    
    fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
    
    fn resize(&mut self, new_size: (u32, u32)) -> Result<(), RenderError> {
        if new_size == self.size {
            return Ok(());
        }
        
        // Recreate texture with new size
        let new_target = Self::new(self.device.clone(), new_size, self.format)?;
        self.texture = new_target.texture;
        self.view = new_target.view;
        self.size = new_target.size;
        Ok(())
    }
}