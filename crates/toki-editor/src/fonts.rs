use std::path::Path;

use toki_core::fonts::{
    builtin_font_family, scan_project_font_registry, BuiltinFontFamily, ProjectFontAsset,
    ProjectFontRegistry,
};

pub fn load_project_fonts_into_egui(
    ctx: &egui::Context,
    project_path: Option<&Path>,
) -> ProjectFontRegistry {
    let registry = project_path
        .map(scan_project_font_registry)
        .unwrap_or_default();
    let mut definitions = egui::FontDefinitions::default();
    let mut loaded_assets = Vec::new();

    for (asset_index, asset) in registry.assets.iter().enumerate() {
        let Ok(bytes) = std::fs::read(&asset.path) else {
            tracing::warn!(
                "Failed to read project font asset '{}' for editor preview registration",
                asset.path.display()
            );
            continue;
        };

        let font_key = format!(
            "project_font_{}_{}",
            asset_index,
            asset
                .path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("font")
        );
        definitions
            .font_data
            .insert(font_key.clone(), egui::FontData::from_owned(bytes).into());
        for family in &asset.families {
            definitions
                .families
                .entry(egui::FontFamily::Name(family.clone().into()))
                .or_default()
                .push(font_key.clone());
        }
        loaded_assets.push(ProjectFontAsset {
            path: asset.path.clone(),
            families: asset.families.clone(),
        });
    }

    ctx.set_fonts(definitions);
    ProjectFontRegistry {
        assets: loaded_assets,
    }
}

pub fn menu_font_family_choices(registry: &ProjectFontRegistry) -> Vec<String> {
    let mut families = vec!["Sans".to_string(), "Serif".to_string(), "Mono".to_string()];
    for family in registry.family_names() {
        if !families.iter().any(|existing| existing == &family) {
            families.push(family);
        }
    }
    families
}

pub fn resolve_preview_font_family(
    requested_family: &str,
    available_families: &[String],
) -> egui::FontFamily {
    match builtin_font_family(requested_family) {
        Some(BuiltinFontFamily::Sans | BuiltinFontFamily::Serif) => egui::FontFamily::Proportional,
        Some(BuiltinFontFamily::Mono) => egui::FontFamily::Monospace,
        None if available_families
            .iter()
            .any(|family| family == requested_family) =>
        {
            egui::FontFamily::Name(requested_family.to_string().into())
        }
        None => egui::FontFamily::Proportional,
    }
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod tests;
