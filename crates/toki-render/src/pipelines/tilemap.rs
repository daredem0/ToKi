use super::{
    dynamic_buffer::DynamicVertexBuffer,
    build_standard_render_pipeline, create_mvp_uniform_buffer, create_texture_bindgroup_for_source,
    write_uniform_buffer, RenderPipeline, TextureSource,
};
use crate::vertex::VertexLayout;
use crate::wgpu_utils::{create_bind_group_layout, create_shader_module};
use crate::RenderError;
use bytemuck::{Pod, Zeroable};
use toki_core::graphics::vertex::QuadVertex;
use wgpu::{Device, Queue, RenderPass, RenderPipeline as WgpuRenderPipeline};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TilemapUniforms {
    mvp: [[f32; 4]; 4],
}

#[derive(Debug)]
pub struct TilemapPipeline {
    render_pipeline: WgpuRenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: DynamicVertexBuffer,
    vertex_count: usize,
}

impl TilemapPipeline {
    fn build_render_pipeline(
        device: &Device,
        surface_format: wgpu::TextureFormat,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> WgpuRenderPipeline {
        let shader = create_shader_module(device);
        build_standard_render_pipeline(
            device,
            surface_format,
            &shader,
            "Tilemap Pipeline Layout",
            "Tilemap Pipeline",
            &[bind_group_layout],
            &[QuadVertex::desc()],
        )
    }

    pub(crate) fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        texture_source: TextureSource<'_>,
    ) -> Result<Self, RenderError> {
        let dummy_uniforms = TilemapUniforms {
            mvp: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let uniform_buffer =
            create_mvp_uniform_buffer(device, "Tilemap Uniform Buffer", dummy_uniforms);

        let bind_group_layout = create_bind_group_layout(device);

        let bind_group = create_texture_bindgroup_for_source(
            device,
            queue,
            &bind_group_layout,
            &uniform_buffer,
            texture_source,
            Some("Tilemap Texture"),
        )?;

        let render_pipeline =
            Self::build_render_pipeline(device, surface_format, &bind_group_layout);

        Ok(Self {
            render_pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer: DynamicVertexBuffer::new("Tilemap Vertex Buffer"),
            vertex_count: 0,
        })
    }

    pub fn update_vertices(&mut self, device: &Device, queue: &Queue, vertices: &[QuadVertex]) {
        if vertices.is_empty() {
            self.vertex_buffer.clear();
            self.vertex_count = 0;
            return;
        }

        let vertex_data = bytemuck::cast_slice(vertices);
        self.vertex_buffer.write(device, queue, vertex_data);
        self.vertex_count = vertices.len();
    }

    pub fn update_projection(&self, queue: &Queue, mvp: glam::Mat4) {
        write_uniform_buffer(
            queue,
            &self.uniform_buffer,
            TilemapUniforms {
                mvp: mvp.to_cols_array_2d(),
            },
        );
    }
}

impl RenderPipeline for TilemapPipeline {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        let Some(buffer) = self.vertex_buffer.buffer() else {
            return;
        };

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, buffer.slice(..));
        render_pass.draw(0..self.vertex_count as u32, 0..1);
    }

    fn update(&mut self) {
        // Currently no per-frame updates needed for tilemap
        // This could be used for animation or dynamic tile changes
    }
}
