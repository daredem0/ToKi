use bytemuck::{Pod, Zeroable};
use toki_core::palette::Palette4;
use toki_core::project_runtime::{PostProcessMode, ResolvedPostProcessSettings};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct PostProcessUniforms {
    mode: u32,
    _padding0: u32,
    _padding1: u32,
    _padding2: u32,
    tint_color: [f32; 4],
    tint_strength: f32,
    gb_contrast: f32,
    _padding3: f32,
    _padding4: f32,
    quantize_palette: [[f32; 4]; 4],
}

fn shader_mode(mode: PostProcessMode) -> u32 {
    match mode {
        PostProcessMode::None => 0,
        PostProcessMode::Tint => 1,
        PostProcessMode::Quantize4 => 2,
        PostProcessMode::GbPalette => 3,
    }
}

fn color_to_vec4(color: [u8; 4]) -> [f32; 4] {
    [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        color[3] as f32 / 255.0,
    ]
}

fn palette_to_uniform(palette: Palette4) -> [[f32; 4]; 4] {
    palette.colors.map(color_to_vec4)
}

fn build_uniforms(settings: ResolvedPostProcessSettings) -> PostProcessUniforms {
    PostProcessUniforms {
        mode: shader_mode(settings.mode),
        _padding0: 0,
        _padding1: 0,
        _padding2: 0,
        tint_color: color_to_vec4(settings.tint_color),
        tint_strength: settings.tint_strength_percent.min(100) as f32 / 100.0,
        gb_contrast: settings.gb_contrast_percent.clamp(-100, 100) as f32 / 100.0,
        _padding3: 0.0,
        _padding4: 0.0,
        quantize_palette: palette_to_uniform(settings.quantize_palette),
    }
}

#[cfg(test)]
fn apply_contrast(value: f32, contrast: f32) -> f32 {
    ((value - 0.5) * (1.0 + contrast) + 0.5).clamp(0.0, 1.0)
}

#[cfg(test)]
fn luminance(rgb: [f32; 3]) -> f32 {
    rgb[0] * 0.299 + rgb[1] * 0.587 + rgb[2] * 0.114
}

#[cfg(test)]
fn quantize_index(luminance: f32) -> usize {
    if luminance < 0.25 {
        0
    } else if luminance < 0.5 {
        1
    } else if luminance < 0.75 {
        2
    } else {
        3
    }
}

#[cfg(test)]
pub(crate) fn apply_post_process_pixel(
    settings: ResolvedPostProcessSettings,
    color: [u8; 4],
) -> [u8; 4] {
    if color[3] == 0 {
        return color;
    }

    let alpha = color[3];
    let rgb = [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
    ];

    match settings.mode {
        PostProcessMode::None => color,
        PostProcessMode::Tint => {
            let tint = color_to_vec4(settings.tint_color);
            let strength = settings.tint_strength_percent.min(100) as f32 / 100.0;
            let out = [
                rgb[0] * (1.0 - strength) + tint[0] * strength,
                rgb[1] * (1.0 - strength) + tint[1] * strength,
                rgb[2] * (1.0 - strength) + tint[2] * strength,
            ];
            [
                (out[0] * 255.0).round() as u8,
                (out[1] * 255.0).round() as u8,
                (out[2] * 255.0).round() as u8,
                alpha,
            ]
        }
        PostProcessMode::Quantize4 => {
            let index = quantize_index(luminance(rgb));
            let target = settings.quantize_palette.colors[index];
            [target[0], target[1], target[2], alpha]
        }
        PostProcessMode::GbPalette => {
            let gb_palette = Palette4::new([
                [0x0F, 0x38, 0x0F, 0xFF],
                [0x30, 0x62, 0x30, 0xFF],
                [0x8B, 0xAC, 0x0F, 0xFF],
                [0x9B, 0xBC, 0x0F, 0xFF],
            ]);
            let contrast = settings.gb_contrast_percent.clamp(-100, 100) as f32 / 100.0;
            let lum = apply_contrast(luminance(rgb), contrast);
            let index = quantize_index(lum);
            let target = gb_palette.colors[index];
            [target[0], target[1], target[2], alpha]
        }
    }
}

pub struct PostProcessPipeline {
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    bind_group: Option<wgpu::BindGroup>,
}

impl PostProcessPipeline {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Post Process Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/post_process.wgsl").into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Post Process Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Post Process Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Post Process Pipeline"),
            cache: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
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

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Post Process Uniform Buffer"),
            contents: bytemuck::cast_slice(&[build_uniforms(
                ResolvedPostProcessSettings {
                    mode: PostProcessMode::None,
                    tint_color: [0, 0, 0, 255],
                    tint_strength_percent: 0,
                    quantize_palette: Palette4::new([
                        [0, 0, 0, 255],
                        [85, 85, 85, 255],
                        [170, 170, 170, 255],
                        [255, 255, 255, 255],
                    ]),
                    gb_contrast_percent: 0,
                },
            )]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Post Process Sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            render_pipeline,
            bind_group_layout,
            sampler,
            uniform_buffer,
            bind_group: None,
        }
    }

    pub fn update_settings(
        &mut self,
        queue: &wgpu::Queue,
        settings: ResolvedPostProcessSettings,
    ) {
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[build_uniforms(settings)]),
        );
    }

    pub fn update_source_texture(
        &mut self,
        device: &wgpu::Device,
        source_view: &wgpu::TextureView,
    ) {
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Process Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
            ],
        }));
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        let Some(bind_group) = &self.bind_group else {
            return;
        };
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_post_process_pixel, quantize_index, PostProcessUniforms};
    use toki_core::palette::Palette4;
    use toki_core::project_runtime::{PostProcessMode, ResolvedPostProcessSettings};

    fn settings(mode: PostProcessMode) -> ResolvedPostProcessSettings {
        ResolvedPostProcessSettings {
            mode,
            tint_color: [40, 80, 160, 255],
            tint_strength_percent: 50,
            quantize_palette: Palette4::new([
                [10, 10, 10, 255],
                [80, 80, 80, 255],
                [160, 160, 160, 255],
                [240, 240, 240, 255],
            ]),
            gb_contrast_percent: 10,
        }
    }

    #[test]
    fn quantize_index_uses_four_bands() {
        assert_eq!(quantize_index(0.0), 0);
        assert_eq!(quantize_index(0.3), 1);
        assert_eq!(quantize_index(0.6), 2);
        assert_eq!(quantize_index(0.9), 3);
    }

    #[test]
    fn tint_post_process_blends_toward_tint_color() {
        let output = apply_post_process_pixel(settings(PostProcessMode::Tint), [100, 100, 100, 255]);
        assert!(output[2] > output[0]);
        assert_eq!(output[3], 255);
    }

    #[test]
    fn quantize_post_process_maps_to_palette_colors() {
        let output =
            apply_post_process_pixel(settings(PostProcessMode::Quantize4), [220, 220, 220, 255]);
        assert_eq!(output, [240, 240, 240, 255]);
    }

    #[test]
    fn gb_post_process_preserves_alpha() {
        let output =
            apply_post_process_pixel(settings(PostProcessMode::GbPalette), [150, 100, 60, 120]);
        assert_eq!(output[3], 120);
    }

    #[test]
    fn uniform_layout_matches_expected_112_byte_size() {
        assert_eq!(std::mem::size_of::<PostProcessUniforms>(), 112);
    }
}
