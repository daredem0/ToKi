use std::time::Instant;

use toki_core::camera::RuntimeState;
use toki_core::{EventHandler, GameUpdateResult, DEFAULT_TIMESTEP_MS};

use super::app_presenter::{render_scene_transition_overlay, WorldFramePresenter};
use super::app_scene_runtime::SceneRuntimeCoordinator;
use super::App;

impl App {
    pub(super) fn tick(&mut self) {
        self.tick_internal(None)
    }

    pub(super) fn tick_with_delta(&mut self, delta_ms: f32) {
        self.tick_internal(Some(delta_ms))
    }

    fn tick_internal(&mut self, delta_ms: Option<f32>) {
        let tick_start = Instant::now();
        tracing::trace!("TICK @ {:?}", tick_start);

        let transition_delta_ms = delta_ms.unwrap_or(DEFAULT_TIMESTEP_MS).max(0.0) as u32;
        let mut world_bounds = glam::UVec2::new(
            self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x,
            self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y,
        );
        let game_result =
            if self.should_gate_gameplay_for_menu() || self.scene_transition.is_active() {
                GameUpdateResult::new()
            } else if let Some(delta) = delta_ms {
                self.game_system.update_with_delta(
                    delta,
                    world_bounds,
                    self.resources.get_tilemap(),
                    self.resources.get_terrain_atlas(),
                )
            } else {
                self.game_system.update(
                    world_bounds,
                    self.resources.get_tilemap(),
                    self.resources.get_terrain_atlas(),
                )
            };

        let listener_position = self
            .game_system
            .player_id()
            .map(|_| self.game_system.player_position());
        self.audio_system.set_listener_position(listener_position);

        for event in &game_result.events {
            self.audio_system.handle(event);
        }

        if let Some(request) = game_result.scene_switch_request.clone() {
            let content_root = self.content_root_path().map(std::path::Path::to_path_buf);
            let mut coordinator = SceneRuntimeCoordinator::new(
                &mut self.game_system,
                &mut self.camera_system,
                &mut self.resources,
                &mut self.rendering,
                &mut self.audio_system,
                &mut self.decoded_project_cache,
                &mut self.asset_load_plan,
                &mut self.scene_transition,
                &self.launch_options.audio_mix,
                content_root,
            );
            coordinator.queue_scene_switch_request(request);
        }

        {
            let content_root = self.content_root_path().map(std::path::Path::to_path_buf);
            let mut coordinator = SceneRuntimeCoordinator::new(
                &mut self.game_system,
                &mut self.camera_system,
                &mut self.resources,
                &mut self.rendering,
                &mut self.audio_system,
                &mut self.decoded_project_cache,
                &mut self.asset_load_plan,
                &mut self.scene_transition,
                &self.launch_options.audio_mix,
                content_root,
            );
            coordinator.advance_scene_transition(transition_delta_ms);
        }
        world_bounds = glam::UVec2::new(
            self.resources.tilemap_size().x * self.resources.tilemap_tile_size().x,
            self.resources.tilemap_size().y * self.resources.tilemap_tile_size().y,
        );

        let player_moved = game_result.player_moved;
        let entities = self.game_system.entities_for_camera();
        let runtime = RuntimeState {
            entities: &entities,
        };
        let cam_changed = self.camera_system.update(&runtime, world_bounds) || player_moved;

        if self.rendering.has_gpu() {
            if cam_changed {
                let view = self.camera_system.view_matrix();
                self.rendering.update_projection(view);

                if self
                    .camera_system
                    .update_chunk_cache(self.resources.get_tilemap())
                {
                    let atlas_size = self.resources.terrain_image_size().unwrap();
                    let verts = self.resources.get_tilemap().generate_vertices_for_chunks(
                        self.resources.get_terrain_atlas(),
                        atlas_size,
                        self.camera_system.cached_visible_chunks(),
                    );
                    self.rendering.update_tilemap_vertices(&verts);
                }
            }
            WorldFramePresenter::new(
                &self.game_system,
                &mut self.resources,
                &mut self.rendering,
                &self.launch_options.display,
                &self.performance,
            )
            .render_world_frame();
            self.render_runtime_menu_overlay();
            render_scene_transition_overlay(&mut self.rendering, &self.scene_transition);
            self.rendering.finalize_ui_shapes();
        }

        self.platform.request_redraw();
    }

    pub(super) fn refresh_tilemap_vertices_for_current_camera(&mut self) {
        if !self.rendering.has_gpu() {
            return;
        }

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
