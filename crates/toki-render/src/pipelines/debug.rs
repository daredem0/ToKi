use super::RenderPipeline;
use crate::vertex::VertexLayout;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{Device, Queue, RenderPass, RenderPipeline as WgpuRenderPipeline};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct DebugVertex {
    pub position: [f32; 2],
    pub color: [f32; 4], // RGBA
}

impl VertexLayout for DebugVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<DebugVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Color
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DebugUniforms {
    mvp: [[f32; 4]; 4],
}

#[derive(Debug)]
pub struct DebugPipeline {
    render_pipeline: WgpuRenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    vertex_buffer: Option<wgpu::Buffer>,
    vertices: Vec<DebugVertex>,
}

impl DebugPipeline {
    fn quad_vertices(x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) -> [DebugVertex; 6] {
        [
            DebugVertex {
                position: [x0, y0],
                color,
            },
            DebugVertex {
                position: [x1, y0],
                color,
            },
            DebugVertex {
                position: [x1, y1],
                color,
            },
            DebugVertex {
                position: [x0, y0],
                color,
            },
            DebugVertex {
                position: [x1, y1],
                color,
            },
            DebugVertex {
                position: [x0, y1],
                color,
            },
        ]
    }

    fn rect_outline_vertices(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: [f32; 4],
    ) -> Vec<DebugVertex> {
        if width <= 0.0 || height <= 0.0 {
            return Vec::new();
        }

        let thickness = 1.0_f32.min(width).min(height);
        let x0 = x;
        let y0 = y;
        let x1 = x + width;
        let y1 = y + height;

        // For very small rectangles, draw a filled quad to avoid inverted borders.
        if width <= thickness * 2.0 || height <= thickness * 2.0 {
            return Self::quad_vertices(x0, y0, x1, y1, color).to_vec();
        }

        let mut vertices = Vec::with_capacity(24);
        // Top edge
        vertices.extend_from_slice(&Self::quad_vertices(x0, y0, x1, y0 + thickness, color));
        // Bottom edge
        vertices.extend_from_slice(&Self::quad_vertices(x0, y1 - thickness, x1, y1, color));
        // Left edge (excluding corners covered by top/bottom)
        vertices.extend_from_slice(&Self::quad_vertices(
            x0,
            y0 + thickness,
            x0 + thickness,
            y1 - thickness,
            color,
        ));
        // Right edge (excluding corners covered by top/bottom)
        vertices.extend_from_slice(&Self::quad_vertices(
            x1 - thickness,
            y0 + thickness,
            x1,
            y1 - thickness,
            color,
        ));
        vertices
    }

    pub fn new(device: &Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Debug Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/debug.wgsl").into()),
        });

        let dummy_uniforms = DebugUniforms {
            mvp: glam::Mat4::IDENTITY.to_cols_array_2d(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Debug Uniform Buffer"),
            contents: bytemuck::cast_slice(&[dummy_uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Debug Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Debug Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Debug Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Debug Pipeline"),
            cache: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[DebugVertex::desc()],
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            render_pipeline,
            uniform_buffer,
            bind_group,
            vertex_buffer: None,
            vertices: Vec::new(),
        }
    }

    /// Clear all debug shapes
    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Add a rectangle outline to be rendered
    pub fn add_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        let vertices = Self::rect_outline_vertices(x, y, width, height, color);
        self.vertices.extend_from_slice(&vertices);
    }

    /// Add a filled rectangle to be rendered
    pub fn add_filled_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let vertices = Self::quad_vertices(x, y, x + width, y + height, color);
        self.vertices.extend_from_slice(&vertices);
    }

    /// Update MVP matrix for camera transformation
    pub fn update_camera(&self, queue: &Queue, mvp_matrix: glam::Mat4) {
        let uniforms = DebugUniforms {
            mvp: mvp_matrix.to_cols_array_2d(),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Update vertex buffer with current debug shapes
    pub fn update_vertices(&mut self, device: &Device) {
        if self.vertices.is_empty() {
            self.vertex_buffer = None;
            return;
        }

        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Debug Vertex Buffer"),
                contents: bytemuck::cast_slice(&self.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }
}

impl RenderPipeline for DebugPipeline {
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if let Some(vertex_buffer) = &self.vertex_buffer {
            if !self.vertices.is_empty() {
                render_pass.set_pipeline(&self.render_pipeline);
                render_pass.set_bind_group(0, &self.bind_group, &[]);
                render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                render_pass.draw(0..self.vertices.len() as u32, 0..1);
            }
        }
    }

    fn update(&mut self) {
        // Debug pipeline updates are handled externally via update_vertices
    }
}

#[cfg(test)]
mod tests {
    use super::DebugPipeline;

    #[test]
    fn rect_outline_vertices_cover_all_corners_with_triangle_outline() {
        let color = [1.0, 0.0, 0.0, 1.0];
        let vertices = DebugPipeline::rect_outline_vertices(0.0, 0.0, 16.0, 16.0, color);

        // 4 edge quads * 2 triangles * 3 vertices
        assert_eq!(vertices.len(), 24);
        assert!(vertices.iter().any(|v| v.position == [0.0, 0.0]));
        assert!(vertices.iter().any(|v| v.position == [16.0, 0.0]));
        assert!(vertices.iter().any(|v| v.position == [16.0, 16.0]));
        assert!(vertices.iter().any(|v| v.position == [0.0, 16.0]));
    }

    #[test]
    fn rect_outline_vertices_tiny_rect_falls_back_to_single_quad() {
        let color = [0.0, 1.0, 0.0, 1.0];
        let vertices = DebugPipeline::rect_outline_vertices(4.0, 8.0, 0.2, 0.2, color);
        assert_eq!(vertices.len(), 6);
        assert!(vertices.iter().any(|v| v.position == [4.0, 8.0]));
        assert!(vertices.iter().any(|v| v.position == [4.2, 8.2]));
    }

    #[test]
    fn quad_vertices_for_fill_emits_two_triangles() {
        let vertices = DebugPipeline::quad_vertices(10.0, 12.0, 30.0, 20.0, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].position, [10.0, 12.0]);
        assert_eq!(vertices[2].position, [30.0, 20.0]);
    }
}
