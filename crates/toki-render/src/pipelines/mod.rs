pub mod debug;
pub mod sprite;
pub mod tilemap;

use wgpu::{Queue, RenderPass};

/// Common trait for all rendering pipelines
pub trait RenderPipeline {
    /// Render using this pipeline
    fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>);

    /// Update pipeline state (buffers, uniforms, etc.)
    fn update(&mut self);

    /// Update pipeline state with queue access (optional)
    fn update_with_queue(&mut self, _queue: &Queue) {
        self.update();
    }
}
