use crate::entity::{Entity, EntityManager, EntityWire, StoredEntity};
use crate::game::{GameState, RestoreError};
use crate::ids::SceneId;
use crate::scene::Scene;
use crate::GameFlags;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const SAVE_DATA_VERSION: u32 = 1;
pub const MAX_SAVE_SLOTS: u8 = 3;
pub const MAX_SAVE_FILE_SIZE: u64 = 8 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum SerializationError {
    #[error("failed to encode JSON: {0}")]
    JsonEncode(#[from] serde_json::Error),
    #[error("failed to read or write file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid save slot {0}; expected 1..={MAX_SAVE_SLOTS}")]
    InvalidSaveSlot(u8),
    #[error("unsupported save data version {actual}; expected {expected}")]
    InvalidSaveVersion { expected: u32, actual: u32 },
    #[error("file is too large to load safely: {path} ({size_bytes} bytes, max {max_bytes})")]
    FileTooLarge {
        path: String,
        size_bytes: u64,
        max_bytes: u64,
    },
    #[error("failed to restore save data: {0}")]
    Restore(#[from] RestoreError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveSlotMetadata {
    pub slot: u8,
    pub scene_name: SceneId,
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
    pub scene_name: SceneId,
    pub entity_id: crate::entity::EntityId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity: Option<StoredEntity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub metadata: SaveSlotMetadata,
    pub active_scene_name: SceneId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<StoredEntity>,
    #[serde(default)]
    pub flags: GameFlags,
    #[serde(default)]
    pub camera: SavedCameraState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scene_snapshots: Vec<Scene>,
    #[serde(default)]
    pub persisted_entities: Vec<PersistedSceneEntityState>,
}

impl SaveData {
    pub fn capture(game_state: &GameState, slot: u8) -> Result<Self, SerializationError> {
        validate_save_slot(slot)?;
        let active_scene_name = game_state
            .scene()
            .scene_manager()
            .active_scene_name()
            .unwrap_or_default()
            .to_string();
        let (camera_position, camera_scale) = game_state
            .scene()
            .scene_manager()
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
                scene_name: active_scene_name.clone().into(),
                play_time_ms: game_state.play_time_ms(),
                saved_at_unix_secs,
            },
            active_scene_name: active_scene_name.into(),
            player: game_state
                .world()
                .player_id()
                .and_then(|player_id| game_state.world().entity_manager().stored_entity(player_id)),
            flags: game_state.game_flags().clone(),
            camera: SavedCameraState {
                position: camera_position,
                scale: camera_scale,
            },
            scene_snapshots: {
                let mut snapshots = game_state
                    .scene()
                    .scene_manager()
                    .scene_entries()
                    .map(|(_, scene)| scene.clone())
                    .collect::<Vec<_>>();
                snapshots.sort_by(|left, right| left.name.cmp(&right.name));
                snapshots
            },
            persisted_entities: game_state
                .persistent_scene_entity_keys()
                .into_iter()
                .map(|(scene_name, entity_id)| PersistedSceneEntityState {
                    entity: game_state
                        .scene()
                        .scene_manager()
                        .get_scene(scene_name.as_str())
                        .and_then(|scene| scene.stored_entity(entity_id)),
                    scene_name: scene_name.into(),
                    entity_id,
                })
                .collect(),
        })
    }
}

pub fn validate_version(version: u32) -> Result<(), SerializationError> {
    if version == SAVE_DATA_VERSION {
        Ok(())
    } else {
        Err(SerializationError::InvalidSaveVersion {
            expected: SAVE_DATA_VERSION,
            actual: version,
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
    let wire = EntityWire::from(StoredEntity::new(
        entity.clone(),
        crate::entity::EntityOptionalComponents::default(),
    ));
    let json = serde_json::to_string_pretty(&wire)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_entity_from_file(path: &str) -> Result<Entity, SerializationError> {
    let json = read_text_file_with_limit(path, MAX_SAVE_FILE_SIZE)?;
    let entity: StoredEntity = serde_json::from_str::<EntityWire>(&json)?.into();
    Ok(entity.entity)
}

pub fn save_scene(entity_manager: &EntityManager, path: &str) -> Result<(), SerializationError> {
    let json = serde_json::to_string_pretty(entity_manager)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_scene(path: &str) -> Result<EntityManager, SerializationError> {
    let json = read_text_file_with_limit(path, MAX_SAVE_FILE_SIZE)?;
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
    let json = read_text_file_with_limit(path.as_ref(), MAX_SAVE_FILE_SIZE)?;
    let save_data: SaveData = serde_json::from_str(&json)?;
    validate_version(save_data.version)?;
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
    game_state: &mut GameState,
    save_root: impl AsRef<Path>,
    slot: u8,
) -> Result<PathBuf, SerializationError> {
    let path = save_slot_file_path(save_root.as_ref(), slot)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::game::SceneSystem::sync_entities_to_active_scene(game_state);
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

fn read_text_file_with_limit(
    path: impl AsRef<Path>,
    max_bytes: u64,
) -> Result<String, SerializationError> {
    let path = path.as_ref();
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_bytes {
        return Err(SerializationError::FileTooLarge {
            path: path.display().to_string(),
            size_bytes: metadata.len(),
            max_bytes,
        });
    }
    Ok(fs::read_to_string(path)?)
}
