use std::io;
use toki_core::CoreError;
use toki_render::RenderError;

#[test]
fn render_error_from_core_error() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let core_error = CoreError::Io(io_error);
    let render_error = RenderError::from(core_error);

    match render_error {
        RenderError::Core(_) => {} // Expected
        _ => panic!("Expected Core variant"),
    }
}

#[test]
fn render_error_unknown_variant() {
    let error = RenderError::Unknown;
    assert_eq!(error.to_string(), "Unknown render error");
}

#[test]
fn render_error_debug_formatting() {
    let error = RenderError::Unknown;
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("Unknown"));
}

#[test]
fn render_error_display_formatting() {
    let error = RenderError::Unknown;
    let display_str = format!("{}", error);
    assert_eq!(display_str, "Unknown render error");
}

#[test]
fn render_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<RenderError>();
}

#[test]
fn render_error_implements_std_error() {
    let error = RenderError::Unknown;
    let _: &dyn std::error::Error = &error;
}
