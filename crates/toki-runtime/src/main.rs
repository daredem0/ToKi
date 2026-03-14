use anyhow::Result;
use std::path::PathBuf;

use toki_runtime::{run_minimal_window, run_minimal_window_with_options, RuntimeLaunchOptions};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true)
        .with_file(false)
        .with_line_number(true)
        .init();

    let launch_options = parse_launch_options(std::env::args().skip(1).collect());
    if let Some(project_path) = &launch_options.project_path {
        if let Err(error) = std::env::set_current_dir(project_path) {
            tracing::warn!(
                "Failed to set runtime current dir to '{}': {}",
                project_path.display(),
                error
            );
        }
    }

    let run_result = if launch_options.project_path.is_some()
        || launch_options.scene_name.is_some()
        || launch_options.map_name.is_some()
    {
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
            unknown => {
                tracing::warn!("Ignoring unknown runtime argument '{}'", unknown);
            }
        }

        index += 1;
    }

    launch_options
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
    use super::{option_value, parse_launch_options};
    use std::path::PathBuf;

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
    }

    #[test]
    fn option_value_rejects_next_flag() {
        let args = vec!["--project".to_string(), "--scene".to_string()];
        assert_eq!(option_value(&args, 1), None);
    }
}
