use std::sync::{Arc, Mutex};
use tracing_subscriber::Layer;

pub struct LogCaptureLayer {
    capture: LogCapture,
}

impl<S> Layer<S> for LogCaptureLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let level = event.metadata().level().to_string().to_uppercase();
        let mut message = String::new();

        // Extract message from event
        event.record(&mut LogMessageVisitor(&mut message));
        self.capture.add_log(level, message);
    }
}

struct LogMessageVisitor<'a>(&'a mut String);

impl<'a> tracing::field::Visit for LogMessageVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.0 = format!("{:?}", value);
        }
    }
}

impl LogCaptureLayer {
    pub fn new(capture: LogCapture) -> Self {
        Self { capture }
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Clone, Debug)]
pub struct LogCapture {
    logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl LogCapture {
    fn with_logs<R>(&self, f: impl FnOnce(&Vec<LogEntry>) -> R) -> R {
        match self.logs.lock() {
            Ok(logs) => f(&logs),
            Err(poisoned) => {
                tracing::warn!("Log capture mutex was poisoned; continuing with inner state");
                f(&poisoned.into_inner())
            }
        }
    }

    fn with_logs_mut<R>(&self, f: impl FnOnce(&mut Vec<LogEntry>) -> R) -> R {
        match self.logs.lock() {
            Ok(mut logs) => f(&mut logs),
            Err(poisoned) => {
                tracing::warn!("Log capture mutex was poisoned; continuing with inner state");
                let mut logs = poisoned.into_inner();
                f(&mut logs)
            }
        }
    }

    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.with_logs(Clone::clone)
    }

    pub fn add_log(&self, level: String, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        let entry = LogEntry {
            level,
            message,
            timestamp,
        };

        self.with_logs_mut(|logs| {
            logs.push(entry);

            // Keep only last 1000 logs to prevent memory issues
            if logs.len() > 1000 {
                logs.remove(0);
            }
        });
    }
}
