use super::{find_atlas_file, find_image_for_atlas, RenderingSystem};
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use toki_core::fonts::find_font_files;
use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::palette::Palette4;
use toki_core::project_runtime::{
    IntegerScaleFactor, PostProcessMode, QuantizeStrategy, ResolvedPostProcessSettings,
    RuntimeViewportMode,
};
use toki_core::sprite::SpriteFrame;
use toki_core::sprite_render::{
    ResolvedSpriteRenderInstance, SpriteRenderMaterial, SpriteRenderOrigin, SpriteSortKey,
};
use toki_core::text::{TextItem, TextStyle};
use toki_core::ui::{UiBlock, UiComposition, UiRect, UiTextBlock};
use toki_render::{
    Rect, RenderFrameControl, SceneClipRect, ShapeBackend, SpriteBackend, TextBackend,
    TextureBackend,
};

#[derive(Default, Debug)]
struct FakeBackend {
    projection_updates: Rc<Cell<usize>>,
    post_process_updates: Rc<Cell<usize>>,
    draw_calls: Rc<Cell<usize>>,
    resize_calls: Rc<Cell<usize>>,
    tilemap_texture_loads: Rc<RefCell<Vec<std::path::PathBuf>>>,
    tilemap_texture_rgba8_loads: Rc<Cell<usize>>,
    sprite_texture_loads: Rc<RefCell<Vec<std::path::PathBuf>>>,
    sprite_texture_rgba8_loads: Rc<Cell<usize>>,
    tilemap_render_enabled: Rc<Cell<bool>>,
    tilemap_vertex_counts: Rc<RefCell<Vec<usize>>>,
    sprite_count: Rc<Cell<usize>>,
    text_count: Rc<Cell<usize>>,
    world_underlay_rect_count: Rc<Cell<usize>>,
    debug_rect_count: Rc<Cell<usize>>,
    ui_rect_count: Rc<Cell<usize>>,
    finalized_world_underlay: Rc<Cell<usize>>,
    finalized_debug: Rc<Cell<usize>>,
    finalized_ui: Rc<Cell<usize>>,
    scene_clip_rect: Rc<RefCell<Option<SceneClipRect>>>,
    draw_error: Rc<RefCell<Option<String>>>,
}

impl RenderFrameControl for FakeBackend {
    fn set_scene_clip_rect(&mut self, rect: Option<SceneClipRect>) {
        *self.scene_clip_rect.borrow_mut() = rect;
    }

    fn update_projection(&mut self, _mvp: glam::Mat4) {
        self.projection_updates
            .set(self.projection_updates.get() + 1);
    }

    fn set_post_process_settings(&mut self, _settings: ResolvedPostProcessSettings) {
        self.post_process_updates
            .set(self.post_process_updates.get() + 1);
    }

    fn set_vsync(&mut self, _enabled: bool) {}

    fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        self.tilemap_render_enabled.set(enabled);
    }

    fn resize(&mut self, _new_size: winit::dpi::PhysicalSize<u32>) {
        self.resize_calls.set(self.resize_calls.get() + 1);
    }

    fn draw(&mut self) -> Result<(), toki_render::RenderError> {
        if let Some(message) = self.draw_error.borrow().clone() {
            return Err(toki_render::RenderError::Other(message));
        }
        self.draw_calls.set(self.draw_calls.get() + 1);
        Ok(())
    }

    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        self.tilemap_vertex_counts.borrow_mut().push(vertices.len());
    }
}

impl TextureBackend for FakeBackend {
    fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.tilemap_texture_loads.borrow_mut().push(texture_path);
        Ok(())
    }

    fn load_tilemap_texture_rgba8(
        &mut self,
        _image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        self.tilemap_texture_rgba8_loads
            .set(self.tilemap_texture_rgba8_loads.get() + 1);
        Ok(())
    }

    fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.sprite_texture_loads.borrow_mut().push(texture_path);
        Ok(())
    }

    fn load_sprite_texture_rgba8(
        &mut self,
        _image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        self.sprite_texture_rgba8_loads
            .set(self.sprite_texture_rgba8_loads.get() + 1);
        Ok(())
    }

    fn load_font_file(
        &mut self,
        _font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        Ok(())
    }
}

