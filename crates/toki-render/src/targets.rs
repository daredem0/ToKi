use crate::RenderError;

/// Platform abstraction for surface creation.
/// Implementors provide a pre-created wgpu surface and dimensions.
/// This decouples the render layer from specific windowing systems like winit.
pub trait SurfaceProvider {
    /// Get the wgpu surface for rendering
    fn surface(&self) -> &wgpu::Surface<'static>;
    /// Get the current dimensions of the surface
    fn dimensions(&self) -> (u32, u32);
}

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
            wgpu::FilterMode::Nearest, // Use nearest neighbor for sharp pixel art
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
        tracing::trace!(
            "OffscreenTarget: Beginning frame ({}x{})",
            self.size.0,
            self.size.1
        );
        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), RenderError> {
        tracing::trace!("OffscreenTarget: Frame complete");
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
        #[cfg(feature = "editor")]
        {
            self.egui_texture_id = None;
        }
        Ok(())
    }
}
