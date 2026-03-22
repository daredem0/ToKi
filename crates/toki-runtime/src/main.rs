use anyhow::Result;
use std::path::PathBuf;
use toki_core::menu::MenuSettings;
use toki_core::project_runtime::{ProjectRuntimeMetadata, RuntimeConfigFile};
use tracing_subscriber::EnvFilter;

use toki_runtime::{
    run_minimal_window, run_minimal_window_with_options, RuntimeAudioMixOptions,
    RuntimeDisplayOptions, RuntimeLaunchOptions,
};

fn main() -> Result<()> {
    let mut env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
    for directive in ["cosmic_text=info", "glyphon=info"] {
        if let Ok(parsed) = directive.parse() {
            env_filter = env_filter.add_directive(parsed);
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_file(false)
        .with_line_number(true)
        .init();

    tracing::info!("Starting ToKi runtime {}", env!("TOKI_VERSION"));

    let mut launch_options = parse_launch_options(std::env::args().skip(1).collect());
    if launch_options.project_path.is_none() {
        launch_options = apply_runtime_config_if_present(launch_options);
    }
    if launch_options.project_path.is_none() && launch_options.pack_path.is_none() {
        launch_options = auto_detect_project_launch_options(launch_options);
    }
    launch_options = apply_project_runtime_settings_from_project_file_if_present(launch_options);
    if let Some(project_path) = &launch_options.project_path {
        if let Err(error) = std::env::set_current_dir(project_path) {
            tracing::warn!(
                "Failed to set runtime current dir to '{}': {}",
                project_path.display(),
                error
            );
        }
    }

    let run_result = if launch_options != RuntimeLaunchOptions::default() {
        run_minimal_window_with_options(launch_options)
    } else {
        run_minimal_window()
    };

    if let Err(e) = run_result {
        tracing::error!("Fatal error: {e:?}");
    }
    Ok(())
}

fn parse_launch_options(args: Vec<String>) -> RuntimeLaunchOptions {
    let mut launch_options = RuntimeLaunchOptions::default();
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--project" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.project_path = Some(PathBuf::from(value));
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--project' without value");
            }
            "--scene" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.scene_name = Some(value.clone());
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--scene' without value");
            }
            "--map" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.map_name = Some(value.clone());
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--map' without value");
            }
            "--splash-duration-ms" => {
                if let Some(value) = option_value(&args, index + 1) {
                    match value.parse::<u64>() {
                        Ok(duration_ms) => {
                            launch_options.splash.duration_ms = duration_ms;
                            index += 2;
                            continue;
                        }
                        Err(error) => {
                            tracing::warn!(
                                "Ignoring '--splash-duration-ms' invalid value '{}': {}",
                                value,
                                error
                            );
                        }
                    }
                } else {
                    tracing::warn!("Ignoring '--splash-duration-ms' without value");
                }
            }
            "--splash-hide-branding" => {
                launch_options.splash.show_branding = false;
                index += 1;
                continue;
            }
            "--pack" => {
                if let Some(value) = option_value(&args, index + 1) {
                    launch_options.pack_path = Some(PathBuf::from(value));
                    index += 2;
                    continue;
                }
                tracing::warn!("Ignoring '--pack' without value");
            }
            unknown => {
                tracing::warn!("Ignoring unknown runtime argument '{}'", unknown);
            }
        }

        index += 1;
    }

    launch_options
}

fn apply_runtime_config_if_present(
    mut launch_options: RuntimeLaunchOptions,
) -> RuntimeLaunchOptions {
    let Some((config, config_dir)) = load_runtime_config() else {
        return launch_options;
    };
    apply_runtime_config(&mut launch_options, config, &config_dir);
    launch_options
}

fn apply_runtime_config(
    launch_options: &mut RuntimeLaunchOptions,
    config: RuntimeConfigFile,
    config_dir: &std::path::Path,
) {
    if launch_options.project_path.is_none() {
        launch_options.project_path = Some(config_dir.to_path_buf());
    }
    if launch_options.scene_name.is_none() {
        launch_options.scene_name = config.startup.and_then(|startup| startup.scene);
    }
    if launch_options.pack_path.is_none() {
        if let Some(pack) = config.pack {
            if pack.enabled {
                launch_options.pack_path = Some(config_dir.join(pack.path));
            }
        }
    }
    if let Some(splash) = config.splash {
        if let Some(duration_ms) = splash.duration_ms {
            launch_options.splash.duration_ms = duration_ms;
        }
    }
    if let Some(audio) = config.audio {
        if let Some(master_percent) = audio.master_percent {
            launch_options.audio_mix.master_percent = master_percent.min(100);
        }
        if let Some(music_percent) = audio.music_percent {
            launch_options.audio_mix.music_percent = music_percent.min(100);
        }
        if let Some(movement_percent) = audio.movement_percent {
            launch_options.audio_mix.movement_percent = movement_percent.min(100);
        }
        if let Some(collision_percent) = audio.collision_percent {
            launch_options.audio_mix.collision_percent = collision_percent.min(100);
        }
    }
    if let Some(display) = config.display {
        if let Some(show_entity_health_bars) = display.show_entity_health_bars {
            launch_options.display.show_entity_health_bars = show_entity_health_bars;
        }
        if let Some(show_ground_shadows) = display.show_ground_shadows {
            launch_options.display.show_ground_shadows = show_ground_shadows;
        }
        if let Some(resolution_width) = display.resolution_width {
            launch_options.display.resolution_width = resolution_width;
        }
        if let Some(resolution_height) = display.resolution_height {
            launch_options.display.resolution_height = resolution_height;
        }
        if let Some(zoom_percent) = display.zoom_percent {
            launch_options.display.zoom_percent = zoom_percent;
        }
        if let Some(vsync) = display.vsync {
            launch_options.display.vsync = vsync;
        }
        if let Some(target_fps) = display.target_fps {
            launch_options.display.target_fps = target_fps;
        }
        if let Some(timing_mode) = display.timing_mode {
            launch_options.display.timing_mode = timing_mode;
        }
    }
    if let Some(menu) = config.menu {
        launch_options.menu = menu;
    }
}

