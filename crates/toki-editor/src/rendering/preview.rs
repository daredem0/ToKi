use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::Path;

use toki_core::assets::atlas::ColorMode;
use toki_core::graphics::image::DecodedImage;
use toki_core::indexed_presentation::{
    load_materialized_indexed_image, materialize_indexed_image, resolve_indexed_palette,
    texture_preview_cache_key, IndexedImageMaterialization, IndexedPresentationSettings,
};
use toki_core::palette::Palette;
use toki_core::project_runtime::ResolvedPostProcessSettings;
use toki_core::sprite::SpriteFrame;
use toki_render::{
    OffscreenTarget, PresentationBlitPipeline, RenderTarget, SceneData, SceneRenderer,
    SpriteInstance,
};

const PREVIEW_SCENE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const PREVIEW_PRESENTATION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorPreviewTexture {
    pub texture_id: egui::TextureId,
    pub size: glam::UVec2,
}

pub struct PresentedOffscreenTexture {
    scene_target: OffscreenTarget,
    presentation_target: OffscreenTarget,
    presentation_pipeline: PresentationBlitPipeline,
    texture_id: Option<egui::TextureId>,
}

impl PresentedOffscreenTexture {
    pub fn new(device: &wgpu::Device, size: (u32, u32)) -> Result<Self, toki_render::RenderError> {
        Ok(Self {
            scene_target: OffscreenTarget::new(device, size, PREVIEW_SCENE_FORMAT)?,
            presentation_target: OffscreenTarget::new(device, size, PREVIEW_PRESENTATION_FORMAT)?,
            presentation_pipeline: PresentationBlitPipeline::new(
                device,
                PREVIEW_PRESENTATION_FORMAT,
            ),
            texture_id: None,
        })
    }

    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        size: (u32, u32),
    ) -> Result<(), toki_render::RenderError> {
        self.scene_target.resize(device, size)?;
        self.presentation_target.resize(device, size)?;
        self.texture_id = None;
        Ok(())
    }

    pub fn scene_target_mut(&mut self) -> &mut OffscreenTarget {
        &mut self.scene_target
    }

    pub fn present_to_egui(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut egui_wgpu::Renderer,
    ) -> Result<egui::TextureId, toki_render::RenderError> {
        let texture_id = match self.texture_id {
            Some(texture_id) => texture_id,
            None => {
                let texture_id = self
                    .presentation_target
                    .register_with_egui(device, egui_renderer);
                self.texture_id = Some(texture_id);
                texture_id
            }
        };

        {
            let source_view = self.scene_target.get_render_view()?;
            self.presentation_pipeline
                .update_source_texture(device, source_view);
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Editor Preview Presentation Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Editor Preview Presentation Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.presentation_target.get_render_view()?,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.presentation_pipeline.render(&mut render_pass);
        }
        queue.submit(std::iter::once(encoder.finish()));
        Ok(texture_id)
    }

    pub fn texture_id(&self) -> Option<egui::TextureId> {
        self.texture_id
    }
}

struct PreviewTextureEntry {
    target: PresentedOffscreenTexture,
    size: glam::UVec2,
}

pub struct EditorPreviewRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    scene_renderer: SceneRenderer,
    textures: HashMap<String, PreviewTextureEntry>,
}

