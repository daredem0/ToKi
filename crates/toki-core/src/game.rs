use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::animation::{AnimationController, AnimationState};
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::collision;
use crate::entity::{
    AiBehavior, Entity, EntityAttributes, EntityId, EntityKind, EntityManager, MovementProfile,
    MovementSoundTrigger, ATTACK_POWER_STAT_ID, HEALTH_STAT_ID,
};
use crate::events::{GameEvent, GameUpdateResult};
use crate::rules::{
    Rule, RuleAction, RuleCondition, RuleKey, RuleSet, RuleSoundChannel, RuleSpawnEntityType,
    RuleTarget, RuleTrigger,
};
use crate::scene_manager::SceneManager;
use crate::sprite::{SpriteFrame, SpriteInstance};

/// Core input keys abstraction (platform-independent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputKey {
    Up,
    Down,
    Left,
    Right,
    DebugToggle, // F4 key for toggling debug rendering
                 // Can extend with more keys as needed
}

/// Profile-scoped action buttons that can be mapped independently from movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InputAction {
    Primary,
}

/// Audio events that can be triggered by game logic
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioEvent {
    /// Play a one-shot sound effect on a logical channel.
    PlaySound {
        channel: AudioChannel,
        sound_id: String,
        source_position: Option<glam::IVec2>,
        hearing_radius: Option<u32>,
    },
    /// Start background music
    BackgroundMusic(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannel {
    Movement,
    Collision,
}

impl GameEvent for AudioEvent {}

/// Core game state that manages entities, scenes, input, and game logic.
///
/// This is platform-independent and contains pure game logic without
/// any runtime or windowing dependencies.
#[derive(Debug, Serialize, Deserialize)]
pub struct GameState {
    /// Scene manager holding all scenes
    scene_manager: SceneManager,

    /// Entity manager for all game objects in the current scene
    entity_manager: EntityManager,

    /// Player entity ID for quick access
    player_id: Option<EntityId>,

    /// Currently held input keys
    #[serde(default)]
    keys_held: HashSet<InputKey>,

    /// Runtime-held movement input scoped by movement profile.
    #[serde(skip, default)]
    profile_keys_held: HashMap<MovementProfile, HashSet<InputKey>>,

    /// Held profile-scoped action buttons used to debounce edge-triggered actions.
    #[serde(skip, default)]
    profile_actions_held: HashMap<MovementProfile, HashSet<InputAction>>,

    /// Pending one-shot profile-scoped action requests to be consumed during update.
    #[serde(skip, default)]
    pending_profile_actions: HashMap<MovementProfile, HashSet<InputAction>>,

    /// Game configuration constants
    movement_step: i32,
    sprite_size: u32,

    /// Debug rendering flags
    #[serde(default)]
    debug_collision_rendering: bool,

    /// Frame counter for NPC AI decisions
    #[serde(default)]
    npc_ai_frame_counter: u32,

    /// Data-driven gameplay rules evaluated each frame.
    #[serde(default)]
    rules: RuleSet,

    /// Runtime-only rule execution state.
    #[serde(skip, default)]
    rule_runtime: RuleRuntimeState,

    /// Pending generic stat changes gathered during update and resolved centrally.
    #[serde(skip, default)]
    pending_stat_changes: Vec<StatChangeRequest>,
}

#[derive(Debug, Default)]
struct RuleRuntimeState {
    started: bool,
    fired_once_rules: HashSet<String>,
    velocities: HashMap<EntityId, glam::IVec2>,
    frame_collision_detected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FacingDirection {
    Down,
    Up,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuleCommand {
    PlaySound {
        channel: AudioChannel,
        sound_id: String,
    },
    PlayMusic {
        track_id: String,
    },
    SetVelocity {
        entity_id: EntityId,
        velocity: glam::IVec2,
    },
    PlayAnimation {
        entity_id: EntityId,
        state: AnimationState,
    },
    Spawn {
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    },
    DestroySelf {
        entity_id: EntityId,
    },
    SwitchScene {
        scene_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatChangeRequest {
    target_entity_id: EntityId,
    stat_id: String,
    delta: i32,
    source_entity_id: Option<EntityId>,
}

impl GameState {
    fn effective_movement_profile(entity: &Entity) -> MovementProfile {
        entity.effective_movement_profile()
    }

    fn controlled_input_entity_ids(&self) -> Vec<EntityId> {
        let mut entity_ids = self
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                let entity = self.entity_manager.get_entity(entity_id)?;
                if matches!(
                    Self::effective_movement_profile(entity),
                    MovementProfile::PlayerWasd
                ) {
                    Some(entity_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        entity_ids.sort_unstable();
        entity_ids
    }

    fn held_keys_for_profile(&self, movement_profile: MovementProfile) -> Vec<InputKey> {
        let mut held_keys = self.keys_held.iter().copied().collect::<HashSet<_>>();
        if let Some(profile_keys) = self.profile_keys_held.get(&movement_profile) {
            held_keys.extend(profile_keys.iter().copied());
        }
        let mut held_keys = held_keys.into_iter().collect::<Vec<_>>();
        held_keys.sort_by_key(|key| Self::input_key_order(*key));
        held_keys
    }

    fn all_held_keys(&self) -> Vec<InputKey> {
        let mut held_keys = self.keys_held.clone();
        for profile_keys in self.profile_keys_held.values() {
            held_keys.extend(profile_keys.iter().copied());
        }
        let mut held_keys = held_keys.into_iter().collect::<Vec<_>>();
        held_keys.sort_by_key(|key| Self::input_key_order(*key));
        held_keys
    }

    fn movement_delta_from_keys(keys: &[InputKey]) -> glam::IVec2 {
        let mut delta = glam::IVec2::ZERO;
        for key in keys {
            match key {
                InputKey::Up => delta.y -= 1,
                InputKey::Down => delta.y += 1,
                InputKey::Left => delta.x -= 1,
                InputKey::Right => delta.x += 1,
                InputKey::DebugToggle => {}
            }
        }
        delta
    }

    fn candidate_input_position(
        &self,
        current_position: glam::IVec2,
        key: InputKey,
        world_bounds: glam::UVec2,
    ) -> glam::IVec2 {
        match key {
            InputKey::Up => glam::IVec2::new(
                current_position.x,
                (current_position.y - self.movement_step).max(0),
            ),
            InputKey::Left => glam::IVec2::new(
                (current_position.x - self.movement_step).max(0),
                current_position.y,
            ),
            InputKey::Down => glam::IVec2::new(
                current_position.x,
                (current_position.y + self.movement_step)
                    .min(world_bounds.y as i32 - self.sprite_size as i32),
            ),
            InputKey::Right => glam::IVec2::new(
                (current_position.x + self.movement_step)
                    .min(world_bounds.x as i32 - self.sprite_size as i32),
                current_position.y,
            ),
            InputKey::DebugToggle => current_position,
        }
    }

    fn apply_input_to_entity(
        &mut self,
        entity_id: EntityId,
        key: InputKey,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        if matches!(key, InputKey::DebugToggle) {
            return;
        }

        let Some(current_position) = self
            .entity_manager
            .get_entity(entity_id)
            .map(|entity| entity.position)
        else {
            return;
        };

        let new_position = self.candidate_input_position(current_position, key, world_bounds);
        if new_position == current_position {
            return;
        }

        if self.can_entity_move_to_position(entity_id, new_position, tilemap, atlas) {
            if let Some((entity, entity_audio)) =
                self.entity_manager.get_entity_with_audio_mut(entity_id)
            {
                entity.position = new_position;
                entity_audio.last_collision_state = false;
            }
        } else {
            self.handle_entity_collision_blocked(entity_id, result);
        }
    }

    fn handle_entity_collision_blocked(
        &mut self,
        entity_id: EntityId,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        let source_position = self
            .entity_manager
            .get_entity(entity_id)
            .map(|entity| entity.position);
        let Some(entity_audio) = self.entity_manager.audio_component_mut(entity_id) else {
            self.rule_runtime.frame_collision_detected = true;
            return;
        };

        let collision_started = !entity_audio.last_collision_state;
        if collision_started {
            if let Some(collision_sound) = entity_audio
                .collision_sound
                .as_deref()
                .filter(|sound_id| !sound_id.is_empty())
            {
                result.add_event(AudioEvent::PlaySound {
                    channel: AudioChannel::Collision,
                    sound_id: collision_sound.to_string(),
                    source_position,
                    hearing_radius: Some(entity_audio.hearing_radius),
                });
            }
            self.rule_runtime.frame_collision_detected = true;
        }
        entity_audio.last_collision_state = true;
    }

    fn can_entity_move_to_position(
        &self,
        entity_id: EntityId,
        new_position: glam::IVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> bool {
        let Some(entity) = self.entity_manager.get_entity(entity_id) else {
            return false;
        };

        collision::can_entity_move_to_position(entity, new_position, tilemap, atlas)
            && !self
                .entity_manager
                .would_collide_with_solid_entity(entity_id, new_position)
    }

    fn movement_distance(from: glam::IVec2, to: glam::IVec2) -> f32 {
        let delta = to - from;
        ((delta.x.pow(2) + delta.y.pow(2)) as f32).sqrt()
    }

    fn emit_entity_movement_audio(
        &mut self,
        entity_id: EntityId,
        distance_moved: f32,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        if distance_moved <= 0.0 {
            return;
        }

        let source_position = self
            .entity_manager
            .get_entity(entity_id)
            .map(|entity| entity.position);
        let Some(entity_audio) = self.entity_manager.audio_component_mut(entity_id) else {
            return;
        };

        entity_audio.footstep_distance_accumulator += distance_moved;

        if matches!(
            entity_audio.movement_sound_trigger,
            MovementSoundTrigger::Distance
        ) {
            if entity_audio.footstep_trigger_distance <= 0.0 {
                if let Some(movement_sound) = entity_audio
                    .movement_sound
                    .as_deref()
                    .filter(|sound_id| !sound_id.is_empty())
                {
                    result.add_event(AudioEvent::PlaySound {
                        channel: AudioChannel::Movement,
                        sound_id: movement_sound.to_string(),
                        source_position,
                        hearing_radius: Some(entity_audio.hearing_radius),
                    });
                }
                return;
            }

            while entity_audio.footstep_distance_accumulator
                >= entity_audio.footstep_trigger_distance
            {
                if let Some(movement_sound) = entity_audio
                    .movement_sound
                    .as_deref()
                    .filter(|sound_id| !sound_id.is_empty())
                {
                    result.add_event(AudioEvent::PlaySound {
                        channel: AudioChannel::Movement,
                        sound_id: movement_sound.to_string(),
                        source_position,
                        hearing_radius: Some(entity_audio.hearing_radius),
                    });
                }
                entity_audio.footstep_distance_accumulator -=
                    entity_audio.footstep_trigger_distance;
            }
        }
    }

    fn emit_animation_loop_movement_audio(
        &mut self,
        entity_id: EntityId,
        completed_loops: u32,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        if completed_loops == 0 {
            return;
        }

        let source_position = self
            .entity_manager
            .get_entity(entity_id)
            .map(|entity| entity.position);
        let Some(entity_audio) = self.entity_manager.audio_component_mut(entity_id) else {
            return;
        };

        if !matches!(
            entity_audio.movement_sound_trigger,
            MovementSoundTrigger::AnimationLoop
        ) || entity_audio.footstep_distance_accumulator <= 0.0
        {
            return;
        }

        let Some(movement_sound) = entity_audio
            .movement_sound
            .as_deref()
            .filter(|sound_id| !sound_id.is_empty())
        else {
            entity_audio.footstep_distance_accumulator = 0.0;
            return;
        };

        for _ in 0..completed_loops {
            result.add_event(AudioEvent::PlaySound {
                channel: AudioChannel::Movement,
                sound_id: movement_sound.to_string(),
                source_position,
                hearing_radius: Some(entity_audio.hearing_radius),
            });
        }
        entity_audio.footstep_distance_accumulator = 0.0;
    }

    fn facing_from_delta(delta: glam::IVec2) -> Option<FacingDirection> {
        if delta == glam::IVec2::ZERO {
            return None;
        }
        if delta.x.abs() > delta.y.abs() {
            if delta.x < 0 {
                Some(FacingDirection::Left)
            } else {
                Some(FacingDirection::Right)
            }
        } else if delta.y < 0 {
            Some(FacingDirection::Up)
        } else {
            Some(FacingDirection::Down)
        }
    }

    fn facing_from_animation_state(state: AnimationState) -> FacingDirection {
        match state {
            AnimationState::IdleUp | AnimationState::WalkUp | AnimationState::AttackUp => {
                FacingDirection::Up
            }
            AnimationState::IdleLeft | AnimationState::WalkLeft | AnimationState::AttackLeft => {
                FacingDirection::Left
            }
            AnimationState::IdleRight | AnimationState::WalkRight | AnimationState::AttackRight => {
                FacingDirection::Right
            }
            AnimationState::Idle
            | AnimationState::Walk
            | AnimationState::Attack
            | AnimationState::IdleDown
            | AnimationState::WalkDown
            | AnimationState::AttackDown => FacingDirection::Down,
        }
    }

    fn directional_animation_state(moving: bool, facing: FacingDirection) -> AnimationState {
        match (moving, facing) {
            (false, FacingDirection::Down) => AnimationState::IdleDown,
            (false, FacingDirection::Up) => AnimationState::IdleUp,
            (false, FacingDirection::Left) => AnimationState::IdleLeft,
            (false, FacingDirection::Right) => AnimationState::IdleRight,
            (true, FacingDirection::Down) => AnimationState::WalkDown,
            (true, FacingDirection::Up) => AnimationState::WalkUp,
            (true, FacingDirection::Left) => AnimationState::WalkLeft,
            (true, FacingDirection::Right) => AnimationState::WalkRight,
        }
    }

    fn animation_state_flip_x(state: AnimationState) -> bool {
        matches!(
            state,
            AnimationState::IdleLeft | AnimationState::WalkLeft | AnimationState::AttackLeft
        )
    }

    fn directional_attack_state(facing: FacingDirection) -> AnimationState {
        match facing {
            FacingDirection::Down => AnimationState::AttackDown,
            FacingDirection::Up => AnimationState::AttackUp,
            FacingDirection::Left => AnimationState::AttackLeft,
            FacingDirection::Right => AnimationState::AttackRight,
        }
    }

    fn is_action_animation_state(state: AnimationState) -> bool {
        matches!(
            state,
            AnimationState::Attack
                | AnimationState::AttackDown
                | AnimationState::AttackUp
                | AnimationState::AttackLeft
                | AnimationState::AttackRight
        )
    }

    fn action_animation_locks_locomotion(animation_controller: &AnimationController) -> bool {
        Self::is_action_animation_state(animation_controller.current_clip_state)
            && !animation_controller.is_finished
    }

    fn primary_action_damage_for_entity(entity: &Entity) -> i32 {
        entity
            .attributes
            .current_stat(ATTACK_POWER_STAT_ID)
            .or_else(|| entity.attributes.base_stat(ATTACK_POWER_STAT_ID))
            .unwrap_or(10)
    }

    fn entity_bounds_for_stat_interaction(entity: &Entity) -> (glam::IVec2, glam::UVec2) {
        if let Some(collision_box) = &entity.collision_box {
            collision_box.world_bounds(entity.position)
        } else {
            (entity.position, entity.size)
        }
    }

    fn primary_action_hitbox(
        entity: &Entity,
        facing: FacingDirection,
    ) -> (glam::IVec2, glam::UVec2) {
        let (origin, size) = Self::entity_bounds_for_stat_interaction(entity);
        match facing {
            FacingDirection::Down => (glam::IVec2::new(origin.x, origin.y + size.y as i32), size),
            FacingDirection::Up => (glam::IVec2::new(origin.x, origin.y - size.y as i32), size),
            FacingDirection::Left => (glam::IVec2::new(origin.x - size.x as i32, origin.y), size),
            FacingDirection::Right => (glam::IVec2::new(origin.x + size.x as i32, origin.y), size),
        }
    }

    fn collect_primary_action_stat_changes(
        &self,
        attacker_id: EntityId,
        facing: FacingDirection,
    ) -> Vec<StatChangeRequest> {
        let Some(attacker) = self.entity_manager.get_entity(attacker_id) else {
            return Vec::new();
        };

        let damage = Self::primary_action_damage_for_entity(attacker);
        if damage <= 0 {
            return Vec::new();
        }

        let (hitbox_pos, hitbox_size) = Self::primary_action_hitbox(attacker, facing);
        let mut target_ids = self.entity_manager.active_entities();
        target_ids.sort_unstable();

        let changes = target_ids
            .into_iter()
            .filter(|&target_id| target_id != attacker_id)
            .filter_map(|target_id| {
                let target = self.entity_manager.get_entity(target_id)?;
                if !target.attributes.active
                    || target.attributes.current_stat(HEALTH_STAT_ID).is_none()
                {
                    return None;
                }
                let (target_pos, target_size) = Self::entity_bounds_for_stat_interaction(target);
                if !collision::aabb_overlap(hitbox_pos, hitbox_size, target_pos, target_size) {
                    return None;
                }
                Some(StatChangeRequest {
                    target_entity_id: target_id,
                    stat_id: HEALTH_STAT_ID.to_string(),
                    delta: -damage,
                    source_entity_id: Some(attacker_id),
                })
            })
            .collect::<Vec<_>>();

        if changes.is_empty() {
            tracing::debug!(
                "Primary action from entity {} facing {:?} produced no damage targets",
                attacker_id,
                facing
            );
        } else {
            for change in &changes {
                tracing::debug!(
                    "Primary action from entity {} queued {} change {} for target {}",
                    attacker_id,
                    change.stat_id,
                    change.delta,
                    change.target_entity_id
                );
            }
        }

        changes
    }

    fn resolve_pending_stat_changes(&mut self) {
        let pending_stat_changes = std::mem::take(&mut self.pending_stat_changes);
        if pending_stat_changes.is_empty() {
            return;
        }

        let mut despawn_ids = Vec::new();
        for change in pending_stat_changes {
            let Some(entity) = self.entity_manager.get_entity_mut(change.target_entity_id) else {
                continue;
            };
            let previous_value = entity.attributes.current_stat(&change.stat_id);
            let Some(new_value) = entity
                .attributes
                .apply_stat_delta(&change.stat_id, change.delta)
            else {
                continue;
            };

            tracing::debug!(
                "Applied stat change: source={:?} target={} stat={} delta={} previous={:?} new={}",
                change.source_entity_id,
                change.target_entity_id,
                change.stat_id,
                change.delta,
                previous_value,
                new_value
            );

            if change.stat_id == HEALTH_STAT_ID && new_value <= 0 {
                tracing::info!(
                    "Entity {} reached zero {} and will be despawned",
                    change.target_entity_id,
                    change.stat_id
                );
                despawn_ids.push(change.target_entity_id);
            }
        }

        despawn_ids.sort_unstable();
        despawn_ids.dedup();
        for entity_id in despawn_ids {
            self.entity_manager.despawn_entity(entity_id);
        }
    }

    fn trigger_entity_primary_action(&mut self, entity_id: EntityId) -> bool {
        let triggered_facing = {
            let Some(animation_controller) = self
                .entity_manager
                .get_entity_mut(entity_id)
                .and_then(|entity| entity.attributes.animation_controller.as_mut())
            else {
                return false;
            };

            let facing = Self::facing_from_animation_state(animation_controller.current_clip_state);
            let directional_attack = Self::directional_attack_state(facing);
            let next_state = if animation_controller.has_clip(directional_attack) {
                directional_attack
            } else if animation_controller.has_clip(AnimationState::Attack) {
                AnimationState::Attack
            } else {
                return false;
            };

            if animation_controller.play(next_state) {
                Some(facing)
            } else {
                None
            }
        };

        let Some(facing) = triggered_facing else {
            return false;
        };

        tracing::debug!(
            "Entity {} triggered primary action facing {:?}",
            entity_id,
            facing
        );

        self.pending_stat_changes
            .extend(self.collect_primary_action_stat_changes(entity_id, facing));
        true
    }

    fn process_profile_actions(&mut self) {
        let pending_actions = std::mem::take(&mut self.pending_profile_actions);
        if pending_actions.is_empty() {
            return;
        }

        let controlled_entity_ids = self.controlled_input_entity_ids();
        if controlled_entity_ids.is_empty() {
            return;
        }

        for (profile, actions) in pending_actions {
            if !actions.contains(&InputAction::Primary) {
                continue;
            }
            for &entity_id in &controlled_entity_ids {
                let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                    continue;
                };
                if Self::effective_movement_profile(entity) != profile {
                    continue;
                }
                self.trigger_entity_primary_action(entity_id);
            }
        }
    }

    fn resolve_animation_state(
        animation_controller: &AnimationController,
        moving: bool,
        delta: glam::IVec2,
    ) -> AnimationState {
        let fallback = if moving {
            AnimationState::Walk
        } else {
            AnimationState::Idle
        };

        let facing = Self::facing_from_delta(delta).unwrap_or_else(|| {
            Self::facing_from_animation_state(animation_controller.current_clip_state)
        });
        let directional = Self::directional_animation_state(moving, facing);

        if animation_controller.has_clip(directional) {
            directional
        } else {
            fallback
        }
    }

    /// Create a new GameState with the given player sprite
    pub fn new(player_sprite: SpriteInstance) -> Self {
        let mut entity_manager = EntityManager::new();

        // Create player entity at the sprite's initial position
        let player_def = Self::default_player_definition();
        let player_id = entity_manager
            .spawn_from_definition(&player_def, player_sprite.position)
            .expect("default player definition should always be valid");
        entity_manager.set_control_role(player_id, crate::entity::ControlRole::PlayerCharacter);
        if let Some(player) = entity_manager.get_entity_mut(player_id) {
            player.entity_kind = crate::entity::EntityKind::Player;
        }

        Self {
            scene_manager: SceneManager::new(),
            entity_manager,
            player_id: Some(player_id),
            keys_held: HashSet::new(),
            profile_keys_held: HashMap::new(),
            profile_actions_held: HashMap::new(),
            pending_profile_actions: HashMap::new(),
            movement_step: 1, // Move exactly 1 pixel per frame
            sprite_size: 16,  // Sprite is 16×16 pixels
            debug_collision_rendering: false,
            npc_ai_frame_counter: 0,
            rules: RuleSet::default(),
            rule_runtime: RuleRuntimeState::default(),
            pending_stat_changes: Vec::new(),
        }
    }

    /// Create a new empty GameState with no entities
    pub fn new_empty() -> Self {
        Self {
            scene_manager: SceneManager::new(),
            entity_manager: EntityManager::new(),
            player_id: None,
            keys_held: HashSet::new(),
            profile_keys_held: HashMap::new(),
            profile_actions_held: HashMap::new(),
            pending_profile_actions: HashMap::new(),
            movement_step: 1,
            sprite_size: 16,
            debug_collision_rendering: false,
            npc_ai_frame_counter: 0,
            rules: RuleSet::default(),
            rule_runtime: RuleRuntimeState::default(),
            pending_stat_changes: Vec::new(),
        }
    }

    /// Initialize the game with a player at the specified position
    pub fn spawn_player_at(&mut self, position: glam::IVec2) -> EntityId {
        let player_def = Self::default_player_definition();
        let player_id = self
            .entity_manager
            .spawn_from_definition(&player_def, position)
            .expect("default player definition should always be valid");
        self.entity_manager
            .set_control_role(player_id, crate::entity::ControlRole::PlayerCharacter);
        if let Some(player) = self.entity_manager.get_entity_mut(player_id) {
            player.entity_kind = crate::entity::EntityKind::Player;
        }
        self.player_id = Some(player_id);
        player_id
    }

    /// Spawn an NPC that looks identical to the player
    pub fn spawn_player_like_npc(&mut self, position: glam::IVec2) -> EntityId {
        let npc_def = Self::player_like_npc_definition();
        self.entity_manager
            .spawn_from_definition(&npc_def, position)
            .expect("default player-like npc definition should always be valid")
    }

    fn default_player_definition() -> crate::entity::EntityDefinition {
        crate::entity::EntityDefinition {
            name: "player".to_string(),
            display_name: "Player".to_string(),
            description: "Default player entity".to_string(),
            rendering: crate::entity::RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
            },
            attributes: crate::entity::AttributesDef {
                health: Some(100),
                speed: 2,
                solid: true,
                active: true,
                can_move: true,
                ai_behavior: crate::entity::AiBehavior::None,
                movement_profile: crate::entity::MovementProfile::PlayerWasd,
                has_inventory: false,
            },
            collision: crate::entity::CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
            },
            audio: crate::entity::AudioDef {
                footstep_trigger_distance: 32.0,
                hearing_radius: 192,
                movement_sound_trigger: crate::entity::MovementSoundTrigger::Distance,
                movement_sound: "sfx_slime_bounce".to_string(),
                collision_sound: Some("sfx_hit2".to_string()),
            },
            animations: crate::entity::AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: vec![
                    crate::entity::AnimationClipDef {
                        state: "idle".to_string(),
                        frame_tiles: vec!["slime/idle_0".to_string(), "slime/idle_1".to_string()],
                        frame_duration_ms: 300.0,
                        loop_mode: "loop".to_string(),
                    },
                    crate::entity::AnimationClipDef {
                        state: "walk".to_string(),
                        frame_tiles: vec![
                            "slime/walk_0".to_string(),
                            "slime/walk_1".to_string(),
                            "slime/walk_2".to_string(),
                            "slime/walk_3".to_string(),
                        ],
                        frame_duration_ms: 150.0,
                        loop_mode: "loop".to_string(),
                    },
                ],
                default_state: "idle".to_string(),
            },
            category: "human".to_string(),
            tags: vec!["player".to_string()],
        }
    }

    fn player_like_npc_definition() -> crate::entity::EntityDefinition {
        crate::entity::EntityDefinition {
            name: "player_like_npc".to_string(),
            display_name: "Player-like NPC".to_string(),
            description: "NPC using the player visual style".to_string(),
            rendering: crate::entity::RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
            },
            attributes: crate::entity::AttributesDef {
                health: Some(50),
                speed: 1,
                solid: true,
                active: true,
                can_move: false,
                ai_behavior: crate::entity::AiBehavior::Wander,
                movement_profile: crate::entity::MovementProfile::None,
                has_inventory: false,
            },
            collision: crate::entity::CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
            },
            audio: crate::entity::AudioDef {
                footstep_trigger_distance: 32.0,
                hearing_radius: 192,
                movement_sound_trigger: crate::entity::MovementSoundTrigger::Distance,
                movement_sound: "sfx_slime_bounce".to_string(),
                collision_sound: Some("sfx_hit2".to_string()),
            },
            animations: crate::entity::AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: vec![
                    crate::entity::AnimationClipDef {
                        state: "idle".to_string(),
                        frame_tiles: vec!["slime/idle_0".to_string(), "slime/idle_1".to_string()],
                        frame_duration_ms: 300.0,
                        loop_mode: "loop".to_string(),
                    },
                    crate::entity::AnimationClipDef {
                        state: "walk".to_string(),
                        frame_tiles: vec![
                            "slime/walk_0".to_string(),
                            "slime/walk_1".to_string(),
                            "slime/walk_2".to_string(),
                            "slime/walk_3".to_string(),
                        ],
                        frame_duration_ms: 150.0,
                        loop_mode: "loop".to_string(),
                    },
                ],
                default_state: "idle".to_string(),
            },
            category: "human".to_string(),
            tags: vec!["npc".to_string()],
        }
    }

    /// Update game state by one tick
    pub fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        let mut result = GameUpdateResult::new();
        let mut rule_commands = Vec::new();
        self.rule_runtime.frame_collision_detected = false;

        if !self.rule_runtime.started {
            self.collect_rule_commands_for_trigger(RuleTrigger::OnStart, &mut rule_commands);
            self.rule_runtime.started = true;
        }
        self.collect_rule_commands_for_trigger(RuleTrigger::OnUpdate, &mut rule_commands);
        self.collect_rule_commands_for_key_triggers(&mut rule_commands);
        let (mut pending_rule_animations, mut pending_scene_switch) =
            self.apply_rule_commands(rule_commands, &mut result);

        let initial_player_position = self
            .player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|entity| entity.position)
            .unwrap_or(glam::IVec2::ZERO);

        let input_result = self.process_input(world_bounds, tilemap, atlas);
        result.player_moved = input_result.player_moved;
        result.add_events(input_result.events);

        if self.apply_rule_velocities(world_bounds, tilemap, atlas, &mut result) {
            result.player_moved = true;
        }

        let intended_player_delta = self
            .player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|entity| self.held_keys_for_profile(Self::effective_movement_profile(entity)))
            .map(|keys| Self::movement_delta_from_keys(&keys))
            .unwrap_or(glam::IVec2::ZERO);

        // Pick moving or idle animation
        if let Some(player_entity) = self.entity_manager.get_player_mut() {
            if let Some(animation_controller) = &mut player_entity.attributes.animation_controller {
                if !Self::action_animation_locks_locomotion(animation_controller) {
                    let actual_player_delta = player_entity.position - initial_player_position;
                    let player_delta = if actual_player_delta == glam::IVec2::ZERO {
                        intended_player_delta
                    } else {
                        actual_player_delta
                    };
                    let desired_player_animation = Self::resolve_animation_state(
                        animation_controller,
                        result.player_moved,
                        player_delta,
                    );
                    if animation_controller.current_clip_state != desired_player_animation {
                        tracing::debug!(
                            "Changing clip from  {:?} to {:?}",
                            animation_controller.current_clip_state,
                            desired_player_animation
                        );
                        animation_controller.play(desired_player_animation);
                    }
                }
            }
        }

        self.process_profile_actions();
        self.resolve_pending_stat_changes();

        // Update NPC AI
        self.update_npc_ai(world_bounds, tilemap, atlas, &mut result);

        let mut reactive_rule_commands = Vec::new();
        if result.player_moved {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnPlayerMove,
                &mut reactive_rule_commands,
            );
        }
        if self.rule_runtime.frame_collision_detected {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnCollision,
                &mut reactive_rule_commands,
            );
        }
        if self.any_entity_overlaps_trigger_tile(tilemap, atlas) {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnTrigger,
                &mut reactive_rule_commands,
            );
        }
        let (mut reactive_animations, reactive_scene_switch) =
            self.apply_rule_commands(reactive_rule_commands, &mut result);
        if pending_scene_switch.is_none() {
            pending_scene_switch = reactive_scene_switch;
        }
        pending_rule_animations.append(&mut reactive_animations);

        self.apply_rule_animations(pending_rule_animations);

        // Update entity animation timing and emit animation-loop-based movement sounds.
        let completed_animation_loops = self.entity_manager.update_animations(17.0);
        for (entity_id, completed_loops) in completed_animation_loops {
            self.emit_animation_loop_movement_audio(entity_id, completed_loops, &mut result);
        }

        if let Some(scene_name) = pending_scene_switch {
            self.apply_rule_scene_switch(&scene_name);
        }

        result
    }

    /// Update NPC AI - makes NPCs move randomly every few frames
    fn update_npc_ai(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
        result: &mut GameUpdateResult<AudioEvent>,
    ) {
        self.npc_ai_frame_counter += 1;

        // Only update NPC AI every 60 frames (roughly once per second at 60fps)
        if !self.npc_ai_frame_counter.is_multiple_of(60) {
            return;
        }

        let npc_entity_ids: Vec<_> = self
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                    // Skip the player entity
                    if Some(entity_id) == self.player_id {
                        return None;
                    }
                    if matches!(entity.attributes.ai_behavior, AiBehavior::Wander) {
                        return Some(entity_id);
                    }
                }
                None
            })
            .collect();

        for npc_id in npc_entity_ids {
            let Some(current_position) = self
                .entity_manager
                .get_entity(npc_id)
                .map(|entity| entity.position)
            else {
                continue;
            };

            // Choose random direction: 0=up, 1=down, 2=left, 3=right, 4=stay
            let random_direction = fastrand::u32(0..5);

            let new_position = match random_direction {
                0 => glam::IVec2::new(
                    current_position.x,
                    (current_position.y - self.movement_step * 5).max(0),
                ),
                1 => glam::IVec2::new(
                    current_position.x,
                    (current_position.y + self.movement_step * 5)
                        .min(world_bounds.y as i32 - self.sprite_size as i32),
                ),
                2 => glam::IVec2::new(
                    (current_position.x - self.movement_step * 5).max(0),
                    current_position.y,
                ),
                3 => glam::IVec2::new(
                    (current_position.x + self.movement_step * 5)
                        .min(world_bounds.x as i32 - self.sprite_size as i32),
                    current_position.y,
                ),
                4 => current_position, // Stay in place
                _ => current_position,
            };

            let npc_moved = if new_position != current_position {
                if self.can_entity_move_to_position(npc_id, new_position, tilemap, atlas) {
                    if let Some(npc_entity) = self.entity_manager.get_entity_mut(npc_id) {
                        npc_entity.position = new_position;
                    }
                    self.emit_entity_movement_audio(
                        npc_id,
                        Self::movement_distance(current_position, new_position),
                        result,
                    );
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if let Some(npc_entity) = self.entity_manager.get_entity_mut(npc_id) {
                // Update NPC animation based on movement
                if let Some(animation_controller) = &mut npc_entity.attributes.animation_controller
                {
                    let desired_animation = if npc_moved {
                        AnimationState::Walk
                    } else {
                        AnimationState::Idle
                    };

                    if animation_controller.current_clip_state != desired_animation {
                        animation_controller.play(desired_animation);
                    }
                }
            }
        }
    }

    /// Process input and update player position
    /// Returns GameUpdateResult with movement info and audio events
    fn process_input(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> GameUpdateResult<AudioEvent> {
        let controlled_entity_ids = self.controlled_input_entity_ids();
        if controlled_entity_ids.is_empty() {
            return GameUpdateResult::new();
        }

        let mut intended_deltas = HashMap::new();
        let initial_positions = controlled_entity_ids
            .iter()
            .filter_map(|&entity_id| {
                self.entity_manager
                    .get_entity(entity_id)
                    .map(|entity| (entity_id, entity.position))
            })
            .collect::<HashMap<_, _>>();
        let mut result = GameUpdateResult::new();

        for &entity_id in &controlled_entity_ids {
            let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                continue;
            };
            let held_keys = self.held_keys_for_profile(Self::effective_movement_profile(entity));
            intended_deltas.insert(entity_id, Self::movement_delta_from_keys(&held_keys));
            for key in held_keys {
                self.apply_input_to_entity(
                    entity_id,
                    key,
                    world_bounds,
                    tilemap,
                    atlas,
                    &mut result,
                );
            }
        }

        for &entity_id in &controlled_entity_ids {
            let Some(initial_position) = initial_positions.get(&entity_id).copied() else {
                continue;
            };
            let Some(final_entity) = self.entity_manager.get_entity(entity_id) else {
                continue;
            };
            let final_position = final_entity.position;
            let entity_moved = final_position != initial_position;

            if Some(entity_id) == self.player_id {
                result.player_moved = entity_moved;
            }

            if let Some(animation_controller) = self
                .entity_manager
                .get_entity_mut(entity_id)
                .and_then(|entity| entity.attributes.animation_controller.as_mut())
            {
                if !Self::action_animation_locks_locomotion(animation_controller) {
                    let actual_delta = final_position - initial_position;
                    let delta = if actual_delta == glam::IVec2::ZERO {
                        intended_deltas
                            .get(&entity_id)
                            .copied()
                            .unwrap_or(glam::IVec2::ZERO)
                    } else {
                        actual_delta
                    };
                    let desired_animation =
                        Self::resolve_animation_state(animation_controller, entity_moved, delta);
                    if animation_controller.current_clip_state != desired_animation {
                        animation_controller.play(desired_animation);
                    }
                }
            }

            if entity_moved {
                self.emit_entity_movement_audio(
                    entity_id,
                    Self::movement_distance(initial_position, final_position),
                    &mut result,
                );
            }
        }

        result
    }

    /// Handle key press events
    pub fn handle_key_press(&mut self, key: InputKey) {
        match key {
            InputKey::DebugToggle => {
                self.debug_collision_rendering = !self.debug_collision_rendering;
                tracing::info!(
                    "Debug collision rendering: {}",
                    self.debug_collision_rendering
                );
            }
            _ => {
                self.keys_held.insert(key);
            }
        }
    }

    /// Handle key release events
    pub fn handle_key_release(&mut self, key: InputKey) {
        self.keys_held.remove(&key);
    }

    /// Handle profile-scoped movement key press events.
    pub fn handle_profile_key_press(&mut self, profile: MovementProfile, key: InputKey) {
        if matches!(key, InputKey::DebugToggle) {
            self.handle_key_press(key);
            return;
        }
        self.profile_keys_held
            .entry(profile)
            .or_default()
            .insert(key);
    }

    /// Handle profile-scoped movement key release events.
    pub fn handle_profile_key_release(&mut self, profile: MovementProfile, key: InputKey) {
        if let Some(keys) = self.profile_keys_held.get_mut(&profile) {
            keys.remove(&key);
            if keys.is_empty() {
                self.profile_keys_held.remove(&profile);
            }
        }
    }

    /// Handle profile-scoped action press events.
    pub fn handle_profile_action_press(&mut self, profile: MovementProfile, action: InputAction) {
        let held_actions = self.profile_actions_held.entry(profile).or_default();
        if held_actions.insert(action) {
            self.pending_profile_actions
                .entry(profile)
                .or_default()
                .insert(action);
        }
    }

    /// Handle profile-scoped action release events.
    pub fn handle_profile_action_release(&mut self, profile: MovementProfile, action: InputAction) {
        if let Some(actions) = self.profile_actions_held.get_mut(&profile) {
            actions.remove(&action);
            if actions.is_empty() {
                self.profile_actions_held.remove(&profile);
            }
        }
    }

    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    pub fn rules_mut(&mut self) -> &mut RuleSet {
        &mut self.rules
    }

    pub fn set_rules(&mut self, rules: RuleSet) {
        self.rules = rules;
        self.rule_runtime = RuleRuntimeState::default();
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.rules.push(rule);
    }

    /// Get reference to all entities (legacy method - preserved for compatibility)
    pub fn entities(&self) -> Vec<&Entity> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&id| self.entity_manager.get_entity(id))
            .collect()
    }

    /// Get access to the entity manager
    pub fn entity_manager(&self) -> &EntityManager {
        &self.entity_manager
    }

    /// Get mutable access to the entity manager
    pub fn entity_manager_mut(&mut self) -> &mut EntityManager {
        &mut self.entity_manager
    }

    /// Get access to the scene manager
    pub fn scene_manager(&self) -> &SceneManager {
        &self.scene_manager
    }

    /// Get mutable access to the scene manager
    pub fn scene_manager_mut(&mut self) -> &mut SceneManager {
        &mut self.scene_manager
    }

    /// Load a scene and make it the active scene
    /// This will clear current entities and load entities from the scene
    pub fn load_scene(&mut self, scene_name: &str) -> Result<(), String> {
        // Get the scene first
        let scene = self
            .scene_manager
            .get_scene(scene_name)
            .ok_or_else(|| format!("Scene '{}' not found", scene_name))?
            .clone();

        // Set as active scene
        self.scene_manager.set_active_scene(scene_name)?;

        // Clear current entities
        self.entity_manager = EntityManager::new();
        self.player_id = None;
        self.profile_keys_held.clear();
        self.profile_actions_held.clear();
        self.pending_profile_actions.clear();
        self.pending_stat_changes.clear();
        self.set_rules(scene.rules.clone());

        // Load entities from scene
        for entity in scene.entities {
            self.entity_manager.add_existing_entity(entity);
        }
        self.player_id = self.entity_manager.get_player_id();

        Ok(())
    }

    /// Add a scene to the scene manager
    pub fn add_scene(&mut self, scene: crate::scene::Scene) {
        self.scene_manager.add_scene(scene);
    }

    /// Get reference to the current active scene
    pub fn active_scene(&self) -> Option<&crate::scene::Scene> {
        self.scene_manager.active_scene()
    }

    /// Sync current entities back to the active scene
    /// Useful for saving changes made during runtime back to scene data
    pub fn sync_entities_to_active_scene(&mut self) {
        let rules = self.rules.clone();
        if let Some(active_scene) = self.scene_manager.active_scene_mut() {
            // Clear scene entities and reload from current entity manager
            active_scene.entities.clear();

            for entity_id in self.entity_manager.active_entities() {
                if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                    active_scene.entities.push(entity.clone());
                }
            }

            active_scene.rules = rules;
        }
    }

    /// Get the player entity ID
    pub fn player_id(&self) -> Option<EntityId> {
        self.player_id
    }

    /// Get reference to player entity
    pub fn player_entity(&self) -> Option<&Entity> {
        self.player_id
            .and_then(|id| self.entity_manager.get_entity(id))
    }

    /// Get entities as owned Vec for camera system compatibility
    pub fn entities_owned(&self) -> Vec<Entity> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&id| self.entity_manager.get_entity(id))
            .cloned()
            .collect()
    }

    /// Get sprite frame for a specific entity
    pub fn get_entity_sprite_frame(
        &self,
        entity_id: EntityId,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> Option<SpriteFrame> {
        tracing::trace!(
            "Getting sprite frame for entity {} with texture size {}x{}",
            entity_id,
            texture_size.x,
            texture_size.y
        );

        if let Some(entity) = self.entity_manager.get_entity(entity_id) {
            tracing::trace!("Found entity {} for sprite frame lookup", entity_id);

            if let Some(animation_controller) = &entity.attributes.animation_controller {
                tracing::trace!("Entity {} has animation controller", entity_id);

                if let Ok(tile_name) = animation_controller.current_tile_name() {
                    tracing::trace!("Entity {} requesting tile: '{}'", entity_id, tile_name);

                    // Look up the tile in the atlas to get UV coordinates
                    if let Some(uvs) = atlas.get_tile_uvs(&tile_name, texture_size) {
                        tracing::trace!(
                            "Found UVs for tile '{}': [{:.3}, {:.3}, {:.3}, {:.3}]",
                            tile_name,
                            uvs[0],
                            uvs[1],
                            uvs[2],
                            uvs[3]
                        );
                        return Some(SpriteFrame {
                            u0: uvs[0],
                            v0: uvs[1],
                            u1: uvs[2],
                            v1: uvs[3],
                        });
                    } else {
                        tracing::warn!(
                            "Tile '{}' not found in atlas for entity {}",
                            tile_name,
                            entity_id
                        );
                        tracing::trace!(
                            "Atlas contains tiles: {:?}",
                            atlas.tiles.keys().collect::<Vec<_>>()
                        );
                    }
                } else {
                    tracing::trace!(
                        "Entity {} animation controller failed to provide tile name",
                        entity_id
                    );
                }
            } else {
                tracing::trace!("Entity {} has no animation controller", entity_id);
            }
        } else {
            tracing::warn!("Entity {} not found when getting sprite frame", entity_id);
        }
        None
    }

    pub fn get_entity_current_atlas_name(&self, entity_id: EntityId) -> Option<String> {
        self.entity_manager
            .get_entity(entity_id)
            .and_then(|entity| entity.attributes.animation_controller.as_ref())
            .and_then(|controller| controller.current_atlas_name().ok())
    }

    pub fn get_entity_sprite_flip_x(&self, entity_id: EntityId) -> bool {
        self.entity_manager
            .get_entity(entity_id)
            .and_then(|entity| entity.attributes.animation_controller.as_ref())
            .map(|controller| Self::animation_state_flip_x(controller.current_clip_state))
            .unwrap_or(false)
    }

    /// Get all renderable entities (entities that are visible and have animation controllers)
    pub fn get_renderable_entities(&self) -> Vec<(EntityId, glam::IVec2, glam::UVec2)> {
        let active_entities = self.entity_manager.active_entities();
        tracing::trace!(
            "Checking {} active entities for renderability",
            active_entities.len()
        );

        let renderable: Vec<_> = self
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                    let is_visible = entity.attributes.visible;
                    let has_animation = entity.attributes.animation_controller.is_some();

                    tracing::trace!(
                        "Entity {}: visible={}, has_animation={}",
                        entity_id,
                        is_visible,
                        has_animation
                    );

                    if is_visible && has_animation {
                        tracing::trace!(
                            "Entity {} is renderable at ({}, {}) with size {}x{}",
                            entity_id,
                            entity.position.x,
                            entity.position.y,
                            entity.size.x,
                            entity.size.y
                        );
                        return Some((entity_id, entity.position, entity.size));
                    }
                }
                None
            })
            .collect();

        tracing::trace!(
            "Found {} renderable entities out of {} active entities",
            renderable.len(),
            active_entities.len()
        );
        renderable
    }

    /// Get the current sprite frame for rendering with proper atlas lookup (legacy method for player)
    pub fn current_sprite_frame(
        &self,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> SpriteFrame {
        if let Some(player_id) = self.player_id {
            if let Some(frame) = self.get_entity_sprite_frame(player_id, atlas, texture_size) {
                return frame;
            }
        }

        // Fallback to default frame if animation or atlas lookup fails
        SpriteFrame {
            u0: 0.0,
            v0: 0.0,
            u1: 0.25,
            v1: 1.0,
        }
    }

    /// Get player position for rendering
    pub fn player_position(&self) -> glam::IVec2 {
        if let Some(player_entity) = self.player_entity() {
            player_entity.position
        } else {
            glam::IVec2::ZERO // Fallback
        }
    }

    /// Get sprite size for rendering calculations
    pub fn sprite_size(&self) -> u32 {
        self.sprite_size
    }

    /// Check if debug collision rendering is enabled
    pub fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.debug_collision_rendering
    }

    /// Get entity collision boxes for debug rendering
    /// Returns Vec of (position, size, is_trigger) tuples
    pub fn get_entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut boxes = Vec::new();

        for entity_id in self.entity_manager.active_entities() {
            if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                if let Some(collision_box) = &entity.collision_box {
                    let (world_pos, size) = collision_box.world_bounds(entity.position);
                    boxes.push((world_pos, size, collision_box.trigger));
                }
            }
        }

        boxes
    }

    /// Get solid tile positions for debug rendering
    /// Returns Vec of (tile_x, tile_y) coordinates of solid tiles
    pub fn get_solid_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut solid_tiles = Vec::new();

        for y in 0..tilemap.size.y {
            for x in 0..tilemap.size.x {
                if let Ok(is_solid) = tilemap.is_tile_solid_at(atlas, x, y) {
                    if is_solid {
                        solid_tiles.push((x, y));
                    }
                }
            }
        }

        solid_tiles
    }

    /// Get trigger tile positions for debug rendering
    /// Returns Vec of (tile_x, tile_y) coordinates of trigger tiles
    pub fn get_trigger_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut trigger_tiles = Vec::new();

        for y in 0..tilemap.size.y {
            for x in 0..tilemap.size.x {
                if let Ok(tile_name) = tilemap.get_tile_name(x, y) {
                    if atlas.is_tile_trigger(tile_name) {
                        trigger_tiles.push((x, y));
                    }
                }
            }
        }

        trigger_tiles
    }

    fn collect_rule_commands_for_trigger(
        &mut self,
        trigger: RuleTrigger,
        command_buffer: &mut Vec<RuleCommand>,
    ) {
        let mut matching_rules = self
            .rules
            .rules
            .iter()
            .filter(|rule| rule.enabled && rule.trigger == trigger)
            .filter(|rule| {
                !(rule.once
                    && self
                        .rule_runtime
                        .fired_once_rules
                        .contains(rule.id.as_str()))
            })
            .cloned()
            .collect::<Vec<_>>();

        // Deterministic order: higher priority first, id as tie-breaker.
        matching_rules.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));

        for rule in matching_rules {
            if !self.rule_conditions_match(&rule.conditions) {
                continue;
            }

            for action in &rule.actions {
                self.buffer_rule_action(action, command_buffer);
            }

            if rule.once {
                self.rule_runtime.fired_once_rules.insert(rule.id);
            }
        }
    }

    fn collect_rule_commands_for_key_triggers(&mut self, command_buffer: &mut Vec<RuleCommand>) {
        let held_keys = self.all_held_keys();

        for input_key in held_keys {
            let trigger = RuleTrigger::OnKey {
                key: Self::to_rule_key(input_key),
            };
            self.collect_rule_commands_for_trigger(trigger, command_buffer);
        }
    }

    fn input_key_order(key: InputKey) -> u8 {
        match key {
            InputKey::Up => 0,
            InputKey::Down => 1,
            InputKey::Left => 2,
            InputKey::Right => 3,
            InputKey::DebugToggle => 4,
        }
    }

    fn to_rule_key(key: InputKey) -> RuleKey {
        match key {
            InputKey::Up => RuleKey::Up,
            InputKey::Down => RuleKey::Down,
            InputKey::Left => RuleKey::Left,
            InputKey::Right => RuleKey::Right,
            InputKey::DebugToggle => RuleKey::DebugToggle,
        }
    }

    fn to_input_key(key: RuleKey) -> InputKey {
        match key {
            RuleKey::Up => InputKey::Up,
            RuleKey::Down => InputKey::Down,
            RuleKey::Left => InputKey::Left,
            RuleKey::Right => InputKey::Right,
            RuleKey::DebugToggle => InputKey::DebugToggle,
        }
    }

    fn any_entity_overlaps_trigger_tile(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> bool {
        for entity_id in self.entity_manager.active_entities() {
            let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                continue;
            };
            if Self::entity_overlaps_trigger_tile(entity, tilemap, atlas) {
                return true;
            }
        }
        false
    }

    fn entity_overlaps_trigger_tile(entity: &Entity, tilemap: &TileMap, atlas: &AtlasMeta) -> bool {
        if tilemap.tile_size.x == 0 || tilemap.tile_size.y == 0 {
            return false;
        }
        if tilemap.size.x == 0 || tilemap.size.y == 0 {
            return false;
        }

        let (box_pos, box_size) = if let Some(collision_box) = &entity.collision_box {
            collision_box.world_bounds(entity.position)
        } else {
            (entity.position, entity.size)
        };
        if box_size.x == 0 || box_size.y == 0 {
            return false;
        }

        let tile_w = tilemap.tile_size.x as i32;
        let tile_h = tilemap.tile_size.y as i32;

        let tile_min_x = (box_pos.x / tile_w).max(0) as u32;
        let tile_min_y = (box_pos.y / tile_h).max(0) as u32;
        let tile_max_x = ((box_pos.x + box_size.x as i32 - 1) / tile_w).max(0) as u32;
        let tile_max_y = ((box_pos.y + box_size.y as i32 - 1) / tile_h).max(0) as u32;

        let map_max_x = tilemap.size.x.saturating_sub(1);
        let map_max_y = tilemap.size.y.saturating_sub(1);
        let tile_min_x = tile_min_x.min(map_max_x);
        let tile_min_y = tile_min_y.min(map_max_y);
        let tile_max_x = tile_max_x.min(map_max_x);
        let tile_max_y = tile_max_y.min(map_max_y);

        for y in tile_min_y..=tile_max_y {
            for x in tile_min_x..=tile_max_x {
                let Ok(tile_name) = tilemap.get_tile_name(x, y) else {
                    continue;
                };
                if atlas.is_tile_trigger(tile_name) {
                    return true;
                }
            }
        }

        false
    }

    fn rule_conditions_match(&self, conditions: &[RuleCondition]) -> bool {
        conditions.iter().all(|condition| match condition {
            RuleCondition::Always => true,
            RuleCondition::TargetExists { target } => self
                .resolve_rule_target(*target)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some(),
            RuleCondition::KeyHeld { key } => {
                self.all_held_keys().contains(&Self::to_input_key(*key))
            }
            RuleCondition::EntityActive { target, is_active } => self
                .resolve_rule_target(*target)
                .and_then(|entity_id| self.entity_manager.get_entity(entity_id))
                .is_some_and(|entity| entity.attributes.active == *is_active),
        })
    }

    fn buffer_rule_action(&self, action: &RuleAction, command_buffer: &mut Vec<RuleCommand>) {
        match action {
            RuleAction::PlaySound { channel, sound_id } => {
                let sound_id = sound_id.trim();
                if sound_id.is_empty() {
                    return;
                }

                let channel = match channel {
                    RuleSoundChannel::Movement => AudioChannel::Movement,
                    RuleSoundChannel::Collision => AudioChannel::Collision,
                };

                command_buffer.push(RuleCommand::PlaySound {
                    channel,
                    sound_id: sound_id.to_string(),
                });
            }
            RuleAction::PlayMusic { track_id } => {
                let track_id = track_id.trim();
                if track_id.is_empty() {
                    return;
                }
                command_buffer.push(RuleCommand::PlayMusic {
                    track_id: track_id.to_string(),
                });
            }
            RuleAction::PlayAnimation { target, state } => {
                if let Some(entity_id) = self.resolve_rule_target(*target) {
                    command_buffer.push(RuleCommand::PlayAnimation {
                        entity_id,
                        state: *state,
                    });
                }
            }
            RuleAction::SetVelocity { target, velocity } => {
                if let Some(entity_id) = self.resolve_rule_target(*target) {
                    command_buffer.push(RuleCommand::SetVelocity {
                        entity_id,
                        velocity: glam::IVec2::new(velocity[0], velocity[1]),
                    });
                }
            }
            RuleAction::Spawn {
                entity_type,
                position,
            } => {
                command_buffer.push(RuleCommand::Spawn {
                    entity_type: *entity_type,
                    position: glam::IVec2::new(position[0], position[1]),
                });
            }
            RuleAction::DestroySelf { target } => {
                if let Some(entity_id) = self.resolve_rule_target(*target) {
                    command_buffer.push(RuleCommand::DestroySelf { entity_id });
                }
            }
            RuleAction::SwitchScene { scene_name } => {
                command_buffer.push(RuleCommand::SwitchScene {
                    scene_name: scene_name.clone(),
                });
            }
        }
    }

    fn apply_rule_commands(
        &mut self,
        commands: Vec<RuleCommand>,
        result: &mut GameUpdateResult<AudioEvent>,
    ) -> (Vec<(EntityId, AnimationState)>, Option<String>) {
        let mut buffered_velocities = HashMap::new();
        let mut buffered_animations = HashMap::new();
        let mut pending_scene_switch = None;

        for command in commands {
            match command {
                RuleCommand::PlaySound { channel, sound_id } => {
                    result.add_event(AudioEvent::PlaySound {
                        channel,
                        sound_id,
                        source_position: None,
                        hearing_radius: None,
                    });
                }
                RuleCommand::PlayMusic { track_id } => {
                    result.add_event(AudioEvent::BackgroundMusic(track_id));
                }
                RuleCommand::SetVelocity {
                    entity_id,
                    velocity,
                } => {
                    // Rules are already sorted by priority desc + id asc, so first command wins.
                    buffered_velocities.entry(entity_id).or_insert(velocity);
                }
                RuleCommand::PlayAnimation { entity_id, state } => {
                    // Rules are already sorted by priority desc + id asc, so first command wins.
                    buffered_animations.entry(entity_id).or_insert(state);
                }
                RuleCommand::Spawn {
                    entity_type,
                    position,
                } => {
                    self.spawn_entity_from_rule(entity_type, position);
                }
                RuleCommand::DestroySelf { entity_id } => {
                    let removed = self.entity_manager.despawn_entity(entity_id);
                    if removed {
                        if self.player_id == Some(entity_id) {
                            self.player_id = None;
                        }
                        self.rule_runtime.velocities.remove(&entity_id);
                    }
                }
                RuleCommand::SwitchScene { scene_name } => {
                    let target = scene_name.trim();
                    if !target.is_empty() && pending_scene_switch.is_none() {
                        pending_scene_switch = Some(target.to_string());
                    }
                }
            }
        }

        for (entity_id, velocity) in buffered_velocities {
            self.rule_runtime.velocities.insert(entity_id, velocity);
        }

        let mut pending_animations = buffered_animations.into_iter().collect::<Vec<_>>();
        pending_animations.sort_by_key(|(entity_id, _)| *entity_id);
        (pending_animations, pending_scene_switch)
    }

    fn resolve_rule_target(&self, target: RuleTarget) -> Option<EntityId> {
        match target {
            RuleTarget::Player => self.player_id,
            RuleTarget::Entity(entity_id) => Some(entity_id),
        }
    }

    fn spawn_entity_from_rule(
        &mut self,
        entity_type: RuleSpawnEntityType,
        position: glam::IVec2,
    ) -> EntityId {
        match entity_type {
            RuleSpawnEntityType::PlayerLikeNpc => self.spawn_player_like_npc(position),
            RuleSpawnEntityType::Npc => self.entity_manager.spawn_entity(
                EntityKind::Npc,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes::default(),
            ),
            RuleSpawnEntityType::Item => self.entity_manager.spawn_entity(
                EntityKind::Item,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes {
                    solid: false,
                    can_move: false,
                    ai_behavior: crate::entity::AiBehavior::None,
                    ..EntityAttributes::default()
                },
            ),
            RuleSpawnEntityType::Decoration => self.entity_manager.spawn_entity(
                EntityKind::Decoration,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes {
                    solid: false,
                    can_move: false,
                    ai_behavior: crate::entity::AiBehavior::None,
                    ..EntityAttributes::default()
                },
            ),
            RuleSpawnEntityType::Trigger => self.entity_manager.spawn_entity(
                EntityKind::Trigger,
                position,
                glam::UVec2::new(16, 16),
                EntityAttributes {
                    solid: false,
                    can_move: false,
                    visible: false,
                    ai_behavior: crate::entity::AiBehavior::None,
                    ..EntityAttributes::default()
                },
            ),
        }
    }

    fn apply_rule_scene_switch(&mut self, scene_name: &str) {
        self.sync_entities_to_active_scene();
        if let Err(error) = self.load_scene(scene_name) {
            tracing::warn!("Rule requested scene switch to '{}': {}", scene_name, error);
        }
    }

    fn apply_rule_velocities(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
        result: &mut GameUpdateResult<AudioEvent>,
    ) -> bool {
        let mut velocities = self
            .rule_runtime
            .velocities
            .iter()
            .map(|(entity_id, velocity)| (*entity_id, *velocity))
            .collect::<Vec<_>>();
        velocities.sort_by_key(|(entity_id, _)| *entity_id);

        let mut moved_player = false;

        for (entity_id, velocity) in velocities {
            if velocity == glam::IVec2::ZERO {
                continue;
            }

            let Some(current_entity) = self.entity_manager.get_entity(entity_id).cloned() else {
                continue;
            };

            let max_x = (world_bounds.x as i32 - current_entity.size.x as i32).max(0);
            let max_y = (world_bounds.y as i32 - current_entity.size.y as i32).max(0);
            let candidate_position = glam::IVec2::new(
                (current_entity.position.x + velocity.x).clamp(0, max_x),
                (current_entity.position.y + velocity.y).clamp(0, max_y),
            );

            if candidate_position == current_entity.position {
                continue;
            }

            if !self.can_entity_move_to_position(entity_id, candidate_position, tilemap, atlas) {
                self.handle_entity_collision_blocked(entity_id, result);
                continue;
            }

            if let Some((entity, entity_audio)) =
                self.entity_manager.get_entity_with_audio_mut(entity_id)
            {
                entity.position = candidate_position;
                entity_audio.last_collision_state = false;
                if Some(entity_id) == self.player_id {
                    moved_player = true;
                }
            }
            self.emit_entity_movement_audio(
                entity_id,
                Self::movement_distance(current_entity.position, candidate_position),
                result,
            );
        }

        moved_player
    }

    fn apply_rule_animations(&mut self, pending_animations: Vec<(EntityId, AnimationState)>) {
        for (entity_id, state) in pending_animations {
            let Some(entity) = self.entity_manager.get_entity_mut(entity_id) else {
                continue;
            };
            let Some(animation_controller) = entity.attributes.animation_controller.as_mut() else {
                continue;
            };

            if animation_controller.current_clip_state != state {
                animation_controller.play(state);
            }
        }
    }
}
