use crate::project::Project;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use toki_core::pack::{
    hash_bytes, infer_pack_asset_type, recommended_pack_compression, PackCompression, PakEntry,
    PakManifest, PAK_MAGIC, PAK_VERSION,
};

#[derive(Debug, Clone)]
struct SourceFile {
    absolute_path: PathBuf,
    relative_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBundleConfig {
    pub version: u32,
    pub bundle_name: String,
    pub pack: RuntimeBundlePackConfig,
    pub startup: RuntimeBundleStartupConfig,
    pub splash: RuntimeBundleSplashConfig,
    pub audio: RuntimeBundleAudioConfig,
    pub display: RuntimeBundleDisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBundlePackConfig {
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBundleStartupConfig {
    pub scene: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBundleSplashConfig {
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBundleAudioConfig {
    pub master_percent: u8,
    pub music_percent: u8,
    pub movement_percent: u8,
    pub collision_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeBundleDisplayConfig {
    pub show_entity_health_bars: bool,
}

pub fn export_hybrid_bundle(
    project: &Project,
    runtime_binary_path: &Path,
    export_root: &Path,
    startup_scene: Option<&str>,
    splash_duration_ms: u64,
) -> Result<PathBuf> {
    if !runtime_binary_path.exists() {
        return Err(anyhow::anyhow!(
            "Runtime binary does not exist: {}",
            runtime_binary_path.display()
        ));
    }

    let bundle_dir = export_root.join(format!("{}-bundle", project.name));
    if bundle_dir == project.path {
        return Err(anyhow::anyhow!(
            "Refusing to export into project source directory '{}'",
            bundle_dir.display()
        ));
    }
    if bundle_dir.exists() {
        fs::remove_dir_all(&bundle_dir).with_context(|| {
            format!(
                "Failed to remove existing export directory '{}'",
                bundle_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&bundle_dir).with_context(|| {
        format!(
            "Failed to create export directory '{}'",
            bundle_dir.display()
        )
    })?;

    let runtime_binary_name = runtime_binary_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Runtime binary path has no filename"))?;
    let runtime_bundle_path = bundle_dir.join(runtime_binary_name);
    fs::copy(runtime_binary_path, &runtime_bundle_path).with_context(|| {
        format!(
            "Failed to copy runtime binary '{}' to '{}'",
            runtime_binary_path.display(),
            runtime_bundle_path.display()
        )
    })?;
    copy_bundle_legal_documents(&bundle_dir)?;

    let source_files = collect_source_files(&project.path, Some(&bundle_dir))?;

    let pak_path = bundle_dir.join("game.toki.pak");
    write_project_pak(&pak_path, &source_files)?;
    write_runtime_bundle_config(
        &bundle_dir,
        &RuntimeBundleConfig {
            version: 1,
            bundle_name: project.name.clone(),
            pack: RuntimeBundlePackConfig {
                path: "game.toki.pak".to_string(),
                enabled: true,
            },
            startup: RuntimeBundleStartupConfig {
                scene: startup_scene.map(str::to_string),
            },
            splash: RuntimeBundleSplashConfig {
                duration_ms: splash_duration_ms,
            },
            audio: RuntimeBundleAudioConfig {
                master_percent: project.metadata.runtime.audio.master_percent,
                music_percent: project.metadata.runtime.audio.music_percent,
                movement_percent: project.metadata.runtime.audio.movement_percent,
                collision_percent: project.metadata.runtime.audio.collision_percent,
            },
            display: RuntimeBundleDisplayConfig {
                show_entity_health_bars: project.metadata.runtime.display.show_entity_health_bars,
            },
        },
    )?;

    Ok(bundle_dir)
}

fn copy_bundle_legal_documents(bundle_dir: &Path) -> Result<()> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve workspace root for exporter"))?;

    for filename in ["LICENSE-TOKI.md", "THIRD_PARTY_LICENSES.md"] {
        let source_path = workspace_root.join(filename);
        let target_path = bundle_dir.join(filename);
        fs::copy(&source_path, &target_path).with_context(|| {
            format!(
                "Failed to copy legal document '{}' to '{}'",
                source_path.display(),
                target_path.display()
            )
        })?;
    }

    Ok(())
}

fn write_runtime_bundle_config(bundle_dir: &Path, config: &RuntimeBundleConfig) -> Result<()> {
    let config_path = bundle_dir.join("runtime_config.json");
    let content = serde_json::to_vec_pretty(config)?;
    fs::write(&config_path, content).with_context(|| {
        format!(
            "Failed to write runtime bundle config '{}'",
            config_path.display()
        )
    })?;
    Ok(())
}

fn write_project_pak(pak_output_path: &Path, source_files: &[SourceFile]) -> Result<()> {
    let mut file = fs::File::create(pak_output_path).with_context(|| {
        format!(
            "Failed to create pak output '{}'",
            pak_output_path.display()
        )
    })?;

    file.write_all(PAK_MAGIC)?;
    file.write_all(&0u64.to_le_bytes())?; // index offset placeholder
    file.write_all(&0u64.to_le_bytes())?; // index size placeholder

    let mut entries = Vec::with_capacity(source_files.len());
    for source in source_files {
        let offset = file.stream_position()?;
        let bytes = fs::read(&source.absolute_path).with_context(|| {
            format!(
                "Failed to read source file '{}' for pak export",
                source.absolute_path.display()
            )
        })?;
        let relative_path = source.relative_path.to_string_lossy().replace('\\', "/");
        let asset_type = infer_pack_asset_type(&source.relative_path);
        let preferred_compression = recommended_pack_compression(&source.relative_path, asset_type);
        let candidate_bytes = preferred_compression.compress(&bytes).with_context(|| {
            format!(
                "Failed to {} '{}' for pak export",
                compression_label(preferred_compression),
                source.absolute_path.display()
            )
        })?;
        let (compression, stored_bytes) =
            choose_final_payload_encoding(preferred_compression, &bytes, candidate_bytes);
        file.write_all(&stored_bytes)?;
        entries.push(PakEntry {
            path: relative_path,
            offset,
            size: bytes.len() as u64,
            stored_size: stored_bytes.len() as u64,
            compression,
            hash: Some(hash_bytes(&bytes)),
            asset_type,
        });
    }

    let index_offset = file.stream_position()?;
    let manifest = PakManifest {
        version: PAK_VERSION,
        entries,
    };
    let index_bytes = serde_json::to_vec_pretty(&manifest)?;
    let index_size = index_bytes.len() as u64;
    file.write_all(&index_bytes)?;

    file.seek(SeekFrom::Start(PAK_MAGIC.len() as u64))?;
    file.write_all(&index_offset.to_le_bytes())?;
    file.write_all(&index_size.to_le_bytes())?;

    Ok(())
}

fn compression_label(compression: PackCompression) -> &'static str {
    match compression {
        PackCompression::Store => "store",
        PackCompression::Zstd => "compress",
    }
}

fn choose_final_payload_encoding(
    preferred_compression: PackCompression,
    source_bytes: &[u8],
    candidate_bytes: Vec<u8>,
) -> (PackCompression, Vec<u8>) {
    match preferred_compression {
        PackCompression::Store => (PackCompression::Store, candidate_bytes),
        PackCompression::Zstd if candidate_bytes.len() < source_bytes.len() => {
            (PackCompression::Zstd, candidate_bytes)
        }
        PackCompression::Zstd => (PackCompression::Store, source_bytes.to_vec()),
    }
}

fn collect_source_files(
    project_root: &Path,
    exclude_dir: Option<&Path>,
) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    collect_source_files_recursive(project_root, project_root, exclude_dir, &mut files)?;
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

fn collect_source_files_recursive(
    current_dir: &Path,
    project_root: &Path,
    exclude_dir: Option<&Path>,
    files: &mut Vec<SourceFile>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(exclude) = exclude_dir {
            if path.starts_with(exclude) {
                continue;
            }
        }

        if path.is_dir() {
            collect_source_files_recursive(&path, project_root, exclude_dir, files)?;
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let relative = path
            .strip_prefix(project_root)
            .with_context(|| {
                format!(
                    "Failed to compute relative path for '{}' from '{}'",
                    path.display(),
                    project_root.display()
                )
            })?
            .to_path_buf();
        files.push(SourceFile {
            absolute_path: path,
            relative_path: relative,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "export_tests.rs"]
mod tests;
