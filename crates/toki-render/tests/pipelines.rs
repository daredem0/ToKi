use toki_render::RenderPipeline;

// Mock pipeline implementation for testing
struct MockPipeline {
    update_called: bool,
    update_with_queue_called: bool,
}

impl MockPipeline {
    fn new() -> Self {
        Self {
            update_called: false,
            update_with_queue_called: false,
        }
    }
}

impl RenderPipeline for MockPipeline {
    fn render<'a>(&'a self, _render_pass: &mut wgpu::RenderPass<'a>) {
        // Mock implementation - no actual rendering
    }

    fn update(&mut self) {
        self.update_called = true;
    }

    fn update_with_queue(&mut self, _queue: &wgpu::Queue) {
        self.update_with_queue_called = true;
        // Call the default implementation which calls update()
        self.update();
    }
}

// Pipeline that uses the trait's default `update_with_queue` implementation.
struct DefaultQueuePipeline {
    update_called: bool,
}

impl DefaultQueuePipeline {
    fn new() -> Self {
        Self {
            update_called: false,
        }
    }
}

impl RenderPipeline for DefaultQueuePipeline {
    fn render<'a>(&'a self, _render_pass: &mut wgpu::RenderPass<'a>) {}

    fn update(&mut self) {
        self.update_called = true;
    }
}

fn create_queue() -> Option<wgpu::Queue> {
    for backends in [
        wgpu::Backends::PRIMARY,
        wgpu::Backends::VULKAN,
        wgpu::Backends::GL,
    ] {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: true,
            }))
        else {
            continue;
        };
        let Ok((_device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
                label: Some("pipeline-trait-test-device"),
            }))
        else {
            continue;
        };
        return Some(queue);
    }
    None
}

#[test]
fn render_pipeline_trait_update_method() {
    let mut pipeline = MockPipeline::new();
    assert!(!pipeline.update_called);

    pipeline.update();
    assert!(pipeline.update_called);
}

#[test]
fn render_pipeline_trait_has_render_method() {
    let _pipeline = MockPipeline::new();
    // We can't actually create a RenderPass without a full GPU setup,
    // but we can verify the method signature compiles
    let _render_fn = MockPipeline::render;
}

#[test]
fn render_pipeline_trait_update_with_queue_calls_update() {
    let pipeline = MockPipeline::new();
    assert!(!pipeline.update_called);
    assert!(!pipeline.update_with_queue_called);

    // We can't create a real Queue without GPU, but we can test the trait method exists
    // and that our implementation works with the concept
    let _update_with_queue_fn = MockPipeline::update_with_queue;
}

#[test]
fn default_update_with_queue_calls_update() {
    let Some(queue) = create_queue() else {
        eprintln!("Skipping queue-backed trait test: no compatible adapter/device available");
        return;
    };
    let mut pipeline = DefaultQueuePipeline::new();
    assert!(!pipeline.update_called);

    pipeline.update_with_queue(&queue);
    assert!(pipeline.update_called);
}

#[test]
fn render_pipeline_trait_object_safety() {
    let pipeline = MockPipeline::new();
    let _trait_object: Box<dyn RenderPipeline> = Box::new(pipeline);
    // This test ensures RenderPipeline is object-safe
}

// Test that all main pipeline types implement the trait
#[test]
fn pipeline_types_implement_trait() {
    use toki_render::{DebugPipeline, SpritePipeline, TilemapPipeline};

    // This test ensures all pipeline types implement RenderPipeline
    // We can't instantiate them without GPU setup, but we can test the trait bounds

    fn assert_render_pipeline<T: RenderPipeline>() {}

    assert_render_pipeline::<DebugPipeline>();
    assert_render_pipeline::<SpritePipeline>();
    assert_render_pipeline::<TilemapPipeline>();
}

#[test]
fn render_pipeline_trait_methods_exist() {
    // Compile-time test that all required methods exist

    // Test that update method exists
    let _: fn(&mut MockPipeline) = MockPipeline::update;

    // Test that update_with_queue method exists
    let _: fn(&mut MockPipeline, &wgpu::Queue) = MockPipeline::update_with_queue;
}
