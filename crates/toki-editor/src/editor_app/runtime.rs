use super::*;
use std::process::Command;

impl EditorApp {
    pub(super) fn handle_play_scene_request(&mut self) {
        if !self.core.ui.project.play_scene_requested {
            return;
        }
        self.core.ui.project.play_scene_requested = false;

        let Some(project_path) = self.core.config.current_project_path().cloned() else {
            tracing::warn!("Cannot play scene: no project is currently open");
            return;
        };
        let Some(active_scene_name) = self.core.ui.active_scene.clone() else {
            tracing::warn!("Cannot play scene: no active scene is selected");
            return;
        };

        if let Err(error) = self
            .core
            .project_manager
            .save_current_project(&self.core.ui.scenes)
        {
            tracing::error!(
                "Cannot play scene '{}': failed to save current project state: {}",
                active_scene_name,
                error
            );
            return;
        }

        let map_name = self
            .find_scene_by_name(&active_scene_name)
            .and_then(|scene| {
                self.session
                    .loaded_scene_maps
                    .get(&active_scene_name)
                    .cloned()
                    .filter(|map| scene.maps.iter().any(|scene_map| scene_map == map))
                    .or_else(|| scene.maps.first().cloned())
            });

        let splash_duration_ms = self
            .core
            .project_manager
            .current_project
            .as_ref()
            .map(|project| project.metadata.runtime.splash.duration_ms);

        if let Err(error) = Self::launch_runtime_process(
            &project_path,
            &active_scene_name,
            map_name.as_deref(),
            splash_duration_ms,
        ) {
            tracing::error!(
                "Failed to launch runtime for scene '{}' from '{}': {}",
                active_scene_name,
                project_path.display(),
                error
            );
            return;
        }

        tracing::info!(
            "Launched runtime for scene '{}' (map: {})",
            active_scene_name,
            map_name.as_deref().unwrap_or("<auto>")
        );
    }

    pub(super) fn handle_export_project_request(&mut self) {
        if !self.core.ui.project.export_project_requested {
            return;
        }
        self.core.ui.project.export_project_requested = false;

        if self.background_tasks.is_running() {
            tracing::warn!("Cannot export game: another background task is running");
            return;
        }

        let Some(project_path) = self.core.config.current_project_path().cloned() else {
            tracing::warn!("Cannot export game: no project is currently open");
            return;
        };

        if let Err(error) = self
            .core
            .project_manager
            .save_current_project(&self.core.ui.scenes)
        {
            tracing::error!(
                "Cannot export game: failed to save current project state: {}",
                error
            );
            return;
        }

        let default_export_root = project_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(Self::workspace_root);
        let export_root = match rfd::FileDialog::new()
            .set_title("Select export destination directory")
            .set_directory(default_export_root)
            .pick_folder()
        {
            Some(path) => path,
            None => {
                tracing::info!("Game export cancelled by user");
                return;
            }
        };

        let startup_scene = self.core.ui.active_scene.clone();
        let splash_duration_ms = self
            .core
            .project_manager
            .current_project
            .as_ref()
            .map(|project| project.metadata.runtime.splash.duration_ms)
            .unwrap_or(3000);

        let Some(project) = self.core.project_manager.current_project.as_ref().cloned() else {
            tracing::warn!("Cannot export game: no project is currently open");
            return;
        };

        let Some(runtime_binary_path) = self.resolve_runtime_binary_for_export() else {
            return;
        };

        let job = ExportBundleJob {
            project,
            runtime_binary_path,
            export_root,
            startup_scene,
            splash_duration_ms,
        };

        if let Err(error) = self.background_tasks.start_export_bundle(job) {
            tracing::error!("Failed to start game export job: {}", error);
        } else {
            self.poll_background_task_updates();
        }
    }

