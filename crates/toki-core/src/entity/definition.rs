//! Entity definition types for JSON deserialization.
//!
//! These types define the structure for loading entity definitions from files.

use super::builder::EntityBuilder;
use super::types::{
    AiBehavior, AiConfig, ControlRole, Entity, EntityAttributes, EntityAudioComponent,
    EntityAudioSettings, EntityId, EntityKind, EntityStats, Inventory, MovementProfile,
    MovementSoundTrigger, PickupDef, PrimaryProjectileDef, StaticObjectRenderDef, HEALTH_STAT_ID,
};
use crate::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_hearing_radius() -> u32 {
    192
}

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
    /// Legacy name-based frame references (e.g., ["player/walk_0", "player/walk_1"])
    #[serde(default)]
    pub frame_tiles: Vec<String>,
    /// Position-based frame references as grid [column, row] pairs (e.g., [[0, 0], [1, 0]])
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_positions: Option<Vec<[u32; 2]>>,
    /// Uniform frame duration in milliseconds (applies to all frames unless overridden)
    pub frame_duration_ms: f32,
    /// Optional per-frame duration overrides in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_durations_ms: Option<Vec<f32>>,
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

    /// Create an Entity instance from this definition at the given position.
    pub fn create_entity(&self, position: IVec2, entity_id: EntityId) -> Result<Entity, String> {
        let entity_kind = Self::runtime_entity_kind_for_category(&self.category);
        let animation_controller = self.build_animation_controller()?;
        let attributes = self.build_attributes(animation_controller);
        let collision_box = self.build_collision_box();
        let audio = self.build_audio_settings();

        let entity = EntityBuilder::new(
            entity_id,
            position,
            UVec2::new(self.rendering.size[0], self.rendering.size[1]),
            entity_kind,
        )
        .category(self.category.clone())
        .definition_name(self.name.clone())
        .control_role(ControlRole::LegacyDefault)
        .audio(audio)
        .attributes(attributes)
        .collision_box_opt(collision_box)
        .tags(self.tags.clone())
        .build();

        Ok(entity)
    }

    fn build_animation_controller(&self) -> Result<Option<AnimationController>, String> {
        if self.animations.clips.is_empty() {
            return Ok(None);
        }

        let mut controller = AnimationController::new();
        for clip_def in &self.animations.clips {
            let state = Self::parse_animation_state(&clip_def.state)?;
            let loop_mode = Self::parse_loop_mode(&clip_def.loop_mode)?;

            let clip = AnimationClip {
                state,
                atlas_name: self.animations.atlas_name.clone(),
                frame_tile_names: clip_def.frame_tiles.clone(),
                frame_positions: clip_def.frame_positions.clone(),
                frame_duration_ms: clip_def.frame_duration_ms,
                frame_durations_ms: clip_def.frame_durations_ms.clone(),
                loop_mode,
            };
            controller.add_clip(clip);
        }

        let default_state = Self::parse_animation_state(&self.animations.default_state)?;
        controller.play(default_state);
        Ok(Some(controller))
    }

    fn parse_loop_mode(mode: &str) -> Result<LoopMode, String> {
        match mode.to_lowercase().as_str() {
            "loop" => Ok(LoopMode::Loop),
            "once" => Ok(LoopMode::Once),
            "ping_pong" => Ok(LoopMode::PingPong),
            _ => Err(format!("Unknown loop mode: {mode}")),
        }
    }

    fn build_attributes(&self, animation_controller: Option<AnimationController>) -> EntityAttributes {
        let stats = self.build_stats();
        let mut attributes = EntityAttributes {
            health: self.attributes.health.or_else(|| {
                stats.base(HEALTH_STAT_ID).and_then(|v| u32::try_from(v).ok())
            }),
            stats,
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
        attributes
    }

    fn build_stats(&self) -> EntityStats {
        let mut stats = EntityStats::default();
        for (stat_id, value) in &self.attributes.stats {
            let authored_value = (*value).max(0);
            stats.base.insert(stat_id.clone(), authored_value);
            stats.current.insert(stat_id.clone(), authored_value);
        }
        if let Some(health) = self.attributes.health {
            let health = health as i32;
            stats.base.insert(HEALTH_STAT_ID.to_string(), health);
            stats.current.insert(HEALTH_STAT_ID.to_string(), health);
        }
        stats
    }

    fn build_collision_box(&self) -> Option<CollisionBox> {
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

    fn build_audio_settings(&self) -> EntityAudioSettings {
        let movement_sound = self.audio.movement_sound.trim();
        let movement_sound = if movement_sound.is_empty() {
            None
        } else {
            Some(movement_sound.to_string())
        };

        let collision_sound = self.audio.collision_sound.as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);

        EntityAudioSettings {
            footstep_trigger_distance: self.audio.footstep_trigger_distance,
            hearing_radius: self.audio.hearing_radius,
            movement_sound_trigger: self.audio.movement_sound_trigger,
            movement_sound,
            collision_sound,
        }
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
