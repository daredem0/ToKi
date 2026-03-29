use std::path::PathBuf;

use toki_core::events::SceneSwitchRequest;
use toki_core::game::SceneSystem;
use toki_core::graphics::image::load_image_rgba8;
use toki_core::palette::recolor_indexed_image;

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
    scene_persistence: bool,
    indexed_palette_override: Option<String>,
    content_root: Option<PathBuf>,
}

pub(super) struct SceneRuntimeRefs<'a> {
    pub game_system: &'a mut GameManager,
    pub camera_system: &'a mut CameraManager,
    pub resources: &'a mut ResourceManager,
    pub rendering: &'a mut RenderingSystem,
    pub audio_system: &'a mut AudioManager,
    pub decoded_project_cache: &'a mut DecodedProjectCache,
    pub asset_load_plan: &'a mut RuntimeAssetLoadPlan,
    pub scene_transition: &'a mut SceneTransitionController,
}

pub(super) struct SceneRuntimeSettings<'a> {
    pub audio_mix: &'a RuntimeAudioMixOptions,
    pub scene_persistence: bool,
    pub indexed_palette_override: Option<String>,
    pub content_root: Option<PathBuf>,
}

impl<'a> SceneRuntimeCoordinator<'a> {
    pub(super) fn new(refs: SceneRuntimeRefs<'a>, settings: SceneRuntimeSettings<'a>) -> Self {
        Self {
            game_system: refs.game_system,
            camera_system: refs.camera_system,
            resources: refs.resources,
            rendering: refs.rendering,
            audio_system: refs.audio_system,
            decoded_project_cache: refs.decoded_project_cache,
            asset_load_plan: refs.asset_load_plan,
            scene_transition: refs.scene_transition,
            audio_mix: settings.audio_mix,
            scene_persistence: settings.scene_persistence,
            indexed_palette_override: settings.indexed_palette_override,
            content_root: settings.content_root,
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
                SceneSystem::sync_persistent_entities_to_active_scene(
                    &mut self.game_system.game_state,
                );
                if self.scene_persistence {
                    self.game_system.sync_entities_to_active_scene();
                }
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
                    if self.indexed_palette_override.is_some() {
                        self.resources
                            .set_indexed_palette_override(self.indexed_palette_override.clone());
                    }
                    *self.asset_load_plan = asset_load_plan;
                    if self.rendering.has_gpu() {
                        self.reload_runtime_render_textures(scene_name);
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

    fn reload_runtime_render_textures(&mut self, scene_name: &str) {
        if let Some(tilemap_texture_path) = self.asset_load_plan.tilemap_texture_path.clone() {
            let terrain_atlas = self.resources.get_terrain_atlas();
            let tilemap_result = if terrain_atlas.is_palette_indexed() {
                match self.resources.resolve_indexed_palette(terrain_atlas, None) {
                    Ok((palette_id, palette)) => match load_image_rgba8(&tilemap_texture_path) {
                        Ok(image) => match recolor_indexed_image(&image, palette) {
                            Ok(recolored) => self.rendering.load_tilemap_texture_rgba8(&recolored),
                            Err(error) => Err(toki_render::RenderError::Other(format!(
                                "Indexed tilemap texture '{}' failed validation for palette '{}': {}",
                                tilemap_texture_path.display(),
                                palette_id,
                                error
                            ))),
                        },
                        Err(error) => Err(toki_render::RenderError::Other(format!(
                            "Failed to decode indexed tilemap texture '{}': {}",
                            tilemap_texture_path.display(),
                            error
                        ))),
                    },
                    Err(error) => Err(toki_render::RenderError::Other(format!("{error:?}"))),
                }
            } else {
                self.rendering
                    .load_tilemap_texture(tilemap_texture_path.clone())
            };

            if let Err(error) = tilemap_result {
                tracing::warn!(
                    "Failed to reload tilemap texture for scene '{}': {}",
                    scene_name,
                    error
                );
            }
        }

        if let Some(sprite_texture_path) = self.asset_load_plan.sprite_texture_path.clone() {
            if let Err(error) = self.rendering.load_sprite_texture(sprite_texture_path) {
                tracing::warn!(
                    "Failed to reload sprite texture for scene '{}': {}",
                    scene_name,
                    error
                );
            }
        }
    }
}

impl App {
    pub(super) fn refresh_runtime_after_scene_restore(&mut self) {
        let mut coordinator = self.create_scene_runtime_coordinator();
        coordinator.handle_runtime_scene_change();
    }

    pub(super) fn reload_runtime_render_textures(&mut self, scene_name: &str) {
        let content_root = self.content_root_path().map(std::path::Path::to_path_buf);
        SceneRuntimeCoordinator::new(
            SceneRuntimeRefs {
                game_system: &mut self.game_system,
                camera_system: &mut self.camera_system,
                resources: &mut self.resources,
                rendering: &mut self.rendering,
                audio_system: &mut self.audio_system,
                decoded_project_cache: &mut self.decoded_project_cache,
                asset_load_plan: &mut self.asset_load_plan,
                scene_transition: &mut self.scene_transition,
            },
            SceneRuntimeSettings {
                audio_mix: &self.launch_options.audio_mix,
                scene_persistence: self.launch_options.scene_persistence,
                indexed_palette_override: self
                    .launch_options
                    .display
                    .indexed_palette_override
                    .clone(),
                content_root,
            },
        )
        .reload_runtime_render_textures(scene_name);
    }
}
