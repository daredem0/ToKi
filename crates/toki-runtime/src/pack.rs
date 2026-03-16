use anyhow::{Context, Result};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use toki_core::pack::{hash_bytes, PakManifest, PAK_MAGIC};

pub fn extract_pak_to_tempdir(pak_path: &Path) -> Result<tempfile::TempDir> {
    let mut file = fs::File::open(pak_path)
        .with_context(|| format!("Failed to open pak '{}'", pak_path.display()))?;

    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)
        .with_context(|| format!("Failed to read pak magic from '{}'", pak_path.display()))?;
    if &magic != PAK_MAGIC {
        return Err(anyhow::anyhow!(
            "Invalid pak magic for '{}'",
            pak_path.display()
        ));
    }

    let mut offset_buf = [0u8; 8];
    let mut size_buf = [0u8; 8];
    file.read_exact(&mut offset_buf)?;
    file.read_exact(&mut size_buf)?;
    let index_offset = u64::from_le_bytes(offset_buf);
    let index_size = u64::from_le_bytes(size_buf);
    if index_size == 0 {
        return Err(anyhow::anyhow!(
            "Invalid pak index size (0) for '{}'",
            pak_path.display()
        ));
    }

    file.seek(SeekFrom::Start(index_offset))?;
    let mut index_bytes = vec![0u8; index_size as usize];
    file.read_exact(&mut index_bytes)?;
    let manifest: PakManifest =
        serde_json::from_slice(&index_bytes).context("Failed to deserialize pak manifest")?;

    let mount_dir = tempfile::tempdir().context("Failed to create temp dir for pak mount")?;
    for entry in &manifest.entries {
        let destination = mount_dir.path().join(&entry.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create pak mount directory '{}'",
                    parent.display()
                )
            })?;
        }

        file.seek(SeekFrom::Start(entry.offset))?;
        let mut stored_payload = vec![0u8; entry.stored_size_or_size() as usize];
        file.read_exact(&mut stored_payload)?;
        let payload = entry
            .compression
            .decompress(&stored_payload)
            .with_context(|| {
                format!(
                    "Failed to decompress pak entry '{}' with {:?}",
                    entry.path, entry.compression
                )
            })?;
        if payload.len() as u64 != entry.size {
            return Err(anyhow::anyhow!(
                "Pak entry '{}' size mismatch: expected {} bytes after decode, got {}",
                entry.path,
                entry.size,
                payload.len()
            ));
        }
        if let Some(expected_hash) = &entry.hash {
            let actual_hash = hash_bytes(&payload);
            if &actual_hash != expected_hash {
                return Err(anyhow::anyhow!(
                    "Pak entry '{}' hash mismatch: expected {}, got {}",
                    entry.path,
                    expected_hash,
                    actual_hash
                ));
            }
        }
        fs::write(&destination, payload).with_context(|| {
            format!(
                "Failed to write pak extracted file '{}'",
                destination.display()
            )
        })?;
    }

    Ok(mount_dir)
}

#[cfg(test)]
#[path = "pack_tests.rs"]
mod tests;
