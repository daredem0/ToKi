use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub tint_alpha: f32,
}

impl QuadVertex {
    pub fn quad(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        u0: f32,
        v0: f32,
        u1: f32,
        v1: f32,
    ) -> [Self; 6] {
        [
            Self {
                position: [x, y],
                tex_coords: [u0, v0],
                tint_alpha: 0.0,
            },
            Self {
                position: [x + width, y],
                tex_coords: [u1, v0],
                tint_alpha: 0.0,
            },
            Self {
                position: [x, y + height],
                tex_coords: [u0, v1],
                tint_alpha: 0.0,
            },
            Self {
                position: [x + width, y],
                tex_coords: [u1, v0],
                tint_alpha: 0.0,
            },
            Self {
                position: [x + width, y + height],
                tex_coords: [u1, v1],
                tint_alpha: 0.0,
            },
            Self {
                position: [x, y + height],
                tex_coords: [u0, v1],
                tint_alpha: 0.0,
            },
        ]
    }
}
