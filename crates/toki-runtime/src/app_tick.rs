use std::time::Instant;

use toki_core::camera::RuntimeState;
use toki_core::{
    game::{AudioEvent, GameSimulation, RenderQueryService},
    EventHandler, GameUpdateResult, DEFAULT_TIMESTEP_MS,
};

use super::app_presenter::{render_scene_transition_overlay, WorldFramePresenter};
use super::app_scene_runtime::{SceneRuntimeCoordinator, SceneRuntimeRefs, SceneRuntimeSettings};
use super::App;

pub(super) struct GameUpdatePhase {
    pub(super) result: GameUpdateResult<AudioEvent>,
    pub(super) viewport_changed: bool,
    pub(super) transition_delta_ms: u32,
}

impl App {
    pub(super) fn current_world_bounds(&self) -> glam::UVec2 {
        glam::UVec2::new(
            self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x,
            self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y,
        )
    }

    pub(super) fn sync_runtime_viewport_to_window(&mut self, world_bounds: glam::UVec2) -> bool {
        if let Some(size) = self.platform.inner_size() {
            self.rendering.update_window_size(size);
        }
        let effective_viewport = self.rendering.effective_runtime_viewport();
        let viewport_changed = self
            .camera_system
            .apply_runtime_viewport(effective_viewport.world_viewport_size(), world_bounds);
        if self.rendering.has_gpu() {
            self.rendering
                .update_projection(self.camera_system.view_matrix());
        }
        viewport_changed
    }

