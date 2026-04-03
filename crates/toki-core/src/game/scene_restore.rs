use super::transition::SceneTransitionPlanner;
use super::{GameState, RestoreError};
use crate::entity::{ControlRole, EntityKind};
use crate::serialization::SaveData;

pub(super) fn restore_from_save_data(
    state: &mut GameState,
    save_data: &SaveData,
) -> Result<(), RestoreError> {
    let scene_name = save_data.active_scene_name.trim();
    if scene_name.is_empty() {
        return Err(RestoreError::MissingActiveSceneName);
    }
    let scene_id = save_data.active_scene_name.clone();

    restore_scene_snapshots(state, save_data);
    restore_legacy_persisted_entities(state, save_data);
    restore_camera_state(state, save_data, &scene_id);

    let scene = state
        .scene
        .scene_manager
        .get_scene_by_id(&scene_id)
        .ok_or_else(|| RestoreError::MissingScene {
            scene_name: scene_id.clone(),
        })?
        .clone();

    let prepared = SceneTransitionPlanner::new(&state.world.entity_definitions)
        .prepare_scene_load(scene, None, save_data.player.clone())
        .map_err(|source| RestoreError::PrepareSceneLoad { source })?;

    state
        .apply_prepared_scene_load(&scene_id, prepared)
        .map_err(|source| RestoreError::ApplySceneLoad { source })?;

    restore_saved_player(state, save_data);
    state.progress.game_flags = save_data.flags.clone();
    state.progress.play_time_ms = save_data.metadata.play_time_ms;
    Ok(())
}

fn restore_scene_snapshots(state: &mut GameState, save_data: &SaveData) {
    if save_data.scene_snapshots.is_empty() {
        return;
    }

    for scene_snapshot in &save_data.scene_snapshots {
        state.scene.scene_manager.add_scene(scene_snapshot.clone());
    }
    state.rebuild_persistent_scene_tracking();
}

fn restore_legacy_persisted_entities(state: &mut GameState, save_data: &SaveData) {
    if !save_data.scene_snapshots.is_empty() {
        return;
    }

    for persisted in &save_data.persisted_entities {
        state
            .scene
            .persistent_scene_entities
            .insert((persisted.scene_name.clone(), persisted.entity_id));
        let Some(scene) = state
            .scene
            .scene_manager
            .get_scene_mut_by_id(&persisted.scene_name)
        else {
            continue;
        };

        match &persisted.entity {
            Some(stored_entity) => {
                if let Some(existing) = scene.entity_mut(persisted.entity_id) {
                    *existing = stored_entity.entity.clone();
                } else {
                    scene.add_entity(stored_entity.entity.clone());
                }
                scene.components_mut().apply_optional_components(
                    persisted.entity_id,
                    stored_entity.components.clone(),
                );
            }
            None => {
                scene.remove_entity(persisted.entity_id);
            }
        }
    }
}

fn restore_camera_state(state: &mut GameState, save_data: &SaveData, scene_name: &crate::SceneId) {
    let Some(scene) = state.scene.scene_manager.get_scene_mut_by_id(scene_name) else {
        return;
    };
    scene.camera_position = save_data.camera.position;
    scene.camera_scale = save_data.camera.scale;
}

fn restore_saved_player(state: &mut GameState, save_data: &SaveData) {
    let (Some(saved_player), Some(player_id)) =
        (save_data.player.as_ref(), state.world.player_id())
    else {
        return;
    };

    if let Some(player) = state.world.entity_manager.get_entity_mut(player_id) {
        let mut restored_player = saved_player.entity.clone();
        restored_player.id = player_id;
        restored_player.control_role = ControlRole::PlayerCharacter;
        restored_player.entity_kind = EntityKind::Player;
        *player = restored_player;
    }
    if let Some(audio) = state
        .world
        .entity_manager
        .storage_mut()
        .audio_component_mut(player_id)
    {
        *audio = saved_player.entity.audio.to_component();
    }
    state
        .world
        .entity_manager_mut()
        .storage_mut()
        .components_mut()
        .apply_optional_components(player_id, saved_player.components.clone());
}
