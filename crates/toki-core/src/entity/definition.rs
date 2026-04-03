//! Entity definition types for JSON deserialization.
//!
//! These types define the structure for loading entity definitions from files.

use super::builder::EntityBuilder;
use super::components::{AiComponent, CombatComponent, InteractionComponent, MovementComponent};
use super::model::{
    ControlRole, Entity, EntityAudioComponent, EntityAudioSettings, EntityGrounding, EntityId,
    EntityKind, EntityRendering, MovementSoundTrigger, StaticObjectRenderDef,
};
use super::runtime_entity_kind_for_category;
use super::storage::{EntitySpawnBundle, OptionalEntityComponents};
use super::{Inventory, PickupDef, PrimaryProjectileDef};
use crate::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use crate::collision::CollisionBox;
use crate::ids::EntityDefName;
use glam::{IVec2, UVec2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

fn default_hearing_radius() -> u32 {
    192
}

fn default_has_shadow() -> bool {
    true
}

const fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDefinition {
    pub name: EntityDefName,
    pub display_name: String,
    pub description: String,
    pub rendering: RenderingDef,
    pub solid: bool,
    pub active: bool,
    #[serde(default, skip_serializing_if = "ComponentsDef::is_empty")]
    pub components: ComponentsDef,
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
    #[serde(default = "default_has_shadow")]
    pub has_shadow: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_drop_shadow: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_object: Option<StaticObjectRenderDef>,
    #[serde(default, skip_serializing_if = "EntityGrounding::is_empty")]
    pub grounding: EntityGrounding,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentsDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub movement: Option<MovementComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<AiComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<InteractionComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<CombatComponent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_projectile: Option<PrimaryProjectileDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pickup: Option<PickupDef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<Inventory>,
}

impl ComponentsDef {
    pub fn is_empty(&self) -> bool {
        self.movement.is_none()
            && self.ai.is_none()
            && self.interaction.is_none()
            && self.combat.is_none()
            && self.primary_projectile.is_none()
            && self.pickup.is_none()
            && self.inventory.is_none()
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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EntityDefinitionError {
    #[error("unknown animation state: {state}")]
    UnknownAnimationState { state: String },
    #[error("unknown loop mode: {mode}")]
    UnknownLoopMode { mode: String },
}

// Conversion implementations
impl EntityDefinition {
    fn parse_animation_state(state: &str) -> Result<AnimationState, EntityDefinitionError> {
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
            _ => Err(EntityDefinitionError::UnknownAnimationState {
                state: state.to_string(),
            }),
        }
    }

    /// Create an Entity instance from this definition at the given position.
    pub fn create_spawn_bundle(
        &self,
        position: IVec2,
        entity_id: EntityId,
    ) -> Result<EntitySpawnBundle, EntityDefinitionError> {
        let entity_kind = runtime_entity_kind_for_category(&self.category);
        let animation_controller = self.build_animation_controller()?;
        let grounding = self.build_grounding();
        let rendering = self.build_rendering(animation_controller, grounding.clone());
        let optional_components = self.build_components();
        let collision_box = self.build_collision_box(&grounding);
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
        .audio(audio.clone())
        .rendering(rendering)
        .solid(self.solid)
        .active(self.active)
        .collision_box_opt(collision_box)
        .tags(self.tags.clone())
        .build();

        Ok(EntitySpawnBundle {
            entity,
            optional_components,
            audio_component: audio.to_component(),
        })
    }

    pub fn create_entity(
        &self,
        position: IVec2,
        entity_id: EntityId,
    ) -> Result<Entity, EntityDefinitionError> {
        Ok(self.create_spawn_bundle(position, entity_id)?.entity)
    }

    fn build_animation_controller(
        &self,
    ) -> Result<Option<AnimationController>, EntityDefinitionError> {
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

    fn parse_loop_mode(mode: &str) -> Result<LoopMode, EntityDefinitionError> {
        match mode.to_lowercase().as_str() {
            "loop" => Ok(LoopMode::Loop),
            "once" => Ok(LoopMode::Once),
            "ping_pong" => Ok(LoopMode::PingPong),
            _ => Err(EntityDefinitionError::UnknownLoopMode {
                mode: mode.to_string(),
            }),
        }
    }

    fn build_rendering(
        &self,
        animation_controller: Option<AnimationController>,
        grounding: EntityGrounding,
    ) -> EntityRendering {
        EntityRendering {
            visible: self.rendering.visible,
            has_shadow: self.rendering.has_shadow,
            has_drop_shadow: self.rendering.has_drop_shadow,
            palette_override: self.rendering.palette_override.clone(),
            animation_controller,
            render_layer: self.rendering.render_layer,
            static_object_render: self.rendering.static_object.clone(),
            grounding,
        }
    }

    fn build_components(&self) -> OptionalEntityComponents {
        let mut combat = self.components.combat.clone();
        if let Some(combat) = combat.as_mut() {
            combat.ensure_health_stat();
        }

        OptionalEntityComponents {
            movement: self.components.movement,
            ai: self.components.ai,
            interaction: self.components.interaction,
            combat,
            primary_projectile: self.components.primary_projectile.clone(),
            projectile: None,
            pickup: self.components.pickup.clone(),
            inventory: self.components.inventory.clone(),
        }
    }

    fn build_grounding(&self) -> EntityGrounding {
        let mut grounding = self.rendering.grounding.clone();
        if grounding.footprint.is_none() {
            grounding.footprint = Some(self.legacy_collision_footprint());
        }
        grounding
    }

    fn legacy_collision_footprint(&self) -> super::model::EntityFootprint {
        super::model::EntityFootprint::new(
            [self.collision.offset[0], self.collision.offset[1]],
            [self.collision.size[0], self.collision.size[1]],
        )
    }

    fn build_collision_box(&self, grounding: &EntityGrounding) -> Option<CollisionBox> {
        if self.collision.enabled {
            let footprint = grounding
                .footprint
                .unwrap_or_else(|| self.legacy_collision_footprint());
            Some(CollisionBox::new(
                IVec2::new(footprint.offset[0], footprint.offset[1]),
                UVec2::new(footprint.size[0], footprint.size[1]),
                self.collision.trigger,
            ))
        } else {
            None
        }
    }

    fn normalize_audio_id(value: &str) -> Option<String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    fn normalized_audio_settings(&self) -> EntityAudioSettings {
        EntityAudioSettings {
            footstep_trigger_distance: self.audio.footstep_trigger_distance,
            hearing_radius: self.audio.hearing_radius,
            movement_sound_trigger: self.audio.movement_sound_trigger,
            movement_sound: Self::normalize_audio_id(&self.audio.movement_sound),
            collision_sound: self
                .audio
                .collision_sound
                .as_deref()
                .and_then(Self::normalize_audio_id),
        }
    }

    fn build_audio_settings(&self) -> EntityAudioSettings {
        self.normalized_audio_settings()
    }

    /// Build a runtime audio component from this definition.
    pub fn create_audio_component(&self) -> EntityAudioComponent {
        self.build_audio_settings().to_component()
    }

    /// Get collision box from entity definition without creating full entity.
    /// Useful for placement validation.
    pub fn get_collision_box(&self) -> Option<CollisionBox> {
        let grounding = self.build_grounding();
        self.build_collision_box(&grounding)
    }

    /// Resolve the effective grounding that runtime entities spawned from this definition use.
    pub fn get_grounding(&self) -> EntityGrounding {
        self.build_grounding()
    }
}
