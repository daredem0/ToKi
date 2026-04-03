//! Simple winit window example.
use serde::{Deserialize, Serialize};
use winit::event_loop::EventLoop;

use std::path::PathBuf;
use std::time::Instant;

use toki_core::camera::{Camera, CameraController, CameraMode};
use toki_core::dialog_runtime::DialogController;
use toki_core::game::SceneSystem;
use toki_core::menu::{DialogThemeOverride, MenuController, MenuSettings};
use toki_core::project_runtime::{
    RuntimeFlagSettings, RuntimePostProcessSettings, RuntimeUiSettings, RuntimeViewportMode,
};
use toki_core::ui_layout::UiController;
use toki_core::TimingSystem;
use toki_render::RenderError;

use crate::systems::AudioManager;
use crate::systems::{
    CameraManager, DecodedProjectCache, FrameLimiter, GameManager, PerformanceMonitor,
    PlatformSystem, RenderingSystem, ResourceManager, RuntimeAssetLoadPlan,
};

const COMMUNITY_SPLASH_MIN_DURATION_MS: u64 = 3000;
const COMMUNITY_SPLASH_MAX_DURATION_MS: u64 = 10000;
const COMMUNITY_SPLASH_DEFAULT_DURATION_MS: u64 = 3000;
const COMMUNITY_SPLASH_BRANDING_TEXT: &str = "Powered by ToKi";
const COMMUNITY_SPLASH_VERSION_TEXT: &str = env!("TOKI_VERSION");
const SPLASH_LOGO_WIDTH: u32 = 128;
const SPLASH_LOGO_HEIGHT: u32 = 108;
const SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER: f32 = 1.25;
const SPLASH_BRANDING_VERSION_GAP_PX: f32 = 4.0;
const SPLASH_VERSION_DEFAULT_SIZE_PX: f32 = 32.0;
const SPLASH_VERSION_MIN_SIZE_PX: f32 = 7.0;
const SPLASH_TEXT_HORIZONTAL_PADDING_PX: f32 = 8.0;
const COMMUNITY_SPLASH_LOGO_PNG: &[u8] = include_bytes!("../../../assets/TokiLogo.png");

#[path = "app_bootstrap.rs"]
mod app_bootstrap;
#[path = "app_lifecycle.rs"]
mod app_lifecycle;
#[path = "app_presenter.rs"]
mod app_presenter;
#[path = "app_runtime_display_settings.rs"]
mod app_runtime_display_settings;
#[path = "app_runtime_persistence.rs"]
mod app_runtime_persistence;
#[path = "app_runtime_settings/mod.rs"]
mod app_runtime_settings;
#[path = "app_scene_runtime.rs"]
mod app_scene_runtime;
#[path = "app_splash.rs"]
mod app_splash;
#[path = "app_tick.rs"]
mod app_tick;
#[path = "app_transition.rs"]
mod app_transition;
#[path = "runtime_menu.rs"]
mod runtime_menu;

use app_runtime_settings::RuntimeMenuOverlay;
use app_splash::{ResolvedSplashConfig, SplashPolicy};
use app_transition::SceneTransitionController;
use toki_core::project_assets::first_existing_path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSplashOptions {
    pub duration_ms: u64,
    pub show_branding: bool,
}