    pub(super) fn create_scene_runtime_coordinator(&mut self) -> SceneRuntimeCoordinator<'_> {
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
                window_size: self.platform.inner_size(),
                content_root,
            },
        )
    }

    pub(super) fn tick(&mut self) {
        self.tick_internal(None)
    }

    pub(super) fn tick_with_delta(&mut self, delta_ms: f32) {
        self.tick_internal(Some(delta_ms))
    }

    pub(super) fn compute_game_update(&mut self, delta_ms: Option<f32>) -> GameUpdatePhase {
        let transition_delta_ms = delta_ms.unwrap_or(DEFAULT_TIMESTEP_MS).max(0.0) as u32;
        let world_bounds = self.current_world_bounds();
        let viewport_changed = self.sync_runtime_viewport_to_window(world_bounds);
        let result = if self.should_gate_gameplay_for_menu() || self.scene_transition.is_active() {
            GameUpdateResult::new()
        } else if let Some(delta) = delta_ms {
            GameSimulation::tick_with_delta(
                &mut self.game_system.game_state,
                delta,
                world_bounds,
                self.resources.get_tilemap(),
                self.resources.get_terrain_atlas(),
            )
        } else {
            GameSimulation::tick_fixed(
                &mut self.game_system.game_state,
                world_bounds,
                self.resources.get_tilemap(),
                self.resources.get_terrain_atlas(),
            )
        };
        GameUpdatePhase {
            result,
            viewport_changed,
            transition_delta_ms,
        }
    }

    pub(super) fn apply_game_update_side_effects(
        &mut self,
        game_result: &mut GameUpdateResult<AudioEvent>,
    ) {
        let listener_position = self.game_system.player_id().map(|_| {
            RenderQueryService::new(
                self.game_system.game_state.world().entity_manager(),
                self.game_system.game_state.world().player_id(),
                self.game_system
                    .game_state
                    .runtime()
                    .debug_collision_rendering(),
            )
            .player_position()
        });
        self.audio_system.set_listener_position(listener_position);

        for event in &game_result.events {
            self.audio_system.handle(event);
        }

        if let Some(request) = game_result.dialog_start_request.take() {
            self.apply_dialog_start_request(request);
        }

        if let Some(request) = game_result.scene_switch_request.take() {
            let mut coordinator = self.create_scene_runtime_coordinator();
            coordinator.queue_scene_switch_request(request);
        }

        if let Some(request) = game_result.persistence_request.take() {
            match request {
                toki_core::PersistenceRequest::SaveSlot { slot } => {
                    if let Err(error) = self.save_to_slot(slot) {
                        tracing::error!("Failed to save slot {} from rule action: {}", slot, error);
                    }
                }
                toki_core::PersistenceRequest::LoadSlot { slot } => {
                    if let Err(error) = self.load_from_slot(slot) {
                        tracing::error!("Failed to load slot {} from rule action: {}", slot, error);
                    }
                }
            }
        }
        for request in game_result.ui_requests.drain(..) {
            self.ui_controller.apply_request(request);
        }
    }

    fn advance_scene_runtime(&mut self, transition_delta_ms: u32) -> glam::UVec2 {
        {
            let mut coordinator = self.create_scene_runtime_coordinator();
            coordinator.advance_scene_transition(transition_delta_ms);
        }
        self.current_world_bounds()
    }

    fn update_camera_and_projection(
        &mut self,
        world_bounds: glam::UVec2,
        player_moved: bool,
        viewport_changed: bool,
    ) -> bool {
        let entities = self.game_system.entities_for_camera();
        let runtime = RuntimeState {
            entities: &entities,
        };
        self.camera_system.update(&runtime, world_bounds) || player_moved || viewport_changed
    }

    fn render_frame_if_ready(&mut self, cam_changed: bool) {
        if self.rendering.has_gpu() {
            self.rendering
                .set_post_process_settings(self.resolved_post_process_settings());
            if cam_changed {
                let view = self.camera_system.view_matrix();
                self.rendering.update_projection(view);

                if self
                    .camera_system
                    .update_chunk_cache(self.resources.get_tilemap())
                {
                    if let Some(atlas_size) = self.resources.terrain_image_size() {
                        let split = self
                            .resources
                            .get_tilemap()
                            .generate_split_vertices_for_chunks(
                                self.resources.get_terrain_atlas(),
                                atlas_size,
                                self.camera_system.cached_visible_chunks(),
                            );
                        self.rendering.update_tilemap_vertices(&split.below);
                        self.rendering.update_overlay_tilemap_vertices(&split.above);
                    } else {
                        tracing::warn!(
                            "Terrain image size unavailable; skipping tilemap vertex refresh"
                        );
                    }
                }
            }
            let viewport_scale_factor =
                self.rendering.viewport_presentation().layout.resolved_scale;
            self.performance
                .set_viewport_scale_factor(viewport_scale_factor);
            WorldFramePresenter::new(
                &self.game_system,
                &mut self.resources,
                &mut self.rendering,
                &self.launch_options.display,
                &self.performance,
            )
            .render_world_frame();
            self.render_authored_ui();
            self.render_runtime_menu_overlay();
            render_scene_transition_overlay(&mut self.rendering, &self.scene_transition);
            self.rendering.finalize_ui_shapes();
        }
    }

    fn tick_internal(&mut self, delta_ms: Option<f32>) {
        let tick_start = Instant::now();
        tracing::trace!("TICK @ {:?}", tick_start);

        let GameUpdatePhase {
            mut result,
            viewport_changed,
            transition_delta_ms,
        } = self.compute_game_update(delta_ms);
        self.apply_game_update_side_effects(&mut result);
        let world_bounds = self.advance_scene_runtime(transition_delta_ms);
        let cam_changed =
            self.update_camera_and_projection(world_bounds, result.player_moved, viewport_changed);
        self.render_frame_if_ready(cam_changed);

        self.platform.request_redraw();
    }

    pub(super) fn refresh_tilemap_vertices_for_current_camera(&mut self) {
        if !self.rendering.has_gpu() {
            return;
        }

        self.camera_system
            .update_chunk_cache(self.resources.get_tilemap());
        if let Some(atlas_size) = self.resources.terrain_image_size() {
            let split = self
                .resources
                .get_tilemap()
                .generate_split_vertices_for_chunks(
                    self.resources.get_terrain_atlas(),
                    atlas_size,
                    self.camera_system.cached_visible_chunks(),
                );
            self.rendering.update_tilemap_vertices(&split.below);
            self.rendering.update_overlay_tilemap_vertices(&split.above);
        } else {
            tracing::warn!("Terrain image size unavailable; skipping tilemap vertex refresh");
        }
    }
}
