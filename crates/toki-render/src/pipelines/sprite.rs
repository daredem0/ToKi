use super::{
    build_standard_render_pipeline, create_mvp_uniform_buffer, create_texture_bindgroup_for_source,
    dynamic_buffer::DynamicVertexBuffer, write_uniform_buffer, RenderPipeline, TextureSource,
};
use crate::draw::build_quad_vertices;
use crate::vertex::VertexLayout;
use crate::wgpu_utils::{create_bind_group_layout, create_shader_module};
use crate::RenderError;
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::sprite::SpriteFrame;
use wgpu::{Device, Queue, RenderPass, RenderPipeline as WgpuRenderPipeline};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteUniforms {
    mvp: [[f32; 4]; 4],
}

#[derive(Debug, Clone)]
pub struct SpriteInstance {
    pub frame: SpriteFrame,
    pub position: Vec2,
    pub size: Vec2,
    pub flip_x: bool,
    pub tint_alpha: f32,
}

#[derive(Debug)]
pub struct SpritePipeline {
    render_pipeline: WgpuRenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: DynamicVertexBuffer,
    instances: Vec<SpriteInstance>,
    needs_buffer_update: bool,
}

impl SpritePipeline {
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
            "Sprite Pipeline Layout",
            "Sprite Pipeline",
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
        let dummy_uniforms = SpriteUniforms {
            mvp: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let uniform_buffer =
            create_mvp_uniform_buffer(device, "Sprite Uniform Buffer", dummy_uniforms);

        let bind_group_layout = create_bind_group_layout(device);

        let bind_group = create_texture_bindgroup_for_source(
            device,
            queue,
            &bind_group_layout,
            &uniform_buffer,
            texture_source,
            Some("Sprite Texture"),
        )?;

        let render_pipeline =
            Self::build_render_pipeline(device, surface_format, &bind_group_layout);

        Ok(Self {
            render_pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer: DynamicVertexBuffer::new("Sprite Vertex Buffer"),
            instances: Vec::new(),
            needs_buffer_update: false,
        })
    }

    pub fn add_sprite(&mut self, instance: SpriteInstance) {
        self.instances.push(instance);
        self.needs_buffer_update = true;
    }

    pub fn clear_sprites(&mut self) {
        self.instances.clear();
        self.needs_buffer_update = true;
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    pub fn update_projection(&self, queue: &Queue, mvp: glam::Mat4) {
        write_uniform_buffer(
            queue,
            &self.uniform_buffer,
            SpriteUniforms {
                mvp: mvp.to_cols_array_2d(),
            },
        );
    }

    fn update_vertex_buffer(&mut self, device: &Device, queue: &Queue) {
        let mut vertices = Vec::new();

        for instance in &self.instances {
            let quad_verts = build_quad_vertices(
                instance.frame,
                instance.size.x,
                instance.size.y,
                instance.position,
                instance.flip_x,
                instance.tint_alpha,
            );
            vertices.extend_from_slice(&quad_verts);
        }

        self.vertex_buffer
            .write(device, queue, bytemuck::cast_slice(&vertices));

        self.needs_buffer_update = false;
    }
}

impl RenderPipeline for SpritePipeline {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        self.render_range(render_pass, 0, self.instances.len());
    }

    fn update(&mut self) {
        // Per-frame updates like animation could go here
        // For now, just handle buffer updates when needed
    }

    fn update_with_queue(&mut self, device: &Device, queue: &Queue) {
        if self.needs_buffer_update {
            self.update_vertex_buffer(device, queue);
        }
    }
}

impl SpritePipeline {
    pub fn render_range<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        start: usize,
        count: usize,
    ) {
        if count == 0 || start >= self.instances.len() {
            return;
        }

        let end = (start + count).min(self.instances.len());
        if end <= start {
            return;
        }

        let Some(vertex_buffer) = self.vertex_buffer.buffer() else {
            return;
        };

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw((start * 6) as u32..(end * 6) as u32, 0..1);
    }
}
