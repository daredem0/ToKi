use std::collections::HashMap;

use crate::animation::AnimationState;
use crate::collision::CollisionBox;
use crate::entity::{
    ControlRole, Entity, EntityDefinition, EntityId, EntityKind, EntityManager,
    EntityOptionalComponents, StoredEntity,
};
use crate::ids::EntityDefName;
use crate::rules::RuleSet;
use crate::scene::{Scene, SceneAnchorFacing};

use super::GameState;

pub(super) struct PreparedSceneLoad {
    pub(super) entity_manager: EntityManager,
    pub(super) player_id: Option<EntityId>,
    pub(super) rules: RuleSet,
}

pub(super) struct SceneTransitionPlanner<'a> {
    entity_definitions: &'a HashMap<EntityDefName, EntityDefinition>,
}

impl<'a> SceneTransitionPlanner<'a> {
    pub(super) fn new(entity_definitions: &'a HashMap<EntityDefName, EntityDefinition>) -> Self {
        Self { entity_definitions }
    }

    pub(super) fn prepare_scene_load(
        &self,
        scene: &Scene,
        transition_spawn_point_id: Option<&str>,
        preserved_player: Option<StoredEntity>,
    ) -> Result<PreparedSceneLoad, String> {
        let mut entity_manager = EntityManager::new();
        let preserve_player_across_transition =
            transition_spawn_point_id.is_some() && preserved_player.is_some();

        for entity in &scene.entities {
            let authored_player = matches!(
                entity.effective_control_role(),
                ControlRole::PlayerCharacter
            );
            if authored_player
                && (scene.player_entry.is_some() || preserve_player_across_transition)
            {
                continue;
            }
            let hydrated = self.hydrate_legacy_scene_entity(entity);
            let components = scene.optional_components(entity.id);
            entity_manager.add_existing_stored_entity(StoredEntity::new(hydrated, components));
        }

        let player_id = if let Some(player_entry) = &scene.player_entry {
            let spawn_point_id = transition_spawn_point_id.unwrap_or(&player_entry.spawn_point_id);
            if let Some(preserved_player) = preserved_player.as_ref() {
                let mut player = self.instantiate_scene_player_entry(
                    scene,
                    &player_entry.entity_definition_name,
                    spawn_point_id,
                    preserved_player,
                )?;
                let player_id = player.entity.id;
                self.reset_spawned_player_transient_state(
                    &mut player.entity,
                    scene,
                    spawn_point_id,
                );
                entity_manager.add_existing_stored_entity(player);
                entity_manager.set_control_role(player_id, ControlRole::PlayerCharacter);
                if let Some(player) = entity_manager.get_entity_mut(player_id) {
                    player.entity_kind = EntityKind::Player;
                }
                Some(player_id)
            } else {
                let (position, facing) = self.resolve_spawn_anchor(scene, spawn_point_id)?;
                let definition = self
                    .entity_definitions
                    .get(&player_entry.entity_definition_name)
                    .ok_or_else(|| {
                        format!(
                            "Scene '{}' references missing player entity definition '{}'",
                            scene.name, player_entry.entity_definition_name
                        )
                    })?;
                let player_id = entity_manager
                    .spawn_from_definition(definition, position)
                    .map_err(|error| error.to_string())?;
                entity_manager.set_control_role(player_id, ControlRole::PlayerCharacter);
                if let Some(player) = entity_manager.get_entity_mut(player_id) {
                    player.entity_kind = EntityKind::Player;
                    Self::reset_player_transient_state(player, facing);
                }
                Some(player_id)
            }
        } else if let Some(mut player) = preserved_player {
            if let Some(spawn_point_id) = transition_spawn_point_id {
                self.reposition_preserved_player(&mut player.entity, scene, spawn_point_id)?;
            } else {
                Self::reset_player_transient_state(&mut player.entity, None);
            }
            let player_id = player.entity.id;
            entity_manager.add_existing_stored_entity(player);
            entity_manager.set_control_role(player_id, ControlRole::PlayerCharacter);
            if let Some(player) = entity_manager.get_entity_mut(player_id) {
                player.entity_kind = EntityKind::Player;
            }
            Some(player_id)
        } else {
            entity_manager.get_player_id()
        };

        Ok(PreparedSceneLoad {
            entity_manager,
            player_id,
            rules: scene.rules.clone(),
        })
    }

