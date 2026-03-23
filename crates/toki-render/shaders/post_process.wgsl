struct PostProcessUniforms {
    mode: u32,
    _padding0: u32,
    _padding1: u32,
    _padding2: u32,
    tint_color: vec4<f32>,
    tint_strength: f32,
    gb_contrast: f32,
    _padding3: f32,
    _padding4: f32,
    quantize_palette: array<vec4<f32>, 4>,
};

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

@group(0) @binding(2)
var<uniform> uniforms: PostProcessUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 3.0,  1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

fn luminance(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.299, 0.587, 0.114));
}

fn quantize_index(value: f32) -> u32 {
    if value < 0.25 {
        return 0u;
    }
    if value < 0.5 {
        return 1u;
    }
    if value < 0.75 {
        return 2u;
    }
    return 3u;
}

fn apply_contrast(value: f32, contrast: f32) -> f32 {
    return clamp((value - 0.5) * (1.0 + contrast) + 0.5, 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, source_sampler, in.uv);
    if color.a <= 0.0 {
        return color;
    }

    switch uniforms.mode {
        case 1u: {
            let tint = mix(color.rgb, uniforms.tint_color.rgb, uniforms.tint_strength);
            return vec4<f32>(tint, color.a);
        }
        case 2u: {
            let index = quantize_index(luminance(color.rgb));
            let palette_color = uniforms.quantize_palette[index];
            return vec4<f32>(palette_color.rgb, color.a);
        }
        case 3u: {
            let gb_palette = array<vec3<f32>, 4>(
                vec3<f32>(15.0 / 255.0, 56.0 / 255.0, 15.0 / 255.0),
                vec3<f32>(48.0 / 255.0, 98.0 / 255.0, 48.0 / 255.0),
                vec3<f32>(139.0 / 255.0, 172.0 / 255.0, 15.0 / 255.0),
                vec3<f32>(155.0 / 255.0, 188.0 / 255.0, 15.0 / 255.0),
            );
            let adjusted = apply_contrast(luminance(color.rgb), uniforms.gb_contrast);
            let index = quantize_index(adjusted);
            return vec4<f32>(gb_palette[index], color.a);
        }
        default: {
            return color;
        }
    }
}
