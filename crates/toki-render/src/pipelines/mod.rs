pub mod debug;
pub mod post_process;
pub mod sprite;
pub mod tilemap;

use bytemuck::Pod;
use std::path::PathBuf;
use toki_core::graphics::image::DecodedImage;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupLayout, Device, Queue, RenderPass, RenderPipeline as WgpuRenderPipeline};

use crate::wgpu_utils::{
    create_texture_bindgroup, create_texture_bindgroup_from_rgba8,
};

/// Common trait for all rendering pipelines
pub trait RenderPipeline {
    /// Render using this pipeline
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>);

    /// Update pipeline state (buffers, uniforms, etc.)
    fn update(&mut self);

    /// Update pipeline state with queue access (optional)
    fn update_with_queue(&mut self, _queue: &Queue) {
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
}

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
) -> wgpu::BindGroup {
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

pub(crate) fn build_standard_render_pipeline(
    device: &Device,
    surface_format: wgpu::TextureFormat,
    shader: &wgpu::ShaderModule,
    pipeline_layout_label: &str,
    pipeline_label: &str,
    bind_group_layouts: &[&BindGroupLayout],
    vertex_buffers: &[wgpu::VertexBufferLayout<'_>],
) -> WgpuRenderPipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(pipeline_layout_label),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(pipeline_label),
        cache: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: vertex_buffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
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
}
