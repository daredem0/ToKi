use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;

pub const PAK_MAGIC: &[u8; 8] = b"TOKIPAK1";
pub const PAK_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackCompression {
    #[serde(alias = "none")]
    Store,
    Zstd,
}

impl PackCompression {
    pub fn compress(self, bytes: &[u8]) -> std::io::Result<Vec<u8>> {
        match self {
            Self::Store => Ok(bytes.to_vec()),
            Self::Zstd => zstd::stream::encode_all(Cursor::new(bytes), 3),
        }
    }

    pub fn decompress(self, bytes: &[u8]) -> std::io::Result<Vec<u8>> {
        match self {
            Self::Store => Ok(bytes.to_vec()),
            Self::Zstd => {
                let mut cursor = Cursor::new(bytes);
                let mut decoded = Vec::new();
                zstd::stream::copy_decode(&mut cursor, &mut decoded)?;
                Ok(decoded)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackAssetType {
    ProjectConfig,
    Scene,
    Entity,
    Map,
    Atlas,
    Image,
    Audio,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PakManifest {
    pub version: u32,
    pub entries: Vec<PakEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PakEntry {
    pub path: String,
    pub offset: u64,
    pub size: u64,
    #[serde(default)]
    pub stored_size: u64,
    #[serde(default = "default_pack_compression")]
    pub compression: PackCompression,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default = "default_pack_asset_type")]
    pub asset_type: PackAssetType,
}

impl PakEntry {
    pub fn stored_size_or_size(&self) -> u64 {
        if self.stored_size == 0 {
            self.size
        } else {
            self.stored_size
        }
    }
}

fn default_pack_compression() -> PackCompression {
    PackCompression::Store
}

fn default_pack_asset_type() -> PackAssetType {
    PackAssetType::Other
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub fn infer_pack_asset_type(path: &Path) -> PackAssetType {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized == "project.toml" {
        return PackAssetType::ProjectConfig;
    }
    if normalized.starts_with("scenes/") {
        return PackAssetType::Scene;
    }
    if normalized.starts_with("entities/") {
        return PackAssetType::Entity;
    }
    if normalized.starts_with("maps/") {
        return PackAssetType::Map;
    }
    if normalized.ends_with(".atlas.json") || normalized.contains("/atlases/") {
        return PackAssetType::Atlas;
    }

    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp") => PackAssetType::Image,
        Some("ogg" | "mp3" | "flac" | "wav") => PackAssetType::Audio,
        _ => PackAssetType::Other,
    }
}

pub fn recommended_pack_compression(path: &Path, asset_type: PackAssetType) -> PackCompression {
    match asset_type {
        PackAssetType::Image | PackAssetType::Audio => PackCompression::Store,
        PackAssetType::ProjectConfig
        | PackAssetType::Scene
        | PackAssetType::Entity
        | PackAssetType::Map
        | PackAssetType::Atlas => PackCompression::Zstd,
        PackAssetType::Other => match path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref()
        {
            Some("json" | "toml" | "txt" | "ron" | "yaml" | "yml") => PackCompression::Zstd,
            _ => PackCompression::Store,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        hash_bytes, infer_pack_asset_type, recommended_pack_compression, PackAssetType,
        PackCompression,
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
}
