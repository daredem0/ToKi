use toki_core::sprite::SpriteFrame;

use toki_core::graphics::vertex::QuadVertex;

pub fn build_quad_vertices(frame: SpriteFrame) -> [QuadVertex; 6] {
    [
        // tri 1
        QuadVertex {
            position: [0.0, 0.0],
            tex_coords: [frame.u0, frame.v0],
        },
        QuadVertex {
            position: [16.0, 0.0],
            tex_coords: [frame.u1, frame.v0],
        },
        QuadVertex {
            position: [16.0, 16.0],
            tex_coords: [frame.u1, frame.v1],
        },
        // tri 2
        QuadVertex {
            position: [0.0, 0.0],
            tex_coords: [frame.u0, frame.v0],
        },
        QuadVertex {
            position: [16.0, 16.0],
            tex_coords: [frame.u1, frame.v1],
        },
        QuadVertex {
            position: [0.0, 16.0],
            tex_coords: [frame.u0, frame.v1],
        },
    ]
}