impl SpriteBackend for FakeBackend {
    fn clear_sprites(&mut self) {
        self.sprite_count.set(0);
    }

    fn add_sprite(
        &mut self,
        _frame: SpriteFrame,
        _position: glam::IVec2,
        _size: glam::UVec2,
        _flip_x: bool,
    ) {
        self.sprite_count.set(self.sprite_count.get() + 1);
    }

    fn add_sprite_with_texture(
        &mut self,
        _texture_path: std::path::PathBuf,
        _frame: SpriteFrame,
        _position: glam::IVec2,
        _size: glam::UVec2,
        _flip_x: bool,
    ) {
        self.sprite_count.set(self.sprite_count.get() + 1);
    }

    fn add_sprite_with_texture_rgba8(
        &mut self,
        _texture_key: std::path::PathBuf,
        _image: &DecodedImage,
        _frame: SpriteFrame,
        _position: glam::IVec2,
        _size: glam::UVec2,
        _flip_x: bool,
    ) {
        self.sprite_count.set(self.sprite_count.get() + 1);
    }
}

impl TextBackend for FakeBackend {
    fn clear_text_items(&mut self) {
        self.text_count.set(0);
    }

    fn add_text_item(&mut self, _text: TextItem) {
        self.text_count.set(self.text_count.get() + 1);
    }
}

impl ShapeBackend for FakeBackend {
    fn clear_world_underlay_shapes(&mut self) {
        self.world_underlay_rect_count.set(0);
    }

    fn add_world_underlay_rect(&mut self, _rect: Rect, _color: [f32; 4]) {
        self.world_underlay_rect_count
            .set(self.world_underlay_rect_count.get() + 1);
    }

    fn add_filled_world_underlay_rect(&mut self, _rect: Rect, _color: [f32; 4]) {
        self.world_underlay_rect_count
            .set(self.world_underlay_rect_count.get() + 1);
    }

    fn finalize_world_underlay_shapes(&mut self) {
        self.finalized_world_underlay
            .set(self.finalized_world_underlay.get() + 1);
    }

    fn clear_debug_shapes(&mut self) {
        self.debug_rect_count.set(0);
    }

    fn add_debug_rect(&mut self, _rect: Rect, _color: [f32; 4]) {
        self.debug_rect_count.set(self.debug_rect_count.get() + 1);
    }

    fn add_filled_debug_rect(&mut self, _rect: Rect, _color: [f32; 4]) {
        self.debug_rect_count.set(self.debug_rect_count.get() + 1);
    }

    fn finalize_debug_shapes(&mut self) {
        self.finalized_debug.set(self.finalized_debug.get() + 1);
    }

    fn clear_ui_shapes(&mut self) {
        self.ui_rect_count.set(0);
    }

    fn add_ui_shape(&mut self, _rect: Rect, _color: [f32; 4]) {
        self.ui_rect_count.set(self.ui_rect_count.get() + 1);
    }

    fn add_filled_ui_shape(&mut self, _rect: Rect, _color: [f32; 4]) {
        self.ui_rect_count.set(self.ui_rect_count.get() + 1);
    }

    fn finalize_ui_shapes(&mut self) {
        self.finalized_ui.set(self.finalized_ui.get() + 1);
    }
}

