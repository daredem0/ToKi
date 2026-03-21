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
