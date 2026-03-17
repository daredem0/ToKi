use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinFontFamily {
    Sans,
    Serif,
    Mono,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFontAsset {
    pub path: PathBuf,
    pub families: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectFontRegistry {
    pub assets: Vec<ProjectFontAsset>,
}

impl ProjectFontRegistry {
    pub fn family_names(&self) -> Vec<String> {
        self.assets
            .iter()
            .flat_map(|asset| asset.families.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn has_family(&self, family_name: &str) -> bool {
        self.assets.iter().any(|asset| {
            asset
                .families
                .iter()
                .any(|candidate| candidate == family_name)
        })
    }
}

pub fn builtin_font_family(name: &str) -> Option<BuiltinFontFamily> {
    match name.trim().to_ascii_lowercase().as_str() {
        "sans" | "sans-serif" | "sansserif" | "proportional" => {
            Some(BuiltinFontFamily::Sans)
        }
        "serif" => Some(BuiltinFontFamily::Serif),
        "mono" | "monospace" | "monospaced" => Some(BuiltinFontFamily::Mono),
        _ => None,
    }
}

pub fn find_font_files(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() || !dir.is_dir() {
        return Vec::new();
    }

    let mut fonts: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        })
        .collect();
    fonts.sort();
    fonts
}

pub fn scan_project_font_registry(project_root: &Path) -> ProjectFontRegistry {
    let fonts_dir = project_root.join("assets").join("fonts");
    let assets = find_font_files(&fonts_dir)
        .into_iter()
        .filter_map(|path| {
            let families = read_font_family_names(&path);
            if families.is_empty() {
                return None;
            }
            Some(ProjectFontAsset { path, families })
        })
        .collect();
    ProjectFontRegistry { assets }
}

fn read_font_family_names(path: &Path) -> Vec<String> {
    let mut db = fontdb::Database::new();
    if db.load_font_file(path).is_err() {
        return Vec::new();
    }

    db.faces()
        .flat_map(|face| face.families.iter().map(|(name, _lang)| name.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod tests;