fn make_unique_temp_dir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("toki_runtime_rendering_tests_{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

fn project_to_screen(
    projection: glam::Mat4,
    surface_width: f32,
    surface_height: f32,
    point: glam::Vec2,
) -> glam::Vec2 {
    let clip = projection * glam::Vec4::new(point.x, point.y, 0.0, 1.0);
    let ndc = clip / clip.w;
    glam::Vec2::new(
        (ndc.x * 0.5 + 0.5) * surface_width,
        (1.0 - (ndc.y * 0.5 + 0.5)) * surface_height,
    )
}

#[test]
fn rendering_system_defaults_and_no_gpu_error_paths() {
    let mut rendering = RenderingSystem::new();
    assert!(!rendering.has_gpu());
    let params = rendering.projection_params();
    assert_eq!(params.width, 160);
    assert_eq!(params.height, 144);
    assert_eq!(params.desired_width, 160);
    assert_eq!(params.desired_height, 144);

    let tilemap_err = rendering
        .load_tilemap_texture(std::path::PathBuf::from("terrain.png"))
        .expect_err("tilemap load without gpu must fail");
    assert!(
        tilemap_err.to_string().contains("GPU not initialized"),
        "unexpected error: {tilemap_err}"
    );

    let sprite_err = rendering
        .load_sprite_texture(std::path::PathBuf::from("creatures.png"))
        .expect_err("sprite load without gpu must fail");
    assert!(
        sprite_err.to_string().contains("GPU not initialized"),
        "unexpected error: {sprite_err}"
    );
}

#[test]
fn calculate_projection_centers_the_viewport_in_wide_windows() {
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.update_window_size(winit::dpi::PhysicalSize::new(320, 144));

    let projection = rendering.calculate_projection();
    let top_left = project_to_screen(projection, 320.0, 144.0, glam::Vec2::ZERO);
    let bottom_right = project_to_screen(projection, 320.0, 144.0, glam::Vec2::new(160.0, 144.0));

    assert!((top_left.x - 80.0).abs() < 0.01);
    assert!((top_left.y - 0.0).abs() < 0.01);
    assert!((bottom_right.x - 240.0).abs() < 0.01);
    assert!((bottom_right.y - 144.0).abs() < 0.01);
}

#[test]
fn calculate_projection_centers_the_viewport_in_tall_windows() {
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.update_window_size(winit::dpi::PhysicalSize::new(160, 320));

    let projection = rendering.calculate_projection();
    let top_left = project_to_screen(projection, 160.0, 320.0, glam::Vec2::ZERO);
    let bottom_right = project_to_screen(projection, 160.0, 320.0, glam::Vec2::new(160.0, 144.0));

    assert!((top_left.x - 0.0).abs() < 0.01);
    assert!((top_left.y - 88.0).abs() < 0.01);
    assert!((bottom_right.x - 160.0).abs() < 0.01);
    assert!((bottom_right.y - 232.0).abs() < 0.01);
}

#[test]
fn aspect_fit_projection_fills_window_height_when_it_is_the_limiting_dimension() {
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.set_viewport_mode(RuntimeViewportMode::AspectFit { fit_percent: 100 });
    rendering.update_window_size(winit::dpi::PhysicalSize::new(320, 200));

    let projection = rendering.calculate_projection();
    let top_left = project_to_screen(projection, 320.0, 200.0, glam::Vec2::ZERO);
    let bottom_right = project_to_screen(projection, 320.0, 200.0, glam::Vec2::new(160.0, 144.0));

    assert!((top_left.x - 48.88889).abs() < 0.05);
    assert!((top_left.y - 0.0).abs() < 0.01);
    assert!((bottom_right.x - 271.1111).abs() < 0.05);
    assert!((bottom_right.y - 200.0).abs() < 0.01);
}

#[test]
fn integer_scale_auto_projection_uses_largest_whole_number_factor() {
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.set_viewport_mode(RuntimeViewportMode::IntegerScale {
        factor: IntegerScaleFactor::Auto,
    });
    rendering.update_window_size(winit::dpi::PhysicalSize::new(800, 600));

    let projection = rendering.calculate_projection();
    let top_left = project_to_screen(projection, 800.0, 600.0, glam::Vec2::ZERO);
    let bottom_right = project_to_screen(projection, 800.0, 600.0, glam::Vec2::new(160.0, 144.0));

    assert!((top_left.x - 80.0).abs() < 0.01);
    assert!((top_left.y - 12.0).abs() < 0.01);
    assert!((bottom_right.x - 720.0).abs() < 0.01);
    assert!((bottom_right.y - 588.0).abs() < 0.01);
}

#[test]
fn integer_scale_fixed_projection_uses_requested_factor_when_available() {
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.set_viewport_mode(RuntimeViewportMode::IntegerScale {
        factor: IntegerScaleFactor::Fixed(3),
    });
    rendering.update_window_size(winit::dpi::PhysicalSize::new(800, 600));

    let projection = rendering.calculate_projection();
    let top_left = project_to_screen(projection, 800.0, 600.0, glam::Vec2::ZERO);
    let bottom_right = project_to_screen(projection, 800.0, 600.0, glam::Vec2::new(160.0, 144.0));

    assert!((top_left.x - 160.0).abs() < 0.01);
    assert!((top_left.y - 84.0).abs() < 0.01);
    assert!((bottom_right.x - 640.0).abs() < 0.01);
    assert!((bottom_right.y - 516.0).abs() < 0.01);
}

#[test]
fn window_fill_projection_uses_full_window_without_letterbox() {
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.set_viewport_mode(RuntimeViewportMode::WindowFill { zoom_percent: 100 });
    rendering.update_window_size(winit::dpi::PhysicalSize::new(320, 144));

    let projection = rendering.calculate_projection();
    let top_left = project_to_screen(projection, 320.0, 144.0, glam::Vec2::ZERO);
    let bottom_right = project_to_screen(projection, 320.0, 144.0, glam::Vec2::new(320.0, 144.0));

    assert!((top_left.x - 0.0).abs() < 0.01);
    assert!((top_left.y - 0.0).abs() < 0.01);
    assert!((bottom_right.x - 320.0).abs() < 0.01);
    assert!((bottom_right.y - 144.0).abs() < 0.01);
}

#[test]
fn update_projection_applies_scene_clip_rect_for_letterboxed_viewport() {
    let fake = FakeBackend::default();
    let scene_clip_rect = fake.scene_clip_rect.clone();
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.set_backend_for_tests(Box::new(fake));
    rendering.set_viewport_mode(RuntimeViewportMode::IntegerScale {
        factor: IntegerScaleFactor::Auto,
    });
    rendering.update_window_size(winit::dpi::PhysicalSize::new(800, 600));

    rendering.update_projection(glam::Mat4::IDENTITY);

    assert_eq!(
        *scene_clip_rect.borrow(),
        Some(SceneClipRect {
            x: 80,
            y: 12,
            width: 640,
            height: 576,
        })
    );
}

#[test]
fn update_projection_uses_full_window_scene_clip_rect_for_window_fill() {
    let fake = FakeBackend::default();
    let scene_clip_rect = fake.scene_clip_rect.clone();
    let mut rendering = RenderingSystem::new_with_desired_resolution(160, 144);
    rendering.set_backend_for_tests(Box::new(fake));
    rendering.set_viewport_mode(RuntimeViewportMode::WindowFill { zoom_percent: 100 });
    rendering.update_window_size(winit::dpi::PhysicalSize::new(320, 144));

    rendering.update_projection(glam::Mat4::IDENTITY);

    assert_eq!(
        *scene_clip_rect.borrow(),
        Some(SceneClipRect {
            x: 0,
            y: 0,
            width: 320,
            height: 144,
        })
    );
}

#[test]
fn rendering_system_forwards_post_process_settings_to_backend() {
    let mut rendering = RenderingSystem::new();
    let backend = FakeBackend::default();
    let updates = backend.post_process_updates.clone();
    rendering.set_backend_for_tests(Box::new(backend));

    rendering.set_post_process_settings(ResolvedPostProcessSettings {
        mode: PostProcessMode::Tint,
        quantize_strategy: QuantizeStrategy::Luminance,
        tint_color: [10, 20, 30, 255],
        tint_strength_percent: 50,
        brightness_percent: 0,
        saturation_percent: 100,
        quantize_palette: Palette4::new([[0, 0, 0, 255]; 4]),
        gb_contrast_percent: 0,
        vignette_strength_percent: 60,
    });

    assert_eq!(updates.get(), 1);
}

#[test]
fn resolved_sprite_instances_submit_through_the_shared_rendering_entrypoint() {
    let sprite_count = Rc::new(Cell::new(0));
    let mut rendering = RenderingSystem::new();
    let backend = FakeBackend {
        sprite_count: sprite_count.clone(),
        ..FakeBackend::default()
    };
    rendering.set_backend_for_tests(Box::new(backend));

    rendering.add_resolved_sprite(&ResolvedSpriteRenderInstance {
        origin: SpriteRenderOrigin::AnimatedEntity(1),
        sort_key: SpriteSortKey {
            primary: 0,
            secondary: 0,
            sequence: 0,
        },
        frame: SpriteFrame {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        },
        position: glam::IVec2::new(4, 6),
        size: glam::UVec2::new(16, 16),
        texture_path: Some(std::path::PathBuf::from("sprites/player.png")),
        material: SpriteRenderMaterial::TrueColor,
        flip_x: true,
    });

    assert_eq!(sprite_count.get(), 1);
}

#[test]
fn palette_indexed_resolved_sprites_use_rgba8_backend_path() {
    let sprite_count = Rc::new(Cell::new(0));
    let mut rendering = RenderingSystem::new();
    let backend = FakeBackend {
        sprite_count: sprite_count.clone(),
        ..FakeBackend::default()
    };
    rendering.set_backend_for_tests(Box::new(backend));

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let texture_path = temp_dir.path().join("indexed.png");
    toki_core::graphics::image::save_image_rgba8(
        &texture_path,
        2,
        2,
        &[
            0x00, 0x00, 0x00, 0xFF, //
            0x55, 0x55, 0x55, 0xFF, //
            0xAA, 0xAA, 0xAA, 0xFF, //
            0xFF, 0xFF, 0xFF, 0xFF,
        ],
    )
    .expect("indexed texture");

    rendering.add_resolved_sprite(&ResolvedSpriteRenderInstance {
        origin: SpriteRenderOrigin::AnimatedEntity(1),
        sort_key: SpriteSortKey {
            primary: 0,
            secondary: 0,
            sequence: 0,
        },
        frame: SpriteFrame {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        },
        position: glam::IVec2::new(4, 6),
        size: glam::UVec2::new(16, 16),
        texture_path: Some(texture_path),
        material: SpriteRenderMaterial::PaletteIndexed {
            palette_id: "gb_default".to_string(),
            palette: Palette4::new([
                [1, 2, 3, 255],
                [4, 5, 6, 255],
                [7, 8, 9, 255],
                [10, 11, 12, 255],
            ]),
        },
        flip_x: false,
    });

    assert_eq!(sprite_count.get(), 1);
}

#[test]
fn atlas_discovery_helpers_find_json_and_matching_image() {
    let tmp = make_unique_temp_dir();
    let sprites_dir = tmp.join("sprites");
    std::fs::create_dir_all(&sprites_dir).expect("sprites dir should exist");

    let atlas_path = sprites_dir.join("creatures.json");
    let image_path = sprites_dir.join("creatures.png");
    std::fs::write(&atlas_path, "{}").expect("atlas file should be created");
    std::fs::write(&image_path, "x").expect("image file should be created");

    let found_atlas =
        find_atlas_file(Path::new(&sprites_dir), "creatures").expect("atlas path should be found");
    assert_eq!(found_atlas, atlas_path);

    let found_image = find_image_for_atlas(&found_atlas).expect("image should be found");
    assert_eq!(found_image, image_path);

    std::fs::remove_dir_all(tmp).expect("temp dir cleanup should succeed");
}

#[test]
fn load_project_textures_returns_ok_when_assets_missing() {
    let mut rendering = RenderingSystem::new();
    let tmp = make_unique_temp_dir();

    // No assets directory -> helper should no-op successfully.
    rendering
        .load_project_textures(&tmp)
        .expect("missing project assets should be treated as no-op");

    std::fs::remove_dir_all(tmp).expect("temp dir cleanup should succeed");
}

#[test]
fn find_font_files_only_returns_supported_extensions_sorted() {
    let tmp = make_unique_temp_dir();
    std::fs::create_dir_all(&tmp).expect("temp dir should exist");
    let supported_a = tmp.join("A.ttf");
    let supported_b = tmp.join("b.otf");
    let supported_c = tmp.join("c.TTC");
    let ignored = tmp.join("readme.txt");
    std::fs::write(&supported_a, "a").expect("font a");
    std::fs::write(&supported_b, "b").expect("font b");
    std::fs::write(&supported_c, "c").expect("font c");
    std::fs::write(&ignored, "x").expect("ignored");

    let found = find_font_files(&tmp);
    assert_eq!(found, vec![supported_a, supported_b, supported_c]);

    std::fs::remove_dir_all(tmp).expect("temp dir cleanup should succeed");
}

#[test]
fn backend_seam_dispatches_runtime_render_commands() {
    let fake = FakeBackend::default();
    let projection_counter = fake.projection_updates.clone();
    let draw_counter = fake.draw_calls.clone();
    let resize_counter = fake.resize_calls.clone();
    let tilemap_texture_loads = fake.tilemap_texture_loads.clone();
    let sprite_texture_loads = fake.sprite_texture_loads.clone();
    let tilemap_render_enabled = fake.tilemap_render_enabled.clone();
    let tilemap_counts = fake.tilemap_vertex_counts.clone();
    let text_count = fake.text_count.clone();
    let world_underlay_rect_count = fake.world_underlay_rect_count.clone();
    let world_underlay_finalize_counter = fake.finalized_world_underlay.clone();
    let debug_rect_count = fake.debug_rect_count.clone();
    let debug_finalize_counter = fake.finalized_debug.clone();
    let ui_rect_count = fake.ui_rect_count.clone();
    let ui_finalize_counter = fake.finalized_ui.clone();

    let mut rendering = RenderingSystem::new();
    rendering.backend = Some(Box::new(fake));
    assert!(
        rendering.has_gpu(),
        "test backend should be treated as initialized"
    );

    rendering.update_projection(glam::Mat4::IDENTITY);
    rendering.resize(winit::dpi::PhysicalSize::new(640, 480));
    rendering
        .load_tilemap_texture(std::path::PathBuf::from("terrain.png"))
        .expect("tilemap load should work");
    rendering
        .load_sprite_texture(std::path::PathBuf::from("creatures.png"))
        .expect("sprite load should work");
    rendering.set_tilemap_render_enabled(false);
    rendering.set_tilemap_render_enabled(true);
    rendering.update_tilemap_vertices(&[
        QuadVertex {
            position: [0.0, 0.0],
            tex_coords: [0.0, 0.0],
        },
        QuadVertex {
            position: [16.0, 16.0],
            tex_coords: [1.0, 1.0],
        },
    ]);
    rendering.clear_sprites();
    rendering.add_sprite(
        SpriteFrame {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        },
        glam::IVec2::new(10, 20),
        glam::UVec2::new(16, 16),
        false,
    );
    rendering.clear_text_items();
    rendering.add_text_item(TextItem::new_screen(
        "Runtime HUD",
        glam::Vec2::new(8.0, 8.0),
        TextStyle::default(),
    ));
    rendering.clear_world_underlay_shapes();
    rendering.add_world_underlay_rect(2.0, 3.0, 10.0, 2.0, [0.0, 0.0, 0.0, 0.25]);
    rendering.add_filled_world_underlay_rect(3.0, 4.0, 8.0, 2.0, [0.0, 0.0, 0.0, 0.3]);
    rendering.finalize_world_underlay_shapes();
    rendering.clear_debug_shapes();
    rendering.add_debug_rect(0.0, 0.0, 16.0, 16.0, [1.0, 0.0, 0.0, 1.0]);
    rendering.add_filled_debug_rect(1.0, 1.0, 14.0, 14.0, [0.0, 1.0, 0.0, 1.0]);
    rendering.finalize_debug_shapes();
    rendering.clear_ui_shapes();
    rendering.add_ui_shape(4.0, 4.0, 12.0, 12.0, [1.0, 1.0, 1.0, 1.0]);
    rendering.add_filled_ui_shape(5.0, 5.0, 10.0, 10.0, [0.0, 0.0, 0.0, 0.5]);
    rendering.finalize_ui_shapes();
    rendering.draw().expect("draw should succeed");

    assert_eq!(projection_counter.get(), 1);
    assert_eq!(draw_counter.get(), 1);
    assert_eq!(resize_counter.get(), 1);
    assert_eq!(
        tilemap_texture_loads.borrow().as_slice(),
        &[std::path::PathBuf::from("terrain.png")]
    );
    assert_eq!(
        sprite_texture_loads.borrow().as_slice(),
        &[std::path::PathBuf::from("creatures.png")]
    );
    assert!(tilemap_render_enabled.get());
    assert_eq!(tilemap_counts.borrow().as_slice(), &[2]);
    assert_eq!(text_count.get(), 1);
    assert_eq!(world_underlay_rect_count.get(), 2);
    assert_eq!(world_underlay_finalize_counter.get(), 1);
    assert_eq!(debug_rect_count.get(), 2);
    assert_eq!(debug_finalize_counter.get(), 1);
    assert_eq!(ui_rect_count.get(), 2);
    assert_eq!(ui_finalize_counter.get(), 1);
}

#[test]
fn rendering_system_draw_propagates_backend_errors() {
    let fake = FakeBackend::default();
    let draw_error = fake.draw_error.clone();
    *draw_error.borrow_mut() = Some("text preparation failed".to_string());

    let mut rendering = RenderingSystem::new();
    rendering.backend = Some(Box::new(fake));

    let error = rendering.draw().expect_err("draw error should propagate");
    assert!(error.to_string().contains("text preparation failed"));
}

#[test]
fn render_ui_composition_dispatches_rectangles_and_text() {
    let fake = FakeBackend::default();
    let ui_rect_count = fake.ui_rect_count.clone();
    let text_count = fake.text_count.clone();
    let mut rendering = RenderingSystem::new();
    rendering.backend = Some(Box::new(fake));

    let mut composition = UiComposition::default();
    composition.push(UiBlock {
        rect: UiRect {
            x: 16.0,
            y: 24.0,
            width: 120.0,
            height: 40.0,
        },
        fill_color: Some([0.1, 0.2, 0.3, 0.8]),
        border_color: Some([0.9, 1.0, 0.9, 1.0]),
        border_thickness: 2.0,
        text: Some(UiTextBlock {
            content: "Paused".to_string(),
            position: glam::Vec2::new(76.0, 34.0),
            anchor: toki_core::text::TextAnchor::TopCenter,
            style: TextStyle::default(),
            layer: 10,
        }),
    });

    rendering.render_ui_composition(&composition);

    assert_eq!(ui_rect_count.get(), 3);
    assert_eq!(text_count.get(), 1);
}

#[test]
fn texture_loads_are_cached_by_path() {
    let fake = FakeBackend::default();
    let tilemap_texture_loads = fake.tilemap_texture_loads.clone();
    let sprite_texture_loads = fake.sprite_texture_loads.clone();

    let mut rendering = RenderingSystem::new();
    rendering.backend = Some(Box::new(fake));

    rendering
        .load_tilemap_texture(std::path::PathBuf::from("terrain.png"))
        .expect("first tilemap load");
    rendering
        .load_tilemap_texture(std::path::PathBuf::from("terrain.png"))
        .expect("cached tilemap load");
    rendering
        .load_sprite_texture(std::path::PathBuf::from("creatures.png"))
        .expect("first sprite load");
    rendering
        .load_sprite_texture(std::path::PathBuf::from("creatures.png"))
        .expect("cached sprite load");

    assert_eq!(
        tilemap_texture_loads.borrow().as_slice(),
        &[std::path::PathBuf::from("terrain.png")]
    );
    assert_eq!(
        sprite_texture_loads.borrow().as_slice(),
        &[std::path::PathBuf::from("creatures.png")]
    );
}

#[test]
fn loading_embedded_sprite_texture_invalidates_path_cache() {
    let fake = FakeBackend::default();
    let sprite_texture_loads = fake.sprite_texture_loads.clone();
    let sprite_texture_rgba8_loads = fake.sprite_texture_rgba8_loads.clone();

    let mut rendering = RenderingSystem::new();
    rendering.backend = Some(Box::new(fake));

    rendering
        .load_sprite_texture(std::path::PathBuf::from("creatures.png"))
        .expect("initial sprite load");
    rendering
        .load_sprite_texture_rgba8(&DecodedImage {
            width: 1,
            height: 1,
            data: vec![255, 255, 255, 255],
        })
        .expect("embedded sprite load");
    rendering
        .load_sprite_texture(std::path::PathBuf::from("creatures.png"))
        .expect("restored sprite load");

    assert_eq!(sprite_texture_rgba8_loads.get(), 1);
    assert_eq!(
        sprite_texture_loads.borrow().as_slice(),
        &[
            std::path::PathBuf::from("creatures.png"),
            std::path::PathBuf::from("creatures.png")
        ]
    );
}
