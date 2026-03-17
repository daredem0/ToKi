use super::{builtin_font_family, find_font_files, scan_project_font_registry, BuiltinFontFamily};

#[test]
fn builtin_font_family_recognizes_engine_aliases() {
    assert_eq!(builtin_font_family("Sans"), Some(BuiltinFontFamily::Sans));
    assert_eq!(
        builtin_font_family("sans-serif"),
        Some(BuiltinFontFamily::Sans)
    );
    assert_eq!(builtin_font_family("Serif"), Some(BuiltinFontFamily::Serif));
    assert_eq!(builtin_font_family("Mono"), Some(BuiltinFontFamily::Mono));
    assert_eq!(builtin_font_family("Monospace"), Some(BuiltinFontFamily::Mono));
    assert_eq!(builtin_font_family("Fancy"), None);
}

#[test]
fn find_font_files_only_returns_supported_extensions_sorted() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let supported_a = tmp.path().join("A.ttf");
    let supported_b = tmp.path().join("b.otf");
    let supported_c = tmp.path().join("c.TTC");
    let ignored = tmp.path().join("readme.txt");
    std::fs::write(&supported_a, "a").expect("font a");
    std::fs::write(&supported_b, "b").expect("font b");
    std::fs::write(&supported_c, "c").expect("font c");
    std::fs::write(&ignored, "x").expect("ignored");

    let found = find_font_files(tmp.path());
    assert_eq!(found, vec![supported_a, supported_b, supported_c]);
}

#[test]
fn scan_project_font_registry_returns_empty_without_assets_fonts_directory() {
    let tmp = tempfile::tempdir().expect("temp dir");
    let registry = scan_project_font_registry(tmp.path());
    assert!(registry.assets.is_empty());
    assert!(registry.family_names().is_empty());
}
