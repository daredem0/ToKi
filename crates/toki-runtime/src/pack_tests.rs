use super::extract_pak_to_tempdir;
use std::fs;
use std::io::{Read, Seek, Write};
use toki_core::pack::{hash_bytes, PackAssetType, PackCompression, PAK_VERSION};

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
        error
            .to_string()
            .contains("Failed to deserialize pak manifest"),
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

#[test]
fn extract_pak_to_tempdir_decompresses_zstd_entries_and_verifies_hashes() {
    let temp = tempfile::tempdir().expect("temp dir");
    let pak_path = temp.path().join("compressed.toki.pak");
    let source = br#"{"name":"main","entities":[]}"#;
    let compressed = PackCompression::Zstd.compress(source).expect("compress");

    let mut file = fs::File::create(&pak_path).expect("create pak");
    file.write_all(b"TOKIPAK1").expect("magic");
    file.write_all(&0u64.to_le_bytes())
        .expect("offset placeholder");
    file.write_all(&0u64.to_le_bytes())
        .expect("size placeholder");
    let payload_offset = file.stream_position().expect("offset");
    file.write_all(&compressed).expect("payload");
    let index_offset = file.stream_position().expect("index offset");
    let index_bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "version": PAK_VERSION,
        "entries": [
            {
                "path": "scenes/main.json",
                "offset": payload_offset,
                "size": source.len(),
                "stored_size": compressed.len(),
                "compression": "zstd",
                "hash": hash_bytes(source),
                "asset_type": PackAssetType::Scene,
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

    let mount = extract_pak_to_tempdir(&pak_path).expect("extract");
    let decoded =
        fs::read_to_string(mount.path().join("scenes/main.json")).expect("read extracted");
    assert_eq!(decoded, String::from_utf8_lossy(source));
}

#[test]
fn extract_pak_to_tempdir_rejects_hash_mismatches() {
    let temp = tempfile::tempdir().expect("temp dir");
    let pak_path = temp.path().join("bad_hash.toki.pak");
    let source = b"hello pack";

    let mut file = fs::File::create(&pak_path).expect("create pak");
    file.write_all(b"TOKIPAK1").expect("magic");
    file.write_all(&0u64.to_le_bytes())
        .expect("offset placeholder");
    file.write_all(&0u64.to_le_bytes())
        .expect("size placeholder");
    let payload_offset = file.stream_position().expect("offset");
    file.write_all(source).expect("payload");
    let index_offset = file.stream_position().expect("index offset");
    let index_bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "version": PAK_VERSION,
        "entries": [
            {
                "path": "project.toml",
                "offset": payload_offset,
                "size": source.len(),
                "stored_size": source.len(),
                "compression": "store",
                "hash": hash_bytes(b"something else"),
                "asset_type": PackAssetType::ProjectConfig,
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

    let error = extract_pak_to_tempdir(&pak_path).expect_err("hash mismatch should fail");
    assert!(
        error.to_string().contains("hash mismatch"),
        "unexpected error: {error}"
    );
}
