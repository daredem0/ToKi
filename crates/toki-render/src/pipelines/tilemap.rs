use super::RenderPipeline;
use crate::vertex::VertexLayout;
use crate::wgpu_utils::{create_bind_group_layout, create_shader_module, create_texture_bindgroup};
use bytemuck::{Pod, Zeroable};
use std::path::PathBuf;
use toki_core::graphics::vertex::QuadVertex;
use wgpu::util::DeviceExt;
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
    vertex_count: usize,
}

impl TilemapPipeline {
    pub fn new(
        device: &Device,
        queue: &Queue,
        surface_format: wgpu::TextureFormat,
        texture_path: PathBuf,
    ) -> Self {
        let shader = create_shader_module(device);

        let dummy_uniforms = TilemapUniforms {
            mvp: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tilemap Uniform Buffer"),
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
            Some("Tilemap Texture"),
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Tilemap Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Tilemap Pipeline"),
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

        Self {
            render_pipeline,
            bind_group,
            uniform_buffer,
            vertex_buffer: None,
            vertex_count: 0,
        }
    }

    pub fn update_vertices(&mut self, device: &Device, vertices: &[QuadVertex]) {
        let vertex_data = bytemuck::cast_slice(vertices);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Tilemap Vertex Buffer"),
            contents: vertex_data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.vertex_buffer = Some(buffer);
        self.vertex_count = vertices.len();
    }

    pub fn update_projection(&self, queue: &Queue, mvp: glam::Mat4) {
        let uniforms = TilemapUniforms {
            mvp: mvp.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
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
