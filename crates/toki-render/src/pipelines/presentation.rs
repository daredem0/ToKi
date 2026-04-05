pub struct PresentationBlitPipeline {
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
}

impl PresentationBlitPipeline {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Presentation Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/presentation_blit.wgsl").into(),
            ),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Presentation Blit Bind Group Layout"),
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Presentation Blit Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Presentation Blit Pipeline"),
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

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Presentation Blit Sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            render_pipeline,
            bind_group_layout,
            sampler,
            bind_group: None,
        }
    }

    pub fn update_source_texture(
        &mut self,
        device: &wgpu::Device,
        source_view: &wgpu::TextureView,
    ) {
        self.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Presentation Blit Bind Group"),
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
fn gamma_from_linear_component(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
fn apply_presentation_blit_pixel(linear_rgba: [f32; 4]) -> [u8; 4] {
    let linear_rgb = [
        linear_rgba[0].clamp(0.0, 1.0),
        linear_rgba[1].clamp(0.0, 1.0),
        linear_rgba[2].clamp(0.0, 1.0),
    ];
    let alpha = linear_rgba[3].clamp(0.0, 1.0);
    [
        (gamma_from_linear_component(linear_rgb[0]) * 255.0).round() as u8,
        (gamma_from_linear_component(linear_rgb[1]) * 255.0).round() as u8,
        (gamma_from_linear_component(linear_rgb[2]) * 255.0).round() as u8,
        (alpha * 255.0).round() as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::{apply_presentation_blit_pixel, gamma_from_linear_component};

    #[test]
    fn gamma_encode_matches_srgb_transfer_curve() {
        assert_eq!(gamma_from_linear_component(0.0), 0.0);
        assert!((gamma_from_linear_component(0.003_130_8) - 0.040_449_936).abs() < 0.000_001);
        assert!((gamma_from_linear_component(0.5) - 0.735_356_9).abs() < 0.000_01);
        assert!((gamma_from_linear_component(1.0) - 1.0).abs() < 0.000_001);
    }

    #[test]
    fn presentation_blit_gamma_encodes_rgb_but_preserves_alpha() {
        assert_eq!(apply_presentation_blit_pixel([0.5, 0.25, 1.0, 0.5]), [188, 137, 255, 128]);
    }
}