fn load_runtime_config() -> Option<(RuntimeConfigFile, PathBuf)> {
    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("runtime_config.json"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("runtime_config.json"));
    }
    load_runtime_config_from_candidates(&candidates)
}

fn load_runtime_config_from_candidates(
    candidates: &[PathBuf],
) -> Option<(RuntimeConfigFile, PathBuf)> {
    for path in candidates {
        if !path.exists() {
            continue;
        }
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_json::from_str::<RuntimeConfigFile>(&content) {
                Ok(config) => {
                    let dir = path.parent().map(std::path::Path::to_path_buf)?;
                    return Some((config, dir));
                }
                Err(error) => {
                    tracing::warn!(
                        "Failed to parse runtime config '{}': {}",
                        path.display(),
                        error
                    );
                }
            },
            Err(error) => {
                tracing::warn!(
                    "Failed to read runtime config '{}': {}",
                    path.display(),
                    error
                );
            }
        }
    }
    None
}

fn auto_detect_project_launch_options(
    mut launch_options: RuntimeLaunchOptions,
) -> RuntimeLaunchOptions {
    let Ok(current_dir) = std::env::current_dir() else {
        return launch_options;
    };
    if !current_dir.join("project.toml").exists() {
        return launch_options;
    }

    launch_options.project_path = Some(current_dir.clone());

    if launch_options.scene_name.is_none() {
        launch_options.scene_name = detect_first_scene_name(&current_dir);
    }

    launch_options
}

fn apply_project_runtime_settings_from_project_file_if_present(
    mut launch_options: RuntimeLaunchOptions,
) -> RuntimeLaunchOptions {
    let should_apply_audio = launch_options.audio_mix == RuntimeAudioMixOptions::default();
    let should_apply_display = launch_options.display == RuntimeDisplayOptions::default();
    let should_apply_menu = launch_options.menu == MenuSettings::default();
    if !should_apply_audio && !should_apply_display && !should_apply_menu {
        return launch_options;
    }

    let Some(project_path) = launch_options.project_path.as_ref() else {
        return launch_options;
    };
    let project_file = project_path.join("project.toml");
    let Ok(content) = std::fs::read_to_string(&project_file) else {
        return launch_options;
    };
    let Ok(metadata) = toml::from_str::<ProjectRuntimeMetadata>(&content) else {
        tracing::warn!(
            "Failed to parse project runtime settings from '{}'",
            project_file.display()
        );
        return launch_options;
    };

    if should_apply_audio {
        launch_options.audio_mix.music_percent = metadata.runtime.audio.music_percent.min(100);
        launch_options.audio_mix.master_percent = metadata.runtime.audio.master_percent.min(100);
        launch_options.audio_mix.movement_percent =
            metadata.runtime.audio.movement_percent.min(100);
        launch_options.audio_mix.collision_percent =
            metadata.runtime.audio.collision_percent.min(100);
    }
    if should_apply_display {
        launch_options.display.show_entity_health_bars =
            metadata.runtime.display.show_entity_health_bars;
        launch_options.display.show_ground_shadows = metadata.runtime.display.show_ground_shadows;
        launch_options.display.resolution_width = metadata.runtime.display.resolution_width;
        launch_options.display.resolution_height = metadata.runtime.display.resolution_height;
        launch_options.display.zoom_percent = metadata.runtime.display.zoom_percent;
        launch_options.display.vsync = metadata.runtime.display.vsync;
        launch_options.display.target_fps = metadata.runtime.display.target_fps;
        launch_options.display.timing_mode = metadata.runtime.display.timing_mode;
    }
    if should_apply_menu {
        launch_options.menu = metadata.runtime.menu;
    }
    launch_options
}

fn detect_first_scene_name(project_path: &std::path::Path) -> Option<String> {
    let scenes_dir = project_path.join("scenes");
    let mut scene_file_stems = std::fs::read_dir(scenes_dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        })
        .filter_map(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    scene_file_stems.sort();
    scene_file_stems.into_iter().next()
}

fn option_value(args: &[String], value_index: usize) -> Option<&String> {
    let value = args.get(value_index)?;
    if value.starts_with("--") {
        return None;
    }
    Some(value)
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
