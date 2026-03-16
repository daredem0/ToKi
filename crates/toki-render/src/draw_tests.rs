
use super::build_quad_vertices;
use glam::Vec2;
use toki_core::sprite::SpriteFrame;

#[test]
fn build_quad_vertices_maps_positions_and_uvs() {
    let frame = SpriteFrame {
        u0: 0.1,
        v0: 0.2,
        u1: 0.9,
        v1: 0.8,
    };
    let vertices = build_quad_vertices(frame, 32.0, 16.0, Vec2::new(5.0, 7.0), false);

    assert_eq!(vertices.len(), 6);
    assert_eq!(vertices[0].position, [5.0, 7.0]);
    assert_eq!(vertices[1].position, [37.0, 7.0]);
    assert_eq!(vertices[2].position, [37.0, 23.0]);
    assert_eq!(vertices[5].position, [5.0, 23.0]);

    assert_eq!(vertices[0].tex_coords, [0.1, 0.2]);
    assert_eq!(vertices[2].tex_coords, [0.9, 0.8]);
}

#[test]
fn build_quad_vertices_swaps_horizontal_uvs_when_flipped() {
    let frame = SpriteFrame {
        u0: 0.1,
        v0: 0.2,
        u1: 0.9,
        v1: 0.8,
    };
    let vertices = build_quad_vertices(frame, 32.0, 16.0, Vec2::new(5.0, 7.0), true);

    assert_eq!(vertices[0].tex_coords, [0.9, 0.2]);
    assert_eq!(vertices[1].tex_coords, [0.1, 0.2]);
    assert_eq!(vertices[2].tex_coords, [0.1, 0.8]);
    assert_eq!(vertices[5].tex_coords, [0.9, 0.8]);
}
