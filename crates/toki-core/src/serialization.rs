use crate::entity::{Entity, EntityManager};
use crate::game::GameState;
use crate::GameFlags;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const SAVE_DATA_VERSION: u32 = 1;
pub const MAX_SAVE_SLOTS: u8 = 3;

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("failed to encode JSON: {0}")]
    JsonEncode(#[from] serde_json::Error),
    #[error("failed to read or write file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid save slot {0}; expected 1..={MAX_SAVE_SLOTS}")]
    InvalidSaveSlot(u8),
    #[error("failed to restore save data: {0}")]
    Restore(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveSlotMetadata {
    pub slot: u8,
    pub scene_name: String,
    pub play_time_ms: u64,
    pub saved_at_unix_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SavedCameraState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<glam::IVec2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSceneEntityState {
    pub scene_name: String,
    pub entity_id: crate::entity::EntityId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub metadata: SaveSlotMetadata,
    pub active_scene_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<Entity>,
    #[serde(default)]
    pub flags: GameFlags,
    #[serde(default)]
    pub camera: SavedCameraState,
    #[serde(default)]
    pub persisted_entities: Vec<PersistedSceneEntityState>,
}

impl SaveData {
    pub fn capture(game_state: &GameState, slot: u8) -> Result<Self, SerializationError> {
        validate_save_slot(slot)?;
        let active_scene_name = game_state
            .scene_manager()
            .active_scene_name()
            .unwrap_or_default()
            .to_string();
        let (camera_position, camera_scale) = game_state
            .active_scene()
            .map(|scene| (scene.camera_position, scene.camera_scale))
            .unwrap_or((None, None));
        let saved_at_unix_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        Ok(Self {
            version: SAVE_DATA_VERSION,
            metadata: SaveSlotMetadata {
                slot,
                scene_name: active_scene_name.clone(),
                play_time_ms: game_state.play_time_ms(),
                saved_at_unix_secs,
            },
            active_scene_name,
            player: game_state.player_entity().cloned(),
            flags: game_state.game_flags().clone(),
            camera: SavedCameraState {
                position: camera_position,
                scale: camera_scale,
            },
            persisted_entities: Vec::new(),
        })
    }
}

fn validate_save_slot(slot: u8) -> Result<(), SerializationError> {
    if (1..=MAX_SAVE_SLOTS).contains(&slot) {
        Ok(())
    } else {
        Err(SerializationError::InvalidSaveSlot(slot))
    }
}

pub fn save_entity_to_file(entity: &Entity, path: &str) -> Result<(), SerializationError> {
    let json = serde_json::to_string_pretty(entity)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_entity_from_file(path: &str) -> Result<Entity, SerializationError> {
    let json = fs::read_to_string(path)?;
    let entity: Entity = serde_json::from_str(&json)?;
    Ok(entity)
}

pub fn save_scene(entity_manager: &EntityManager, path: &str) -> Result<(), SerializationError> {
    let json = serde_json::to_string_pretty(entity_manager)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_scene(path: &str) -> Result<EntityManager, SerializationError> {
    let json = fs::read_to_string(path)?;
    let entity_manager: EntityManager = serde_json::from_str(&json)?;
    Ok(entity_manager)
}

pub fn save_save_data(
    save_data: &SaveData,
    path: impl AsRef<Path>,
) -> Result<(), SerializationError> {
    let json = serde_json::to_string_pretty(save_data)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_save_data(path: impl AsRef<Path>) -> Result<SaveData, SerializationError> {
    let json = fs::read_to_string(path)?;
    let save_data: SaveData = serde_json::from_str(&json)?;
    Ok(save_data)
}

pub fn save_slot_file_path(
    save_root: impl AsRef<Path>,
    slot: u8,
) -> Result<PathBuf, SerializationError> {
    validate_save_slot(slot)?;
    Ok(save_root.as_ref().join(format!("slot_{slot}.json")))
}

pub fn save_game_to_slot(
    game_state: &GameState,
    save_root: impl AsRef<Path>,
    slot: u8,
) -> Result<PathBuf, SerializationError> {
    let path = save_slot_file_path(save_root.as_ref(), slot)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let save_data = SaveData::capture(game_state, slot)?;
    save_save_data(&save_data, &path)?;
    Ok(path)
}

pub fn load_save_data_from_slot(
    save_root: impl AsRef<Path>,
    slot: u8,
) -> Result<SaveData, SerializationError> {
    let path = save_slot_file_path(save_root, slot)?;
    load_save_data(path)
}

pub fn read_save_slot_metadata(
    save_root: impl AsRef<Path>,
    slot: u8,
) -> Result<Option<SaveSlotMetadata>, SerializationError> {
    let path = save_slot_file_path(save_root, slot)?;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(load_save_data(path)?.metadata))
}

pub fn list_save_slot_metadata(
    save_root: impl AsRef<Path>,
) -> Result<Vec<Option<SaveSlotMetadata>>, SerializationError> {
    (1..=MAX_SAVE_SLOTS)
        .map(|slot| read_save_slot_metadata(save_root.as_ref(), slot))
        .collect()
}
