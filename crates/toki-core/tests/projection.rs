use toki_core::math::projection::screen_space_projection;

#[test]
fn screen_space_projection_maps_viewport_corners_to_clip_space() {
    let projection = screen_space_projection(320.0, 180.0);

    let top_left = projection * glam::Vec4::new(0.0, 0.0, 0.0, 1.0);
    let bottom_right = projection * glam::Vec4::new(320.0, 180.0, 0.0, 1.0);
    let center = projection * glam::Vec4::new(160.0, 90.0, 0.0, 1.0);

    assert_eq!(top_left.truncate(), glam::Vec3::new(-1.0, 1.0, 0.0));
    assert_eq!(bottom_right.truncate(), glam::Vec3::new(1.0, -1.0, 0.0));
    assert_eq!(center.truncate(), glam::Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(top_left.w, 1.0);
    assert_eq!(bottom_right.w, 1.0);
}
