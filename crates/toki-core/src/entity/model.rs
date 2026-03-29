use crate::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use crate::collision::CollisionBox;
use crate::ids::EntityDefName;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::EnumIter;

pub type EntityId = u32;
pub const HEALTH_STAT_ID: &str = "health";
pub const ATTACK_POWER_STAT_ID: &str = "attack_power";

const fn default_has_shadow() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityFootprint {
    pub offset: [i32; 2],
    pub size: [u32; 2],
}

impl EntityFootprint {
    pub const fn new(offset: [i32; 2], size: [u32; 2]) -> Self {
        Self { offset, size }
    }

    pub fn world_bounds(&self, entity_position: IVec2) -> (IVec2, UVec2) {
        (
            entity_position + IVec2::new(self.offset[0], self.offset[1]),
            UVec2::new(self.size[0], self.size[1]),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EntityGrounding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<[i32; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<EntityFootprint>,
}

impl EntityGrounding {
    pub fn is_empty(&self) -> bool {
        self.origin.is_none() && self.footprint.is_none()
    }

    pub fn resolved_origin(&self, render_size: UVec2) -> IVec2 {
        self.origin
            .map(|origin| IVec2::new(origin[0], origin[1]))
            .unwrap_or_else(|| {
                IVec2::new(
                    (render_size.x / 2) as i32,
                    render_size.y.saturating_sub(1) as i32,
                )
            })
    }

    pub fn resolved_footprint(
        &self,
        render_size: UVec2,
        collision_box: Option<&CollisionBox>,
    ) -> EntityFootprint {
        self.footprint.unwrap_or_else(|| {
            collision_box
                .map(|collision| {
                    EntityFootprint::new(
                        [collision.offset.x, collision.offset.y],
                        [collision.size.x, collision.size.y],
                    )
                })
                .unwrap_or_else(|| EntityFootprint::new([0, 0], [render_size.x, render_size.y]))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StaticObjectRenderDef {
    pub sheet: String,
    pub object_name: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_name: Option<EntityDefName>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub persistent_across_saves: bool,
    #[serde(default, skip_serializing_if = "ControlRole::is_legacy_default")]
    pub control_role: ControlRole,
    #[serde(default, skip_serializing_if = "EntityAudioSettings::is_default")]
    pub audio: EntityAudioSettings,
    pub attributes: EntityAttributes,
    pub collision_box: Option<CollisionBox>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip, default)]
    pub movement_accumulator: glam::Vec2,
}

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

const fn is_false(value: &bool) -> bool {
    !*value
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    pub behavior: AiBehavior,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityGameplay {
    pub health: Option<u32>,
    #[serde(default, skip_serializing_if = "EntityStats::is_empty")]
    pub stats: EntityStats,
    pub speed: f32,
    pub solid: bool,
}

impl Default for EntityGameplay {
    fn default() -> Self {
        Self {
            health: None,
            stats: EntityStats::default(),
            speed: 2.0,
            solid: true,
        }
    }
}

impl EntityGameplay {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRendering {
    pub visible: bool,
    #[serde(default = "default_has_shadow")]
    pub has_shadow: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette_override: Option<String>,
    pub animation_controller: Option<AnimationController>,
    pub render_layer: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_object_render: Option<StaticObjectRenderDef>,
    #[serde(default, skip_serializing_if = "EntityGrounding::is_empty")]
    pub grounding: EntityGrounding,
}

impl Default for EntityRendering {
    fn default() -> Self {
        Self {
            visible: true,
            has_shadow: true,
            palette_override: None,
            animation_controller: None,
            render_layer: 0,
            static_object_render: None,
            grounding: EntityGrounding::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntityBehavior {
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
    #[serde(default)]
    pub has_inventory: bool,
}

impl Default for EntityBehavior {
    fn default() -> Self {
        Self {
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::default(),
            has_inventory: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityAttributes {
    pub gameplay: EntityGameplay,
    pub rendering: EntityRendering,
    pub behavior: EntityBehavior,
}

impl Entity {
    pub fn interaction_bounds(&self) -> (IVec2, UVec2) {
        self.collision_box
            .as_ref()
            .map(|collision_box| collision_box.world_bounds(self.position))
            .unwrap_or((self.position, self.size))
    }

    pub fn resolved_ground_origin(&self) -> IVec2 {
        self.position
            + self
                .attributes
                .rendering
                .grounding
                .resolved_origin(self.size)
    }

    pub fn resolved_footprint(&self) -> EntityFootprint {
        self.attributes
            .rendering
            .grounding
            .resolved_footprint(self.size, self.collision_box.as_ref())
    }

    pub fn footprint_world_bounds(&self) -> (IVec2, UVec2) {
        self.resolved_footprint().world_bounds(self.position)
    }

    pub fn ground_contact_y(&self) -> i32 {
        let (footprint_pos, footprint_size) = self.footprint_world_bounds();
        footprint_pos.y + footprint_size.y as i32
    }

    pub fn effective_control_role(&self) -> ControlRole {
        self.control_role.resolved()
    }

    pub fn effective_movement_profile(&self) -> MovementProfile {
        self.attributes
            .behavior
            .movement_profile
            .resolved_for_control_role(self.effective_control_role())
    }
}

impl EntityAttributes {
    pub fn ensure_legacy_health_stat(&mut self) {
        self.gameplay.ensure_legacy_health_stat();
    }

    pub fn current_stat(&self, stat_id: &str) -> Option<i32> {
        self.gameplay.current_stat(stat_id)
    }

    pub fn base_stat(&self, stat_id: &str) -> Option<i32> {
        self.gameplay.base_stat(stat_id)
    }

    pub fn apply_stat_delta(&mut self, stat_id: &str, delta: i32) -> Option<i32> {
        self.gameplay.apply_stat_delta(stat_id, delta)
    }
}
