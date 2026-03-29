mod common;
pub mod debug;
mod dynamic_buffer;
pub mod post_process;
pub mod sprite;
pub mod tilemap;

use bytemuck::Pod;
use std::path::PathBuf;
use toki_core::graphics::image::DecodedImage;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, Device, Queue, RenderPass};

use crate::wgpu_utils::{create_texture_bindgroup, create_texture_bindgroup_from_rgba8};
use crate::RenderError;

pub(crate) use common::build_standard_render_pipeline;

/// Common trait for all rendering pipelines
pub trait RenderPipeline {
    /// Render using this pipeline
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>);

    /// Update pipeline state (buffers, uniforms, etc.)
    fn update(&mut self);

    /// Update pipeline state with queue access (optional)
    fn update_with_queue(&mut self, _device: &Device, _queue: &Queue) {
        self.update();
    }
}

pub(crate) enum TextureSource<'a> {
    Path(PathBuf),
    Rgba8(&'a DecodedImage),
}

impl<'a> TextureSource<'a> {
    pub(crate) fn path(path: PathBuf) -> Self {
        Self::Path(path)
    }

    pub(crate) fn rgba8(image: &'a DecodedImage) -> Self {
        Self::Rgba8(image)
    }

    /// A 1x1 white pixel, matching the existing no-texture fallback behavior.
    pub(crate) fn placeholder() -> Self {
        Self::Rgba8(PLACEHOLDER_IMAGE.get_or_init(|| DecodedImage {
            width: 1,
            height: 1,
            data: vec![255, 255, 255, 255],
        }))
    }
}

static PLACEHOLDER_IMAGE: std::sync::OnceLock<DecodedImage> = std::sync::OnceLock::new();

pub(crate) fn create_mvp_uniform_buffer<T: Pod>(
    device: &Device,
    label: &str,
    uniforms: T,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

pub(crate) fn write_uniform_buffer<T: Pod>(
    queue: &Queue,
    uniform_buffer: &wgpu::Buffer,
    uniforms: T,
) {
    queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
}

pub(crate) fn create_texture_bindgroup_for_source(
    device: &Device,
    queue: &Queue,
    bind_group_layout: &BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    texture_source: TextureSource<'_>,
    texture_label: Option<&str>,
) -> Result<wgpu::BindGroup, RenderError> {
    match texture_source {
        TextureSource::Path(texture_path) => create_texture_bindgroup(
            device,
            queue,
            bind_group_layout,
            uniform_buffer,
            texture_path,
            texture_label,
        ),
        TextureSource::Rgba8(image) => create_texture_bindgroup_from_rgba8(
            device,
            queue,
            bind_group_layout,
            uniform_buffer,
            image,
            texture_label,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::TextureSource;
    use std::path::PathBuf;
    use toki_core::graphics::image::DecodedImage;

    #[test]
    fn texture_source_path_variant_preserves_path() {
        let source = TextureSource::path(PathBuf::from("sprites/test.png"));

        match source {
            TextureSource::Path(path) => assert_eq!(path, PathBuf::from("sprites/test.png")),
            TextureSource::Rgba8(_) => panic!("expected path source"),
        }
    }

    #[test]
    fn texture_source_rgba8_variant_preserves_image_reference() {
        let image = DecodedImage {
            width: 2,
            height: 3,
            data: vec![255; 2 * 3 * 4],
        };

        let source = TextureSource::rgba8(&image);

        match source {
            TextureSource::Path(_) => panic!("expected rgba8 source"),
            TextureSource::Rgba8(decoded) => {
                assert_eq!(decoded.width, 2);
                assert_eq!(decoded.height, 3);
            }
        }
    }

    #[test]
    fn texture_source_placeholder_is_1x1_white() {
        let source = TextureSource::placeholder();

        match source {
            TextureSource::Path(_) => panic!("expected rgba8 placeholder"),
            TextureSource::Rgba8(decoded) => {
                assert_eq!(decoded.width, 1);
                assert_eq!(decoded.height, 1);
                assert_eq!(decoded.data, vec![255, 255, 255, 255]);
            }
        }
    }
}
