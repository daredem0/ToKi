use std::sync::Arc;
use toki_core::graphics::image::DecodedImage;
use toki_core::graphics::vertex::QuadVertex;
use toki_core::math::projection::{calculate_projection, ProjectionParameter};
use toki_core::sprite::SpriteFrame;
use toki_core::text::TextItem;
use toki_render::GpuState;
use winit::window::Window;

trait RuntimeRenderBackend: std::fmt::Debug {
    fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError>;
    fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError>;
    fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError>;
    fn load_font_file(
        &mut self,
        font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError>;
    fn update_projection(&mut self, mvp: glam::Mat4);
    fn set_tilemap_render_enabled(&mut self, enabled: bool);
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>);
    fn draw(&mut self);
    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]);
    fn clear_sprites(&mut self);
    fn add_sprite(&mut self, frame: SpriteFrame, position: glam::IVec2, size: glam::UVec2);
    fn clear_text_items(&mut self);
    fn add_text_item(&mut self, text: TextItem);
    fn clear_debug_shapes(&mut self);
    fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]);
    fn finalize_debug_shapes(&mut self);
}

#[derive(Debug)]
struct WgpuRenderBackend {
    gpu: GpuState,
}

impl WgpuRenderBackend {
    fn new(window: Arc<Window>) -> Self {
        Self {
            gpu: GpuState::new(window),
        }
    }

    fn new_with_textures(
        window: Arc<Window>,
        tilemap_texture: Option<std::path::PathBuf>,
        sprite_texture: Option<std::path::PathBuf>,
    ) -> Result<Self, toki_render::RenderError> {
        Ok(Self {
            gpu: GpuState::new_with_textures(window, tilemap_texture, sprite_texture)?,
        })
    }
}

impl RuntimeRenderBackend for WgpuRenderBackend {
    fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_tilemap_texture(texture_path)
    }

    fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_sprite_texture(texture_path)
    }

    fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_sprite_texture_rgba8(image)
    }

    fn load_font_file(
        &mut self,
        font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        self.gpu.load_font_file(&font_path)
    }

    fn update_projection(&mut self, mvp: glam::Mat4) {
        self.gpu.update_projection(mvp);
    }

    fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        self.gpu.set_tilemap_render_enabled(enabled);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.gpu.resize(new_size);
    }

    fn draw(&mut self) {
        self.gpu.draw();
    }

    fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        self.gpu.update_tilemap_vertices(vertices);
    }

    fn clear_sprites(&mut self) {
        self.gpu.clear_sprites();
    }

    fn add_sprite(&mut self, frame: SpriteFrame, position: glam::IVec2, size: glam::UVec2) {
        self.gpu.add_sprite(frame, position, size);
    }

    fn clear_text_items(&mut self) {
        self.gpu.clear_text_items();
    }

    fn add_text_item(&mut self, text: TextItem) {
        self.gpu.add_text_item(text);
    }

    fn clear_debug_shapes(&mut self) {
        self.gpu.clear_debug_shapes();
    }

    fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.gpu.add_debug_rect(x, y, width, height, color);
    }

    fn finalize_debug_shapes(&mut self) {
        self.gpu.finalize_debug_shapes();
    }
}

/// Rendering system that manages GPU state and projection calculations.
///
/// Centralizes all rendering-related state and provides clean APIs for
/// graphics operations while abstracting GPU implementation details.
#[derive(Debug)]
pub struct RenderingSystem {
    backend: Option<Box<dyn RuntimeRenderBackend>>,
    projection_params: ProjectionParameter,
    loaded_tilemap_texture_path: Option<std::path::PathBuf>,
    loaded_sprite_texture_path: Option<std::path::PathBuf>,
}

impl Default for RenderingSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderingSystem {
    /// Create a new RenderingSystem with default projection parameters
    pub fn new() -> Self {
        Self {
            backend: None,
            projection_params: ProjectionParameter {
                width: 160,
                height: 144,
                desired_width: 160,
                desired_height: 144,
            },
            loaded_tilemap_texture_path: None,
            loaded_sprite_texture_path: None,
        }
    }

    /// Create a new RenderingSystem with custom projection parameters (for editor)
    pub fn new_with_projection(projection_params: ProjectionParameter) -> Self {
        Self {
            backend: None,
            projection_params,
            loaded_tilemap_texture_path: None,
            loaded_sprite_texture_path: None,
        }
    }

    /// Set new projection parameters at runtime
    pub fn set_projection_params(&mut self, params: ProjectionParameter) {
        self.projection_params = params;
    }

