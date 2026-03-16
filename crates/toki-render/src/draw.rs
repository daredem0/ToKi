use toki_core::sprite::SpriteFrame;

use glam::Vec2;
use toki_core::graphics::vertex::QuadVertex;
pub fn build_quad_vertices(
    frame: SpriteFrame,
    width: f32,
    height: f32,
    origin: Vec2, // <— new
    flip_x: bool,
) -> [QuadVertex; 6] {
    let ox = origin.x;
    let oy = origin.y;
    let (u0, u1) = if flip_x {
        (frame.u1, frame.u0)
    } else {
        (frame.u0, frame.u1)
    };
    [
        QuadVertex {
            position: [ox, oy],
            tex_coords: [u0, frame.v0],
        },
        QuadVertex {
            position: [ox + width, oy],
            tex_coords: [u1, frame.v0],
        },
        QuadVertex {
            position: [ox + width, oy + height],
            tex_coords: [u1, frame.v1],
        },
        QuadVertex {
            position: [ox, oy],
            tex_coords: [u0, frame.v0],
        },
        QuadVertex {
            position: [ox + width, oy + height],
            tex_coords: [u1, frame.v1],
        },
        QuadVertex {
            position: [ox, oy + height],
            tex_coords: [u0, frame.v1],
        },
    ]
}

#[cfg(test)]
#[path = "draw_tests.rs"]
mod tests;
