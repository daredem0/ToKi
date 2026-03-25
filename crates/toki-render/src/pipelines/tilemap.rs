use super::{
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
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_buffer_capacity: usize,
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
            vertex_buffer: None,
            vertex_buffer_capacity: 0,
            vertex_count: 0,
        })
    }

    pub fn update_vertices(&mut self, device: &Device, queue: &Queue, vertices: &[QuadVertex]) {
        if vertices.is_empty() {
            self.vertex_buffer = None;
            self.vertex_buffer_capacity = 0;
            self.vertex_count = 0;
            return;
        }

        let vertex_data = bytemuck::cast_slice(vertices);
        let required_capacity = vertex_data.len();
        let needs_reallocation = self
            .vertex_buffer
            .as_ref()
            .is_none_or(|_| self.vertex_buffer_capacity < required_capacity);

        if needs_reallocation {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Tilemap Vertex Buffer"),
                size: required_capacity as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.vertex_buffer = Some(buffer);
            self.vertex_buffer_capacity = required_capacity;
        }

        if let Some(buffer) = &self.vertex_buffer {
            queue.write_buffer(buffer, 0, vertex_data);
        }
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
        if let Some(buffer) = &self.vertex_buffer {
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, buffer.slice(..));
            render_pass.draw(0..self.vertex_count as u32, 0..1);
        }
    }

    fn update(&mut self) {
        // Currently no per-frame updates needed for tilemap
        // This could be used for animation or dynamic tile changes
    }
}
