//! Simple winit window example.
// winit imports
use winit::window::Window; // Window: window handle; Attributes: window config; ID: unique per window

// wgpu imports
use wgpu::util::DeviceExt;
use wgpu::Device; // Abstraction over GPU hardware; used to create GPU resources (buffers, pipelines, etc.)
use wgpu::Queue; // Used to submit rendering commands to the GPU
use wgpu::Surface; // Represents the drawing surface (your window's framebuffer)
use wgpu::SurfaceConfiguration; // Configuration for how to draw to the surface (format, vsync, etc.)

use bytemuck::{Pod, Zeroable};
use glam::Vec2;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
// Local modules

use toki_core::sprite::{SpriteFrame, SpriteSheetMeta};

use crate::vertex::VertexLayout;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::Camera;

use crate::draw::build_quad_vertices;
use crate::pipeline::{
    create_bind_group_layout, create_device_and_surface, create_shader_module,
    create_texture_bindgroup,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
}
#[allow(dead_code)]
#[derive(Debug)]
pub struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    map_bind_group: wgpu::BindGroup,
    creature_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    tilemap_vertex_buffer: Option<wgpu::Buffer>,
    tilemap_vertex_count: usize,
}

fn to_absolute_path<P: AsRef<Path>>(relative: P) -> std::io::Result<PathBuf> {
    fs::canonicalize(relative)
}

impl GpuState {
    pub fn update_vertex_buffer(&mut self, frame: SpriteFrame, pos: Vec2) {
        let verts = build_quad_vertices(frame, 16.0, 16.0, pos);
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn new(window: Arc<Window>) -> Self {
        let (device, queue, surface, config) = create_device_and_surface(Arc::clone(&window));

        let shader = create_shader_module(&device);

        let dummy_uniforms = Uniforms {
            mvp: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[dummy_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_bind_group_layout = create_bind_group_layout(&device);

        let creature_bind_group = create_texture_bindgroup(
            &device,
            &queue,
            &texture_bind_group_layout,
            &uniform_buffer,
            to_absolute_path("./assets/creatures.png").unwrap(),
            Some("Creatures Texture"),
        );
        let map_bind_group = create_texture_bindgroup(
            &device,
            &queue,
            &texture_bind_group_layout,
            &uniform_buffer,
            to_absolute_path("./assets/terrain.png").unwrap(),
            Some("Terrain Texture"),
        );

        let f0 = SpriteSheetMeta {
            frame_size: (16, 16),
            sheet_size: (64, 16),
            frame_count: 4,
        }
        .uv_rect(0);

        let vertices: [QuadVertex; 6] = build_quad_vertices(f0, 16.0, 16.0, Vec2::ZERO);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices[..]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
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
                    format: config.format,
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
            surface,
            config,
            device,
            queue,
            vertex_buffer,
            render_pipeline,
            creature_bind_group,
            map_bind_group,
            uniform_buffer,
            tilemap_vertex_buffer: None,
            tilemap_vertex_count: 0,
        }
    }

    pub fn update_tilemap_vertex_buffer(&mut self, vertices: &[QuadVertex]) {
        let vertex_data = bytemuck::cast_slice(vertices);

        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Tilemap Vertex Buffer"),
                contents: vertex_data,
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.tilemap_vertex_buffer = Some(buffer);
        self.tilemap_vertex_count = vertices.len();
    }

    pub fn update_projection(&self, mvp: glam::Mat4) {
        let uniforms = Uniforms {
            mvp: mvp.to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn draw(&mut self, camera: &Camera) {
        let output = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.update_projection(camera.calculate_projection());

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_viewport(
                0.0,
                0.0,
                self.config.width as f32,
                self.config.height as f32,
                0.0,
                1.0,
            );

            // Draw tilemap
            if let Some(buffer) = &self.tilemap_vertex_buffer {
                render_pass.set_bind_group(0, &self.map_bind_group, &[]);
                render_pass.set_vertex_buffer(0, buffer.slice(..));
                render_pass.draw(0..self.tilemap_vertex_count as u32, 0..1);
            }

            // Draw sprite
            render_pass.set_bind_group(0, &self.creature_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        if let Err(e) = self.device.poll(wgpu::PollType::Wait) {
            tracing::error!("Device poll failed: {e:?}");
        }
        output.present();
    }
}
