use crate::entity::EntityDefinition;
use crate::{GameState, Scene};

pub fn build_game_state_from_scene(
    scene: Scene,
    entity_definitions: impl IntoIterator<Item = EntityDefinition>,
) -> Result<GameState, String> {
    let startup_scene_name = scene.name.clone();
    build_game_state_from_project_content([scene], entity_definitions, &startup_scene_name)
}

pub fn build_game_state_from_project_content(
    scenes: impl IntoIterator<Item = Scene>,
    entity_definitions: impl IntoIterator<Item = EntityDefinition>,
    startup_scene_name: &str,
) -> Result<GameState, String> {
    let mut game_state = GameState::new_empty();

    for definition in entity_definitions {
        game_state.add_entity_definition(definition);
    }

    for scene in scenes {
        game_state.add_scene(scene);
    }

    game_state.load_scene(startup_scene_name)?;
    Ok(game_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{
        AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
        MovementProfile, MovementSoundTrigger, RenderingDef,
    };
    use crate::scene::{SceneAnchor, SceneAnchorKind, ScenePlayerEntry};
    use glam::IVec2;

    fn sample_player_definition(name: &str) -> EntityDefinition {
        EntityDefinition {
            name: name.to_string(),
            display_name: "Player".to_string(),
            description: "Player definition".to_string(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 1,
                visible: true,
                has_shadow: true,
                static_object: None,
            },
            attributes: AttributesDef {
                health: Some(100),
                stats: std::collections::HashMap::from([
                    ("health".to_string(), 100),
                    ("attack_power".to_string(), 8),
                ]),
                speed: 2.0,
                solid: true,
                active: true,
                can_move: true,
                interactable: false,
                interaction_reach: 0,
                ai_config: crate::entity::AiConfig::default(),
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
                footstep_trigger_distance: 16.0,
                hearing_radius: 100,
                movement_sound_trigger: MovementSoundTrigger::AnimationLoop,
                movement_sound: "sfx_step".to_string(),
                collision_sound: None,
            },
            animations: AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: vec![AnimationClipDef {
                    state: "idle_down".to_string(),
                    frame_tiles: vec!["idle".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                }],
                default_state: "idle_down".to_string(),
            },
            category: "human".to_string(),
            tags: vec!["player".to_string()],
        }
    }

    #[test]
    fn build_game_state_from_scene_uses_supplied_entity_definitions() {
        let mut scene = Scene::new("Main Scene".to_string());
        scene.anchors.push(SceneAnchor {
            id: "spawn_a".to_string(),
            kind: SceneAnchorKind::SpawnPoint,
            position: IVec2::new(64, 80),
            facing: None,
        });
        scene.player_entry = Some(ScenePlayerEntry {
            entity_definition_name: "player".to_string(),
            spawn_point_id: "spawn_a".to_string(),
        });

        let game_state = build_game_state_from_scene(scene, [sample_player_definition("player")])
            .expect("scene should load with supplied player definition");

        assert_eq!(
            game_state.active_scene().map(|scene| scene.name.as_str()),
            Some("Main Scene")
        );
        assert!(game_state.player_entity().is_some());
    }

    #[test]
    fn build_game_state_from_scene_errors_when_required_definition_is_missing() {
        let mut scene = Scene::new("Main Scene".to_string());
        scene.anchors.push(SceneAnchor {
            id: "spawn_a".to_string(),
            kind: SceneAnchorKind::SpawnPoint,
            position: IVec2::new(64, 80),
            facing: None,
        });
        scene.player_entry = Some(ScenePlayerEntry {
            entity_definition_name: "player".to_string(),
            spawn_point_id: "spawn_a".to_string(),
        });

        let error = build_game_state_from_scene(scene, std::iter::empty())
            .expect_err("scene should fail without required player definition");

        assert!(error.contains("player"));
    }

    #[test]
    fn build_game_state_from_project_content_loads_requested_startup_scene() {
        let main_scene = Scene::new("Main Scene".to_string());
        let other_scene = Scene::new("Scene 2".to_string());

        let game_state =
            build_game_state_from_project_content([main_scene, other_scene], [], "Scene 2")
                .expect("startup scene should load");

        assert_eq!(
            game_state.active_scene().map(|scene| scene.name.as_str()),
            Some("Scene 2")
        );
    }
}
