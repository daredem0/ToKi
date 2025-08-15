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

use std::sync::Arc;
// Local modules

use toki_core::sprite::{SpriteFrame, SpriteSheetMeta};

use crate::vertex::VertexLayout;
use toki_core::graphics::vertex::QuadVertex;

use crate::draw::build_quad_vertices;
use crate::pipeline::{
    create_bind_group, create_bind_group_layout, create_device_and_surface, create_shader_module,
};
use crate::texture::GpuTexture;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
}
#[derive(Debug)]
pub struct GpuState {
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    vertex_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl GpuState {
    pub fn update_vertex_buffer(&mut self, frame: SpriteFrame) {
        let verts = build_quad_vertices(frame);
        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&verts));
    }
    pub fn new(window: Arc<Window>) -> Self {
        let (device, queue, surface, config) = create_device_and_surface(Arc::clone(&window));

        let shader = create_shader_module(&device);

        // Gen transfomation matrix
        let ortho = glam::Mat4::orthographic_rh_gl(
            0.0, 160.0, // left to right
            144.0, 0.0, -1.0, 1.0,
        );

        // Move sprite to pixel position (e.g. 32, 32)
        let model = glam::Mat4::from_translation(glam::vec3(32.0, 32.0, 0.0));
        let mvp = ortho * model;
        let uniforms = Uniforms {
            mvp: mvp.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let texture_bind_group_layout = create_bind_group_layout(&device);

        let slime_texture = GpuTexture::from_file(
            &device,
            &queue,
            "./assets/slime_sprite_idle_64_16.png",
            Some("Slime Texture"),
        )
        .map_err(|e| {
            tracing::error!("Failed to load slime texture: {e}");
            panic!();
        })
        .unwrap();

        let texture_bind_group = create_bind_group(
            &device,
            &texture_bind_group_layout,
            &slime_texture,
            &uniform_buffer,
        );

        let f0 = SpriteSheetMeta {
            frame_size: (16, 16),
            sheet_size: (64, 16),
            frame_count: 4,
        }
        .uv_rect(0);

        let vertices: [QuadVertex; 6] = build_quad_vertices(f0);

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
            texture_bind_group,
            uniform_buffer,
        }
    }

    pub fn update_projection(&self, mvp: glam::Mat4) {
        let uniforms = Uniforms {
            mvp: mvp.to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn draw(&mut self) {
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
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
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
