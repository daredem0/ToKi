struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
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
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, input.tex_coords);
}
