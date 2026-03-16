use super::default_texture_path;

#[test]
fn default_texture_path_is_empty_to_trigger_generated_texture_fallback() {
    assert!(default_texture_path().as_os_str().is_empty());
}
