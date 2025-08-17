use toki_core::sprite::SpriteFrame;

use glam::Vec2;
use toki_core::graphics::vertex::QuadVertex;
pub fn build_quad_vertices(
    frame: SpriteFrame,
    width: f32,
    height: f32,
    origin: Vec2, // <— new
) -> [QuadVertex; 6] {
    let ox = origin.x;
    let oy = origin.y;
    [
        QuadVertex {
            position: [ox, oy],
            tex_coords: [frame.u0, frame.v0],
        },
        QuadVertex {
            position: [ox + width, oy],
            tex_coords: [frame.u1, frame.v0],
        },
        QuadVertex {
            position: [ox + width, oy + height],
            tex_coords: [frame.u1, frame.v1],
        },
        QuadVertex {
            position: [ox, oy],
            tex_coords: [frame.u0, frame.v0],
        },
        QuadVertex {
            position: [ox + width, oy + height],
            tex_coords: [frame.u1, frame.v1],
        },
        QuadVertex {
            position: [ox, oy + height],
            tex_coords: [frame.u0, frame.v1],
        },
    ]
}
