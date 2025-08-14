use crate::{errors::RenderError, texture};
use image::GenericImageView;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use wgpu::{Device, Queue, Sampler, Texture, TextureView};

pub struct GpuTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl GpuTexture {
    pub fn from_file(
        device: &Device,
        queue: &Queue,
        path: &str,
        label: Option<&str>,
    ) -> Result<Self, RenderError> {
        // load the img
        let path_obj = Path::new(path);
        let img = image::open(&path)
            .map_err(|e| RenderError::FileLoad(path_obj.to_path_buf(), e.to_string()))?
            .to_rgba8();

        // Get the dimensins
        let (width, height) = img.dimensions();
        let size = wgpu::Extent3d {
            width,
            height,
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
            &img,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        // Create view and sampler
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}
