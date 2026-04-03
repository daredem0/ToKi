use std::time::Instant;

use directories::ProjectDirs;
use toki_core::game::SceneSystem;
use toki_core::menu::MenuInput;
use toki_core::serialization::{load_save_data_from_slot, save_game_to_slot, save_slot_file_path};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::WindowId;

use super::App;

impl App {
    fn event_matches_key(event: &KeyEvent, physical: KeyCode, logical: NamedKey) -> bool {
        matches!(event.physical_key, PhysicalKey::Code(keycode) if keycode == physical)
            || matches!(&event.logical_key, Key::Named(named) if *named == logical)
            || matches!(event.key_without_modifiers(), Key::Named(named) if named == logical)
    }

    pub(super) fn resolve_save_root_from_base(
        base: Option<&std::path::Path>,
        cwd: Option<&std::path::Path>,
        project_path: Option<&std::path::Path>,
    ) -> std::path::PathBuf {
        let project_name = project_path
            .and_then(std::path::Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("default");

        base.map(std::path::Path::to_path_buf)
            .or_else(|| cwd.map(std::path::Path::to_path_buf))
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("toki")
            .join(project_name)
            .join("saves")
    }

    pub(super) fn resolve_save_root(&self) -> std::path::PathBuf {
        Self::resolve_save_root_for_launch_options(&self.launch_options)
    }

    pub(super) fn resolve_save_root_for_launch_options(
        launch_options: &super::RuntimeLaunchOptions,
    ) -> std::path::PathBuf {
        let data_dir =
            ProjectDirs::from("", "", "toki").map(|dirs| dirs.data_local_dir().to_path_buf());
        Self::resolve_save_root_from_base(
            data_dir.as_deref(),
            std::env::current_dir().ok().as_deref(),
            launch_options.project_path.as_deref(),
        )
    }

    pub(super) fn save_to_slot(&mut self, slot: u8) -> anyhow::Result<std::path::PathBuf> {
        let save_root = self.resolve_save_root();
        let path = save_game_to_slot(&mut self.game_system.game_state, &save_root, slot)?;
        tracing::info!(
            "Saved slot {} to '{}' (scene='{}')",
            slot,
            path.display(),
            self.game_system.active_scene_name().unwrap_or("<none>")
        );
        Ok(path)
    }

    pub(super) fn load_from_slot(&mut self, slot: u8) -> anyhow::Result<()> {
        let save_root = self.resolve_save_root();
        let save_path = save_slot_file_path(&save_root, slot)?;
        let save_data = load_save_data_from_slot(&save_root, slot)?;
        let active_scene_name = save_data.active_scene_name.clone();
        SceneSystem::restore_from_save_data(&mut self.game_system.game_state, &save_data)
            .map_err(anyhow::Error::from)?;
        self.refresh_runtime_after_scene_restore();
        tracing::info!(
            "Loaded slot {} from '{}' (scene='{}')",
            slot,
            save_path.display(),
            active_scene_name
        );
        Ok(())
    }

    fn handle_keyboard_input_event(&mut self, event: winit::event::KeyEvent) {
        match event.state {
            ElementState::Pressed => {
                if Self::event_matches_key(&event, KeyCode::F3, NamedKey::F3) {
                    self.performance.toggle_hud_display();
                    return;
                }
                if Self::event_matches_key(&event, KeyCode::F7, NamedKey::F7) {
                    self.performance.toggle_console_display();
                    return;
                }
                if Self::event_matches_key(&event, KeyCode::F5, NamedKey::F5) {
                    tracing::info!("Hotkey requested save to slot 1");
                    if let Err(e) = self.save_to_slot(1) {
                        tracing::error!("Failed to save game: {}", e);
                    }
                    return;
                }
                if Self::event_matches_key(&event, KeyCode::F6, NamedKey::F6) {
                    tracing::info!("Hotkey requested load from slot 1");
                    match self.load_from_slot(1) {
                        Ok(()) => {}
                        Err(e) => tracing::error!("Failed to load game: {}", e),
                    }
                    return;
                }

                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match keycode {
                        KeyCode::Escape => {
                            if self.is_dialog_open() || self.is_menu_open() {
                                self.handle_menu_input(MenuInput::Back);
                            } else {
                                self.open_pause_menu();
                            }
                        }
                        KeyCode::ArrowUp | KeyCode::KeyW
                            if self.is_dialog_open() || self.is_menu_open() =>
                        {
                            self.handle_menu_input(MenuInput::Up);
                        }
                        KeyCode::ArrowDown | KeyCode::KeyS
                            if self.is_dialog_open() || self.is_menu_open() =>
                        {
                            self.handle_menu_input(MenuInput::Down);
                        }
                        KeyCode::ArrowLeft | KeyCode::KeyA
                            if self.is_dialog_open() || self.is_menu_open() =>
                        {
                            self.handle_menu_input(MenuInput::Left);
                        }
                        KeyCode::ArrowRight | KeyCode::KeyD
                            if self.is_dialog_open() || self.is_menu_open() =>
                        {
                            self.handle_menu_input(MenuInput::Right);
                        }
                        KeyCode::Enter | KeyCode::Space
                            if self.is_dialog_open() || self.is_menu_open() =>
                        {
                            self.handle_menu_input(MenuInput::Confirm);
                        }
                        KeyCode::Tab if !self.is_dialog_open() && !self.is_menu_open() => {
                            if self.focus_next_authored_ui_widget() {
                                return;
                            }
                            self.game_system.handle_keyboard_input(keycode, true);
                        }
                        KeyCode::Enter | KeyCode::Space
                            if !self.is_dialog_open() && !self.is_menu_open() =>
                        {
                            if self.activate_focused_authored_ui_widget() {
                                return;
                            }
                            self.game_system.handle_keyboard_input(keycode, true);
                        }
                        _ => {
                            if !self.is_dialog_open() && !self.is_menu_open() {
                                self.game_system.handle_keyboard_input(keycode, true);
                            }
                        }
                    }
                }
            }
            ElementState::Released => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    if !self.is_dialog_open() && !self.is_menu_open() {
                        self.game_system.handle_keyboard_input(keycode, false);
                    }
                }
            }
        }
    }

    fn handle_resize_event(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.rendering.resize(new_size);
        let world_bounds = self.current_world_bounds();
        self.sync_runtime_viewport_to_window(world_bounds);
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
                let world_bounds = self.current_world_bounds();
                self.sync_runtime_viewport_to_window(world_bounds);
                self.refresh_tilemap_vertices_for_current_camera();
                self.tick();
                self.timing.reset();
                self.platform.request_redraw();
            }

            let world_bounds = self.current_world_bounds();
            self.sync_runtime_viewport_to_window(world_bounds);
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
            if let Err(error) = self.rendering.draw() {
                tracing::error!("Failed to render frame: {error}");
            }
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

        // Window size is the game resolution (zoom affects camera view, not window size)
        self.platform.initialize_window(
            event_loop,
            self.launch_options.display.resolution_width,
            self.launch_options.display.resolution_height,
        );

        if let Some(window) = self.platform.window_for_gpu() {
            if let Err(error) = self.rendering.initialize_gpu_with_textures(
                window.clone(),
                self.launch_options.display.vsync,
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
                if let Err(fallback_error) = self
                    .rendering
                    .initialize_gpu(window, self.launch_options.display.vsync)
                {
                    tracing::error!(
                        "Failed to initialize GPU with default textures: {fallback_error}"
                    );
                }
            } else {
                self.post_splash_sprite_texture_path =
                    self.asset_load_plan.sprite_texture_path.clone();
            }
        }

        if self.rendering.has_gpu() {
            if let Some(scene_name) = self.game_system.active_scene_name().map(str::to_string) {
                self.reload_runtime_render_textures(&scene_name);
            } else if let Some(content_root) = content_root.as_deref() {
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
        self.rendering
            .set_post_process_settings(self.resolved_post_process_settings());

        self.platform.request_redraw();
        self.refresh_tilemap_vertices_for_current_camera();

        self.audio_system.list_available_sounds();
        if let Some(track_id) = SceneSystem::active_scene(&self.game_system.game_state)
            .and_then(|scene| scene.background_music_track_id.as_deref())
        {
            if let Err(error) = self.scene_transition.ensure_scene_music(
                &mut self.audio_system,
                Some(track_id),
                self.launch_options.audio_mix.music_percent,
            ) {
                tracing::warn!(
                    "Failed to ensure startup scene background music '{}': {}",
                    track_id,
                    error
                );
            }
        }
        if self.launch_options.scene_name.is_none()
            && SceneSystem::active_scene(&self.game_system.game_state)
                .and_then(|scene| scene.background_music_track_id.as_deref())
                .is_none()
        {
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

        match self.launch_options.display.timing_mode {
            toki_core::TimingMode::Fixed => {
                // Process game ticks at fixed rate (60 FPS game logic)
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
            }
            toki_core::TimingMode::Delta => {
                // Process single tick with actual elapsed time
                let now = Instant::now();
                let delta_ms = self
                    .last_tick_instant
                    .map(|last| now.duration_since(last).as_secs_f32() * 1000.0)
                    .unwrap_or(toki_core::DEFAULT_TIMESTEP_MS);
                self.last_tick_instant = Some(now);

                let tick_start = Instant::now();
                self.tick_with_delta(delta_ms);
                let tick_time = tick_start.elapsed();
                self.performance.record_tick_time(tick_time);
            }
        }

        // Visual frame rate limiting (separate from game tick rate)
        let now = Instant::now();
        let wait_duration = self.frame_limiter.next_frame(now);
        if !wait_duration.is_zero() {
            std::thread::sleep(wait_duration);
        }

        self.platform.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        tracing::trace!("{event:?}");

        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_input_event(event);
                if self.exit_requested {
                    tracing::info!("Menu requested runtime exit; stopping");
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(glam::Vec2::new(position.x as f32, position.y as f32));
                if self.left_mouse_down {
                    self.handle_menu_pointer_drag(glam::Vec2::new(
                        position.x as f32,
                        position.y as f32,
                    ));
                }
                self.handle_menu_pointer_hover(glam::Vec2::new(
                    position.x as f32,
                    position.y as f32,
                ));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.left_mouse_down = true;
                if let Some(cursor_position) = self.cursor_position {
                    self.handle_menu_pointer_click(cursor_position);
                    if self.exit_requested {
                        tracing::info!("Menu requested runtime exit; stopping");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.left_mouse_down = false;
                self.clear_menu_pointer_drag();
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

#[cfg(test)]
mod tests {
    use super::App;
    use std::path::Path;

    #[test]
    fn resolve_save_root_from_base_uses_project_name_when_available() {
        let root = App::resolve_save_root_from_base(
            Some(Path::new("/data")),
            None,
            Some(Path::new("/projects/Demo")),
        );

        assert_eq!(
            root,
            Path::new("/data").join("toki").join("Demo").join("saves")
        );
    }

    #[test]
    fn resolve_save_root_from_base_falls_back_to_current_dir_and_default_project() {
        let root = App::resolve_save_root_from_base(None, Some(Path::new("/cwd")), None);

        assert_eq!(
            root,
            Path::new("/cwd").join("toki").join("default").join("saves")
        );
    }
}
