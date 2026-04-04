use bytemuck::{Pod, Zeroable};
use toki_core::palette::{Palette, PaletteSize};
use toki_core::project_runtime::{PostProcessMode, QuantizeStrategy, ResolvedPostProcessSettings};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct PostProcessUniforms {
    mode: u32,
    quantize_strategy: u32,
    _padding1: u32,
    _padding2: u32,
    tint_color: [f32; 4],
    tint_strength: f32,
    gb_contrast: f32,
    brightness: f32,
    saturation: f32,
    vignette_strength: f32,
    _padding3: f32,
    _padding4: f32,
    _padding5: f32,
    quantize_palette: [[f32; 4]; 4],
}

fn shader_mode(mode: PostProcessMode) -> u32 {
    match mode {
        PostProcessMode::None => 0,
        PostProcessMode::Tint => 1,
        PostProcessMode::BrightnessSaturation => 2,
        PostProcessMode::Quantize4 => 3,
        PostProcessMode::OrderedDitherQuantize => 4,
        PostProcessMode::GbPalette => 5,
        PostProcessMode::Vignette => 6,
    }
}

fn shader_quantize_strategy(strategy: QuantizeStrategy) -> u32 {
    match strategy {
        QuantizeStrategy::Luminance => 0,
        QuantizeStrategy::RgbDistance => 1,
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

fn palette_to_uniform(palette: &Palette) -> [[f32; 4]; 4] {
    let colors = palette.colors();
    [
        color_to_vec4(colors[0]),
        color_to_vec4(colors[1]),
        color_to_vec4(colors[2]),
        color_to_vec4(colors[3]),
    ]
}

fn build_uniforms(settings: &ResolvedPostProcessSettings) -> PostProcessUniforms {
    PostProcessUniforms {
        mode: shader_mode(settings.mode),
        quantize_strategy: shader_quantize_strategy(settings.quantize_strategy),
        _padding1: 0,
        _padding2: 0,
        tint_color: color_to_vec4(settings.tint_color),
        tint_strength: settings.tint_strength_percent.min(100) as f32 / 100.0,
        gb_contrast: settings.gb_contrast_percent.clamp(-100, 100) as f32 / 100.0,
        brightness: settings.brightness_percent.clamp(-100, 100) as f32 / 100.0,
        saturation: settings.saturation_percent.min(200) as f32 / 100.0,
        vignette_strength: settings.vignette_strength_percent.min(100) as f32 / 100.0,
        _padding3: 0.0,
        _padding4: 0.0,
        _padding5: 0.0,
        quantize_palette: palette_to_uniform(&settings.quantize_palette),
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
fn rgb_distance_sq(lhs: [f32; 3], rhs: [f32; 3]) -> f32 {
    let dr = lhs[0] - rhs[0];
    let dg = lhs[1] - rhs[1];
    let db = lhs[2] - rhs[2];
    dr * dr + dg * dg + db * db
}

#[cfg(test)]
fn nearest_palette_color(rgb: [f32; 3], palette: &Palette) -> [u8; 4] {
    let mut best = palette.color(0);
    let mut best_distance = f32::MAX;
    for &color in palette.colors() {
        let candidate_rgb = [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
        ];
        let distance = rgb_distance_sq(rgb, candidate_rgb);
        if distance < best_distance {
            best_distance = distance;
            best = color;
        }
    }
    best
}

#[cfg(test)]
fn apply_contrast_rgb(rgb: [f32; 3], contrast: f32) -> [f32; 3] {
    [
        apply_contrast(rgb[0], contrast),
        apply_contrast(rgb[1], contrast),
        apply_contrast(rgb[2], contrast),
    ]
}

#[cfg(test)]
fn apply_brightness_saturation(rgb: [f32; 3], brightness: f32, saturation: f32) -> [f32; 3] {
    let lum = luminance(rgb);
    let gray = [lum, lum, lum];
    [
        (gray[0] * (1.0 - saturation) + rgb[0] * saturation + brightness).clamp(0.0, 1.0),
        (gray[1] * (1.0 - saturation) + rgb[1] * saturation + brightness).clamp(0.0, 1.0),
        (gray[2] * (1.0 - saturation) + rgb[2] * saturation + brightness).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
fn bayer4x4_threshold(pixel: [u32; 2]) -> f32 {
    const BAYER4X4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    let x = (pixel[0] % 4) as usize;
    let y = (pixel[1] % 4) as usize;
    (BAYER4X4[y][x] as f32 + 0.5) / 16.0
}

#[cfg(test)]
fn ordered_dither_quantize_index(lum: f32, pixel: [u32; 2]) -> usize {
    let threshold_bias = (bayer4x4_threshold(pixel) - 0.5) / 4.0;
    quantize_index((lum + threshold_bias).clamp(0.0, 1.0))
}

#[cfg(test)]
fn apply_vignette(rgb: [f32; 3], uv: [f32; 2], strength: f32) -> [f32; 3] {
    let dx = uv[0] * 2.0 - 1.0;
    let dy = uv[1] * 2.0 - 1.0;
    let dist = (dx * dx + dy * dy).sqrt();
    let edge = ((dist - 0.35) / (1.0 - 0.35)).clamp(0.0, 1.0);
    let smooth = edge * edge * (3.0 - 2.0 * edge);
    let vignette = 1.0 - smooth * strength;
    [rgb[0] * vignette, rgb[1] * vignette, rgb[2] * vignette]
}

#[cfg(test)]
pub(crate) fn apply_post_process_pixel(
    settings: &ResolvedPostProcessSettings,
    color: [u8; 4],
) -> [u8; 4] {
    apply_post_process_pixel_at(settings, color, [0, 0], [0.5, 0.5])
}

#[cfg(test)]
pub(crate) fn apply_post_process_pixel_at(
    settings: &ResolvedPostProcessSettings,
    color: [u8; 4],
    pixel: [u32; 2],
    uv: [f32; 2],
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
        PostProcessMode::BrightnessSaturation => {
            let out = apply_brightness_saturation(
                rgb,
                settings.brightness_percent.clamp(-100, 100) as f32 / 100.0,
                settings.saturation_percent.min(200) as f32 / 100.0,
            );
            [
                (out[0] * 255.0).round() as u8,
                (out[1] * 255.0).round() as u8,
                (out[2] * 255.0).round() as u8,
                alpha,
            ]
        }
        PostProcessMode::Quantize4 => {
            let target = match settings.quantize_strategy {
                QuantizeStrategy::Luminance => {
                    let index = quantize_index(luminance(rgb));
                    settings.quantize_palette.color(index)
                }
                QuantizeStrategy::RgbDistance => {
                    nearest_palette_color(rgb, &settings.quantize_palette)
                }
            };
            [target[0], target[1], target[2], alpha]
        }
        PostProcessMode::OrderedDitherQuantize => {
            let index = ordered_dither_quantize_index(luminance(rgb), pixel);
            let target = settings.quantize_palette.color(index);
            [target[0], target[1], target[2], alpha]
        }
        PostProcessMode::GbPalette => {
            let gb_palette = Palette::new(
                PaletteSize::Pal4,
                vec![
                    [0x0F, 0x38, 0x0F, 0xFF],
                    [0x30, 0x62, 0x30, 0xFF],
                    [0x8B, 0xAC, 0x0F, 0xFF],
                    [0x9B, 0xBC, 0x0F, 0xFF],
                ],
            )
            .expect("hard-coded GB palette");
            let contrast = settings.gb_contrast_percent.clamp(-100, 100) as f32 / 100.0;
            let target = match settings.quantize_strategy {
                QuantizeStrategy::Luminance => {
                    let lum = apply_contrast(luminance(rgb), contrast);
                    let index = quantize_index(lum);
                    gb_palette.color(index)
                }
                QuantizeStrategy::RgbDistance => {
                    let adjusted = apply_contrast_rgb(rgb, contrast);
                    nearest_palette_color(adjusted, &gb_palette)
                }
            };
            [target[0], target[1], target[2], alpha]
        }
        PostProcessMode::Vignette => {
            let out = apply_vignette(
                rgb,
                uv,
                settings.vignette_strength_percent.min(100) as f32 / 100.0,
            );
            [
                (out[0] * 255.0).round() as u8,
                (out[1] * 255.0).round() as u8,
                (out[2] * 255.0).round() as u8,
                alpha,
            ]
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
            contents: bytemuck::cast_slice(&[build_uniforms(&ResolvedPostProcessSettings {
                mode: PostProcessMode::None,
                quantize_strategy: QuantizeStrategy::Luminance,
                tint_color: [0, 0, 0, 255],
                tint_strength_percent: 0,
                brightness_percent: 0,
                saturation_percent: 100,
                quantize_palette: Palette::new(
                    PaletteSize::Pal4,
                    vec![
                        [0, 0, 0, 255],
                        [85, 85, 85, 255],
                        [170, 170, 170, 255],
                        [255, 255, 255, 255],
                    ],
                )
                .expect("hard-coded default palette"),
                gb_contrast_percent: 0,
                vignette_strength_percent: 60,
            })]),
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

    pub fn update_settings(&mut self, queue: &wgpu::Queue, settings: &ResolvedPostProcessSettings) {
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
    use super::{
        apply_post_process_pixel, apply_post_process_pixel_at, quantize_index, PostProcessUniforms,
    };
    use toki_core::palette::{Palette, PaletteSize};
    use toki_core::project_runtime::{
        PostProcessMode, QuantizeStrategy, ResolvedPostProcessSettings,
    };

    fn settings(mode: PostProcessMode) -> ResolvedPostProcessSettings {
        ResolvedPostProcessSettings {
            mode,
            quantize_strategy: QuantizeStrategy::Luminance,
            tint_color: [40, 80, 160, 255],
            tint_strength_percent: 50,
            brightness_percent: 0,
            saturation_percent: 100,
            quantize_palette: Palette::new(
                PaletteSize::Pal4,
                vec![
                    [10, 10, 10, 255],
                    [80, 80, 80, 255],
                    [160, 160, 160, 255],
                    [240, 240, 240, 255],
                ],
            )
            .expect("test palette"),
            gb_contrast_percent: 10,
            vignette_strength_percent: 60,
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
        let output =
            apply_post_process_pixel(&settings(PostProcessMode::Tint), [100, 100, 100, 255]);
        assert!(output[2] > output[0]);
        assert_eq!(output[3], 255);
    }

    #[test]
    fn quantize_post_process_maps_to_palette_colors() {
        let output =
            apply_post_process_pixel(&settings(PostProcessMode::Quantize4), [220, 220, 220, 255]);
        assert_eq!(output, [240, 240, 240, 255]);
    }

    #[test]
    fn brightness_saturation_mode_adjusts_color_grade() {
        let mut post = settings(PostProcessMode::BrightnessSaturation);
        post.brightness_percent = 20;
        post.saturation_percent = 150;
        let output = apply_post_process_pixel(&post, [120, 90, 60, 255]);
        assert!(output[0] > 120);
        assert!(output[0] > output[1]);
        assert!(output[1] > output[2]);
    }

    #[test]
    fn ordered_dither_quantize_uses_pixel_position() {
        let color = [125, 125, 125, 255];
        let low = apply_post_process_pixel_at(
            &settings(PostProcessMode::OrderedDitherQuantize),
            color,
            [0, 0],
            [0.2, 0.2],
        );
        let high = apply_post_process_pixel_at(
            &settings(PostProcessMode::OrderedDitherQuantize),
            color,
            [0, 1],
            [0.2, 0.2],
        );
        assert_ne!(low, high);
    }

    #[test]
    fn gb_post_process_preserves_alpha() {
        let output =
            apply_post_process_pixel(&settings(PostProcessMode::GbPalette), [150, 100, 60, 120]);
        assert_eq!(output[3], 120);
    }

    #[test]
    fn vignette_darkens_edges_more_than_center() {
        let center = apply_post_process_pixel_at(
            &settings(PostProcessMode::Vignette),
            [180, 180, 180, 255],
            [0, 0],
            [0.5, 0.5],
        );
        let edge = apply_post_process_pixel_at(
            &settings(PostProcessMode::Vignette),
            [180, 180, 180, 255],
            [0, 0],
            [0.0, 0.0],
        );
        assert!(edge[0] < center[0]);
    }

    #[test]
    fn quantize_rgb_distance_keeps_exact_palette_color_stable() {
        let mut post = settings(PostProcessMode::Quantize4);
        post.quantize_strategy = QuantizeStrategy::RgbDistance;
        let color = post.quantize_palette.color(2);
        let output = apply_post_process_pixel(&post, color);
        assert_eq!(output, color);
    }

    #[test]
    fn gb_rgb_distance_keeps_exact_gb_palette_color_stable() {
        let mut post = settings(PostProcessMode::GbPalette);
        post.quantize_strategy = QuantizeStrategy::RgbDistance;
        let color = [0x8B, 0xAC, 0x0F, 0xFF];
        let output = apply_post_process_pixel(&post, color);
        assert_eq!(output, color);
    }

    #[test]
    fn uniform_layout_matches_expected_128_byte_size() {
        assert_eq!(std::mem::size_of::<PostProcessUniforms>(), 128);
    }
}
