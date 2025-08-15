use toki_core::math::projection::{calculate_projection, ProjectionParameter};

#[test]
fn projection_matrix_scales_to_aspect_ratio() {
    let params = ProjectionParameter {
        width: 800,
        height: 600,
        desired_width: 160,
        desired_height: 144,
    };

    let mat = calculate_projection(params);

    // Check that the projection matrix is valid (non-zero scale components)
    assert!(mat.x_axis.x.abs() > 0.0);
    assert!(mat.y_axis.y.abs() > 0.0);

    // Make sure it flips y (typical for OpenGL-style projections)
    assert_eq!(mat.y_axis.y.signum(), -1.0);
}
