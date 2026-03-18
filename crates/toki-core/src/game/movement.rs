use super::{AudioChannel, AudioEvent, GameState, InputKey};
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::collision;
use crate::entity::{EntityId, MovementSoundTrigger};
use crate::events::GameUpdateResult;
use std::collections::HashMap;

impl GameState {
    pub(super) fn movement_delta_from_keys(keys: &[InputKey]) -> glam::IVec2 {
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
        current_position: glam::IVec2,
        key: InputKey,
        world_bounds: glam::UVec2,
        entity_speed: f32,
        entity_size: glam::UVec2,
    ) -> glam::IVec2 {
        let movement_step = entity_speed as i32;
        let max_x = (world_bounds.x as i32 - entity_size.x as i32).max(0);
        let max_y = (world_bounds.y as i32 - entity_size.y as i32).max(0);

        match key {
            InputKey::Up => glam::IVec2::new(
                current_position.x,
                (current_position.y - movement_step).max(0),
            ),
            InputKey::Left => glam::IVec2::new(
                (current_position.x - movement_step).max(0),
                current_position.y,
            ),
            InputKey::Down => glam::IVec2::new(
                current_position.x,
                (current_position.y + movement_step).min(max_y),
            ),
            InputKey::Right => glam::IVec2::new(
                (current_position.x + movement_step).min(max_x),
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

        let Some(entity) = self.entity_manager.get_entity(entity_id) else {
            return;
        };

        let current_position = entity.position;
        let entity_speed = entity.attributes.speed;
        let entity_size = entity.size;

        let new_position = Self::candidate_input_position(
            current_position,
            key,
            world_bounds,
            entity_speed,
            entity_size,
        );
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

    pub(super) fn handle_entity_collision_blocked(
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

    pub(super) fn can_entity_move_to_position(
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

    pub(super) fn movement_distance(from: glam::IVec2, to: glam::IVec2) -> f32 {
        let delta = to - from;
        ((delta.x.pow(2) + delta.y.pow(2)) as f32).sqrt()
    }

    pub(super) fn emit_entity_movement_audio(
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

    pub(super) fn emit_animation_loop_movement_audio(
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

    /// Process input and update player position
    /// Returns GameUpdateResult with movement info and audio events
    pub(super) fn process_input(
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

    pub(super) fn apply_rule_velocities(
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
}
