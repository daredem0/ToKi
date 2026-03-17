use super::{menu_font_family_choices, resolve_preview_font_family};
use toki_core::fonts::{ProjectFontAsset, ProjectFontRegistry};

#[test]
fn preview_font_family_maps_builtin_aliases_to_safe_egui_families() {
    assert_eq!(
        resolve_preview_font_family("Sans", &[]),
        egui::FontFamily::Proportional
    );
    assert_eq!(
        resolve_preview_font_family("Serif", &[]),
        egui::FontFamily::Proportional
    );
    assert_eq!(
        resolve_preview_font_family("Mono", &[]),
        egui::FontFamily::Monospace
    );
}

#[test]
fn preview_font_family_uses_custom_name_only_when_registered() {
    let available = vec!["My Fancy Font".to_string()];
    assert_eq!(
        resolve_preview_font_family("My Fancy Font", &available),
        egui::FontFamily::Name("My Fancy Font".into())
    );
    assert_eq!(
        resolve_preview_font_family("Missing Font", &available),
        egui::FontFamily::Proportional
    );
}

#[test]
fn menu_font_family_choices_include_builtins_and_project_fonts_without_duplicates() {
    let registry = ProjectFontRegistry {
        assets: vec![ProjectFontAsset {
            path: std::path::PathBuf::from("assets/fonts/fancy.ttf"),
            families: vec!["Fancy".to_string(), "Sans".to_string()],
        }],
    };

    assert_eq!(
        menu_font_family_choices(&registry),
        vec![
            "Sans".to_string(),
            "Serif".to_string(),
            "Mono".to_string(),
            "Fancy".to_string(),
        ]
    );
}
