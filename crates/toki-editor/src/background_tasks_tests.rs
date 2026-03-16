
use super::{
    BackgroundTaskKind, BackgroundTaskManager, BackgroundTaskUpdate, TaskResult, WorkerMessage,
};
use std::sync::atomic::Ordering;
use std::time::Duration;

#[test]
fn task_manager_rejects_concurrent_task_start() {
    let mut manager = BackgroundTaskManager::default();
    manager
        .start_task(
            BackgroundTaskKind::ExportBundle,
            "test started".to_string(),
            |_, _| {
                std::thread::sleep(Duration::from_millis(50));
                TaskResult::Completed("done".to_string())
            },
        )
        .expect("first task should start");

    let second = manager.start_task(
        BackgroundTaskKind::ValidateAssets,
        "test started".to_string(),
        |_, _| TaskResult::Completed("done".to_string()),
    );
    assert!(second.is_err(), "second task should be rejected");
}

#[test]
fn task_manager_emits_progress_and_completion_updates() {
    let mut manager = BackgroundTaskManager::default();
    manager
        .start_task(
            BackgroundTaskKind::ValidateAssets,
            "test started".to_string(),
            |_, sender| {
                let _ = sender.send(WorkerMessage::Progress("step one".to_string()));
                let _ = sender.send(WorkerMessage::Progress("step two".to_string()));
                TaskResult::Completed("all done".to_string())
            },
        )
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
        .start_task(
            BackgroundTaskKind::ExportBundle,
            "test started".to_string(),
            |cancel_flag, _sender| {
                while !cancel_flag.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(5));
                }
                TaskResult::Cancelled
            },
        )
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
