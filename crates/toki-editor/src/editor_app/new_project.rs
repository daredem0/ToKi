use super::*;

impl EditorApp {
    pub(super) fn handle_new_project_requested(
        &mut self,
        template: ProjectTemplateKind,
        parent_path: std::path::PathBuf,
        project_name: String,
    ) {
        tracing::info!(
            "Creating project '{}' from template '{}' in {:?}",
            project_name,
            template.label(),
            parent_path
        );

        let create_result = match template {
            ProjectTemplateKind::Empty => self
                .project_manager
                .create_new_project(project_name.clone(), parent_path.clone()),
            ProjectTemplateKind::TopDownStarter => {
                self.project_manager.create_new_project_with_template(
                    project_name.clone(),
                    parent_path.clone(),
                    template,
                )
            }
        };

        match create_result {
            Ok(game_state) => {
                let project_path = parent_path.join(&project_name);
                self.activate_loaded_project(game_state, project_path, "new project");
                tracing::info!(
                    "Created '{}' project '{}' successfully",
                    template.label(),
                    project_name
                );
            }
            Err(error) => {
                tracing::error!("Failed to create new project: {}", error);
            }
        }
    }

    pub(super) fn suggested_new_project_parent_path(
        current_project_path: &std::path::Path,
    ) -> std::path::PathBuf {
        current_project_path
            .parent()
            .unwrap_or(current_project_path)
            .to_path_buf()
    }

    #[cfg(test)]
    pub(super) fn split_new_project_destination(
        destination_path: &std::path::Path,
    ) -> Option<(std::path::PathBuf, String)> {
        if destination_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("project.toml"))
        {
            let project_dir = destination_path.parent()?;
            let project_name = project_dir.file_name()?.to_str()?.trim().to_string();
            if project_name.is_empty() {
                return None;
            }
            let parent_path = project_dir.parent()?.to_path_buf();
            return Some((parent_path, project_name));
        }

        let project_name = destination_path.file_name()?.to_str()?.trim().to_string();
        if project_name.is_empty() {
            return None;
        }
        let parent_path = destination_path.parent()?.to_path_buf();
        Some((parent_path, project_name))
    }

    pub(super) fn next_available_project_name(
        parent_path: &std::path::Path,
        base_name: &str,
    ) -> String {
        let mut project_name = base_name.to_string();
        let mut counter = 1;

        while parent_path.join(&project_name).exists() {
            project_name = format!("{}{}", base_name, counter);
            counter += 1;
        }

        project_name
    }
}