    fn instantiate_scene_player_entry(
        &self,
        scene: &Scene,
        entity_definition_name: &str,
        spawn_point_id: &str,
        preserved_player: &StoredEntity,
    ) -> Result<StoredEntity, String> {
        let definition = self
            .entity_definitions
            .get(entity_definition_name)
            .ok_or_else(|| {
                format!(
                    "Scene '{}' references missing player entity definition '{}'",
                    scene.name, entity_definition_name
                )
            })?;
        let (position, _) = self.resolve_spawn_anchor(scene, spawn_point_id)?;
        let bundle = definition
            .create_spawn_bundle(position, preserved_player.entity.id)
            .map_err(|error| error.to_string())?;
        let mut player = bundle.entity;
        player.control_role = ControlRole::PlayerCharacter;
        player.entity_kind = EntityKind::Player;
        let mut components = bundle.optional_components;
        Self::apply_durable_player_state(&mut player, &mut components, preserved_player);
        Ok(StoredEntity::new(player, components))
    }

    fn hydrate_legacy_scene_entity(&self, entity: &Entity) -> Entity {
        let Some(definition_name) = entity.definition_name.as_deref() else {
            return entity.clone();
        };
        let Some(definition) = self.entity_definitions.get(definition_name) else {
            return entity.clone();
        };

        let mut hydrated = entity.clone();
        if !hydrated.attributes.rendering.grounding.is_empty() {
            return hydrated;
        }

        if let Some(collision_box) = hydrated.collision_box.as_ref() {
            if Self::is_legacy_default_collision(entity, collision_box) {
                hydrated.attributes.rendering.grounding = definition.get_grounding();
                hydrated.collision_box = definition.get_collision_box();
            }
        }

        hydrated
    }

    fn is_legacy_default_collision(entity: &Entity, collision_box: &CollisionBox) -> bool {
        collision_box.offset == glam::IVec2::ZERO
            && collision_box.size == entity.size
            && !collision_box.trigger
    }

    fn reposition_preserved_player(
        &self,
        player: &mut Entity,
        scene: &Scene,
        spawn_point_id: &str,
    ) -> Result<(), String> {
        let (position, facing) = self.resolve_spawn_anchor(scene, spawn_point_id)?;
        player.position = position;
        Self::reset_player_transient_state(player, facing);
        Ok(())
    }

    fn reset_spawned_player_transient_state(
        &self,
        player: &mut Entity,
        scene: &Scene,
        spawn_point_id: &str,
    ) {
        let anchor_facing = scene
            .get_anchor(spawn_point_id)
            .and_then(|anchor| anchor.facing);
        Self::reset_player_transient_state(player, anchor_facing);
    }

    fn reset_player_transient_state(player: &mut Entity, anchor_facing: Option<SceneAnchorFacing>) {
        player.movement_accumulator = glam::Vec2::ZERO;
        if let Some(animation_controller) =
            player.attributes.rendering.animation_controller.as_mut()
        {
            let facing = anchor_facing
                .map(Self::scene_anchor_facing_to_animation_state)
                .or_else(|| {
                    Some(GameState::directional_animation_state(
                        false,
                        GameState::facing_from_animation_state(
                            animation_controller.current_clip_state,
                        ),
                    ))
                })
                .unwrap_or(AnimationState::Idle);
            let desired_state = if animation_controller.has_clip(facing) {
                facing
            } else if animation_controller.has_clip(AnimationState::Idle) {
                AnimationState::Idle
            } else {
                animation_controller.current_clip_state
            };
            let _ = animation_controller.play(desired_state);
        }
    }

