use crate::errors::RenderError;
use std::path::Path;
use toki_core::graphics::image::{load_image_rgba8, DecodedImage};

pub struct GpuTexture {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl GpuTexture {
    pub fn from_file(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
        label: Option<&str>,
    ) -> Result<Self, RenderError> {
        // Check for empty path and create default texture
        if path.is_empty() {
            tracing::trace!("Creating default white texture for label: {:?}", label);
            return Self::create_default_white_texture(device, queue, label);
        }

        // load the img
        let path_obj = Path::new(path);
        let image = load_image_rgba8(path_obj)?;
        Self::from_rgba8(device, queue, &image, label)
    }

    /// Create a default 1x1 white texture for when no texture path is provided
    pub fn create_default_white_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: Option<&str>,
    ) -> Result<Self, RenderError> {
        // Create a 1x1 white pixel
        let image = DecodedImage {
            width: 1,
            height: 1,
            data: vec![255, 255, 255, 255], // RGBA white
        };

        Self::from_rgba8(device, queue, &image, label)
    }
    pub fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image: &DecodedImage,
        label: Option<&str>,
    ) -> Result<Self, RenderError> {
        // Get the dimensins
        let size = wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        };

        // Create the GPU texture
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Upload the image data to the texture
        queue.write_texture(
            // format for destination
            texture.as_image_copy(),
            &image.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * image.width),
                rows_per_image: Some(image.height),
            },
            size,
        );

        // Create view and sampler
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Ok(Self { view, sampler })
    }
}
