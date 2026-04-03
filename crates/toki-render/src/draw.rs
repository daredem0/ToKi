use toki_core::sprite::SpriteFrame;

use glam::Vec2;
use toki_core::graphics::vertex::QuadVertex;
pub fn build_quad_vertices(
    frame: SpriteFrame,
    width: f32,
    height: f32,
    origin: Vec2,
    flip_x: bool,
    tint_alpha: f32,
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
            tint_alpha,
        },
        QuadVertex {
            position: [ox + width, oy],
            tex_coords: [u1, frame.v0],
            tint_alpha,
        },
        QuadVertex {
            position: [ox + width, oy + height],
            tex_coords: [u1, frame.v1],
            tint_alpha,
        },
        QuadVertex {
            position: [ox, oy],
            tex_coords: [u0, frame.v0],
            tint_alpha,
        },
        QuadVertex {
            position: [ox + width, oy + height],
            tex_coords: [u1, frame.v1],
            tint_alpha,
        },
        QuadVertex {
            position: [ox, oy + height],
            tex_coords: [u0, frame.v1],
            tint_alpha,
        },
    ]
}

#[cfg(test)]
#[path = "draw_tests.rs"]
mod tests;
