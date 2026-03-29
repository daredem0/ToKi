use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::entity::{Entity, EntityComponentStore, EntityId, EntityOptionalComponents, StoredEntity};
use crate::ids::EntityDefName;
use crate::rules::RuleSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAnchorKind {
    SpawnPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAnchorFacing {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneAnchor {
    pub id: String,
    pub kind: SceneAnchorKind,
    pub position: glam::IVec2,
    pub facing: Option<SceneAnchorFacing>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenePlayerEntry {
    pub entity_definition_name: EntityDefName,
    pub spawn_point_id: String,
}

/// Represents a game scene - a complete game environment with entities, maps, and metadata.
///
/// A scene is a self-contained game environment that can be loaded, saved, and edited.
/// Unlike GameState which is for runtime execution, Scene is for data persistence and editing.
#[derive(Debug, Clone)]
pub struct Scene {
    /// Scene metadata
    pub name: String,
    pub description: Option<String>,

    /// Map configuration
    /// List of map names associated with this scene
    pub maps: Vec<String>,

    /// Entities in this scene
    pub entities: Vec<Entity>,

    pub components: EntityComponentStore,

    /// Data-driven rules authored for this scene.
    pub rules: RuleSet,

    /// Scene-specific camera settings (optional override)
    pub camera_position: Option<glam::IVec2>,
    pub camera_scale: Option<u32>,

    /// Optional background music track id for this scene.
    pub background_music_track_id: Option<String>,

    /// Placeable authored scene anchors such as spawn points.
    pub anchors: Vec<SceneAnchor>,

    /// Optional scene-authored player preview/entry configuration.
    ///
    /// Scenes are not required to author a player entry. This is used when a
    /// scene wants to define which player entity definition should preview and
    /// enter at which spawn point.
    pub player_entry: Option<ScenePlayerEntry>,
}

#[derive(Serialize, Deserialize)]
struct SceneWire {
    pub name: String,
    pub description: Option<String>,
    pub maps: Vec<String>,
    pub entities: Vec<StoredEntity>,
    #[serde(default)]
    pub rules: RuleSet,
    pub camera_position: Option<glam::IVec2>,
    pub camera_scale: Option<u32>,
    #[serde(default)]
    pub background_music_track_id: Option<String>,
    #[serde(default)]
    pub anchors: Vec<SceneAnchor>,
    #[serde(default)]
    pub player_entry: Option<ScenePlayerEntry>,
}

impl Serialize for Scene {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entities = self
            .entities
            .iter()
            .cloned()
            .map(|entity| {
                let id = entity.id;
                StoredEntity::new(entity, self.components.optional_components(id))
            })
            .collect::<Vec<_>>();
        SceneWire {
            name: self.name.clone(),
            description: self.description.clone(),
            maps: self.maps.clone(),
            entities,
            rules: self.rules.clone(),
            camera_position: self.camera_position,
            camera_scale: self.camera_scale,
            background_music_track_id: self.background_music_track_id.clone(),
            anchors: self.anchors.clone(),
            player_entry: self.player_entry.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Scene {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SceneWire::deserialize(deserializer)?;
        let mut scene = Scene::new(wire.name);
        scene.description = wire.description;
        scene.maps = wire.maps;
        scene.rules = wire.rules;
        scene.camera_position = wire.camera_position;
        scene.camera_scale = wire.camera_scale;
        scene.background_music_track_id = wire.background_music_track_id;
        scene.anchors = wire.anchors;
        scene.player_entry = wire.player_entry;
        for stored in wire.entities {
            scene.add_stored_entity(stored);
        }
        Ok(scene)
    }
}

impl Scene {
    /// Create a new empty scene with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            maps: Vec::new(),
            entities: Vec::new(),
            components: EntityComponentStore::default(),
            rules: RuleSet::default(),
            camera_position: None,
            camera_scale: None,
            background_music_track_id: None,
            anchors: Vec::new(),
            player_entry: None,
        }
    }

    /// Create a scene with maps
    pub fn with_maps(name: String, maps: Vec<String>) -> Self {
        Self {
            name,
            description: None,
            maps,
            entities: Vec::new(),
            components: EntityComponentStore::default(),
            rules: RuleSet::default(),
            camera_position: None,
            camera_scale: None,
            background_music_track_id: None,
            anchors: Vec::new(),
            player_entry: None,
        }
    }

    /// Add an entity to the scene
    pub fn add_entity(&mut self, entity: Entity) -> EntityId {
        self.add_stored_entity(StoredEntity::new(entity, EntityOptionalComponents::default()))
    }

    pub fn add_stored_entity(&mut self, stored: StoredEntity) -> EntityId {
        let id = stored.entity.id;
        self.components.set_optional_components(id, stored.components);
        self.entities.push(stored.entity);
        id
    }

    /// Remove an entity from the scene
    pub fn remove_entity(&mut self, entity_id: EntityId) -> bool {
        let initial_len = self.entities.len();
        self.entities.retain(|e| e.id != entity_id);
        let removed = self.entities.len() != initial_len;
        if removed {
            self.components.remove_all(entity_id);
        }
        removed
    }

    /// Get an entity by ID
    pub fn get_entity(&self, entity_id: EntityId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == entity_id)
    }

    /// Get a mutable reference to an entity by ID
    pub fn get_entity_mut(&mut self, entity_id: EntityId) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == entity_id)
    }

    pub fn stored_entity(&self, entity_id: EntityId) -> Option<StoredEntity> {
        self.get_entity(entity_id)
            .cloned()
            .map(|entity| StoredEntity::new(entity, self.components.optional_components(entity_id)))
    }

    pub fn optional_components(&self, entity_id: EntityId) -> EntityOptionalComponents {
        self.components.optional_components(entity_id)
    }

    /// Add a map to this scene
    pub fn add_map(&mut self, map_name: String) {
        if !self.maps.contains(&map_name) {
            self.maps.push(map_name);
        }
    }

    /// Remove a map from this scene
    pub fn remove_map(&mut self, map_name: &str) -> bool {
        let initial_len = self.maps.len();
        self.maps.retain(|m| m != map_name);
        self.maps.len() != initial_len
    }

    /// Check if this scene has a specific map
    pub fn has_map(&self, map_name: &str) -> bool {
        self.maps.contains(&map_name.to_string())
    }

    /// Add a scene anchor.
    pub fn add_anchor(&mut self, anchor: SceneAnchor) {
        self.anchors.push(anchor);
    }

    /// Remove an anchor by id.
    pub fn remove_anchor(&mut self, anchor_id: &str) -> bool {
        let initial_len = self.anchors.len();
        self.anchors.retain(|anchor| anchor.id != anchor_id);
        self.anchors.len() != initial_len
    }

    /// Get an anchor by id.
    pub fn get_anchor(&self, anchor_id: &str) -> Option<&SceneAnchor> {
        self.anchors.iter().find(|anchor| anchor.id == anchor_id)
    }

    /// Get a mutable anchor by id.
    pub fn get_anchor_mut(&mut self, anchor_id: &str) -> Option<&mut SceneAnchor> {
        self.anchors
            .iter_mut()
            .find(|anchor| anchor.id == anchor_id)
    }

    pub fn spawn_point_ids(&self) -> Vec<String> {
        self.anchors
            .iter()
            .filter(|anchor| matches!(anchor.kind, SceneAnchorKind::SpawnPoint))
            .map(|anchor| anchor.id.clone())
            .collect()
    }
}
