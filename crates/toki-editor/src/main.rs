use anyhow::Result;
mod background_tasks;
mod config;
mod editor_app;
mod logging;
use logging::LogCapture;
mod project;
mod rendering;
mod scene;
mod ui;
mod validation;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

use editor_app::run_editor;

fn main() -> Result<()> {
    let config = config::EditorConfig::load().unwrap_or_default();
    let log_capture = if config.log_to_terminal {
        let level_str = config.log_level.as_str();

        // Create filter that suppresses verbose external crate logs
        let filter = EnvFilter::new(format!(
            "toki={},naga=warn,wgpu_hal=warn,wgpu_core=warn,naga::front=warn,naga::valid=warn,egui_wgpu=warn",
            level_str.to_lowercase()
        ));

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_file(false)
                    .with_line_number(true),
            )
            .with(filter)
            .init();
        None
    } else {
        let level_str = config.log_level.as_str();

        // Create filter that suppresses verbose external crate logs
        let filter = EnvFilter::new(format!(
            "toki={},naga=warn,wgpu_hal=warn,wgpu_core=warn,naga::front=warn,naga::valid=warn,egui_wgpu=warn",
            level_str.to_lowercase()
        ));

        let capture = LogCapture::new();
        let layer = logging::LogCaptureLayer::new(capture.clone());

        tracing_subscriber::registry()
            .with(layer.with_filter(filter))
            .init();

        Some(capture)
    };
    if let Err(e) = run_editor(log_capture) {
        tracing::error!("Fatal error: {e:?}");
    }

    Ok(())
}
