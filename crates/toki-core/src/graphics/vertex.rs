use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub tint_alpha: f32,
}

impl QuadVertex {
    pub fn quad(position: [f32; 2], size: [f32; 2], tex_coords: [f32; 4]) -> [Self; 6] {
        let [x, y] = position;
        let [width, height] = size;
        let [u0, v0, u1, v1] = tex_coords;
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
