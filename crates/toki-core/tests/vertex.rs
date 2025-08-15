use toki_core::graphics::vertex::QuadVertex;

#[test]
fn quad_vertex_has_correct_defaults() {
    let v = QuadVertex {
        position: [1.0, 2.0],
        tex_coords: [0.5, 0.5],
    };
    assert_eq!(v.position, [1.0, 2.0]);
    assert_eq!(v.tex_coords, [0.5, 0.5]);
}

#[test]
fn quad_vertex_is_pod_and_zeroable() {
    use bytemuck::{Pod, Zeroable};
    fn assert_pod<T: Pod>() {}
    fn assert_zeroable<T: Zeroable>() {}

    assert_pod::<QuadVertex>();
    assert_zeroable::<QuadVertex>();
}
