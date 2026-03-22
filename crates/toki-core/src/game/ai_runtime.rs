use std::collections::HashMap;

use crate::ai::{AiSystem, AiUpdateResult, SpawnMode};
use crate::entity::{EntityDefinition, EntityId, EntityManager};

use super::GameState;

#[derive(Debug, Default)]
pub(super) struct AiRuntimeEffects {
    pub(super) movement_audio: Vec<(EntityId, f32)>,
}

pub(super) struct AiRuntimeApplier<'a> {
    entity_manager: &'a mut EntityManager,
    ai_system: &'a mut AiSystem,
    entity_definitions: &'a HashMap<String, EntityDefinition>,
}

impl<'a> AiRuntimeApplier<'a> {
    pub(super) fn new(
        entity_manager: &'a mut EntityManager,
        ai_system: &'a mut AiSystem,
        entity_definitions: &'a HashMap<String, EntityDefinition>,
    ) -> Self {
        Self {
            entity_manager,
            ai_system,
            entity_definitions,
        }
    }

    pub(super) fn apply_updates(
        &mut self,
        ai_updates: Vec<AiUpdateResult>,
    ) -> AiRuntimeEffects {
        let mut effects = AiRuntimeEffects::default();
        for ai_result in ai_updates {
            self.apply_result(ai_result, &mut effects);
        }
        effects
    }

    fn apply_result(&mut self, ai_result: AiUpdateResult, effects: &mut AiRuntimeEffects) {
        if let Some(new_position) = ai_result.new_position {
            if let Some(entity) = self.entity_manager.get_entity_mut(ai_result.entity_id) {
                entity.position = new_position;
            }
            if ai_result.movement_distance > 0.0 {
                effects
                    .movement_audio
                    .push((ai_result.entity_id, ai_result.movement_distance));
            }
        }

        if let Some(animation) = ai_result.new_animation {
            if let Some(entity) = self.entity_manager.get_entity_mut(ai_result.entity_id) {
                if let Some(controller) = &mut entity.attributes.animation_controller {
                    if controller.current_clip_state != animation {
                        controller.play(animation);
                    }
                }
            }
        }

        if let Some(spawn_request) = ai_result.spawn_request {
            let spawn_result = match &spawn_request.mode {
                SpawnMode::Clone { source_entity_id } => self
                    .entity_manager
                    .clone_entity(*source_entity_id, spawn_request.position)
                    .ok_or_else(|| format!("Source entity {} not found", source_entity_id)),
                SpawnMode::FromDefinition { definition_name } => self
                    .spawn_entity_from_definition_name(definition_name, spawn_request.position),
            };

            match spawn_result {
                Ok(new_entity_id) => {
                    if let Some(entity) = self.entity_manager.get_entity(new_entity_id) {
                        tracing::debug!(
                            entity_id = new_entity_id,
                            definition_name = ?entity.definition_name,
                            position = ?entity.position,
                            ai_behavior = ?entity.attributes.ai_config.behavior,
                            detection_radius = entity.attributes.ai_config.detection_radius,
                            solid = entity.attributes.solid,
                            speed = entity.attributes.speed,
                            "AI spawn: child entity configuration"
                        );
                    }

                    if !spawn_request.parent_entity_ids.is_empty() {
                        self.ai_system.enter_separation_state(
                            new_entity_id,
                            spawn_request.parent_entity_ids,
                            spawn_request.separation_distance,
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "AI spawn request failed");
                }
            }
        }
    }

    fn spawn_entity_from_definition_name(
        &mut self,
        definition_name: &str,
        position: glam::IVec2,
    ) -> Result<EntityId, String> {
        let definition = self
            .entity_definitions
            .get(definition_name)
            .ok_or_else(|| format!("Entity definition '{}' not found", definition_name))?
            .clone();

        self.entity_manager
            .spawn_from_definition(&definition, position)
    }
}

impl GameState {
    pub(super) fn ai_runtime_applier(&mut self) -> AiRuntimeApplier<'_> {
        AiRuntimeApplier::new(
            &mut self.entity_manager,
            &mut self.ai_system,
            &self.entity_definitions,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ai::{AiSpawnRequest, AiSystem, AiUpdateResult, SpawnMode};
    use crate::entity::{AiConfig, AttributesDef, AudioDef, CollisionDef, EntityDefinition, EntityManager, MovementProfile, MovementSoundTrigger, RenderingDef};

    use super::AiRuntimeApplier;

    #[test]
    fn ai_runtime_applier_spawns_from_definition_and_enters_separation_state() {
        let definition = EntityDefinition {
            name: "slime".to_string(),
            display_name: "Slime".to_string(),
            description: String::new(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                static_object: None,
            },
            attributes: AttributesDef {
                health: Some(10),
                stats: HashMap::new(),
                speed: 1.0,
                solid: true,
                active: true,
                can_move: true,
                interactable: false,
                interaction_reach: 0,
                ai_config: AiConfig::default(),
                movement_profile: MovementProfile::None,
                primary_projectile: None,
                pickup: None,
                has_inventory: false,
            },
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
            },
            audio: AudioDef {
                footstep_trigger_distance: 32.0,
                hearing_radius: 192,
                movement_sound_trigger: MovementSoundTrigger::Distance,
                movement_sound: "step".to_string(),
                collision_sound: None,
            },
            animations: crate::entity::AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: Vec::new(),
                default_state: "idle".to_string(),
            },
            category: "creature".to_string(),
            tags: vec!["npc".to_string()],
        };

        let mut entity_manager = EntityManager::new();
        let mut ai_system = AiSystem::new();
        let definitions = HashMap::from([(definition.name.clone(), definition)]);
        let mut applier = AiRuntimeApplier::new(&mut entity_manager, &mut ai_system, &definitions);

        let effects = applier.apply_updates(vec![AiUpdateResult {
            entity_id: 1,
            new_position: None,
            new_animation: None,
            movement_distance: 0.0,
            spawn_request: Some(AiSpawnRequest {
                position: glam::IVec2::new(32, 32),
                parent_entity_ids: vec![7, 8],
                separation_distance: 24.0,
                mode: SpawnMode::FromDefinition {
                    definition_name: "slime".to_string(),
                },
            }),
        }]);

        assert!(effects.movement_audio.is_empty());
        assert_eq!(applier.entity_manager.active_entities().len(), 1);
        let spawned_id = applier.entity_manager.active_entities()[0];
        assert!(applier.ai_system.is_entity_separating(spawned_id));
    }
}
