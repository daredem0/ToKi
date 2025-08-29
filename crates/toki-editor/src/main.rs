use anyhow::Result;
mod config;
mod editor_app;
mod logging;
use logging::LogCapture;
mod project;
mod rendering;
mod scene;
mod ui;
use tracing_subscriber::prelude::*;

use editor_app::run_editor;

fn main() -> Result<()> {
    let config = config::EditorConfig::load().unwrap_or_default();
    let log_capture = if config.log_to_terminal {
        let level = match config.log_level.as_str() {
            "ERROR" => tracing::Level::ERROR,
            "WARN" => tracing::Level::WARN,
            "INFO" => tracing::Level::INFO,
            "DEBUG" => tracing::Level::DEBUG,
            "TRACE" => tracing::Level::TRACE,
            _ => tracing::Level::INFO,
        };
        
        tracing_subscriber::fmt()
            .with_max_level(level)
            .with_target(false)
            .init();
        None
    } else {
        let level = match config.log_level.as_str() {
            "ERROR" => tracing::Level::ERROR,
            "WARN" => tracing::Level::WARN,
            "INFO" => tracing::Level::INFO,
            "DEBUG" => tracing::Level::DEBUG,
            "TRACE" => tracing::Level::TRACE,
            _ => tracing::Level::INFO,
        };
        
        let capture = LogCapture::new();
        let layer = logging::LogCaptureLayer::new(capture.clone());
        
        tracing_subscriber::registry()
            .with(layer.with_filter(tracing_subscriber::filter::LevelFilter::from_level(level)))
            .init();
            
        Some(capture)
    };
    if let Err(e) = run_editor(log_capture) {
        tracing::error!("Fatal error: {e:?}");
    }

    Ok(())
}
