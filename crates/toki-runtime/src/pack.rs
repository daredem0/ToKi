use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const PAK_MAGIC: &[u8; 8] = b"TOKIPAK1";

#[derive(Debug, Clone, Deserialize)]
struct PakManifest {
    #[allow(dead_code)]
    version: u32,
    entries: Vec<PakEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct PakEntry {
    path: String,
    offset: u64,
    size: u64,
    #[allow(dead_code)]
    compression: String,
}

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
        let mut payload = vec![0u8; entry.size as usize];
        file.read_exact(&mut payload)?;
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
mod tests {
    use super::extract_pak_to_tempdir;
    use std::fs;
    use std::io::{Read, Seek, Write};

    fn write_minimal_pak(
        pak_path: &std::path::Path,
        entries: &[(&str, &[u8])],
        override_index_bytes: Option<Vec<u8>>,
        override_index_size: Option<u64>,
    ) {
        let mut file = fs::File::create(pak_path).expect("create pak");
        file.write_all(b"TOKIPAK1").expect("magic");
        file.write_all(&0u64.to_le_bytes())
            .expect("offset placeholder");
        file.write_all(&0u64.to_le_bytes())
            .expect("size placeholder");
        let mut manifest_entries = Vec::new();
        for (path, payload) in entries {
            let offset = file.stream_position().expect("offset");
            file.write_all(payload).expect("payload");
            manifest_entries.push(serde_json::json!({
                "path": path,
                "offset": offset,
                "size": payload.len(),
                "compression": "none"
            }));
        }
        let index_offset = file.stream_position().expect("index offset");
        let index_bytes = override_index_bytes.unwrap_or_else(|| {
            serde_json::to_vec_pretty(&serde_json::json!({
                "version": 1,
                "entries": manifest_entries
            }))
            .expect("manifest")
        });
        file.write_all(&index_bytes).expect("index");
        let index_size = override_index_size.unwrap_or(index_bytes.len() as u64);
        file.seek(std::io::SeekFrom::Start(8)).expect("seek header");
        file.write_all(&index_offset.to_le_bytes())
            .expect("write offset");
        file.write_all(&index_size.to_le_bytes())
            .expect("write size");
    }

    #[test]
    fn extract_pak_to_tempdir_restores_files() {
        let temp = tempfile::tempdir().expect("temp dir");
        let pak_path = temp.path().join("game.toki.pak");
        fs::write(temp.path().join("a.txt"), "aaa").expect("a");
        fs::create_dir_all(temp.path().join("dir")).expect("dir");
        fs::write(temp.path().join("dir").join("b.txt"), "bbb").expect("b");

        write_minimal_pak(
            &pak_path,
            &[("a.txt", b"aaa"), ("dir/b.txt", b"bbb")],
            None,
            None,
        );

        let mount = extract_pak_to_tempdir(&pak_path).expect("extract");
        let mut a = String::new();
        fs::File::open(mount.path().join("a.txt"))
            .expect("a open")
            .read_to_string(&mut a)
            .expect("a read");
        assert_eq!(a, "aaa");
        let mut b = String::new();
        fs::File::open(mount.path().join("dir").join("b.txt"))
            .expect("b open")
            .read_to_string(&mut b)
            .expect("b read");
        assert_eq!(b, "bbb");
    }

    #[test]
    fn extract_pak_to_tempdir_rejects_invalid_magic() {
        let temp = tempfile::tempdir().expect("temp dir");
        let pak_path = temp.path().join("bad_magic.toki.pak");
        fs::write(&pak_path, b"NOTTOKI!").expect("write invalid pak");

        let error = extract_pak_to_tempdir(&pak_path).expect_err("invalid magic should fail");
        assert!(
            error.to_string().contains("Invalid pak magic"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn extract_pak_to_tempdir_rejects_zero_index_size() {
        let temp = tempfile::tempdir().expect("temp dir");
        let pak_path = temp.path().join("zero_index.toki.pak");
        write_minimal_pak(&pak_path, &[("a.txt", b"aaa")], None, Some(0));

        let error = extract_pak_to_tempdir(&pak_path).expect_err("zero index should fail");
        assert!(
            error.to_string().contains("Invalid pak index size (0)"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn extract_pak_to_tempdir_rejects_malformed_manifest() {
        let temp = tempfile::tempdir().expect("temp dir");
        let pak_path = temp.path().join("malformed_manifest.toki.pak");
        write_minimal_pak(
            &pak_path,
            &[("a.txt", b"aaa")],
            Some(b"{ definitely-not-valid-json ".to_vec()),
            None,
        );

        let error = extract_pak_to_tempdir(&pak_path).expect_err("malformed manifest should fail");
        assert!(
            error.to_string().contains("Failed to deserialize pak manifest"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn extract_pak_to_tempdir_rejects_out_of_bounds_payload_entry() {
        let temp = tempfile::tempdir().expect("temp dir");
        let pak_path = temp.path().join("out_of_bounds.toki.pak");
        let mut file = fs::File::create(&pak_path).expect("create pak");
        file.write_all(b"TOKIPAK1").expect("magic");
        file.write_all(&0u64.to_le_bytes())
            .expect("offset placeholder");
        file.write_all(&0u64.to_le_bytes())
            .expect("size placeholder");
        let payload_offset = file.stream_position().expect("offset");
        file.write_all(b"abc").expect("payload");
        let index_offset = file.stream_position().expect("index offset");
        let index_bytes = serde_json::to_vec_pretty(&serde_json::json!({
            "version": 1,
            "entries": [
                {
                    "path": "a.txt",
                    "offset": payload_offset,
                    "size": 4096,
                    "compression": "none"
                }
            ]
        }))
        .expect("manifest");
        file.write_all(&index_bytes).expect("index");
        file.seek(std::io::SeekFrom::Start(8)).expect("seek header");
        file.write_all(&index_offset.to_le_bytes())
            .expect("write offset");
        file.write_all(&(index_bytes.len() as u64).to_le_bytes())
            .expect("write size");

        let error = extract_pak_to_tempdir(&pak_path).expect_err("oversized payload should fail");
        let text = error.to_string();
        assert!(
            text.contains("failed to fill whole buffer") || text.contains("unexpected end of file"),
            "unexpected error: {error}"
        );
    }
}
