use std::time::Instant;

use toki_core::serialization::{load_game, save_game};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowId;

use super::App;

impl App {
    fn handle_keyboard_input_event(&mut self, event: winit::event::KeyEvent) {
        use winit::event::ElementState;
        if let PhysicalKey::Code(keycode) = event.physical_key {
            match event.state {
                ElementState::Pressed => match keycode {
                    KeyCode::F3 => {
                        self.performance.toggle_hud_display();
                    }
                    KeyCode::F7 => {
                        self.performance.toggle_console_display();
                    }
                    KeyCode::F5 => {
                        if let Err(e) = save_game(&self.game_system.game_state, "savegame.json") {
                            tracing::error!("Failed to save game: {}", e);
                        } else {
                            tracing::info!("Game saved to savegame.json");
                        }
                    }
                    KeyCode::F6 => match load_game("savegame.json") {
                        Ok(loaded_state) => {
                            self.game_system.game_state = loaded_state;
                            tracing::info!("Game loaded from savegame.json");
                        }
                        Err(e) => tracing::error!("Failed to load game: {}", e),
                    },
                    _ => {
                        self.game_system.handle_keyboard_input(keycode, true);
                    }
                },
                ElementState::Released => {
                    self.game_system.handle_keyboard_input(keycode, false);
                }
            }
        }
    }

    fn handle_resize_event(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.rendering.resize(new_size);
        let view = self.camera_system.view_matrix();
        self.rendering.update_projection(view);
        self.platform.request_redraw();
    }

    fn handle_redraw_request_event(&mut self) {
        let frame_start = Instant::now();
        self.performance.record_frame_interval(frame_start);
        self.platform.pre_present_notify();

        if self.rendering.has_gpu() {
            if self.splash_active {
                let started_at = self.splash_started_at.unwrap_or_else(|| {
                    let now = Instant::now();
                    self.splash_started_at = Some(now);
                    now
                });

                if started_at.elapsed() < self.splash_config.duration {
                    self.render_startup_splash();
                    return;
                }

                self.splash_active = false;
                self.rendering.set_tilemap_render_enabled(true);
                self.restore_runtime_sprite_texture_after_splash();
                self.rendering
                    .update_projection(self.camera_system.view_matrix());
                self.refresh_tilemap_vertices_for_current_camera();
                self.tick();
                self.timing.reset();
                self.platform.request_redraw();
            }

            if let Some(size) = self.platform.inner_size() {
                self.rendering.update_window_size(size);
            }
            let left = self.camera_system.position().x;
            let top = self.camera_system.position().y;
            let right = left + self.camera_system.viewport_size().x as i32;
            let bottom = top + self.camera_system.viewport_size().y as i32;

            tracing::trace!(
                "Camera Viewport in world space: left={}, right={}, top={}, bottom={}",
                left,
                right,
                top,
                bottom
            );
            tracing::trace!("Camera position: {:?}", self.camera_system.position());
            tracing::trace!("Window size: {:?}", self.platform.inner_size());
            tracing::trace!(
                "Camera projection: {:?}",
                self.camera_system.projection_matrix()
            );
            tracing::trace!("Window Scale Factor: {:?}", self.platform.scale_factor());

            let cpu_work_time = frame_start.elapsed();
            let draw_start = Instant::now();
            self.rendering.draw();
            let draw_time = draw_start.elapsed();
            let total_frame_time = frame_start.elapsed();
            self.performance.record_performance_breakdown(
                cpu_work_time,
                draw_time,
                total_frame_time,
            );
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let content_root = self.content_root_path().map(std::path::Path::to_path_buf);

        self.platform.initialize_window(event_loop);

        if let Some(window) = self.platform.window_for_gpu() {
            if let Err(error) = self.rendering.initialize_gpu_with_textures(
                window.clone(),
                self.asset_load_plan.tilemap_texture_path.clone(),
                self.asset_load_plan.sprite_texture_path.clone(),
            ) {
                if let Some(content_root) = content_root.as_deref() {
                    tracing::error!(
                        "Failed to initialize GPU with runtime asset plan from '{}': {}",
                        content_root.display(),
                        error
                    );
                } else {
                    tracing::error!("Failed to initialize GPU with runtime asset plan: {error}");
                }
                self.rendering.initialize_gpu(window);
            } else {
                self.post_splash_sprite_texture_path =
                    self.asset_load_plan.sprite_texture_path.clone();
            }
        }

        if self.rendering.has_gpu() {
            if let Some(content_root) = content_root.as_deref() {
                if let Err(error) = self.rendering.load_project_textures(content_root) {
                    tracing::warn!(
                        "Failed to load project textures from '{}': {}",
                        content_root.display(),
                        error
                    );
                }
            }
        }

        self.post_splash_sprite_texture_path =
            self.post_splash_sprite_texture_path.clone().or_else(|| {
                Self::resolve_post_splash_sprite_texture_path(
                    &self.launch_options,
                    content_root.as_deref(),
                )
            });
        self.initialize_splash_resources();

        if let Some(size) = self.platform.inner_size() {
            self.rendering.update_window_size(size);
        }

        let view = self.camera_system.view_matrix();
        self.rendering.update_projection(view);

        self.platform.request_redraw();
        self.refresh_tilemap_vertices_for_current_camera();

        self.audio_system.list_available_sounds();
        if self.launch_options.scene_name.is_none() {
            if let Err(e) = self.audio_system.play_background_music("lavandia", -10.0) {
                tracing::warn!("Failed to start background music: {}", e);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.splash_active && self.rendering.has_gpu() {
            self.platform.request_redraw();
            return;
        }

        let mut tick_count = 0;
        while self.timing.should_tick() {
            let tick_start = Instant::now();
            self.tick();
            let tick_time = tick_start.elapsed();
            self.performance.record_tick_time(tick_time);
            self.timing.consume_timestep();
            tick_count += 1;
            if tick_count > 10 {
                break;
            }
        }
        self.platform.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        tracing::trace!("{event:?}");

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input_event(event);
            }
            WindowEvent::CloseRequested => {
                tracing::info!("Close was requested; stopping");
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resize_event(new_size);
            }
            WindowEvent::RedrawRequested => {
                self.handle_redraw_request_event();
            }
            _ => (),
        }
    }
}
