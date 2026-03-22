use std::path::PathBuf;

use toki_core::events::SceneSwitchRequest;

use crate::systems::{
    AudioManager, CameraManager, DecodedProjectCache, GameManager, RenderingSystem,
    ResourceManager, RuntimeAssetLoadPlan,
};

use super::{App, RuntimeAudioMixOptions, SceneTransitionController};

pub(super) struct SceneRuntimeCoordinator<'a> {
    game_system: &'a mut GameManager,
    camera_system: &'a mut CameraManager,
    resources: &'a mut ResourceManager,
    rendering: &'a mut RenderingSystem,
    audio_system: &'a mut AudioManager,
    decoded_project_cache: &'a mut DecodedProjectCache,
    asset_load_plan: &'a mut RuntimeAssetLoadPlan,
    scene_transition: &'a mut SceneTransitionController,
    audio_mix: &'a RuntimeAudioMixOptions,
    content_root: Option<PathBuf>,
}

impl<'a> SceneRuntimeCoordinator<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        game_system: &'a mut GameManager,
        camera_system: &'a mut CameraManager,
        resources: &'a mut ResourceManager,
        rendering: &'a mut RenderingSystem,
        audio_system: &'a mut AudioManager,
        decoded_project_cache: &'a mut DecodedProjectCache,
        asset_load_plan: &'a mut RuntimeAssetLoadPlan,
        scene_transition: &'a mut SceneTransitionController,
        audio_mix: &'a RuntimeAudioMixOptions,
        content_root: Option<PathBuf>,
    ) -> Self {
        Self {
            game_system,
            camera_system,
            resources,
            rendering,
            audio_system,
            decoded_project_cache,
            asset_load_plan,
            scene_transition,
            audio_mix,
            content_root,
        }
    }

    pub(super) fn queue_scene_switch_request(&mut self, request: SceneSwitchRequest) {
        let target_track = self
            .game_system
            .scene_named(&request.scene_name)
            .and_then(|scene| scene.background_music_track_id.clone());
        self.scene_transition
            .request_scene_switch(request, target_track);
    }

    pub(super) fn advance_scene_transition(&mut self, transition_delta_ms: u32) {
        match self.scene_transition.advance(
            transition_delta_ms,
            self.audio_system,
            self.audio_mix.music_percent,
        ) {
            super::app_transition::TransitionAdvance::ReadyToSwap(request) => {
                let switch_result = self
                    .game_system
                    .transition_to_scene(&request.scene_name, &request.spawn_point_id);
                match switch_result {
                    Ok(()) => {
                        self.handle_runtime_scene_change();
                        let active_track = self
                            .game_system
                            .active_scene()
                            .and_then(|scene| scene.background_music_track_id.as_deref());
                        if let Err(error) = self.scene_transition.complete_scene_switch(
                            self.audio_system,
                            true,
                            active_track,
                        ) {
                            tracing::warn!(
                                "Failed to complete scene-transition audio handoff: {error}"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            "Failed to transition to scene '{}' via '{}': {}",
                            request.scene_name,
                            request.spawn_point_id,
                            error
                        );
                        if let Err(audio_error) = self.scene_transition.complete_scene_switch(
                            self.audio_system,
                            false,
                            self.game_system
                                .active_scene()
                                .and_then(|scene| scene.background_music_track_id.as_deref()),
                        ) {
                            tracing::warn!(
                                "Failed to restore audio after scene-transition failure: {audio_error}"
                            );
                        }
                    }
                }
            }
            super::app_transition::TransitionAdvance::None
            | super::app_transition::TransitionAdvance::Completed => {}
        }
    }

    fn handle_runtime_scene_change(&mut self) {
        let active_scene_name = self.game_system.active_scene_name().map(str::to_string);
        let active_scene = self.game_system.active_scene().cloned();

        if let (Some(content_root), Some(scene_name), Some(scene)) = (
            self.content_root.as_deref(),
            active_scene_name.as_deref(),
            active_scene.as_ref(),
        ) {
            let map_name = scene.maps.first().map(String::as_str);
            match App::load_project_resources_with_cache(
                content_root,
                Some(scene_name),
                map_name,
                self.decoded_project_cache,
            ) {
                Ok((resources, asset_load_plan)) => {
                    *self.resources = resources;
                    *self.asset_load_plan = asset_load_plan;
                    if self.rendering.has_gpu() {
                        if let Some(tilemap_texture_path) =
                            self.asset_load_plan.tilemap_texture_path.clone()
                        {
                            if let Err(error) =
                                self.rendering.load_tilemap_texture(tilemap_texture_path)
                            {
                                tracing::warn!(
                                    "Failed to reload tilemap texture for scene '{}': {}",
                                    scene_name,
                                    error
                                );
                            }
                        }
                        if let Some(sprite_texture_path) =
                            self.asset_load_plan.sprite_texture_path.clone()
                        {
                            if let Err(error) =
                                self.rendering.load_sprite_texture(sprite_texture_path)
                            {
                                tracing::warn!(
                                    "Failed to reload sprite texture for scene '{}': {}",
                                    scene_name,
                                    error
                                );
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::error!(
                        "Failed to reload resources for scene '{}': {}",
                        scene_name,
                        error
                    );
                }
            }
        }

        let world_bounds = glam::UVec2::new(
            self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x,
            self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y,
        );

        let new_mode = if let Some(player_id) = self.game_system.player_id() {
            self.camera_system
                .camera_mut()
                .center_on(self.game_system.player_position());
            toki_core::camera::CameraMode::FollowEntity(player_id)
        } else {
            toki_core::camera::CameraMode::FreeScroll
        };
        self.camera_system.controller_mut().mode = new_mode;
        self.camera_system
            .camera_mut()
            .clamp_to_world_bounds(world_bounds);

        if self.rendering.has_gpu() {
            let view = self.camera_system.view_matrix();
            self.rendering.update_projection(view);
            self.camera_system
                .update_chunk_cache(self.resources.get_tilemap());
            let atlas_size = self.resources.terrain_image_size().unwrap();
            let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
                self.resources.get_terrain_atlas(),
                atlas_size,
                self.camera_system.cached_visible_chunks(),
            );
            self.rendering.update_tilemap_vertices(&verts);
        }
    }
}
