struct PostProcessUniforms {
    mode: u32,
    quantize_strategy: u32,
    _padding1: u32,
    _padding2: u32,
    tint_color: vec4<f32>,
    tint_strength: f32,
    gb_contrast: f32,
    brightness: f32,
    saturation: f32,
    vignette_strength: f32,
    _padding3: f32,
    _padding4: f32,
    _padding5: f32,
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

fn rgb_distance_sq(lhs: vec3<f32>, rhs: vec3<f32>) -> f32 {
    let delta = lhs - rhs;
    return dot(delta, delta);
}

fn nearest_palette_color(rgb: vec3<f32>, palette: array<vec4<f32>, 4>) -> vec3<f32> {
    var best = palette[0].rgb;
    var best_distance = rgb_distance_sq(rgb, best);
    for (var index = 1u; index < 4u; index = index + 1u) {
        let candidate = palette[index].rgb;
        let distance = rgb_distance_sq(rgb, candidate);
        if distance < best_distance {
            best = candidate;
            best_distance = distance;
        }
    }
    return best;
}

fn apply_contrast_rgb(rgb: vec3<f32>, contrast: f32) -> vec3<f32> {
    return vec3<f32>(
        apply_contrast(rgb.r, contrast),
        apply_contrast(rgb.g, contrast),
        apply_contrast(rgb.b, contrast),
    );
}

fn apply_brightness_saturation(rgb: vec3<f32>, brightness: f32, saturation: f32) -> vec3<f32> {
    let lum = luminance(rgb);
    let gray = vec3<f32>(lum, lum, lum);
    return clamp(mix(gray, rgb, saturation) + vec3<f32>(brightness), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn bayer4x4_threshold(pixel: vec2<u32>) -> f32 {
    let x = pixel.x % 4u;
    let y = pixel.y % 4u;
    if y == 0u {
        if x == 0u { return (0.0 + 0.5) / 16.0; }
        if x == 1u { return (8.0 + 0.5) / 16.0; }
        if x == 2u { return (2.0 + 0.5) / 16.0; }
        return (10.0 + 0.5) / 16.0;
    }
    if y == 1u {
        if x == 0u { return (12.0 + 0.5) / 16.0; }
        if x == 1u { return (4.0 + 0.5) / 16.0; }
        if x == 2u { return (14.0 + 0.5) / 16.0; }
        return (6.0 + 0.5) / 16.0;
    }
    if y == 2u {
        if x == 0u { return (3.0 + 0.5) / 16.0; }
        if x == 1u { return (11.0 + 0.5) / 16.0; }
        if x == 2u { return (1.0 + 0.5) / 16.0; }
        return (9.0 + 0.5) / 16.0;
    }
    if x == 0u { return (15.0 + 0.5) / 16.0; }
    if x == 1u { return (7.0 + 0.5) / 16.0; }
    if x == 2u { return (13.0 + 0.5) / 16.0; }
    return (5.0 + 0.5) / 16.0;
}

fn ordered_dither_index(value: f32, pixel: vec2<u32>) -> u32 {
    let threshold_bias = (bayer4x4_threshold(pixel) - 0.5) / 4.0;
    return quantize_index(clamp(value + threshold_bias, 0.0, 1.0));
}

fn apply_vignette(rgb: vec3<f32>, uv: vec2<f32>, strength: f32) -> vec3<f32> {
    let centered = uv * 2.0 - vec2<f32>(1.0, 1.0);
    let dist = length(centered);
    let edge = clamp((dist - 0.35) / (1.0 - 0.35), 0.0, 1.0);
    let edge_falloff = edge * edge * (3.0 - 2.0 * edge);
    let factor = 1.0 - edge_falloff * strength;
    return rgb * factor;
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
            let graded = apply_brightness_saturation(
                color.rgb,
                uniforms.brightness,
                uniforms.saturation,
            );
            return vec4<f32>(graded, color.a);
        }
        case 3u: {
            if uniforms.quantize_strategy == 0u {
                let index = quantize_index(luminance(color.rgb));
                let palette_color = uniforms.quantize_palette[index];
                return vec4<f32>(palette_color.rgb, color.a);
            }
            let palette_color = nearest_palette_color(color.rgb, uniforms.quantize_palette);
            return vec4<f32>(palette_color, color.a);
        }
        case 4u: {
            let pixel = vec2<u32>(u32(in.clip_position.x), u32(in.clip_position.y));
            let index = ordered_dither_index(luminance(color.rgb), pixel);
            return vec4<f32>(uniforms.quantize_palette[index].rgb, color.a);
        }
        case 5u: {
            let gb_palette = array<vec4<f32>, 4>(
                vec4<f32>(15.0 / 255.0, 56.0 / 255.0, 15.0 / 255.0, 1.0),
                vec4<f32>(48.0 / 255.0, 98.0 / 255.0, 48.0 / 255.0, 1.0),
                vec4<f32>(139.0 / 255.0, 172.0 / 255.0, 15.0 / 255.0, 1.0),
                vec4<f32>(155.0 / 255.0, 188.0 / 255.0, 15.0 / 255.0, 1.0),
            );
            if uniforms.quantize_strategy == 0u {
                let adjusted = apply_contrast(luminance(color.rgb), uniforms.gb_contrast);
                let index = quantize_index(adjusted);
                return vec4<f32>(gb_palette[index].rgb, color.a);
            }
            let adjusted = apply_contrast_rgb(color.rgb, uniforms.gb_contrast);
            let palette_color = nearest_palette_color(adjusted, gb_palette);
            return vec4<f32>(palette_color, color.a);
        }
        case 6u: {
            let graded = apply_vignette(color.rgb, in.uv, uniforms.vignette_strength);
            return vec4<f32>(graded, color.a);
        }
        default: {
            return color;
        }
    }
}
