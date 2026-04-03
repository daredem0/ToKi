use super::*;

impl EditorApp {
    pub(super) fn poll_background_task_updates(&mut self) {
        for update in self.command_coordinator.background_tasks.poll_updates() {
            self.apply_background_task_update(update);
        }
    }

    pub(super) fn apply_background_task_update(&mut self, update: BackgroundTaskUpdate) {
        match update {
            BackgroundTaskUpdate::Started { kind, message } => {
                self.tabs.ui.project.background_task_running = true;
                self.tabs.ui.project.background_task_status =
                    Some(format!("{}: {}", kind.label(), message));
                tracing::info!(
                    "{}",
                    self.tabs
                        .ui
                        .project
                        .background_task_status
                        .as_deref()
                        .unwrap_or("")
                );
            }
            BackgroundTaskUpdate::Progress { kind, message } => {
                self.tabs.ui.project.background_task_running = true;
                self.tabs.ui.project.background_task_status =
                    Some(format!("{}: {}", kind.label(), message));
            }
            BackgroundTaskUpdate::Completed { kind, message } => {
                self.tabs.ui.project.background_task_running = false;
                self.tabs.ui.project.background_task_status = None;
                tracing::info!("{} completed: {}", kind.label(), message);
            }
            BackgroundTaskUpdate::Failed { kind, message } => {
                self.tabs.ui.project.background_task_running = false;
                self.tabs.ui.project.background_task_status = None;
                tracing::error!("{} failed: {}", kind.label(), message);
            }
            BackgroundTaskUpdate::Cancelled { kind } => {
                self.tabs.ui.project.background_task_running = false;
                self.tabs.ui.project.background_task_status = None;
                tracing::info!("{} cancelled", kind.label());
            }
        }
    }
}