    /// Update desired resolution (useful for editor viewport scaling)
    pub fn set_desired_resolution(&mut self, width: u32, height: u32) {
        self.projection_params.desired_width = width;
        self.projection_params.desired_height = height;
    }

    /// Initialize GPU state with the given window (uses default textures)
    pub fn initialize_gpu(&mut self, window: Arc<Window>) {
        let backend = WgpuRenderBackend::new(window);
        self.backend = Some(Box::new(backend));
    }

    /// Initialize GPU state with custom textures (for editor use)
    pub fn initialize_gpu_with_textures(
        &mut self,
        window: Arc<Window>,
        tilemap_texture: Option<std::path::PathBuf>,
        sprite_texture: Option<std::path::PathBuf>,
    ) -> Result<(), toki_render::RenderError> {
        let backend =
            WgpuRenderBackend::new_with_textures(window, tilemap_texture, sprite_texture)?;
        self.loaded_tilemap_texture_path = None;
        self.loaded_sprite_texture_path = None;
        self.backend = Some(Box::new(backend));
        Ok(())
    }

    /// Load new tilemap texture at runtime
    pub fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if self.loaded_tilemap_texture_path.as_ref() == Some(&texture_path) {
            return Ok(());
        }
        if let Some(backend) = &mut self.backend {
            backend.load_tilemap_texture(texture_path.clone())?;
            self.loaded_tilemap_texture_path = Some(texture_path);
            Ok(())
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    /// Load new sprite texture at runtime
    pub fn load_sprite_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if self.loaded_sprite_texture_path.as_ref() == Some(&texture_path) {
            return Ok(());
        }
        if let Some(backend) = &mut self.backend {
            backend.load_sprite_texture(texture_path.clone())?;
            self.loaded_sprite_texture_path = Some(texture_path);
            Ok(())
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    pub fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), toki_render::RenderError> {
        if let Some(backend) = &mut self.backend {
            backend.load_sprite_texture_rgba8(image)?;
            self.loaded_sprite_texture_path = None;
            Ok(())
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    /// Load font from a specific file path.
    pub fn load_font_file(
        &mut self,
        font_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if let Some(backend) = &mut self.backend {
            backend.load_font_file(font_path)
        } else {
            Err(toki_render::RenderError::Other(
                "GPU not initialized".to_string(),
            ))
        }
    }

    /// Helper to load textures from a project assets directory
    pub fn load_project_textures(
        &mut self,
        project_path: &std::path::Path,
    ) -> Result<(), toki_render::RenderError> {
        let assets_path = project_path.join("assets");
        let sprites_path = assets_path.join("sprites");
        let tilemaps_path = assets_path.join("tilemaps");

        // Look for common sprite atlas files (only .json format supported)
        if let Some(creatures_atlas) = find_atlas_file(&sprites_path, "creatures") {
            if let Some(creatures_image) = find_image_for_atlas(&creatures_atlas) {
                self.load_sprite_texture(creatures_image)?;
            }
        }

        // Look for common tilemap atlas files (only .json format supported)
        if let Some(terrain_atlas) = find_atlas_file(&tilemaps_path, "terrain")
            .or_else(|| find_atlas_file(&sprites_path, "terrain"))
        {
            if let Some(terrain_image) = find_image_for_atlas(&terrain_atlas) {
                self.load_tilemap_texture(terrain_image)?;
            }
        }

        let fonts_path = assets_path.join("fonts");
        for font_file in find_font_files(&fonts_path) {
            self.load_font_file(font_file)?;
        }

        Ok(())
    }

    /// Update projection parameters with new window size
    pub fn update_window_size(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.projection_params.width = size.width;
        self.projection_params.height = size.height;
    }

    /// Calculate current projection matrix
    pub fn calculate_projection(&self) -> glam::Mat4 {
        calculate_projection(self.projection_params)
    }

    /// Update GPU projection matrix with view transform
    pub fn update_projection(&mut self, view_matrix: glam::Mat4) {
        let projection = self.calculate_projection();
        if let Some(backend) = &mut self.backend {
            backend.update_projection(projection * view_matrix);
        }
    }

    /// Resize GPU render targets
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let Some(backend) = &mut self.backend {
            backend.resize(new_size);
        }
        self.update_window_size(new_size);
    }

    /// Draw the current frame
    pub fn draw(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.draw();
        }
    }

    /// Check if GPU is initialized
    pub fn has_gpu(&self) -> bool {
        self.backend.is_some()
    }

    pub fn set_tilemap_render_enabled(&mut self, enabled: bool) {
        if let Some(backend) = &mut self.backend {
            backend.set_tilemap_render_enabled(enabled);
        }
    }

    /// Get current projection parameters
    pub fn projection_params(&self) -> ProjectionParameter {
        self.projection_params
    }

    pub fn update_tilemap_vertices(&mut self, vertices: &[QuadVertex]) {
        if let Some(backend) = &mut self.backend {
            backend.update_tilemap_vertices(vertices);
        }
    }

    pub fn clear_sprites(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.clear_sprites();
        }
    }

