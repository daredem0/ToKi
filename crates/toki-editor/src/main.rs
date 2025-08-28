use anyhow::Result;
use tracing_subscriber;

mod editor_app;
mod renderer;
mod ui;
mod game_viewport;
mod editor_renderer;

use editor_app::run_editor;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();
    
    if let Err(e) = run_editor() {
        tracing::error!("Fatal error: {e:?}");
    }
    
    Ok(())
}
