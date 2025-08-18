use anyhow::Result;

use toki_runtime::run_minimal_window;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .init();
    if let Err(e) = run_minimal_window() {
        tracing::error!("Fatal error: {e:?}");
    }
    Ok(())
}