    pub fn add_sprite(&mut self, frame: SpriteFrame, position: glam::IVec2, size: glam::UVec2) {
        if let Some(backend) = &mut self.backend {
            backend.add_sprite(frame, position, size);
        }
    }

    pub fn clear_text_items(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.clear_text_items();
        }
    }

    pub fn add_text_item(&mut self, text: TextItem) {
        if let Some(backend) = &mut self.backend {
            backend.add_text_item(text);
        }
    }

    pub fn clear_debug_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.clear_debug_shapes();
        }
    }

    pub fn add_debug_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        if let Some(backend) = &mut self.backend {
            backend.add_debug_rect(x, y, width, height, color);
        }
    }

    pub fn finalize_debug_shapes(&mut self) {
        if let Some(backend) = &mut self.backend {
            backend.finalize_debug_shapes();
        }
    }

    #[cfg(test)]
    fn set_backend_for_tests(&mut self, backend: Box<dyn RuntimeRenderBackend>) {
        self.backend = Some(backend);
    }
}

/// Helper function to find atlas files by name in a directory (only .json supported)
fn find_atlas_file(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    if !dir.exists() {
        return None;
    }

    // Look for .json atlas files (only supported format)
    let json_path = dir.join(format!("{}.json", name));
    if json_path.exists() {
        return Some(json_path);
    }

    None
}

/// Helper function to find the image file corresponding to a .json atlas
fn find_image_for_atlas(atlas_path: &std::path::Path) -> Option<std::path::PathBuf> {
    if let Some(dir) = atlas_path.parent() {
        if let Some(stem) = atlas_path.file_stem() {
            if let Some(name) = stem.to_str() {
                // Common image formats
                for ext in &["png", "jpg", "jpeg"] {
                    let image_path = dir.join(format!("{}.{}", name, ext));
                    if image_path.exists() {
                        return Some(image_path);
                    }
                }
            }
        }
    }
    None
}

fn find_font_files(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }

    let mut fonts: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
                .unwrap_or(false)
        })
        .collect();
    fonts.sort();
    fonts
}

#[cfg(test)]
mod tests {
    use super::{
        find_atlas_file, find_font_files, find_image_for_atlas, RenderingSystem,
        RuntimeRenderBackend,
    };
    use std::cell::{Cell, RefCell};
    use std::path::Path;
    use std::rc::Rc;
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
        finalized_debug: Rc<Cell<usize>>,
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

        fn add_sprite(&mut self, _frame: SpriteFrame, _position: glam::IVec2, _size: glam::UVec2) {
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

        fn add_debug_rect(
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

        let found_atlas = find_atlas_file(Path::new(&sprites_dir), "creatures")
            .expect("atlas path should be found");
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
        let debug_finalize_counter = fake.finalized_debug.clone();

        let mut rendering = RenderingSystem::new();
        rendering.set_backend_for_tests(Box::new(fake));
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
        );
        rendering.clear_text_items();
        rendering.add_text_item(TextItem::new_screen(
            "Runtime HUD",
            glam::Vec2::new(8.0, 8.0),
            TextStyle::default(),
        ));
        rendering.clear_debug_shapes();
        rendering.add_debug_rect(0.0, 0.0, 16.0, 16.0, [1.0, 0.0, 0.0, 1.0]);
        rendering.finalize_debug_shapes();
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
        assert_eq!(debug_finalize_counter.get(), 1);
    }

    #[test]
    fn texture_loads_are_cached_by_path() {
        let fake = FakeBackend::default();
        let tilemap_texture_loads = fake.tilemap_texture_loads.clone();
        let sprite_texture_loads = fake.sprite_texture_loads.clone();

        let mut rendering = RenderingSystem::new();
        rendering.set_backend_for_tests(Box::new(fake));

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
        rendering.set_backend_for_tests(Box::new(fake));

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
}
