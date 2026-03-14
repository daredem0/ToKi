use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

use toki_runtime::{run_minimal_window, run_minimal_window_with_options, RuntimeLaunchOptions};

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

    let mut launch_options = parse_launch_options(std::env::args().skip(1).collect());
    if launch_options.project_path.is_none() {
        launch_options = auto_detect_project_launch_options(launch_options);
    }
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
            unknown => {
                tracing::warn!("Ignoring unknown runtime argument '{}'", unknown);
            }
        }

        index += 1;
    }

    launch_options
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
        .filter_map(|path| path.file_stem().map(|stem| stem.to_string_lossy().to_string()))
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
mod tests {
    use super::{
        auto_detect_project_launch_options, detect_first_scene_name, option_value,
        parse_launch_options,
    };
    use std::path::PathBuf;
    use toki_runtime::RuntimeLaunchOptions;

    #[test]
    fn parse_launch_options_reads_project_scene_and_map() {
        let options = parse_launch_options(vec![
            "--project".to_string(),
            "/tmp/project".to_string(),
            "--scene".to_string(),
            "Main Scene".to_string(),
            "--map".to_string(),
            "map_01".to_string(),
        ]);

        assert_eq!(options.project_path, Some(PathBuf::from("/tmp/project")));
        assert_eq!(options.scene_name.as_deref(), Some("Main Scene"));
        assert_eq!(options.map_name.as_deref(), Some("map_01"));
        assert_eq!(options.splash.duration_ms, 3000);
        assert!(options.splash.show_branding);
    }

    #[test]
    fn parse_launch_options_ignores_missing_values() {
        let options = parse_launch_options(vec![
            "--project".to_string(),
            "--scene".to_string(),
            "--map".to_string(),
        ]);

        assert!(options.project_path.is_none());
        assert!(options.scene_name.is_none());
        assert!(options.map_name.is_none());
    }

    #[test]
    fn parse_launch_options_ignores_unknown_flags() {
        let options = parse_launch_options(vec![
            "--unknown".to_string(),
            "value".to_string(),
            "--project".to_string(),
            "/tmp/project".to_string(),
        ]);

        assert_eq!(options.project_path, Some(PathBuf::from("/tmp/project")));
        assert!(options.scene_name.is_none());
        assert!(options.map_name.is_none());
        assert!(options.splash.show_branding);
    }

    #[test]
    fn parse_launch_options_reads_splash_flags() {
        let options = parse_launch_options(vec![
            "--splash-duration-ms".to_string(),
            "2750".to_string(),
            "--splash-hide-branding".to_string(),
        ]);

        assert_eq!(options.splash.duration_ms, 2750);
        assert!(!options.splash.show_branding);
    }

    #[test]
    fn parse_launch_options_ignores_invalid_splash_duration() {
        let options = parse_launch_options(vec![
            "--splash-duration-ms".to_string(),
            "not-a-number".to_string(),
        ]);

        assert_eq!(options.splash.duration_ms, 3000);
    }

    #[test]
    fn option_value_rejects_next_flag() {
        let args = vec!["--project".to_string(), "--scene".to_string()];
        assert_eq!(option_value(&args, 1), None);
    }

    #[test]
    fn detect_first_scene_name_reads_sorted_scene_stem() {
        let dir = tempfile::tempdir().expect("temp dir");
        let scenes = dir.path().join("scenes");
        std::fs::create_dir_all(&scenes).expect("scenes dir");
        std::fs::write(scenes.join("z_scene.json"), "{}").expect("z scene");
        std::fs::write(scenes.join("a_scene.json"), "{}").expect("a scene");
        std::fs::write(scenes.join("notes.txt"), "ignored").expect("notes");

        let detected = detect_first_scene_name(dir.path());
        assert_eq!(detected.as_deref(), Some("a_scene"));
    }

    #[test]
    fn auto_detect_project_launch_options_sets_project_and_scene_when_bundle_layout_exists() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("project.toml"), "[project]\nname='Demo'\n")
            .expect("project");
        let scenes = dir.path().join("scenes");
        std::fs::create_dir_all(&scenes).expect("scenes dir");
        std::fs::write(scenes.join("main.json"), "{}").expect("scene");

        let original_dir = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("set cwd");
        let detected = auto_detect_project_launch_options(RuntimeLaunchOptions::default());
        std::env::set_current_dir(original_dir).expect("restore cwd");

        assert_eq!(detected.project_path, Some(dir.path().to_path_buf()));
        assert_eq!(detected.scene_name.as_deref(), Some("main"));
    }
}
