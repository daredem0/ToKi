//! Build-time version injection derived from git describe output.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=TOKI_VERSION_OVERRIDE");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let manifest_path = PathBuf::from(manifest_dir);

    if let Some(repo_root) = resolve_repo_root(&manifest_path) {
        let git_dir = repo_root.join(".git");
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
        println!("cargo:rerun-if-changed={}", git_dir.join("refs").display());
        println!(
            "cargo:rerun-if-changed={}",
            git_dir.join("packed-refs").display()
        );
    }

    let version = resolve_version(&manifest_path);
    println!("cargo:rustc-env=TOKI_VERSION={version}");
}

fn resolve_version(manifest_path: &Path) -> String {
    if let Ok(override_version) = std::env::var("TOKI_VERSION_OVERRIDE") {
        let trimmed = override_version.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    git_describe(manifest_path)
        .map(normalize_tag_prefix)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

fn git_describe(manifest_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args([
            "-C",
            manifest_path.to_string_lossy().as_ref(),
            "describe",
            "--tags",
            "--dirty",
            "--always",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn resolve_repo_root(manifest_path: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args([
            "-C",
            manifest_path.to_string_lossy().as_ref(),
            "rev-parse",
            "--show-toplevel",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let root = String::from_utf8(output.stdout).ok()?;
    let root = root.trim();
    if root.is_empty() {
        None
    } else {
        Some(PathBuf::from(root))
    }
}

fn normalize_tag_prefix(version: String) -> String {
    if let Some(stripped) = version.strip_prefix('v') {
        if stripped
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_digit())
        {
            return stripped.to_string();
        }
    }

    version
}
