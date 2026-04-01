use std::collections::HashMap;

use crate::ai::{AiSystem, AiUpdateResult, SpawnMode};
use crate::entity::{EntityDefinition, EntityDefinitionError, EntityId, EntityManager};
use crate::ids::EntityDefName;

use super::GameState;

#[derive(Debug, thiserror::Error)]
enum AiRuntimeError {
    #[error("source entity {source_entity_id} not found")]
    MissingSourceEntity { source_entity_id: EntityId },
    #[error("entity definition '{definition_name}' not found")]
    MissingEntityDefinition { definition_name: EntityDefName },
    #[error("failed to create entity from definition '{definition_name}': {source}")]
    SpawnFromDefinition {
        definition_name: EntityDefName,
        #[source]
        source: EntityDefinitionError,
    },
}

pub(super) struct AiRuntimeApplier<'a> {
    entity_manager: &'a mut EntityManager,
    ai_system: &'a mut AiSystem,
    entity_definitions: &'a HashMap<EntityDefName, EntityDefinition>,
}

impl<'a> AiRuntimeApplier<'a> {
    pub(super) fn new(
        entity_manager: &'a mut EntityManager,
        ai_system: &'a mut AiSystem,
        entity_definitions: &'a HashMap<EntityDefName, EntityDefinition>,
    ) -> Self {
        Self {
            entity_manager,
            ai_system,
            entity_definitions,
        }
    }

    pub(super) fn apply_updates(&mut self, ai_updates: Vec<AiUpdateResult>) {
        for ai_result in ai_updates {
            self.apply_result(ai_result);
        }
    }

    fn apply_result(&mut self, ai_result: AiUpdateResult) {
        if let Some(animation) = ai_result.new_animation {
            if let Some(entity) = self.entity_manager.get_entity_mut(ai_result.entity_id) {
                if let Some(controller) = &mut entity.rendering.animation_controller {
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
                    .ok_or(AiRuntimeError::MissingSourceEntity {
                        source_entity_id: *source_entity_id,
                    }),
                SpawnMode::FromDefinition { definition_name } => {
                    self.spawn_entity_from_definition_name(definition_name, spawn_request.position)
                }
            };

            match spawn_result {
                Ok(new_entity_id) => {
                    if let Some(entity) = self.entity_manager.get_entity(new_entity_id) {
                        let ai = self.entity_manager.ai(new_entity_id);
                        let movement = self.entity_manager.movement(new_entity_id);
                        tracing::debug!(
                            entity_id = new_entity_id,
                            definition_name = ?entity.definition_name,
                            position = ?entity.position,
                            ai_behavior = ?ai.map(|component| component.ai_config.behavior),
                            detection_radius = ai.map(|component| component.ai_config.detection_radius).unwrap_or(0),
                            solid = entity.solid,
                            speed = movement.map(|component| component.speed).unwrap_or(0.0),
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
        definition_name: &EntityDefName,
        position: glam::IVec2,
    ) -> Result<EntityId, AiRuntimeError> {
        let definition = self
            .entity_definitions
            .get(definition_name)
            .ok_or_else(|| AiRuntimeError::MissingEntityDefinition {
                definition_name: definition_name.clone(),
            })?
            .clone();

        self.entity_manager
            .spawn_from_definition(&definition, position)
            .map_err(|source| AiRuntimeError::SpawnFromDefinition {
                definition_name: definition_name.clone(),
                source,
            })
    }
}

impl GameState {
    pub(super) fn ai_runtime_applier(&mut self) -> AiRuntimeApplier<'_> {
        AiRuntimeApplier::new(
            &mut self.world.entity_manager,
            &mut self.runtime.ai.system,
            &self.world.entity_definitions,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::ai::{AiSpawnRequest, AiSystem, AiUpdateResult, SpawnMode};
    use crate::entity::{
        AiConfig, AudioDef, CollisionDef, CombatComponent, ComponentsDef, EntityDefinition,
        EntityManager, MovementComponent, MovementProfile, MovementSoundTrigger, RenderingDef,
    };

    use super::AiRuntimeApplier;

    #[test]
    fn ai_runtime_applier_spawns_from_definition_and_enters_separation_state() {
        let definition = EntityDefinition {
            name: "slime".into(),
            display_name: "Slime".to_string(),
            description: String::new(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                has_shadow: true,
                palette_override: None,
                static_object: None,
                grounding: Default::default(),
            },
            solid: true,
            active: true,
            components: ComponentsDef {
                movement: Some(MovementComponent {
                    speed: 1.0,
                    movement_profile: MovementProfile::None,
                    can_move: true,
                }),
                ai: Some(crate::entity::AiComponent {
                    ai_config: AiConfig::default(),
                }),
                interaction: None,
                combat: Some(CombatComponent {
                    health: Some(10),
                    stats: Default::default(),
                }),
                primary_projectile: None,
                pickup: None,
                inventory: None,
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

        applier.apply_updates(vec![AiUpdateResult {
            entity_id: 1,
            movement_intent: None,
            new_animation: None,
            spawn_request: Some(AiSpawnRequest {
                position: glam::IVec2::new(32, 32),
                parent_entity_ids: vec![7, 8],
                separation_distance: 24.0,
                mode: SpawnMode::FromDefinition {
                    definition_name: "slime".into(),
                },
            }),
        }]);

        assert_eq!(applier.entity_manager.active_entities().len(), 1);
        let spawned_id = applier.entity_manager.active_entities()[0];
        assert!(applier.ai_system.is_entity_separating(spawned_id));
    }
}
