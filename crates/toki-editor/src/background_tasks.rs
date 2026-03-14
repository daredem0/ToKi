use crate::project::Project;
use crate::project::ProjectAssets;
use crate::validation::AssetValidator;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundTaskKind {
    ExportBundle,
    ValidateAssets,
}

impl BackgroundTaskKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ExportBundle => "Export Game",
            Self::ValidateAssets => "Validate Assets",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackgroundTaskUpdate {
    Started {
        kind: BackgroundTaskKind,
        message: String,
    },
    Progress {
        kind: BackgroundTaskKind,
        message: String,
    },
    Completed {
        kind: BackgroundTaskKind,
        message: String,
    },
    Failed {
        kind: BackgroundTaskKind,
        message: String,
    },
    Cancelled {
        kind: BackgroundTaskKind,
    },
}

#[derive(Debug, Clone)]
pub struct ExportBundleJob {
    pub project: Project,
    pub workspace_root: PathBuf,
    pub export_root: PathBuf,
    pub startup_scene: Option<String>,
    pub splash_duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ValidateAssetsJob {
    pub project_path: PathBuf,
}

#[derive(Debug)]
enum TaskResult {
    Completed(String),
    Failed(String),
    Cancelled,
}

#[derive(Debug)]
enum WorkerMessage {
    Progress(String),
    Result(TaskResult),
}

#[derive(Debug)]
struct RunningTask {
    kind: BackgroundTaskKind,
    receiver: mpsc::Receiver<WorkerMessage>,
    cancel_flag: Arc<AtomicBool>,
    join_handle: JoinHandle<()>,
}

#[derive(Debug, Default)]
pub struct BackgroundTaskManager {
    running: Option<RunningTask>,
    pending_updates: VecDeque<BackgroundTaskUpdate>,
}

impl BackgroundTaskManager {
    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    pub fn start_export_bundle(&mut self, job: ExportBundleJob) -> Result<(), String> {
        self.start_task(
            BackgroundTaskKind::ExportBundle,
            format!("{} started", BackgroundTaskKind::ExportBundle.label()),
            move |cancel_flag, sender| run_export_bundle_job(job, cancel_flag, sender),
        )
    }

    pub fn start_validate_assets(&mut self, job: ValidateAssetsJob) -> Result<(), String> {
        self.start_task(
            BackgroundTaskKind::ValidateAssets,
            format!("{} started", BackgroundTaskKind::ValidateAssets.label()),
            move |cancel_flag, sender| run_validate_assets_job(job, cancel_flag, sender),
        )
    }

    pub fn request_cancel(&mut self) -> bool {
        if let Some(task) = &self.running {
            task.cancel_flag.store(true, Ordering::SeqCst);
            self.pending_updates
                .push_back(BackgroundTaskUpdate::Progress {
                    kind: task.kind,
                    message: "Cancellation requested".to_string(),
                });
            return true;
        }
        false
    }

    pub fn poll_updates(&mut self) -> Vec<BackgroundTaskUpdate> {
        let mut updates = self.pending_updates.drain(..).collect::<Vec<_>>();

        let mut should_finalize = false;
        if let Some(task) = &mut self.running {
            while let Ok(message) = task.receiver.try_recv() {
                match message {
                    WorkerMessage::Progress(message) => {
                        updates.push(BackgroundTaskUpdate::Progress {
                            kind: task.kind,
                            message,
                        });
                    }
                    WorkerMessage::Result(result) => {
                        should_finalize = true;
                        match result {
                            TaskResult::Completed(message) => {
                                updates.push(BackgroundTaskUpdate::Completed {
                                    kind: task.kind,
                                    message,
                                });
                            }
                            TaskResult::Failed(message) => {
                                updates.push(BackgroundTaskUpdate::Failed {
                                    kind: task.kind,
                                    message,
                                });
                            }
                            TaskResult::Cancelled => {
                                updates.push(BackgroundTaskUpdate::Cancelled { kind: task.kind });
                            }
                        }
                    }
                }
            }
        }

        if should_finalize {
            if let Some(task) = self.running.take() {
                let _ = task.join_handle.join();
            }
        }

        updates
    }

