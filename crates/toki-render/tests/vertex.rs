use toki_core::graphics::vertex::QuadVertex;
use toki_render::VertexLayout;

#[test]
fn quad_vertex_layout_has_correct_stride() {
    let layout = QuadVertex::desc();

    // QuadVertex has 2 f32 values for position and 2 f32 values for tex_coords
    // Total: 4 * f32 = 4 * 4 bytes = 16 bytes
    assert_eq!(layout.array_stride, 16);
}

#[test]
fn quad_vertex_layout_uses_vertex_step_mode() {
    let layout = QuadVertex::desc();
    assert!(matches!(layout.step_mode, wgpu::VertexStepMode::Vertex));
}

#[test]
fn quad_vertex_layout_has_two_attributes() {
    let layout = QuadVertex::desc();
    assert_eq!(layout.attributes.len(), 2);
}

#[test]
fn quad_vertex_layout_position_attribute() {
    let layout = QuadVertex::desc();
    let position_attr = &layout.attributes[0];

    assert_eq!(position_attr.offset, 0);
    assert_eq!(position_attr.shader_location, 0);
    assert!(matches!(
        position_attr.format,
        wgpu::VertexFormat::Float32x2
    ));
}

#[test]
fn quad_vertex_layout_texcoord_attribute() {
    let layout = QuadVertex::desc();
    let texcoord_attr = &layout.attributes[1];

    // tex_coords comes after position (2 f32s = 8 bytes)
    assert_eq!(texcoord_attr.offset, 8);
    assert_eq!(texcoord_attr.shader_location, 1);
    assert!(matches!(
        texcoord_attr.format,
        wgpu::VertexFormat::Float32x2
    ));
}

#[test]
fn quad_vertex_layout_consistent_stride() {
    let layout = QuadVertex::desc();

    // Verify that array_stride matches the actual size of QuadVertex
    let expected_stride = std::mem::size_of::<QuadVertex>();
    assert_eq!(layout.array_stride as usize, expected_stride);
}

#[test]
fn quad_vertex_layout_offsets_are_sequential() {
    let layout = QuadVertex::desc();

    assert_eq!(layout.attributes[0].offset, 0);
    assert_eq!(layout.attributes[1].offset, 8); // After 2 f32s (position)
}

#[test]
fn vertex_layout_trait_implemented() {
    // This test ensures the trait is properly implemented
    let _layout = <QuadVertex as VertexLayout>::desc();
}
