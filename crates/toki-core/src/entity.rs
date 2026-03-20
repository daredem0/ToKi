use crate::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use strum::EnumIter;

pub type EntityId = u32;
pub const HEALTH_STAT_ID: &str = "health";
pub const ATTACK_POWER_STAT_ID: &str = "attack_power";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PickupDef {
    pub item_id: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticObjectRenderDef {
    pub sheet: String,
    pub object_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Inventory {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub items: HashMap<String, u32>,
}

impl Inventory {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn item_count(&self, item_id: &str) -> u32 {
        self.items.get(item_id).copied().unwrap_or(0)
    }

    pub fn add_item(&mut self, item_id: &str, count: u32) {
        if item_id.is_empty() || count == 0 {
            return;
        }

        let entry = self.items.entry(item_id.to_string()).or_insert(0);
        *entry = entry.saturating_add(count);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrimaryProjectileDef {
    pub sheet: String,
    pub object_name: String,
    pub size: [u32; 2],
    pub speed: u32,
    pub damage: i32,
    pub lifetime_ticks: u32,
    #[serde(default)]
    pub spawn_offset: [i32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectileState {
    pub sheet: String,
    pub object_name: String,
    pub size: [u32; 2],
    pub velocity: [i32; 2],
    pub remaining_ticks: u32,
    pub damage: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<EntityId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub position: glam::IVec2,
    pub size: glam::UVec2,
    #[serde(alias = "entity_type")]
    pub entity_kind: EntityKind,
    #[serde(default)]
    pub category: String,
    /// Source entity definition name used to instantiate this entity.
    /// This lets editor workflows (e.g. drag-to-move) re-enter placement mode without guessing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_name: Option<String>,
    #[serde(default, skip_serializing_if = "ControlRole::is_legacy_default")]
    pub control_role: ControlRole,
    #[serde(default, skip_serializing_if = "EntityAudioSettings::is_default")]
    pub audio: EntityAudioSettings,
    pub attributes: EntityAttributes,
    pub collision_box: Option<CollisionBox>,
    /// Tags for categorizing and filtering entities in rules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Runtime-only accumulator for sub-pixel movement.
    /// Allows speeds < 1.0 to accumulate over frames until a full pixel is reached.
    #[serde(skip, default)]
    pub movement_accumulator: glam::Vec2,
}

/// Runtime audio component attached to an entity.
///
/// This keeps transient audio behavior out of the core `Entity` model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityAudioComponent {
    pub footstep_distance_accumulator: f32,
    pub footstep_trigger_distance: f32,
    #[serde(default = "default_hearing_radius")]
    pub hearing_radius: u32,
    #[serde(default)]
    pub movement_sound_trigger: MovementSoundTrigger,
    pub last_collision_state: bool,
    #[serde(default)]
    pub movement_sound: Option<String>,
    #[serde(default)]
    pub collision_sound: Option<String>,
}

impl Default for EntityAudioComponent {
    fn default() -> Self {
        Self {
            footstep_distance_accumulator: 0.0,
            footstep_trigger_distance: 32.0,
            hearing_radius: default_hearing_radius(),
            movement_sound_trigger: MovementSoundTrigger::default(),
            last_collision_state: false,
            movement_sound: None,
            collision_sound: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MovementSoundTrigger {
    #[default]
    Distance,
    AnimationLoop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityAudioSettings {
    pub footstep_trigger_distance: f32,
    #[serde(default = "default_hearing_radius")]
    pub hearing_radius: u32,
    #[serde(default)]
    pub movement_sound_trigger: MovementSoundTrigger,
    #[serde(default)]
    pub movement_sound: Option<String>,
    #[serde(default)]
    pub collision_sound: Option<String>,
}

impl Default for EntityAudioSettings {
    fn default() -> Self {
        Self {
            footstep_trigger_distance: 32.0,
            hearing_radius: default_hearing_radius(),
            movement_sound_trigger: MovementSoundTrigger::default(),
            movement_sound: None,
            collision_sound: None,
        }
    }
}

impl EntityAudioSettings {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    pub fn to_component(&self) -> EntityAudioComponent {
        EntityAudioComponent {
            footstep_distance_accumulator: 0.0,
            footstep_trigger_distance: self.footstep_trigger_distance,
            hearing_radius: self.hearing_radius,
            movement_sound_trigger: self.movement_sound_trigger,
            last_collision_state: false,
            movement_sound: self.movement_sound.clone(),
            collision_sound: self.collision_sound.clone(),
        }
    }
}

fn default_hearing_radius() -> u32 {
    192
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize, EnumIter)]
pub enum EntityKind {
    Player,
    Npc,
    Item,
    Decoration,
    Trigger,
    Projectile,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ControlRole {
    #[default]
    LegacyDefault,
    None,
    PlayerCharacter,
}

impl ControlRole {
    pub fn resolved(self) -> Self {
        match self {
            Self::LegacyDefault => Self::None,
            explicit => explicit,
        }
    }

    pub fn is_legacy_default(&self) -> bool {
        matches!(self, Self::LegacyDefault)
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiBehavior {
    #[default]
    None,
    Wander,
    Chase,
    Run,
    RunAndMultiply,
}

/// Authored AI configuration for an entity.
/// This replaces the bare `AiBehavior` flag with a structured config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    pub behavior: AiBehavior,
    /// Detection radius in pixels for Chase, Run, and RunAndMultiply behaviors
    #[serde(default)]
    pub detection_radius: u32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            behavior: AiBehavior::None,
            detection_radius: 0,
        }
    }
}

impl AiConfig {
    /// Create an AiConfig from a legacy AiBehavior value
    pub fn from_legacy_behavior(behavior: AiBehavior) -> Self {
        Self {
            behavior,
            detection_radius: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MovementProfile {
    #[default]
    LegacyDefault,
    None,
    PlayerWasd,
}

impl MovementProfile {
    pub fn resolved_for_control_role(self, control_role: ControlRole) -> Self {
        match self {
            Self::LegacyDefault => match control_role.resolved() {
                ControlRole::PlayerCharacter => Self::PlayerWasd,
                ControlRole::LegacyDefault | ControlRole::None => Self::None,
            },
            explicit => explicit,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAttributes {
    // Core gameplay
    pub health: Option<u32>,
    #[serde(default, skip_serializing_if = "EntityStats::is_empty")]
    pub stats: EntityStats,
    pub speed: f32,  // Movement speed in pixels per tick
    pub solid: bool, // Can we collide with other entities

    // Rendering
    pub visible: bool, // Can we be seen by the player
    pub animation_controller: Option<AnimationController>, // takes care of all animations of the entity
    pub render_layer: i32,                                 // lower layers are drawn first

    // Behavior flags
    pub active: bool,
    pub can_move: bool, // Can we be moved by the player
    #[serde(default)]
    pub interactable: bool, // Can player interact with this entity
    #[serde(default)]
    pub interaction_reach: u32, // Extra pixels of reach for interaction (0 = must overlap)
    #[serde(default)]
    pub ai_config: AiConfig,
    #[serde(default)]
    pub movement_profile: MovementProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_projectile: Option<PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile: Option<ProjectileState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_object_render: Option<StaticObjectRenderDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<PickupDef>,
    #[serde(default, skip_serializing_if = "Inventory::is_empty")]
    pub inventory: Inventory,

    // Extended attributes for entity definitions
    #[serde(default)]
    pub has_inventory: bool, // Can this entity carry items
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EntityStats {
    #[serde(default)]
    pub base: HashMap<String, i32>,
    #[serde(default)]
    pub current: HashMap<String, i32>,
}

impl EntityStats {
    pub fn is_empty(&self) -> bool {
        self.base.is_empty() && self.current.is_empty()
    }

    pub fn from_legacy_health(health: Option<u32>) -> Self {
        let mut stats = Self::default();
        if let Some(health) = health {
            let health = health as i32;
            stats.base.insert(HEALTH_STAT_ID.to_string(), health);
            stats.current.insert(HEALTH_STAT_ID.to_string(), health);
        }
        stats
    }

    pub fn current(&self, stat_id: &str) -> Option<i32> {
        self.current
            .get(stat_id)
            .copied()
            .or_else(|| self.base.get(stat_id).copied())
    }

    pub fn base(&self, stat_id: &str) -> Option<i32> {
        self.base.get(stat_id).copied()
    }

    pub fn ensure_stat(&mut self, stat_id: &str, value: i32) {
        self.base.entry(stat_id.to_string()).or_insert(value);
        self.current.entry(stat_id.to_string()).or_insert(value);
    }

    pub fn apply_delta(&mut self, stat_id: &str, delta: i32) -> Option<i32> {
        let current = self.current.get_mut(stat_id)?;
        *current = (*current + delta).max(0);
        Some(*current)
    }
}

impl Default for EntityAttributes {
    fn default() -> Self {
        Self {
            health: None,
            stats: EntityStats::default(),
            speed: 2.0,
            solid: true,
            visible: true,
            animation_controller: None,
            render_layer: 0,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::default(),
            primary_projectile: None,
            projectile: None,
            static_object_render: None,
            pickup: None,
            inventory: Inventory::default(),
            has_inventory: false,
        }
    }
}

impl Entity {
    pub fn effective_control_role(&self) -> ControlRole {
        self.control_role.resolved()
    }

    pub fn effective_movement_profile(&self) -> MovementProfile {
        self.attributes
            .movement_profile
            .resolved_for_control_role(self.effective_control_role())
    }
}

impl EntityAttributes {
    pub fn ensure_legacy_health_stat(&mut self) {
        if let Some(health) = self.health {
            self.stats.ensure_stat(HEALTH_STAT_ID, health as i32);
        }
    }

    pub fn current_stat(&self, stat_id: &str) -> Option<i32> {
        self.stats.current(stat_id)
    }

    pub fn base_stat(&self, stat_id: &str) -> Option<i32> {
        self.stats.base(stat_id)
    }

    pub fn apply_stat_delta(&mut self, stat_id: &str, delta: i32) -> Option<i32> {
        let new_value = self.stats.apply_delta(stat_id, delta)?;
        if stat_id == HEALTH_STAT_ID {
            self.health = u32::try_from(new_value).ok();
        }
        Some(new_value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityManager {
    entities: HashMap<EntityId, Entity>,
    next_id: EntityId,

    // Quick lookups
    player_id: Option<EntityId>,
    entities_by_kind: HashMap<EntityKind, HashSet<EntityId>>,

    // This is prepared for spatial queries (collission)
    active_entities: HashSet<EntityId>,

    /// Runtime audio components keyed by entity id.
    #[serde(default)]
    audio_components: HashMap<EntityId, EntityAudioComponent>,
}

impl EntityManager {
    fn tracks_player_role(entity: &Entity) -> bool {
        matches!(
            entity.effective_control_role(),
            ControlRole::PlayerCharacter
        )
    }

    fn legacy_category_for_kind(entity_kind: &EntityKind) -> &'static str {
        match entity_kind {
            EntityKind::Player => "human",
            EntityKind::Npc => "creature",
            EntityKind::Item => "item",
            EntityKind::Decoration => "decoration",
            EntityKind::Trigger => "trigger",
            EntityKind::Projectile => "projectile",
        }
    }

    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1, // we start at 1 to use 0 for invalid entities
            player_id: None,
            entities_by_kind: HashMap::new(),
            active_entities: HashSet::new(),
            audio_components: HashMap::new(),
        }
    }

    /// Update animations for all entities
    pub fn update_animations(&mut self, delta_time_ms: f32) -> HashMap<EntityId, u32> {
        let mut completed_loops = HashMap::new();
        for (entity_id, entity) in &mut self.entities {
            if let Some(animation_controller) = &mut entity.attributes.animation_controller {
                let loop_count = animation_controller.update(delta_time_ms);
                if loop_count > 0 {
                    completed_loops.insert(*entity_id, loop_count);
                }
            }
        }
        completed_loops
    }

    pub fn spawn_entity(
        &mut self,
        entity_kind: EntityKind,
        position: IVec2,
        size: UVec2,
        mut attributes: EntityAttributes,
    ) -> EntityId {
        attributes.ensure_legacy_health_stat();
        let id = self.next_id;
        self.next_id += 1;
        // Create a default collision box for solid entities
        let collision_box = if attributes.solid {
            Some(CollisionBox::solid_box(size))
        } else {
            None
        };

        let entity = Entity {
            id,
            position,
            size,
            entity_kind,
            category: Self::legacy_category_for_kind(&entity_kind).to_string(),
            definition_name: None,
            control_role: ControlRole::LegacyDefault,
            audio: EntityAudioSettings::default(),
            attributes,
            collision_box,
            tags: Vec::new(),
            movement_accumulator: glam::Vec2::ZERO,
        };
        self.audio_components
            .insert(id, EntityAudioComponent::default());

        // Insert into main storage
        self.entities.insert(id, entity);

        // Update lookup tables
        if Self::tracks_player_role(self.entities.get(&id).unwrap()) {
            self.player_id = Some(id);
        }

        self.entities_by_kind
            .entry(entity_kind)
            .or_default()
            .insert(id);

        if self.entities.get(&id).unwrap().attributes.active {
            self.active_entities.insert(id);
        }

        id
    }

    /// Spawn an entity from an entity definition.
    pub fn spawn_from_definition(
        &mut self,
        definition: &EntityDefinition,
        position: IVec2,
    ) -> Result<EntityId, String> {
        let id = self.next_id;
        self.next_id += 1;

        let entity = definition.create_entity(position, id)?;
        let entity_kind = entity.entity_kind;
        let audio_component = definition.create_audio_component();

        if Self::tracks_player_role(&entity) {
            self.player_id = Some(id);
        }

        self.entities_by_kind
            .entry(entity_kind)
            .or_default()
            .insert(id);

        if entity.attributes.active {
            self.active_entities.insert(id);
        }

        self.entities.insert(id, entity);
        self.audio_components.insert(id, audio_component);
        Ok(id)
    }

    /// Add an existing entity to the manager (used for scene-to-gamestate conversion)
    pub fn add_existing_entity(&mut self, mut entity: Entity) -> EntityId {
        entity.attributes.ensure_legacy_health_stat();

        let id = entity.id;
        let entity_kind = entity.entity_kind;

        // Update next_id if needed to avoid conflicts
        if id >= self.next_id {
            self.next_id = id + 1;
        }

        // Track player entity
        if Self::tracks_player_role(&entity) && self.player_id.is_none() {
            self.player_id = Some(id);
        }

        // Update lookups
        self.entities_by_kind
            .entry(entity_kind)
            .or_default()
            .insert(id);

        self.active_entities.insert(id);
        self.audio_components
            .insert(id, entity.audio.to_component());

        // Store the entity
        self.entities.insert(id, entity);

        tracing::trace!("Added existing entity {} to EntityManager", id);
        id
    }

    pub fn despawn_entity(&mut self, id: EntityId) -> bool {
        let Some(entity) = self.entities.remove(&id) else {
            return false;
        };

        // Clean up lookup tables
        if self.player_id.is_some_and(|pid| pid == id) {
            self.player_id = None;
        }

        if let Some(kind_set) = self.entities_by_kind.get_mut(&entity.entity_kind) {
            kind_set.remove(&id);
        }

        // We don't care whether it was present; just ensure it's gone.
        self.active_entities.remove(&id);
        self.audio_components.remove(&id);

        true
    }

    // Basic getters
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn set_control_role(&mut self, id: EntityId, control_role: ControlRole) -> bool {
        let Some(entity) = self.entities.get_mut(&id) else {
            return false;
        };

        entity.control_role = control_role;
        if matches!(
            entity.effective_control_role(),
            ControlRole::PlayerCharacter
        ) {
            self.player_id = Some(id);
        } else if self.player_id == Some(id) {
            self.player_id = None;
        }
        true
    }

    pub fn audio_component(&self, id: EntityId) -> Option<&EntityAudioComponent> {
        self.audio_components.get(&id)
    }

    pub fn audio_component_mut(&mut self, id: EntityId) -> Option<&mut EntityAudioComponent> {
        self.audio_components.get_mut(&id)
    }

    pub fn get_entity_with_audio_mut(
        &mut self,
        id: EntityId,
    ) -> Option<(&mut Entity, &mut EntityAudioComponent)> {
        let (entities, audio_components) = (&mut self.entities, &mut self.audio_components);
        let entity = entities.get_mut(&id)?;
        let audio_component = audio_components.entry(id).or_default();
        Some((entity, audio_component))
    }

    // Convenience methods
    pub fn get_player(&self) -> Option<&Entity> {
        self.player_id.and_then(|id| self.entities.get(&id))
    }

    pub fn get_player_mut(&mut self) -> Option<&mut Entity> {
        self.player_id.and_then(|id| self.entities.get_mut(&id))
    }

    pub fn get_player_id(&self) -> Option<EntityId> {
        self.player_id
    }

    // Queries
    pub fn entities_of_kind(&self, entity_kind: &EntityKind) -> Vec<EntityId> {
        self.entities_by_kind
            .get(entity_kind)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn active_entities(&self) -> Vec<EntityId> {
        self.active_entities.iter().copied().collect()
    }

    pub fn would_collide_with_solid_entity(
        &self,
        moving_entity_id: EntityId,
        new_position: IVec2,
    ) -> bool {
        self.find_colliding_entity(moving_entity_id, new_position)
            .is_some()
    }

    /// Finds the first solid entity that would collide with `moving_entity_id`
    /// if it moved to `new_position`.
    ///
    /// Returns `Some(entity_id)` of the colliding entity, or `None` if no collision.
    pub fn find_colliding_entity(
        &self,
        moving_entity_id: EntityId,
        new_position: IVec2,
    ) -> Option<EntityId> {
        let moving_entity = self.entities.get(&moving_entity_id)?;
        let moving_box = moving_entity.collision_box.as_ref()?;
        if moving_box.trigger || !moving_entity.attributes.solid {
            return None;
        }

        let (moving_pos, moving_size) = moving_box.world_bounds(new_position);

        for other_id in &self.active_entities {
            if *other_id == moving_entity_id {
                continue;
            }

            let Some(other_entity) = self.entities.get(other_id) else {
                continue;
            };
            if !other_entity.attributes.solid {
                continue;
            }

            let Some(other_box) = &other_entity.collision_box else {
                continue;
            };
            if other_box.trigger {
                continue;
            }

            let (other_pos, other_size) = other_box.world_bounds(other_entity.position);
            if crate::collision::aabb_overlap(moving_pos, moving_size, other_pos, other_size) {
                return Some(*other_id);
            }
        }

        None
    }

    pub fn visible_entities(&self) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, entity)| entity.attributes.visible)
            .map(|(id, _)| *id)
            .collect()
    }

    // Update entity active status
    pub fn set_entity_active(&mut self, id: EntityId, active: bool) {
        if let Some(entity) = self.entities.get_mut(&id) {
            let was_active = entity.attributes.active;
            entity.attributes.active = active;
            // Update active_entities set
            if active && !was_active {
                self.active_entities.insert(id);
            } else if !active && was_active {
                self.active_entities.remove(&id);
            }
        }
    }
}
impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

// Entity Definition JSON format structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDefinition {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub rendering: RenderingDef,
    pub attributes: AttributesDef,
    pub collision: CollisionDef,
    pub audio: AudioDef,
    pub animations: AnimationsDef,
    #[serde(alias = "entity_type")]
    pub category: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderingDef {
    pub size: [u32; 2],
    pub render_layer: i32,
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_object: Option<StaticObjectRenderDef>,
}

/// Wire format for deserializing attributes with backward compatibility
#[derive(Debug, Clone, Deserialize)]
struct AttributesDefWire {
    pub health: Option<u32>,
    #[serde(default)]
    pub stats: HashMap<String, i32>,
    pub speed: f32,
    pub solid: bool,
    pub active: bool,
    pub can_move: bool,
    #[serde(default)]
    pub interactable: bool,
    #[serde(default)]
    pub interaction_reach: u32,
    /// Legacy field for backward compatibility
    #[serde(default)]
    pub ai_behavior: Option<AiBehavior>,
    /// New AI configuration (takes precedence over ai_behavior)
    #[serde(default)]
    pub ai_config: Option<AiConfig>,
    #[serde(default)]
    pub movement_profile: MovementProfile,
    #[serde(default)]
    pub primary_projectile: Option<PrimaryProjectileDef>,
    #[serde(default)]
    pub pickup: Option<PickupDef>,
    pub has_inventory: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttributesDef {
    pub health: Option<u32>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub stats: HashMap<String, i32>,
    pub speed: f32,
    pub solid: bool,
    pub active: bool,
    pub can_move: bool,
    #[serde(default)]
    pub interactable: bool,
    #[serde(default)]
    pub interaction_reach: u32,
    #[serde(default)]
    pub ai_config: AiConfig,
    #[serde(default)]
    pub movement_profile: MovementProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_projectile: Option<PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<PickupDef>,
    pub has_inventory: bool,
}

impl<'de> Deserialize<'de> for AttributesDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = AttributesDefWire::deserialize(deserializer)?;

        // Resolve AI config: ai_config takes precedence over legacy ai_behavior
        let ai_config = match (wire.ai_config, wire.ai_behavior) {
            (Some(config), _) => config,
            (None, Some(behavior)) => AiConfig::from_legacy_behavior(behavior),
            (None, None) => AiConfig::default(),
        };

        Ok(Self {
            health: wire.health,
            stats: wire.stats,
            speed: wire.speed,
            solid: wire.solid,
            active: wire.active,
            can_move: wire.can_move,
            interactable: wire.interactable,
            interaction_reach: wire.interaction_reach,
            ai_config,
            movement_profile: wire.movement_profile,
            primary_projectile: wire.primary_projectile,
            pickup: wire.pickup,
            has_inventory: wire.has_inventory,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionDef {
    pub enabled: bool,
    pub offset: [i32; 2],
    pub size: [u32; 2],
    pub trigger: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDef {
    pub footstep_trigger_distance: f32,
    #[serde(default = "default_hearing_radius")]
    pub hearing_radius: u32,
    #[serde(default)]
    pub movement_sound_trigger: MovementSoundTrigger,
    pub movement_sound: String,
    #[serde(default)]
    pub collision_sound: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationsDef {
    pub atlas_name: String,
    pub clips: Vec<AnimationClipDef>,
    pub default_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClipDef {
    pub state: String,
    pub frame_tiles: Vec<String>,
    pub frame_duration_ms: f32,
    pub loop_mode: String, // "loop", "once", "ping_pong"
}

// Conversion implementations
impl EntityDefinition {
    fn runtime_entity_kind_for_category(category: &str) -> EntityKind {
        match category.trim().to_ascii_lowercase().as_str() {
            "item" | "items" => EntityKind::Item,
            "trigger" | "triggers" => EntityKind::Trigger,
            "projectile" | "projectiles" => EntityKind::Projectile,
            "decoration" | "decorations" | "building" | "buildings" | "plant" | "plants" => {
                EntityKind::Decoration
            }
            _ => EntityKind::Npc,
        }
    }

    fn parse_animation_state(state: &str) -> Result<AnimationState, String> {
        match state.to_lowercase().as_str() {
            "idle" => Ok(AnimationState::Idle),
            "walk" => Ok(AnimationState::Walk),
            "attack" => Ok(AnimationState::Attack),
            "idle_down" => Ok(AnimationState::IdleDown),
            "idle_up" => Ok(AnimationState::IdleUp),
            "idle_left" => Ok(AnimationState::IdleLeft),
            "idle_right" => Ok(AnimationState::IdleRight),
            "walk_down" => Ok(AnimationState::WalkDown),
            "walk_up" => Ok(AnimationState::WalkUp),
            "walk_left" => Ok(AnimationState::WalkLeft),
            "walk_right" => Ok(AnimationState::WalkRight),
            "attack_down" => Ok(AnimationState::AttackDown),
            "attack_up" => Ok(AnimationState::AttackUp),
            "attack_left" => Ok(AnimationState::AttackLeft),
            "attack_right" => Ok(AnimationState::AttackRight),
            _ => Err(format!("Unknown animation state: {state}")),
        }
    }

    /// Create an Entity instance from this definition at the given position
    pub fn create_entity(&self, position: IVec2, entity_id: EntityId) -> Result<Entity, String> {
        let entity_kind = Self::runtime_entity_kind_for_category(&self.category);

        // Build animation controller when clips are authored.
        let animation_controller = if self.animations.clips.is_empty() {
            None
        } else {
            let mut animation_controller = AnimationController::new();
            for clip_def in &self.animations.clips {
                let state = Self::parse_animation_state(&clip_def.state)?;

                let loop_mode = match clip_def.loop_mode.to_lowercase().as_str() {
                    "loop" => LoopMode::Loop,
                    "once" => LoopMode::Once,
                    "ping_pong" => LoopMode::PingPong,
                    _ => return Err(format!("Unknown loop mode: {}", clip_def.loop_mode)),
                };

                let clip = AnimationClip {
                    state,
                    atlas_name: self.animations.atlas_name.clone(),
                    frame_tile_names: clip_def.frame_tiles.clone(),
                    frame_duration_ms: clip_def.frame_duration_ms,
                    loop_mode,
                };
                animation_controller.add_clip(clip);
            }

            let default_state = Self::parse_animation_state(&self.animations.default_state)?;
            animation_controller.play(default_state);
            Some(animation_controller)
        };

        // Build attributes
        let mut authored_stats = EntityStats::default();
        for (stat_id, value) in &self.attributes.stats {
            let authored_value = (*value).max(0);
            authored_stats.base.insert(stat_id.clone(), authored_value);
            authored_stats
                .current
                .insert(stat_id.clone(), authored_value);
        }
        if let Some(health) = self.attributes.health {
            let health = health as i32;
            authored_stats
                .base
                .insert(HEALTH_STAT_ID.to_string(), health);
            authored_stats
                .current
                .insert(HEALTH_STAT_ID.to_string(), health);
        }

        let mut attributes = EntityAttributes {
            health: self.attributes.health.or_else(|| {
                authored_stats
                    .base(HEALTH_STAT_ID)
                    .and_then(|value| u32::try_from(value).ok())
            }),
            stats: authored_stats,
            speed: self.attributes.speed,
            solid: self.attributes.solid,
            visible: self.rendering.visible,
            animation_controller,
            render_layer: self.rendering.render_layer,
            active: self.attributes.active,
            can_move: self.attributes.can_move,
            interactable: self.attributes.interactable,
            interaction_reach: self.attributes.interaction_reach,
            ai_config: self.attributes.ai_config,
            movement_profile: self.attributes.movement_profile,
            primary_projectile: self.attributes.primary_projectile.clone(),
            projectile: None,
            static_object_render: self.rendering.static_object.clone(),
            pickup: self.attributes.pickup.clone(),
            inventory: Inventory::default(),
            has_inventory: self.attributes.has_inventory,
        };
        attributes.ensure_legacy_health_stat();

        // Build collision box if enabled
        let collision_box = if self.collision.enabled {
            Some(CollisionBox::new(
                IVec2::new(self.collision.offset[0], self.collision.offset[1]),
                UVec2::new(self.collision.size[0], self.collision.size[1]),
                self.collision.trigger,
            ))
        } else {
            None
        };

        let movement_sound = {
            let trimmed = self.audio.movement_sound.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        let collision_sound = self
            .audio
            .collision_sound
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        // Create entity
        Ok(Entity {
            id: entity_id,
            position,
            size: UVec2::new(self.rendering.size[0], self.rendering.size[1]),
            entity_kind,
            category: self.category.clone(),
            definition_name: Some(self.name.clone()),
            control_role: ControlRole::LegacyDefault,
            audio: EntityAudioSettings {
                footstep_trigger_distance: self.audio.footstep_trigger_distance,
                hearing_radius: self.audio.hearing_radius,
                movement_sound_trigger: self.audio.movement_sound_trigger,
                movement_sound,
                collision_sound,
            },
            attributes,
            collision_box,
            tags: self.tags.clone(),
            movement_accumulator: glam::Vec2::ZERO,
        })
    }

    /// Build a runtime audio component from this definition.
    pub fn create_audio_component(&self) -> EntityAudioComponent {
        let movement_sound = {
            let trimmed = self.audio.movement_sound.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        let collision_sound = self
            .audio
            .collision_sound
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        EntityAudioComponent {
            footstep_distance_accumulator: 0.0,
            footstep_trigger_distance: self.audio.footstep_trigger_distance,
            hearing_radius: self.audio.hearing_radius,
            movement_sound_trigger: self.audio.movement_sound_trigger,
            last_collision_state: false,
            movement_sound,
            collision_sound,
        }
    }

    /// Get collision box from entity definition without creating full entity.
    /// Useful for placement validation.
    pub fn get_collision_box(&self) -> Option<CollisionBox> {
        if self.collision.enabled {
            Some(CollisionBox::new(
                IVec2::new(self.collision.offset[0], self.collision.offset[1]),
                UVec2::new(self.collision.size[0], self.collision.size[1]),
                self.collision.trigger,
            ))
        } else {
            None
        }
    }
}
