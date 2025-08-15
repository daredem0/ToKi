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

pub fn calculate_projection(size: winit::dpi::PhysicalSize<u32>) -> glam::Mat4 {
    let aspect = size.width as f32 / size.height as f32;
    let desired_aspect = 160.0 / 144.0;

    let (view_width, view_height) = if aspect > desired_aspect {
        let height = 144.0;
        let width = height * aspect;
        (width, height)
    } else {
        let width = 160.0;
        let height = width / aspect;
        (width, height)
    };

    glam::Mat4::orthographic_rh_gl(0.0, view_width, view_height, 0.0, -1.0, 1.0)
}