    fn apply_durable_player_state(
        target: &mut Entity,
        target_components: &mut EntityOptionalComponents,
        source: &StoredEntity,
    ) {
        target.attributes.gameplay.health = source.entity.attributes.gameplay.health;
        target.attributes.gameplay.stats = source.entity.attributes.gameplay.stats.clone();
        target.attributes.behavior.has_inventory = target.attributes.behavior.has_inventory
            || source.entity.attributes.behavior.has_inventory
            || source
                .components
                .inventory
                .as_ref()
                .is_some_and(|inventory| !inventory.is_empty());
        target_components.inventory = source.components.inventory.clone();
        target_components.primary_projectile = source.components.primary_projectile.clone();
    }

    fn scene_anchor_facing_to_animation_state(facing: SceneAnchorFacing) -> AnimationState {
        match facing {
            SceneAnchorFacing::Up => AnimationState::IdleUp,
            SceneAnchorFacing::Down => AnimationState::IdleDown,
            SceneAnchorFacing::Left => AnimationState::IdleLeft,
            SceneAnchorFacing::Right => AnimationState::IdleRight,
        }
    }

    fn resolve_spawn_anchor(
        &self,
        scene: &Scene,
        spawn_point_id: &str,
    ) -> Result<(glam::IVec2, Option<SceneAnchorFacing>), String> {
        let anchor = scene.get_anchor(spawn_point_id).ok_or_else(|| {
            format!(
                "Scene '{}' could not resolve spawn point '{}'",
                scene.name, spawn_point_id
            )
        })?;
        Ok((anchor.position, anchor.facing))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::SceneTransitionPlanner;
    use crate::animation::AnimationState;
    use crate::entity::{
        AiConfig, AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef,
        ControlRole, EntityDefinition, EntityOptionalComponents, MovementProfile,
        MovementSoundTrigger, RenderingDef, StoredEntity,
    };
    use crate::entity::{EntityFootprint, EntityGrounding};
    use crate::scene::{Scene, SceneAnchor, SceneAnchorKind, ScenePlayerEntry};

    #[test]
    fn scene_transition_planner_preserves_durable_player_state_for_scene_player_entry() {
        let definition = EntityDefinition {
            name: "player_knight".into(),
            display_name: "Knight".to_string(),
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
            attributes: AttributesDef {
                health: Some(100),
                stats: HashMap::new(),
                speed: 2.0,
                solid: true,
                active: true,
                can_move: true,
                interactable: false,
                interaction_reach: 0,
                ai_config: AiConfig::default(),
                movement_profile: MovementProfile::PlayerWasd,
                primary_projectile: None,
                pickup: None,
                has_inventory: true,
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
            animations: AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: vec![AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["slime/idle_0".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 200.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                }],
                default_state: "idle".to_string(),
            },
            category: "human".to_string(),
            tags: vec!["player".to_string()],
        };
        let definitions = HashMap::from([(definition.name.clone(), definition.clone())]);
        let planner = SceneTransitionPlanner::new(&definitions);
        let mut scene = Scene::new("Scene B".to_string());
        scene.anchors.push(SceneAnchor {
            id: "gate".to_string(),
            position: glam::IVec2::new(32, 48),
            kind: SceneAnchorKind::SpawnPoint,
            facing: Some(crate::scene::SceneAnchorFacing::Right),
        });
        scene.player_entry = Some(ScenePlayerEntry {
            entity_definition_name: definition.name.clone(),
            spawn_point_id: "gate".to_string(),
        });

        let mut preserved = definition
            .create_entity(glam::IVec2::new(0, 0), 7)
            .expect("player should instantiate");
        preserved.control_role = ControlRole::PlayerCharacter;
        preserved.attributes.apply_stat_delta("health", -25);
        let mut preserved_components = EntityOptionalComponents {
            inventory: Some(Default::default()),
            ..EntityOptionalComponents::default()
        };
        preserved_components
            .inventory
            .as_mut()
            .expect("inventory should exist")
            .add_item("coin", 2);
        if let Some(controller) = preserved
            .attributes
            .rendering
            .animation_controller
            .as_mut()
        {
            controller.play(AnimationState::AttackLeft);
        }

        let prepared = planner
            .prepare_scene_load(
                &scene,
                Some("gate"),
                Some(StoredEntity::new(preserved, preserved_components)),
            )
            .expect("scene load should be prepared");

        let player = prepared
            .entity_manager
            .get_entity(prepared.player_id.expect("player should exist"))
            .expect("prepared player should exist");
        assert_eq!(player.position, glam::IVec2::new(32, 48));
        assert_eq!(player.attributes.current_stat("health"), Some(75));
        assert_eq!(
            prepared
                .entity_manager
                .inventory(player.id)
                .expect("player inventory should exist")
                .item_count("coin"),
            2
        );
        assert_eq!(player.control_role, ControlRole::PlayerCharacter);
    }

    #[test]
    fn scene_transition_planner_migrates_legacy_default_scene_entity_collision_from_definition() {
        let definition = EntityDefinition {
            name: "player".into(),
            display_name: "Player".to_string(),
            description: String::new(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 1,
                visible: true,
                has_shadow: true,
                palette_override: None,
                static_object: None,
                grounding: EntityGrounding {
                    origin: None,
                    footprint: Some(EntityFootprint::new([4, 12], [8, 4])),
                },
            },
            attributes: AttributesDef {
                health: Some(100),
                stats: HashMap::new(),
                speed: 2.0,
                solid: true,
                active: true,
                can_move: true,
                interactable: false,
                interaction_reach: 0,
                ai_config: AiConfig::default(),
                movement_profile: MovementProfile::PlayerWasd,
                primary_projectile: None,
                pickup: None,
                has_inventory: true,
            },
            collision: CollisionDef {
                enabled: true,
                offset: [4, 12],
                size: [8, 4],
                trigger: false,
            },
            audio: AudioDef {
                footstep_trigger_distance: 32.0,
                hearing_radius: 192,
                movement_sound_trigger: MovementSoundTrigger::Distance,
                movement_sound: "step".to_string(),
                collision_sound: None,
            },
            animations: AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: vec![AnimationClipDef {
                    state: "idle".to_string(),
                    frame_tiles: vec!["slime/idle_0".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 200.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                }],
                default_state: "idle".to_string(),
            },
            category: "human".to_string(),
            tags: vec!["player".to_string()],
        };
        let definitions = HashMap::from([(definition.name.clone(), definition.clone())]);
        let planner = SceneTransitionPlanner::new(&definitions);
        let mut scene = Scene::new("Scene A".to_string());

        let mut legacy_entity = definition
            .create_entity(glam::IVec2::new(48, 128), 14)
            .expect("entity should instantiate");
        legacy_entity.attributes.rendering.grounding = EntityGrounding::default();
        legacy_entity.collision_box = Some(crate::collision::CollisionBox::solid_box(
            legacy_entity.size,
        ));
        scene.entities.push(legacy_entity);

        let prepared = planner
            .prepare_scene_load(&scene, None, None)
            .expect("scene should load");

        let entity = prepared
            .entity_manager
            .active_entities()
            .into_iter()
            .find_map(|entity_id| {
                let entity = prepared.entity_manager.get_entity(entity_id)?;
                (entity.definition_name.as_deref() == Some("player")).then_some(entity)
            })
            .expect("player entity should be present");

        assert_eq!(
            entity.attributes.rendering.grounding.footprint,
            Some(EntityFootprint::new([4, 12], [8, 4]))
        );
        let collision_box = entity
            .collision_box
            .as_ref()
            .expect("collision should exist");
        assert_eq!(collision_box.offset, glam::IVec2::new(4, 12));
        assert_eq!(collision_box.size, glam::UVec2::new(8, 4));
    }
}
