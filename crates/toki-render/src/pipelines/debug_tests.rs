use super::DebugPipeline;

#[test]
fn rect_outline_vertices_cover_all_corners_with_triangle_outline() {
    let color = [1.0, 0.0, 0.0, 1.0];
    let vertices = DebugPipeline::rect_outline_vertices(0.0, 0.0, 16.0, 16.0, color);

    // 4 edge quads * 2 triangles * 3 vertices
    assert_eq!(vertices.len(), 24);
    assert!(vertices.iter().any(|v| v.position == [0.0, 0.0]));
    assert!(vertices.iter().any(|v| v.position == [16.0, 0.0]));
    assert!(vertices.iter().any(|v| v.position == [16.0, 16.0]));
    assert!(vertices.iter().any(|v| v.position == [0.0, 16.0]));
}

#[test]
fn rect_outline_vertices_tiny_rect_falls_back_to_single_quad() {
    let color = [0.0, 1.0, 0.0, 1.0];
    let vertices = DebugPipeline::rect_outline_vertices(4.0, 8.0, 0.2, 0.2, color);
    assert_eq!(vertices.len(), 6);
    assert!(vertices.iter().any(|v| v.position == [4.0, 8.0]));
    assert!(vertices.iter().any(|v| v.position == [4.2, 8.2]));
}

#[test]
fn quad_vertices_for_fill_emits_two_triangles() {
    let vertices = DebugPipeline::quad_vertices(10.0, 12.0, 30.0, 20.0, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(vertices.len(), 6);
    assert_eq!(vertices[0].position, [10.0, 12.0]);
    assert_eq!(vertices[2].position, [30.0, 20.0]);
}

#[test]
fn line_vertices_emit_two_triangles_for_thick_segment() {
    let vertices = DebugPipeline::line_vertices(
        glam::Vec2::new(0.0, 0.0),
        glam::Vec2::new(10.0, 0.0),
        2.0,
        [0.0, 0.0, 1.0, 1.0],
    );
    assert_eq!(vertices.len(), 6);
    assert!(vertices.iter().any(|v| v.position == [0.0, 1.0]));
    assert!(vertices.iter().any(|v| v.position == [10.0, 1.0]));
    assert!(vertices.iter().any(|v| v.position == [10.0, -1.0]));
    assert!(vertices.iter().any(|v| v.position == [0.0, -1.0]));
}
