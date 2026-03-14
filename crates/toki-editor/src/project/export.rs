use crate::project::Project;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const PAK_MAGIC: &[u8; 8] = b"TOKIPAK1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PakManifest {
    version: u32,
    entries: Vec<PakEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PakEntry {
    path: String,
    offset: u64,
    size: u64,
    compression: String,
}

#[derive(Debug, Clone)]
struct SourceFile {
    absolute_path: PathBuf,
    relative_path: PathBuf,
}

pub fn export_hybrid_bundle(
    project: &Project,
    runtime_binary_path: &Path,
    export_root: &Path,
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

    let source_files = collect_source_files(&project.path, Some(&bundle_dir))?;
    copy_project_files_to_bundle(&bundle_dir, &source_files)?;

    let pak_path = bundle_dir.join("game.toki.pak");
    write_project_pak(&pak_path, &source_files)?;

    Ok(bundle_dir)
}

fn copy_project_files_to_bundle(bundle_dir: &Path, source_files: &[SourceFile]) -> Result<()> {
    for source in source_files {
        let destination = bundle_dir.join(&source.relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create bundle directory '{}'", parent.display())
            })?;
        }
        fs::copy(&source.absolute_path, &destination).with_context(|| {
            format!(
                "Failed to copy '{}' to '{}'",
                source.absolute_path.display(),
                destination.display()
            )
        })?;
    }
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
        file.write_all(&bytes)?;
        entries.push(PakEntry {
            path: source.relative_path.to_string_lossy().replace('\\', "/"),
            offset,
            size: bytes.len() as u64,
            compression: "none".to_string(),
        });
    }

    let index_offset = file.stream_position()?;
    let manifest = PakManifest {
        version: 1,
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
mod tests {
    use super::{collect_source_files, export_hybrid_bundle, PAK_MAGIC};
    use crate::project::Project;
    use std::fs;
    use std::io::{Read, Seek, SeekFrom};

    #[test]
    fn collect_source_files_returns_sorted_relative_paths() {
        let temp = tempfile::tempdir().expect("temp dir");
        let project_root = temp.path().join("MyGame");
        fs::create_dir_all(project_root.join("assets/sprites")).expect("assets dir");
        fs::create_dir_all(project_root.join("scenes")).expect("scenes dir");
        fs::write(project_root.join("project.toml"), "name = 'MyGame'").expect("project");
        fs::write(project_root.join("scenes/main.json"), "{}").expect("scene");
        fs::write(project_root.join("assets/sprites/a.png"), "a").expect("asset");

        let files = collect_source_files(&project_root, None).expect("collect files");
        let relative = files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            relative,
            vec![
                "assets/sprites/a.png".to_string(),
                "project.toml".to_string(),
                "scenes/main.json".to_string(),
            ]
        );
    }

    #[test]
    fn export_hybrid_bundle_writes_runtime_and_pak_manifest() {
        let temp = tempfile::tempdir().expect("temp dir");
        let parent = temp.path();
        let project_root = parent.join("MyGame");
        fs::create_dir_all(project_root.join("assets/audio")).expect("assets dir");
        fs::create_dir_all(project_root.join("scenes")).expect("scenes dir");
        fs::write(project_root.join("project.toml"), "name = 'MyGame'").expect("project");
        fs::write(project_root.join("scenes/main.json"), "{\"name\":\"main\"}").expect("scene");
        fs::write(project_root.join("assets/audio/test.ogg"), "audio").expect("audio");

        let runtime_bin = parent.join(if cfg!(target_os = "windows") {
            "toki-runtime.exe"
        } else {
            "toki-runtime"
        });
        fs::write(&runtime_bin, "runtime-binary").expect("runtime");

        let project = Project::new("MyGame".to_string(), project_root.clone());
        let export_root = parent.join("exports");
        fs::create_dir_all(&export_root).expect("exports dir");

        let bundle_dir =
            export_hybrid_bundle(&project, &runtime_bin, &export_root).expect("bundle export");
        assert!(bundle_dir.join(runtime_bin.file_name().unwrap()).exists());
        assert!(bundle_dir.join("project.toml").exists());
        assert!(bundle_dir.join("scenes/main.json").exists());
        assert!(bundle_dir.join("assets/audio/test.ogg").exists());
        let pak_path = bundle_dir.join("game.toki.pak");
        assert!(pak_path.exists());

        let mut pak_file = fs::File::open(&pak_path).expect("open pak");
        let mut magic = [0u8; 8];
        pak_file.read_exact(&mut magic).expect("read magic");
        assert_eq!(&magic, PAK_MAGIC);

        let mut index_offset_buf = [0u8; 8];
        let mut index_size_buf = [0u8; 8];
        pak_file
            .read_exact(&mut index_offset_buf)
            .expect("read offset");
        pak_file.read_exact(&mut index_size_buf).expect("read size");
        let index_offset = u64::from_le_bytes(index_offset_buf);
        let index_size = u64::from_le_bytes(index_size_buf);
        assert!(index_offset > 0);
        assert!(index_size > 0);

        pak_file
            .seek(SeekFrom::Start(index_offset))
            .expect("seek index");
        let mut index_bytes = vec![0u8; index_size as usize];
        pak_file.read_exact(&mut index_bytes).expect("read index");
        let index_text = String::from_utf8(index_bytes).expect("utf8 index");
        assert!(index_text.contains("\"project.toml\""));
        assert!(index_text.contains("\"scenes/main.json\""));
        assert!(index_text.contains("\"assets/audio/test.ogg\""));
    }

    #[test]
    fn export_hybrid_bundle_uses_safe_bundle_directory_suffix() {
        let temp = tempfile::tempdir().expect("temp dir");
        let parent = temp.path();
        let project_root = parent.join("MyGame");
        fs::create_dir_all(project_root.join("assets")).expect("assets");
        fs::write(project_root.join("project.toml"), "name='MyGame'").expect("project");
        fs::write(project_root.join("assets/file.txt"), "payload").expect("asset");

        let runtime_bin = parent.join(if cfg!(target_os = "windows") {
            "toki-runtime.exe"
        } else {
            "toki-runtime"
        });
        fs::write(&runtime_bin, "runtime-binary").expect("runtime");

        let project = Project::new("MyGame".to_string(), project_root.clone());
        let bundle_dir =
            export_hybrid_bundle(&project, &runtime_bin, parent).expect("bundle export");

        assert_eq!(bundle_dir, parent.join("MyGame-bundle"));
        assert!(project_root.join("project.toml").exists());
        assert!(project_root.join("assets/file.txt").exists());
    }
}