impl Default for RuntimeSplashOptions {
    fn default() -> Self {
        Self {
            duration_ms: COMMUNITY_SPLASH_DEFAULT_DURATION_MS,
            show_branding: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeAudioMixOptions {
    pub master_percent: u8,
    pub music_percent: u8,
    pub movement_percent: u8,
    pub collision_percent: u8,
}

impl Default for RuntimeAudioMixOptions {
    fn default() -> Self {
        Self {
            master_percent: 100,
            music_percent: 100,
            movement_percent: 100,
            collision_percent: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeDisplayOptions {
    pub show_entity_health_bars: bool,
    pub show_ground_shadows: bool,
    pub indexed_palette_override: Option<String>,
    pub post_process: RuntimePostProcessSettings,
    pub resolution_width: u32,
    pub resolution_height: u32,
    /// Zoom level as percentage (100 = 1.0x, 200 = 2.0x, etc.)
    pub zoom_percent: u32,
    /// Presentation policy for fitting the logical viewport into the window.
    pub viewport: RuntimeViewportMode,
    /// Enable vsync (ties frame rate to display refresh rate).
    /// When enabled, frame limiter is bypassed.
    pub vsync: bool,
    /// Target frames per second when vsync is disabled.
    /// Set to 0 for unlimited frame rate.
    pub target_fps: u32,
    /// Timing mode for game logic (fixed or delta timestep).
    pub timing_mode: toki_core::TimingMode,
}

impl Default for RuntimeDisplayOptions {
    fn default() -> Self {
        Self {
            show_entity_health_bars: false,
            show_ground_shadows: toki_core::project_runtime::default_show_ground_shadows(),
            indexed_palette_override: None,
            post_process: RuntimePostProcessSettings::default(),
            resolution_width: toki_core::project_runtime::default_resolution_width(),
            resolution_height: toki_core::project_runtime::default_resolution_height(),
            zoom_percent: toki_core::project_runtime::default_zoom_percent(),
            viewport: toki_core::project_runtime::default_runtime_viewport_mode(),
            vsync: true,
            target_fps: 60,
            timing_mode: toki_core::TimingMode::default(),
        }
    }
}

impl RuntimeDisplayOptions {
    /// Returns the zoom level as a float (1.0 = 100%, 2.0 = 200%, etc.)
    pub fn zoom_factor(&self) -> f32 {
        self.zoom_percent as f32 / 100.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeTransitionOptions {
    pub fade_duration_ms: u32,
}

impl Default for RuntimeTransitionOptions {
    fn default() -> Self {
        Self {
            fade_duration_ms: 250,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeLaunchOptions {
    pub project_path: Option<PathBuf>,
    pub pack_path: Option<PathBuf>,
    pub scene_name: Option<String>,
    pub map_name: Option<String>,
    pub scene_persistence: bool,
    pub splash: RuntimeSplashOptions,
    pub audio_mix: RuntimeAudioMixOptions,
    pub display: RuntimeDisplayOptions,
    pub transition: RuntimeTransitionOptions,
    pub flags: RuntimeFlagSettings,
    pub menu: MenuSettings,
    pub dialog_theme_override: DialogThemeOverride,
    pub ui: RuntimeUiSettings,
}

#[derive(Debug)]
struct StartupCoordinator {
    splash_policy: SplashPolicy,
}

#[derive(Debug, Default)]
struct TickCoordinator {
    /// Last tick instant for delta time calculation in delta timing mode.
    last_tick_instant: Option<Instant>,
}

#[derive(Debug)]
struct RenderCoordinator {
    splash_config: ResolvedSplashConfig,
    splash_active: bool,
    splash_started_at: Option<Instant>,
    splash_logo_loaded: bool,
    post_splash_sprite_texture_path: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct MenuCoordinator {
    runtime_overlay: Option<RuntimeMenuOverlay>,
    exit_requested: bool,
    pending_ui_events: Vec<String>,
    cursor_position: Option<glam::Vec2>,
    left_mouse_down: bool,
    runtime_overlay_slider_drag: Option<usize>,
}

#[derive(Debug, Default)]
struct PersistenceCoordinator;

#[derive(Debug)]
struct App {
    // Core systems
    game_system: GameManager,
    camera_system: CameraManager,
    resources: ResourceManager,
    performance: PerformanceMonitor,
    audio_system: AudioManager,

    // Grouped systems
    platform: PlatformSystem,
    rendering: RenderingSystem,
    timing: TimingSystem,
    frame_limiter: FrameLimiter,
    launch_options: RuntimeLaunchOptions,
    menu_system: MenuController,
    dialog_system: DialogController,
    ui_controller: UiController,
    startup: StartupCoordinator,
    tick_coordinator: TickCoordinator,
    render_coordinator: RenderCoordinator,
    menu_coordinator: MenuCoordinator,
    persistence: PersistenceCoordinator,
    asset_load_plan: RuntimeAssetLoadPlan,
    scene_transition: SceneTransitionController,
    decoded_project_cache: DecodedProjectCache,
    pack_mount: Option<tempfile::TempDir>,
}

struct BuiltStartupState {
    resources: ResourceManager,
    game_state: toki_core::GameState,
    dialogs: Vec<toki_core::dialog::DialogTree>,
    ui_layouts: Vec<toki_core::ui_layout::UiLayoutAsset>,
    pack_mount: Option<tempfile::TempDir>,
    asset_load_plan: RuntimeAssetLoadPlan,
    decoded_project_cache: DecodedProjectCache,
}

struct AppBuilder {
    launch_options: RuntimeLaunchOptions,
    splash_policy: SplashPolicy,
}

impl AppBuilder {
    fn new(launch_options: RuntimeLaunchOptions) -> Self {
        Self {
            launch_options,
            splash_policy: SplashPolicy::Community,
        }
    }

    fn apply_persisted_runtime_settings(&mut self) {
        App::apply_persisted_runtime_settings_from_disk(&mut self.launch_options);
    }

    fn resolve_splash_config(&self) -> ResolvedSplashConfig {
        self.splash_policy.resolve(&self.launch_options.splash)
    }

    fn build_startup_state(&self) -> BuiltStartupState {
        let (
            resources,
            game_state,
            dialogs,
            ui_layouts,
            pack_mount,
            asset_load_plan,
            decoded_project_cache,
        ) = App::build_startup_state(&self.launch_options);
        BuiltStartupState {
            resources,
            game_state,
            dialogs,
            ui_layouts,
            pack_mount,
            asset_load_plan,
            decoded_project_cache,
        }
    }

    fn build_rendering_system(&self) -> RenderingSystem {
        let mut rendering = RenderingSystem::new_with_desired_resolution(
            self.launch_options.display.resolution_width,
            self.launch_options.display.resolution_height,
        );
        rendering.set_viewport_mode(self.launch_options.display.viewport);
        rendering
    }

    fn build(mut self) -> App {
        self.apply_persisted_runtime_settings();
        let splash_config = self.resolve_splash_config();
        let startup = self.build_startup_state();
        let game_system = GameManager::new(startup.game_state);
        let camera_system = App::build_camera_system(&self.launch_options, &game_system);
        let mut audio_system = App::build_audio_system(
            &self.launch_options,
            startup.pack_mount.as_ref(),
            &startup.asset_load_plan,
        );
        let menu_system = MenuController::new(self.launch_options.menu.clone());
        let dialog_system = DialogController::new(startup.dialogs);
        let ui_controller = UiController::new(startup.ui_layouts);
        let mut scene_transition =
            SceneTransitionController::new(self.launch_options.transition.clone());
        App::prime_initial_scene_music(
            &game_system,
            &mut scene_transition,
            &mut audio_system,
            &self.launch_options,
        );
        let frame_limiter = App::build_frame_limiter(&self.launch_options);
        let rendering = self.build_rendering_system();

        App {
            game_system,
            camera_system,
            resources: startup.resources,
            performance: PerformanceMonitor::new(),
            audio_system,
            platform: PlatformSystem::new(),
            rendering,
            timing: TimingSystem::new(),
            frame_limiter,
            launch_options: self.launch_options,
            menu_system,
            dialog_system,
            ui_controller,
            startup: StartupCoordinator::new(self.splash_policy),
            tick_coordinator: TickCoordinator::default(),
            render_coordinator: RenderCoordinator::new(splash_config),
            menu_coordinator: MenuCoordinator::default(),
            persistence: PersistenceCoordinator,
            asset_load_plan: startup.asset_load_plan,
            scene_transition,
            decoded_project_cache: startup.decoded_project_cache,
            pack_mount: startup.pack_mount,
        }
    }
}

impl StartupCoordinator {
    fn new(splash_policy: SplashPolicy) -> Self {
        Self { splash_policy }
    }
}

impl TickCoordinator {
    fn next_delta_ms(&mut self, now: Instant) -> f32 {
        let delta_ms = self
            .last_tick_instant
            .map(|last| now.duration_since(last).as_secs_f32() * 1000.0)
            .unwrap_or(toki_core::DEFAULT_TIMESTEP_MS);
        self.last_tick_instant = Some(now);
        delta_ms
    }
}

impl RenderCoordinator {
    fn new(splash_config: ResolvedSplashConfig) -> Self {
        Self {
            splash_config,
            splash_active: true,
            splash_started_at: None,
            splash_logo_loaded: false,
            post_splash_sprite_texture_path: None,
        }
    }
}

impl App {
    fn build_camera_system(
        launch_options: &RuntimeLaunchOptions,
        game_system: &GameManager,
    ) -> CameraManager {
        let resolution_width = launch_options.display.resolution_width;
        let resolution_height = launch_options.display.resolution_height;
        let zoom_factor = launch_options.display.zoom_factor();
        let mut camera =
            Camera::with_resolution_and_zoom(resolution_width, resolution_height, zoom_factor);
        camera.center_on(glam::IVec2::new(
            (resolution_width / 2) as i32,
            (resolution_height / 2) as i32,
        ));

        let controller = if let Some(player_id) = game_system.player_id() {
            CameraController {
                mode: CameraMode::FollowEntity(player_id),
            }
        } else {
            CameraController {
                mode: CameraMode::FreeScroll,
            }
        };
        CameraManager::new(camera, controller)
    }

    fn resolve_audio_root(
        launch_options: &RuntimeLaunchOptions,
        pack_mount: Option<&tempfile::TempDir>,
    ) -> std::path::PathBuf {
        pack_mount
            .map(tempfile::TempDir::path)
            .or(launch_options.project_path.as_deref())
            .map(std::path::Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .expect("Failed to resolve audio root path")
    }

    fn build_audio_system(
        launch_options: &RuntimeLaunchOptions,
        pack_mount: Option<&tempfile::TempDir>,
        asset_load_plan: &RuntimeAssetLoadPlan,
    ) -> AudioManager {
        let audio_root = Self::resolve_audio_root(launch_options, pack_mount);
        let mut audio_system = AudioManager::new_with_assets_root_and_preload_names(
            audio_root,
            &asset_load_plan.preloaded_sfx_names,
        )
        .expect("Failed to initialize audio system");
        audio_system.set_master_volume_percent(launch_options.audio_mix.master_percent);
        audio_system.set_channel_volume_percent("music", launch_options.audio_mix.music_percent);
        audio_system.set_channel_volume_percent("music_a", launch_options.audio_mix.music_percent);
        audio_system.set_channel_volume_percent("music_b", launch_options.audio_mix.music_percent);
        audio_system
            .set_channel_volume_percent("movement", launch_options.audio_mix.movement_percent);
        audio_system
            .set_channel_volume_percent("collision", launch_options.audio_mix.collision_percent);
        audio_system
    }

    fn prime_initial_scene_music(
        game_system: &GameManager,
        scene_transition: &mut SceneTransitionController,
        audio_system: &mut AudioManager,
        launch_options: &RuntimeLaunchOptions,
    ) {
        if let Some(track_id) = SceneSystem::active_scene(&game_system.game_state)
            .and_then(|scene| scene.background_music_track_id.as_deref())
        {
            if let Err(error) = scene_transition.prime_scene_music(
                audio_system,
                Some(track_id),
                launch_options.audio_mix.music_percent,
            ) {
                tracing::warn!(
                    "Failed to start initial scene background music '{track_id}': {error}"
                );
            }
        }
    }

    fn build_frame_limiter(launch_options: &RuntimeLaunchOptions) -> FrameLimiter {
        if launch_options.display.vsync {
            FrameLimiter::new_unlimited()
        } else {
            FrameLimiter::new_with_target_fps(launch_options.display.target_fps)
        }
    }

    fn resolved_post_process_settings(
        &self,
    ) -> toki_core::project_runtime::ResolvedPostProcessSettings {
        self.launch_options
            .display
            .post_process
            .resolve(self.resources.project_palettes())
    }

    fn content_root_path(&self) -> Option<&std::path::Path> {
        self.pack_mount
            .as_ref()
            .map(tempfile::TempDir::path)
            .or(self.launch_options.project_path.as_deref())
    }

    fn new(launch_options: RuntimeLaunchOptions) -> Self {
        AppBuilder::new(launch_options).build()
    }
}
/// Runs a minimal window using the winit library.
pub fn run_minimal_window() -> Result<(), RenderError> {
    run_minimal_window_with_options(RuntimeLaunchOptions::default())
}

pub fn run_minimal_window_with_options(
    launch_options: RuntimeLaunchOptions,
) -> Result<(), RenderError> {
    let event_loop = EventLoop::new()?;

    // Create an instance of the App struct
    let mut app = App::new(launch_options);

    // Run the application
    event_loop.run_app(&mut app)?;

    // Return Ok if the application was closed successfully
    Ok(())
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
