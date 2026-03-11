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

#[cfg(test)]
mod tests {
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
        let vertices = build_quad_vertices(frame, 32.0, 16.0, Vec2::new(5.0, 7.0));

        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].position, [5.0, 7.0]);
        assert_eq!(vertices[1].position, [37.0, 7.0]);
        assert_eq!(vertices[2].position, [37.0, 23.0]);
        assert_eq!(vertices[5].position, [5.0, 23.0]);

        assert_eq!(vertices[0].tex_coords, [0.1, 0.2]);
        assert_eq!(vertices[2].tex_coords, [0.9, 0.8]);
    }
}
