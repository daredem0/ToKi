@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

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

fn gamma_from_linear_component(value: f32) -> f32 {
    if value <= 0.0031308 {
        return value * 12.92;
    }
    return 1.055 * pow(value, 1.0 / 2.4) - 0.055;
}

fn gamma_from_linear_rgb(rgb: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        gamma_from_linear_component(clamp(rgb.r, 0.0, 1.0)),
        gamma_from_linear_component(clamp(rgb.g, 0.0, 1.0)),
        gamma_from_linear_component(clamp(rgb.b, 0.0, 1.0)),
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color_linear = textureSample(source_texture, source_sampler, in.uv);
    let color_gamma = gamma_from_linear_rgb(color_linear.rgb);
    return vec4<f32>(color_gamma, color_linear.a);
}
