use std::sync::Arc;
use toki_core::math::projection::{calculate_projection, ProjectionParameter};
use toki_render::GpuState;
use winit::window::Window;

/// Rendering system that manages GPU state and projection calculations.
///
/// Centralizes all rendering-related state and provides clean APIs for
/// graphics operations while abstracting GPU implementation details.
#[derive(Debug)]
pub struct RenderingSystem {
    gpu: Option<GpuState>,
    projection_params: ProjectionParameter,
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
            gpu: None,
            projection_params: ProjectionParameter {
                width: 160,
                height: 144,
                desired_width: 160,
                desired_height: 144,
            },
        }
    }

    /// Create a new RenderingSystem with custom projection parameters (for editor)
    pub fn new_with_projection(projection_params: ProjectionParameter) -> Self {
        Self {
            gpu: None,
            projection_params,
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
        let gpu = GpuState::new(window);
        self.gpu = Some(gpu);
    }

    /// Initialize GPU state with custom textures (for editor use)
    pub fn initialize_gpu_with_textures(
        &mut self,
        window: Arc<Window>,
        tilemap_texture: Option<std::path::PathBuf>,
        sprite_texture: Option<std::path::PathBuf>,
    ) -> Result<(), toki_render::RenderError> {
        let gpu = GpuState::new_with_textures(window, tilemap_texture, sprite_texture)?;
        self.gpu = Some(gpu);
        Ok(())
    }

    /// Load new tilemap texture at runtime
    pub fn load_tilemap_texture(
        &mut self,
        texture_path: std::path::PathBuf,
    ) -> Result<(), toki_render::RenderError> {
        if let Some(gpu) = &mut self.gpu {
            gpu.load_tilemap_texture(texture_path)
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
        if let Some(gpu) = &mut self.gpu {
            gpu.load_sprite_texture(texture_path)
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

        Ok(())
    }

    /// Get mutable reference to GPU state
    pub fn gpu_mut(&mut self) -> Option<&mut GpuState> {
        self.gpu.as_mut()
    }

    /// Get reference to GPU state
    pub fn gpu(&self) -> Option<&GpuState> {
        self.gpu.as_ref()
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
        if let Some(gpu) = &mut self.gpu {
            gpu.update_projection(projection * view_matrix);
        }
    }

    /// Resize GPU render targets
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if let Some(gpu) = &mut self.gpu {
            gpu.resize(new_size);
        }
        self.update_window_size(new_size);
    }

    /// Draw the current frame
    pub fn draw(&mut self) {
        if let Some(gpu) = &mut self.gpu {
            gpu.draw();
        }
    }

    /// Check if GPU is initialized
    pub fn has_gpu(&self) -> bool {
        self.gpu.is_some()
    }

    /// Get current projection parameters
    pub fn projection_params(&self) -> ProjectionParameter {
        self.projection_params
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

#[cfg(test)]
mod tests {
    use super::{find_atlas_file, find_image_for_atlas, RenderingSystem};
    use std::path::Path;

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
}
