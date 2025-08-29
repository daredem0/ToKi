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
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.lock().unwrap().clone()
    }


    pub fn add_log(&self, level: String, message: String) {
        let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
        let entry = LogEntry {
            level,
            message,
            timestamp,
        };

        let mut logs = self.logs.lock().unwrap();
        logs.push(entry);

        // Keep only last 1000 logs to prevent memory issues
        if logs.len() > 1000 {
            logs.remove(0);
        }
    }
}
