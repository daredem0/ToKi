use glam;

#[derive(Debug, Copy, Clone)]
pub struct ProjectionParameter {
    pub width: u32,
    pub height: u32,
    pub desired_width: u32,
    pub desired_height: u32,
}

pub fn screen_space_projection(width: f32, height: f32) -> glam::Mat4 {
    let width = width.max(1.0);
    let height = height.max(1.0);
    glam::Mat4::orthographic_rh_gl(0.0, width, height, 0.0, -1.0, 1.0)
}

pub fn calculate_projection(parameters: ProjectionParameter) -> glam::Mat4 {
    let aspect = parameters.width as f32 / parameters.height as f32;
    let desired_aspect = parameters.desired_width as f32 / parameters.desired_height as f32;

    let (view_width, view_height) = if aspect > desired_aspect {
        let height = parameters.desired_height as f32;
        let width = height * aspect;
        (width, height)
    } else {
        let width = parameters.desired_width as f32;
        let height = width / aspect;
        (width, height)
    };

    screen_space_projection(view_width, view_height)
}
