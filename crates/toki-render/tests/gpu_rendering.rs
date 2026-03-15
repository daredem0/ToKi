use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use glam::{IVec2, Mat4, UVec2, Vec2};
use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use toki_core::assets::tilemap::TileMap;
use toki_core::sprite::SpriteFrame;
use toki_render::{
    DebugShape, DebugShapeType, OffscreenTarget, RenderTarget, SceneData, SceneRenderer,
    SpriteInstance,
};

fn create_device_and_queue() -> Option<(wgpu::Device, wgpu::Queue)> {
    for backends in [
        wgpu::Backends::PRIMARY,
        wgpu::Backends::VULKAN,
        wgpu::Backends::GL,
    ] {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            flags: wgpu::InstanceFlags::default(),
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

        let Ok((device, queue)) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::default(),
                label: Some("toki-render-test-device"),
            }))
        else {
            continue;
        };

        return Some((device, queue));
    }

    None
}

fn write_test_png(path: &Path, rgba: [u8; 4]) {
    let image = image::RgbaImage::from_pixel(1, 1, image::Rgba(rgba));
    image
        .save(path)
        .expect("test png should be written successfully");
}

fn make_unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("toki_render_tests_{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

fn build_scene_data(visible_chunks: Vec<(u32, u32)>) -> SceneData {
    let mut atlas_tiles = HashMap::new();
    atlas_tiles.insert(
        "floor".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );

    let tilemap = TileMap {
        size: UVec2::new(2, 2),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec![
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
        ],
        objects: vec![],
    };
    let atlas = AtlasMeta {
        image: PathBuf::from("terrain.png"),
        tile_size: UVec2::new(16, 16),
        tiles: atlas_tiles,
    };

    SceneData {
        tilemap: Some(tilemap),
        atlas: Some(atlas),
        texture_size: UVec2::new(16, 16),
        visible_chunks,
        sprites: vec![SpriteInstance {
            frame: SpriteFrame {
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            },
            position: IVec2::new(4, 8),
            size: UVec2::new(16, 16),
            texture_path: None,
            flip_x: false,
        }],
        debug_shapes: vec![
            DebugShape {
                shape_type: DebugShapeType::Rectangle,
                position: Vec2::new(1.0, 2.0),
                size: Vec2::new(3.0, 4.0),
                color: [1.0, 0.0, 0.0, 1.0],
            },
            DebugShape {
                shape_type: DebugShapeType::Circle,
                position: Vec2::new(3.0, 3.0),
                size: Vec2::new(2.0, 2.0),
                color: [0.0, 1.0, 0.0, 1.0],
            },
            DebugShape {
                shape_type: DebugShapeType::Line {
                    end: Vec2::new(8.0, 8.0),
                },
                position: Vec2::new(1.0, 1.0),
                size: Vec2::new(0.0, 0.0),
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ],
    }
}

#[test]
fn offscreen_target_lifecycle_and_resize_works() {
    let Some((device, _queue)) = create_device_and_queue() else {
        eprintln!("Skipping GPU-backed test: no compatible adapter/device available");
        return;
    };
    let mut target = OffscreenTarget::new(device, (64, 48), wgpu::TextureFormat::Bgra8UnormSrgb)
        .expect("offscreen target should be created");

    assert_eq!(target.size(), (64, 48));
    assert_eq!(target.format(), wgpu::TextureFormat::Bgra8UnormSrgb);

    target.begin_frame().expect("begin_frame should succeed");
    let _ = target
        .get_render_view()
        .expect("render view should be available");
    target.end_frame().expect("end_frame should succeed");

    target
        .resize((64, 48))
        .expect("resize with unchanged size should be a no-op");
    target
        .resize((128, 96))
        .expect("resize with new size should recreate target");
    assert_eq!(target.size(), (128, 96));
    let _ = target.texture();
}

#[test]
fn scene_renderer_renders_with_and_without_chunking() {
    let Some((device, queue)) = create_device_and_queue() else {
        eprintln!("Skipping GPU-backed test: no compatible adapter/device available");
        return;
    };
    let mut renderer = SceneRenderer::new(
        device.clone(),
        queue.clone(),
        wgpu::TextureFormat::Bgra8UnormSrgb,
        None,
        None,
    )
    .expect("scene renderer should be created");
    let mut target = OffscreenTarget::new(device, (160, 144), wgpu::TextureFormat::Bgra8UnormSrgb)
        .expect("offscreen target should be created");

    let full_scene = build_scene_data(Vec::new());
    renderer
        .render_scene(&mut target, &full_scene)
        .expect("full-scene render should succeed");

    let chunked_scene = build_scene_data(vec![(0, 0)]);
    renderer
        .render_scene_with_projection(&mut target, &chunked_scene, Mat4::IDENTITY)
        .expect("chunked-scene render with custom projection should succeed");

    // Also exercise empty scene defaults path.
    renderer
        .render_scene(&mut target, &SceneData::default())
        .expect("rendering default scene should succeed");
}

#[test]
fn scene_renderer_texture_reload_paths_are_supported() {
    let Some((device, queue)) = create_device_and_queue() else {
        eprintln!("Skipping GPU-backed test: no compatible adapter/device available");
        return;
    };
    let tmp = make_unique_temp_dir();
    let terrain_png = tmp.join("terrain.png");
    let creatures_png = tmp.join("creatures.png");
    write_test_png(&terrain_png, [255, 255, 255, 255]);
    write_test_png(&creatures_png, [255, 0, 255, 255]);

    let mut renderer = SceneRenderer::new(
        device,
        queue,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        Some(terrain_png.clone()),
        Some(creatures_png.clone()),
    )
    .expect("scene renderer should be created with explicit textures");

    renderer
        .load_tilemap_texture(terrain_png.clone())
        .expect("tilemap texture reload should succeed");
    renderer
        .load_sprite_texture(creatures_png.clone())
        .expect("sprite texture reload should succeed");
    // Cached same-texture path should return quickly and still succeed.
    renderer
        .load_sprite_texture(creatures_png)
        .expect("sprite texture cache fast path should succeed");

    std::fs::remove_dir_all(tmp).expect("temp dir cleanup should succeed");
}
