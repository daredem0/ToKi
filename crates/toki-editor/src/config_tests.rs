use super::{EditorConfig, GridSettings};

#[test]
fn grid_settings_deserializes_legacy_scalar_grid_size() {
    let json = r#"{
          "project_path": null,
          "editor_settings": {
            "window_size": [1200, 800],
            "panels": {
              "hierarchy_visible": true,
              "inspector_visible": true,
              "console_visible": true
            },
            "grid": {
              "show_grid": true,
              "grid_size": 16,
              "snap_to_grid": true
            },
            "camera": {
              "pan_speed": 1.0,
              "zoom_speed": 1.0
            }
          },
          "recent_projects": [],
          "rendering": {
            "vsync": true,
            "target_fps": 60,
            "show_collision_boxes": true,
            "show_debug_info": false
          },
          "log_to_terminal": true,
          "log_level": "INFO"
        }"#;

    let config: EditorConfig =
        serde_json::from_str(json).expect("legacy scalar grid config should parse");
    assert_eq!(config.editor_settings.grid.grid_size, [16, 16]);
}

#[test]
fn grid_settings_serializes_as_2d_grid_size() {
    let mut config = EditorConfig::default();
    config.editor_settings.grid = GridSettings {
        show_grid: true,
        grid_size: [8, 12],
        snap_to_grid: false,
    };

    let json = serde_json::to_string(&config).expect("editor config should serialize to json");
    assert!(json.contains("\"grid_size\":[8,12]"));
}
