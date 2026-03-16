use super::{
    hash_bytes, infer_pack_asset_type, recommended_pack_compression, PackAssetType, PackCompression,
};
use std::path::Path;

#[test]
fn asset_type_and_compression_policy_match_expected_content_classes() {
    let scene = Path::new("scenes/Main Scene.json");
    assert_eq!(infer_pack_asset_type(scene), PackAssetType::Scene);
    assert_eq!(
        recommended_pack_compression(scene, infer_pack_asset_type(scene)),
        PackCompression::Zstd
    );

    let config = Path::new("project.toml");
    assert_eq!(infer_pack_asset_type(config), PackAssetType::ProjectConfig);
    assert_eq!(
        recommended_pack_compression(config, infer_pack_asset_type(config)),
        PackCompression::Zstd
    );

    let image = Path::new("assets/sprites/terrain.png");
    assert_eq!(infer_pack_asset_type(image), PackAssetType::Image);
    assert_eq!(
        recommended_pack_compression(image, infer_pack_asset_type(image)),
        PackCompression::Store
    );

    let audio = Path::new("assets/audio/music/lavandia.ogg");
    assert_eq!(infer_pack_asset_type(audio), PackAssetType::Audio);
    assert_eq!(
        recommended_pack_compression(audio, infer_pack_asset_type(audio)),
        PackCompression::Store
    );
}

#[test]
fn zstd_compression_round_trips_payload() {
    let source = br#"{"scene":"Main Scene","entities":[{"id":1}]}"#;
    let compressed = PackCompression::Zstd.compress(source).expect("compress");
    assert_ne!(compressed, source);
    let decoded = PackCompression::Zstd
        .decompress(&compressed)
        .expect("decompress");
    assert_eq!(decoded, source);
}

#[test]
fn hashing_is_stable_and_hex_encoded() {
    let hash_a = hash_bytes(b"hello");
    let hash_b = hash_bytes(b"hello");
    let hash_c = hash_bytes(b"world");
    assert_eq!(hash_a, hash_b);
    assert_ne!(hash_a, hash_c);
    assert_eq!(hash_a.len(), 64);
    assert!(hash_a.chars().all(|ch| ch.is_ascii_hexdigit()));
}
