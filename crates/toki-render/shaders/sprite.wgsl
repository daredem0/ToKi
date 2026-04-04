struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) tint_alpha: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) tint_alpha: f32,
};

struct Uniforms {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var sprite_texture: texture_2d<f32>;

@group(0) @binding(1)
var sprite_sampler: sampler;

@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let world_pos = vec4<f32>(input.position, 0.0, 1.0);
    output.position = uniforms.mvp * world_pos;
    output.tex_coords = input.tex_coords;
    output.tint_alpha = input.tint_alpha;
    return output;
}

/// Sample the average alpha of the 3x3 neighbourhood around `uv`.
/// This softens the silhouette edges for drop shadows.
fn soft_shadow_alpha(uv: vec2<f32>, tint_alpha: f32) -> f32 {
    let tex_size = vec2<f32>(textureDimensions(sprite_texture, 0));
    let texel = 1.0 / tex_size;

    var total = 0.0;
    for (var dy = -1; dy <= 1; dy = dy + 1) {
        for (var dx = -1; dx <= 1; dx = dx + 1) {
            let offset = vec2<f32>(f32(dx), f32(dy)) * texel;
            let sample = textureSample(sprite_texture, sprite_sampler, uv + offset);
            total = total + sample.a;
        }
    }
    let avg_alpha = total / 9.0;
    return avg_alpha * tint_alpha;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(sprite_texture, sprite_sampler, input.tex_coords);
    if input.tint_alpha > 0.0 {
        let alpha = soft_shadow_alpha(input.tex_coords, input.tint_alpha);
        return vec4<f32>(0.0, 0.0, 0.0, alpha);
    }
    return tex;
}