    fn start_task<F>(
        &mut self,
        kind: BackgroundTaskKind,
        started_message: String,
        worker: F,
    ) -> Result<(), String>
    where
        F: FnOnce(Arc<AtomicBool>, mpsc::Sender<WorkerMessage>) -> TaskResult + Send + 'static,
    {
        if self.running.is_some() {
            return Err("Another background task is already running".to_string());
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel_flag);
        let (sender, receiver) = mpsc::channel::<WorkerMessage>();
        let join_handle = std::thread::spawn(move || {
            let result = worker(worker_cancel, sender.clone());
            let _ = sender.send(WorkerMessage::Result(result));
        });

        self.running = Some(RunningTask {
            kind,
            receiver,
            cancel_flag,
            join_handle,
        });
        self.pending_updates
            .push_back(BackgroundTaskUpdate::Started {
                kind,
                message: started_message,
            });
        Ok(())
    }

    #[cfg(test)]
    fn start_test_task<F>(&mut self, kind: BackgroundTaskKind, worker: F) -> Result<(), String>
    where
        F: FnOnce(Arc<AtomicBool>, mpsc::Sender<WorkerMessage>) -> TaskResult + Send + 'static,
    {
        self.start_task(kind, "test started".to_string(), worker)
    }
}

fn run_export_bundle_job(
    job: ExportBundleJob,
    cancel_flag: Arc<AtomicBool>,
    sender: mpsc::Sender<WorkerMessage>,
) -> TaskResult {
    let _ = sender.send(WorkerMessage::Progress(
        "Building runtime for game export".to_string(),
    ));

    match build_runtime_binary_for_export(&job.workspace_root, &cancel_flag) {
        Ok(runtime_binary_path) => {
            if cancel_flag.load(Ordering::SeqCst) {
                return TaskResult::Cancelled;
            }

            let _ = sender.send(WorkerMessage::Progress(
                "Writing runtime pack and config".to_string(),
            ));
            match crate::project::export::export_hybrid_bundle(
                &job.project,
                &runtime_binary_path,
                &job.export_root,
                job.startup_scene.as_deref(),
                job.splash_duration_ms,
            ) {
                Ok(bundle_dir) => {
                    TaskResult::Completed(format!("Exported game to '{}'", bundle_dir.display()))
                }
                Err(error) => TaskResult::Failed(format!("Game export failed: {}", error)),
            }
        }
        Err(BuildRuntimeError::Cancelled) => TaskResult::Cancelled,
        Err(BuildRuntimeError::Failed(error)) => TaskResult::Failed(error),
    }
}

fn run_validate_assets_job(
    job: ValidateAssetsJob,
    cancel_flag: Arc<AtomicBool>,
    sender: mpsc::Sender<WorkerMessage>,
) -> TaskResult {
    let _ = sender.send(WorkerMessage::Progress(
        "Scanning project assets".to_string(),
    ));
    let mut project_assets = ProjectAssets::new(job.project_path.clone());
    if let Err(error) = project_assets.scan_assets() {
        return TaskResult::Failed(format!("Asset scan failed: {}", error));
    }
    if cancel_flag.load(Ordering::SeqCst) {
        return TaskResult::Cancelled;
    }

    let _ = sender.send(WorkerMessage::Progress(
        "Validating project assets".to_string(),
    ));
    let validator = match AssetValidator::new() {
        Ok(validator) => validator,
        Err(error) => return TaskResult::Failed(format!("Validator setup failed: {}", error)),
    };
    match validator.validate_project_assets(&project_assets) {
        Ok(()) => TaskResult::Completed("Asset validation finished successfully".to_string()),
        Err(error) => TaskResult::Failed(format!("Asset validation failed: {}", error)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BuildRuntimeError {
    Cancelled,
    Failed(String),
}

fn build_runtime_binary_for_export(
    workspace_root: &std::path::Path,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<PathBuf, BuildRuntimeError> {
    let mut child = Command::new("cargo")
        .current_dir(workspace_root)
        .arg("build")
        .arg("-p")
        .arg("toki-runtime")
        .spawn()
        .map_err(|error| {
            BuildRuntimeError::Failed(format!("Failed to launch cargo build: {}", error))
        })?;

    loop {
        if cancel_flag.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(BuildRuntimeError::Cancelled);
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(BuildRuntimeError::Failed(format!(
                        "cargo build -p toki-runtime failed with status {}",
                        status
                    )));
                }

                let runtime_binary_name = if cfg!(target_os = "windows") {
                    "toki-runtime.exe"
                } else {
                    "toki-runtime"
                };
                let runtime_binary_path = workspace_root
                    .join("target")
                    .join("debug")
                    .join(runtime_binary_name);
                if !runtime_binary_path.exists() {
                    return Err(BuildRuntimeError::Failed(format!(
                        "Runtime binary not found after build: {}",
                        runtime_binary_path.display()
                    )));
                }
                return Ok(runtime_binary_path);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                return Err(BuildRuntimeError::Failed(format!(
                    "Failed while waiting for cargo build: {}",
                    error
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BackgroundTaskKind, BackgroundTaskManager, BackgroundTaskUpdate, TaskResult, WorkerMessage,
    };
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    #[test]
    fn task_manager_rejects_concurrent_task_start() {
        let mut manager = BackgroundTaskManager::default();
        manager
            .start_test_task(BackgroundTaskKind::ExportBundle, |_, _| {
                std::thread::sleep(Duration::from_millis(50));
                TaskResult::Completed("done".to_string())
            })
            .expect("first task should start");

        let second = manager.start_test_task(BackgroundTaskKind::ValidateAssets, |_, _| {
            TaskResult::Completed("done".to_string())
        });
        assert!(second.is_err(), "second task should be rejected");
    }

    #[test]
    fn task_manager_emits_progress_and_completion_updates() {
        let mut manager = BackgroundTaskManager::default();
        manager
            .start_test_task(BackgroundTaskKind::ValidateAssets, |_, sender| {
                let _ = sender.send(WorkerMessage::Progress("step one".to_string()));
                let _ = sender.send(WorkerMessage::Progress("step two".to_string()));
                TaskResult::Completed("all done".to_string())
            })
            .expect("task should start");

        let mut updates = Vec::new();
        for _ in 0..20 {
            updates.extend(manager.poll_updates());
            if updates
                .iter()
                .any(|update| matches!(update, BackgroundTaskUpdate::Completed { .. }))
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(
            updates
                .iter()
                .any(|update| matches!(update, BackgroundTaskUpdate::Started { .. })),
            "expected started update"
        );
        assert!(
            updates.iter().any(|update| {
                matches!(
                    update,
                    BackgroundTaskUpdate::Progress { message, .. } if message == "step one"
                )
            }),
            "expected first progress update"
        );
        assert!(
            updates.iter().any(|update| {
                matches!(
                    update,
                    BackgroundTaskUpdate::Progress { message, .. } if message == "step two"
                )
            }),
            "expected second progress update"
        );
        assert!(
            updates.iter().any(|update| {
                matches!(
                    update,
                    BackgroundTaskUpdate::Completed { message, .. } if message == "all done"
                )
            }),
            "expected completed update"
        );
        assert!(!manager.is_running(), "task should be finalized");
    }

    #[test]
    fn task_manager_cancellation_requests_and_reports_cancelled() {
        let mut manager = BackgroundTaskManager::default();
        manager
            .start_test_task(BackgroundTaskKind::ExportBundle, |cancel_flag, _sender| {
                while !cancel_flag.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                TaskResult::Cancelled
            })
            .expect("task should start");
        assert!(manager.request_cancel(), "cancel should be accepted");

        let mut updates = Vec::new();
        for _ in 0..40 {
            updates.extend(manager.poll_updates());
            if updates
                .iter()
                .any(|update| matches!(update, BackgroundTaskUpdate::Cancelled { .. }))
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(
            updates
                .iter()
                .any(|update| matches!(update, BackgroundTaskUpdate::Cancelled { .. })),
            "expected cancelled update"
        );
        assert!(!manager.is_running(), "task should be finalized");
    }
}
