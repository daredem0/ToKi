use super::*;

impl EditorApp {
    pub(super) fn handle_new_project_requested(&mut self, template: ProjectTemplateKind) {
        self.ui.new_project_requested = false;
        self.ui.new_top_down_project_requested = false;

        let folder_path = if let Some(config_path) = &self.config.project_path {
            tracing::info!(
                "Using project path from config as parent: {:?}",
                config_path
            );
            Some(config_path.clone())
        } else {
            tracing::info!("No project path in config, asking user to select folder");
            rfd::FileDialog::new()
                .set_title("Select folder for new project")
                .pick_folder()
        };

        let Some(parent_path) = folder_path else {
            return;
        };

        let project_name = Self::next_available_project_name(&parent_path, "NewProject");

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