impl EditorPreviewRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, toki_render::RenderError> {
        let mut scene_renderer = SceneRenderer::new(
            device.clone(),
            queue.clone(),
            PREVIEW_SCENE_FORMAT,
            None,
            None,
        )?;
        scene_renderer.set_clear_color(wgpu::Color::TRANSPARENT);
        Ok(Self {
            device: device.clone(),
            queue: queue.clone(),
            scene_renderer,
            textures: HashMap::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn texture_preview_from_path(
        &mut self,
        egui_renderer: &mut egui_wgpu::Renderer,
        texture_path: &Path,
        color_mode: ColorMode,
        available_palettes: &BTreeMap<String, Palette>,
        settings: &IndexedPresentationSettings,
        local_override: Option<&str>,
        asset_palette: Option<&str>,
    ) -> Result<EditorPreviewTexture, String> {
        let resolved_palette_id = resolve_indexed_palette(
            color_mode,
            available_palettes,
            settings,
            local_override,
            asset_palette,
        )?
        .map(|(palette_id, _)| palette_id);
        let cache_key = texture_preview_cache_key(
            &texture_path.display().to_string(),
            color_mode,
            resolved_palette_id.as_deref(),
            &settings.resolve_post_process(available_palettes),
        );
        if let Some(texture) = self.cached_preview(&cache_key) {
            return Ok(texture);
        }

        let materialized = load_materialized_indexed_image(
            texture_path,
            color_mode,
            available_palettes,
            settings,
            local_override,
            asset_palette,
            false,
        )?;
        self.render_full_image_preview(
            egui_renderer,
            cache_key,
            materialized.image,
            settings.resolve_post_process(available_palettes),
        )
        .map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn texture_preview_from_image(
        &mut self,
        egui_renderer: &mut egui_wgpu::Renderer,
        cache_source: &str,
        decoded: &DecodedImage,
        color_mode: ColorMode,
        available_palettes: &BTreeMap<String, Palette>,
        settings: &IndexedPresentationSettings,
        local_override: Option<&str>,
        asset_palette: Option<&str>,
    ) -> Result<EditorPreviewTexture, String> {
        let resolved_palette_id = resolve_indexed_palette(
            color_mode,
            available_palettes,
            settings,
            local_override,
            asset_palette,
        )?
        .map(|(palette_id, _)| palette_id);
        let source_key = format!("{cache_source}:{}", image_content_hash(decoded));
        let cache_key = texture_preview_cache_key(
            &source_key,
            color_mode,
            resolved_palette_id.as_deref(),
            &settings.resolve_post_process(available_palettes),
        );
        if let Some(texture) = self.cached_preview(&cache_key) {
            return Ok(texture);
        }

        let materialized = materialize_indexed_image(
            decoded,
            &source_key,
            color_mode,
            available_palettes,
            settings,
            IndexedImageMaterialization {
                local_override,
                asset_palette,
                apply_post_process: false,
            },
        )?;
        self.render_full_image_preview(
            egui_renderer,
            cache_key,
            materialized.image,
            settings.resolve_post_process(available_palettes),
        )
        .map_err(|error| error.to_string())
    }

    fn cached_preview(&self, cache_key: &str) -> Option<EditorPreviewTexture> {
        let entry = self.textures.get(cache_key)?;
        let texture_id = entry.target.texture_id?;
        Some(EditorPreviewTexture {
            texture_id,
            size: entry.size,
        })
    }

    fn render_full_image_preview(
        &mut self,
        egui_renderer: &mut egui_wgpu::Renderer,
        cache_key: String,
        image: DecodedImage,
        post_process: ResolvedPostProcessSettings,
    ) -> Result<EditorPreviewTexture, toki_render::RenderError> {
        let size = glam::UVec2::new(image.width.max(1), image.height.max(1));
        let entry = self.textures.entry(cache_key.clone()).or_insert_with(|| {
            let target = PresentedOffscreenTexture::new(&self.device, (size.x, size.y))
                .expect("preview target should initialize");
            PreviewTextureEntry { target, size }
        });
        if entry.size != size {
            entry.target.resize(&self.device, (size.x, size.y))?;
            entry.size = size;
        }

        self.scene_renderer.set_post_process_settings(post_process);
        let mut scene_data = SceneData::default();
        scene_data.sprites.push(SpriteInstance {
            frame: SpriteFrame {
                u0: 0.0,
                v0: 0.0,
                u1: 1.0,
                v1: 1.0,
            },
            position: glam::IVec2::ZERO,
            size,
            texture_path: None,
            texture_image: Some(image),
            texture_cache_key: Some(format!("{cache_key}#source")),
            flip_x: false,
        });

        let projection =
            glam::Mat4::orthographic_rh_gl(0.0, size.x as f32, size.y as f32, 0.0, -1.0, 1.0);
        self.scene_renderer.render_scene_with_projection(
            entry.target.scene_target_mut(),
            &scene_data,
            projection,
        )?;
        let texture_id = entry
            .target
            .present_to_egui(&self.device, &self.queue, egui_renderer)?;
        Ok(EditorPreviewTexture { texture_id, size })
    }
}

fn image_content_hash(image: &DecodedImage) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    image.width.hash(&mut hasher);
    image.height.hash(&mut hasher);
    image.data.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_backend_uses_runtime_scene_format_and_egui_safe_presentation_format() {
        assert_eq!(PREVIEW_SCENE_FORMAT, wgpu::TextureFormat::Rgba8UnormSrgb);
        assert_eq!(PREVIEW_PRESENTATION_FORMAT, wgpu::TextureFormat::Rgba8Unorm);
    }
}