    pub(super) fn launch_runtime_process(
        project_path: &std::path::Path,
        scene_name: &str,
        map_name: Option<&str>,
        splash_duration_ms: Option<u64>,
    ) -> Result<()> {
        let runtime_args =
            Self::build_runtime_launch_args(project_path, scene_name, map_name, splash_duration_ms);

        let mut cargo_command = Command::new("cargo");
        cargo_command
            .current_dir(Self::workspace_root())
            .arg("run")
            .arg("-p")
            .arg("toki-runtime")
            .arg("--")
            .args(&runtime_args);

        match cargo_command.spawn() {
            Ok(_) => return Ok(()),
            Err(cargo_error) => {
                tracing::warn!(
                    "Failed to launch runtime via cargo ({}), trying direct binary fallback",
                    cargo_error
                );
            }
        }

        let runtime_bin_name = Self::runtime_binary_name();
        let runtime_bin_path = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|parent| parent.join(runtime_bin_name)))
            .ok_or_else(|| anyhow::anyhow!("Could not resolve runtime binary path"))?;

        Command::new(runtime_bin_path).args(&runtime_args).spawn()?;
        Ok(())
    }

    pub(super) fn build_runtime_launch_args(
        project_path: &std::path::Path,
        scene_name: &str,
        map_name: Option<&str>,
        splash_duration_ms: Option<u64>,
    ) -> Vec<String> {
        let mut runtime_args = vec![
            "--project".to_string(),
            project_path.display().to_string(),
            "--scene".to_string(),
            scene_name.to_string(),
        ];
        if let Some(map_name) = map_name {
            runtime_args.push("--map".to_string());
            runtime_args.push(map_name.to_string());
        }
        if let Some(duration_ms) = splash_duration_ms {
            runtime_args.push("--splash-duration-ms".to_string());
            runtime_args.push(duration_ms.to_string());
        }
        runtime_args
    }

    pub(super) fn runtime_binary_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "toki-runtime.exe"
        } else {
            "toki-runtime"
        }
    }

    pub(super) fn runtime_search_candidates_from_sources(
        configured_runtime_path: Option<&std::path::Path>,
        current_exe: Option<&std::path::Path>,
        workspace_root: &std::path::Path,
    ) -> Vec<std::path::PathBuf> {
        let mut candidates = Vec::new();

        if let Some(path) = configured_runtime_path {
            candidates.push(path.to_path_buf());
        }

        if let Some(current_exe) = current_exe {
            if let Some(parent) = current_exe.parent() {
                candidates.push(parent.join(Self::runtime_binary_name()));
            }
        }

        candidates.push(
            workspace_root
                .join("target")
                .join("release")
                .join(Self::runtime_binary_name()),
        );

        candidates
    }

    pub(super) fn discover_runtime_binary_from_sources(
        configured_runtime_path: Option<&std::path::Path>,
        current_exe: Option<&std::path::Path>,
        workspace_root: &std::path::Path,
    ) -> Option<std::path::PathBuf> {
        Self::runtime_search_candidates_from_sources(
            configured_runtime_path,
            current_exe,
            workspace_root,
        )
        .into_iter()
        .find(|path| path.is_file())
    }

    pub(super) fn runtime_discovery_error_message(candidates: &[std::path::PathBuf]) -> String {
        let mut lines = vec![
            "Could not find a ToKi runtime binary for export.".to_string(),
            "".to_string(),
            "Searched:".to_string(),
        ];
        lines.extend(
            candidates
                .iter()
                .map(|path| format!("- {}", path.display())),
        );
        lines.push("".to_string());
        lines.push("Select a toki-runtime binary manually.".to_string());
        lines.join("\n")
    }

    fn browse_for_runtime_binary() -> Option<std::path::PathBuf> {
        let mut dialog = rfd::FileDialog::new().set_title("Select ToKi Runtime Binary");
        if cfg!(target_os = "windows") {
            dialog = dialog.add_filter("ToKi Runtime", &["exe"]);
        }
        dialog.pick_file()
    }

    fn resolve_runtime_binary_for_export(&mut self) -> Option<std::path::PathBuf> {
        let current_exe = std::env::current_exe().ok();
        let workspace_root = Self::workspace_root();
        let configured = self.core.config.current_runtime_binary_path().cloned();
        let candidates = Self::runtime_search_candidates_from_sources(
            configured.as_deref(),
            current_exe.as_deref(),
            &workspace_root,
        );

        if let Some(path) = Self::discover_runtime_binary_from_sources(
            configured.as_deref(),
            current_exe.as_deref(),
            &workspace_root,
        ) {
            return Some(path);
        }

        let message = Self::runtime_discovery_error_message(&candidates);
        let _ = rfd::MessageDialog::new()
            .set_title("Runtime Binary Not Found")
            .set_description(&message)
            .set_buttons(rfd::MessageButtons::Ok)
            .set_level(rfd::MessageLevel::Error)
            .show();

        let chosen = Self::browse_for_runtime_binary()?;
        if !chosen.is_file() {
            tracing::warn!(
                "Selected runtime binary path is not a file: {}",
                chosen.display()
            );
            return None;
        }

        self.core
            .config
            .set_runtime_binary_path(Some(chosen.clone()));
        if let Err(error) = self.core.config.save() {
            tracing::warn!(
                "Failed to persist selected runtime binary path '{}': {}",
                chosen.display(),
                error
            );
        }

        Some(chosen)
    }

    pub(super) fn workspace_root() -> std::path::PathBuf {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or(manifest_dir)
    }
}
