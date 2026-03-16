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
#[path = "pack_tests.rs"]
mod tests;
