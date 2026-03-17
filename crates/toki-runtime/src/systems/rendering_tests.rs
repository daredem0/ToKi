use super::{find_atlas_file, find_image_for_atlas, RenderingSystem, RuntimeRenderBackend};
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use toki_core::fonts::find_font_files;
use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::sprite::SpriteFrame;
use toki_core::text::{TextItem, TextStyle};

#[derive(Default, Debug)]
struct FakeBackend {
    projection_updates: Rc<Cell<usize>>,
    draw_calls: Rc<Cell<usize>>,
    resize_calls: Rc<Cell<usize>>,
    tilemap_texture_loads: Rc<RefCell<Vec<std::path::PathBuf>>>,
    sprite_texture_loads: Rc<RefCell<Vec<std::path::PathBuf>>>,
    sprite_texture_rgba8_loads: Rc<Cell<usize>>,
    tilemap_render_enabled: Rc<Cell<bool>>,
    tilemap_vertex_counts: Rc<RefCell<Vec<usize>>>,
    sprite_count: Rc<Cell<usize>>,
    text_count: Rc<Cell<usize>>,
    debug_rect_count: Rc<Cell<usize>>,
    ui_rect_count: Rc<Cell<usize>>,
    finalized_debug: Rc<Cell<usize>>,
    finalized_ui: Rc<Cell<usize>>,
}

impl RuntimeRenderBackend for FakeBackend {
    fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.tilemap_texture_loads.borrow_mut().push(texture_path);
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

    fn update_projection(&mut self, _mvp: glam::Mat4) {
        self.projection_updates
            .set(self.projection_updates.get() + 1);
    }

    fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        self.tilemap_render_enabled.set(enabled);
    }

    fn resize(&mut self, _new_size: winit::dpi::PhysicalSize<u32>) {
        self.resize_calls.set(self.resize_calls.get() + 1);
    }

    fn draw(&mut self) {
        self.draw_calls.set(self.draw_calls.get() + 1);
    }

    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        self.tilemap_vertex_counts.borrow_mut().push(vertices.len());
    }

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

    fn clear_text_items(&mut self) {
        self.text_count.set(0);
    }

    fn add_text_item(&mut self, _text: TextItem) {
        self.text_count.set(self.text_count.get() + 1);
    }

    fn clear_debug_shapes(&mut self) {
        self.debug_rect_count.set(0);
    }

    fn add_debug_rect(&mut self, _x: f32, _y: f32, _width: f32, _height: f32, _color: [f32; 4]) {
        self.debug_rect_count.set(self.debug_rect_count.get() + 1);
    }

    fn add_filled_debug_rect(
        &mut self,
        _x: f32,
        _y: f32,
        _width: f32,
        _height: f32,
        _color: [f32; 4],
    ) {
        self.debug_rect_count.set(self.debug_rect_count.get() + 1);
    }

    fn finalize_debug_shapes(&mut self) {
        self.finalized_debug.set(self.finalized_debug.get() + 1);
    }

    fn clear_ui_shapes(&mut self) {
        self.ui_rect_count.set(0);
    }

    fn add_ui_rect(&mut self, _x: f32, _y: f32, _width: f32, _height: f32, _color: [f32; 4]) {
        self.ui_rect_count.set(self.ui_rect_count.get() + 1);
    }

    fn add_filled_ui_rect(
        &mut self,
        _x: f32,
        _y: f32,
        _width: f32,
        _height: f32,
        _color: [f32; 4],
    ) {
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
    rendering.clear_debug_shapes();
    rendering.add_debug_rect(0.0, 0.0, 16.0, 16.0, [1.0, 0.0, 0.0, 1.0]);
    rendering.add_filled_debug_rect(1.0, 1.0, 14.0, 14.0, [0.0, 1.0, 0.0, 1.0]);
    rendering.finalize_debug_shapes();
    rendering.clear_ui_shapes();
    rendering.add_ui_rect(4.0, 4.0, 12.0, 12.0, [1.0, 1.0, 1.0, 1.0]);
    rendering.add_filled_ui_rect(5.0, 5.0, 10.0, 10.0, [0.0, 0.0, 0.0, 0.5]);
    rendering.finalize_ui_shapes();
    rendering.draw();

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
    assert_eq!(debug_rect_count.get(), 2);
    assert_eq!(debug_finalize_counter.get(), 1);
    assert_eq!(ui_rect_count.get(), 2);
    assert_eq!(ui_finalize_counter.get(), 1);
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
