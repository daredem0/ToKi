use super::RenderPipeline;
use crate::draw::build_quad_vertices;
use crate::vertex::VertexLayout;
use crate::wgpu_utils::{create_bind_group_layout, create_shader_module, create_texture_bindgroup};
use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use std::path::PathBuf;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::sprite::SpriteFrame;
use wgpu::util::DeviceExt;
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
}

#[derive(Debug)]
pub struct SpritePipeline {
    render_pipeline: WgpuRenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    instances: Vec<SpriteInstance>,
    needs_buffer_update: bool,
}

impl SpritePipeline {
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        texture_path: PathBuf,
    ) -> Self {
        let shader = create_shader_module(device);

        let dummy_uniforms = SpriteUniforms {
            mvp: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite Uniform Buffer"),
            contents: bytemuck::cast_slice(&[dummy_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = create_bind_group_layout(device);

        let bind_group = create_texture_bindgroup(
            device,
            queue,
            &bind_group_layout,
            &uniform_buffer,
            texture_path,
            Some("Sprite Texture"),
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Pipeline"),
            cache: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[QuadVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
        });

        // Create initial empty vertex buffer
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Vertex Buffer"),
            size: std::mem::size_of::<QuadVertex>() as u64 * 6 * 1000, // Space for 1000 sprites
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            render_pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            instances: Vec::new(),
            needs_buffer_update: false,
        }
    }

    pub fn add_sprite(&mut self, instance: SpriteInstance) {
        self.instances.push(instance);
        self.needs_buffer_update = true;
    }

    pub fn clear_sprites(&mut self) {
        self.instances.clear();
        self.needs_buffer_update = true;
    }

    pub fn update_projection(&self, queue: &Queue, mvp: glam::Mat4) {
        let uniforms = SpriteUniforms {
            mvp: mvp.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    fn update_vertex_buffer(&mut self, queue: &Queue) {
        let mut vertices = Vec::new();

        for instance in &self.instances {
            let quad_verts = build_quad_vertices(
                instance.frame,
                instance.size.x,
                instance.size.y,
                instance.position,
            );
            vertices.extend_from_slice(&quad_verts);
        }

        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        self.needs_buffer_update = false;
    }
}

impl RenderPipeline for SpritePipeline {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if !self.instances.is_empty() {
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..(self.instances.len() * 6) as u32, 0..1);
        }
    }

    fn update(&mut self) {
        // Per-frame updates like animation could go here
        // For now, just handle buffer updates when needed
    }

    fn update_with_queue(&mut self, queue: &Queue) {
        if self.needs_buffer_update {
            self.update_vertex_buffer(queue);
        }
    }
}
