use toki_core::math::projection::screen_space_projection;
use toki_core::project_runtime::PostProcessMode;

use crate::targets::OffscreenTarget;

use super::*;

impl GpuState {
    pub fn draw(&mut self) -> Result<(), crate::RenderError> {
        self.update_render_resources();
        let text_backgrounds = self.prepare_text_backgrounds()?;
        self.refresh_ui_text_backgrounds(&text_backgrounds);

        let Some(output) = self.acquire_surface_texture() else {
            return Ok(());
        };
        self.render_frame_to_surface(&output)?;
        output.present();
        Ok(())
    }

    fn update_render_resources(&mut self) {
        self.tilemap_pipeline
            .update_with_queue(&self.device, &self.queue);
        self.overlay_tilemap_pipeline
            .update_with_queue(&self.device, &self.queue);
        for pipeline in self.tilemap_pipelines_by_texture.values_mut() {
            pipeline.update_with_queue(&self.device, &self.queue);
        }
        self.sprite_pipeline
            .update_with_queue(&self.device, &self.queue);
        for pipeline in self.sprite_pipelines_by_texture.values_mut() {
            pipeline.update_with_queue(&self.device, &self.queue);
        }
    }

    fn prepare_text_backgrounds(&mut self) -> Result<Vec<TextBackgroundRect>, crate::RenderError> {
        self.text_renderer.prepare(
            &self.device,
            &self.queue,
            self.config.width,
            self.config.height,
            &self.text_items,
            self.current_mvp,
        )
    }

    fn acquire_surface_texture(&mut self) -> Option<wgpu::SurfaceTexture> {
        match self.surface.get_current_texture() {
            Ok(output) => Some(output),
            Err(error) => {
                self.handle_surface_error(error);
                None
            }
        }
    }

    fn handle_surface_error(&mut self, error: wgpu::SurfaceError) {
        tracing::warn!("Failed to acquire next swap chain texture: {error}");
        match error {
            wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost => {
                self.surface.configure(&self.device, &self.config);
            }
            wgpu::SurfaceError::OutOfMemory => {
                tracing::error!("Surface out of memory; skipping frame");
            }
            wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {}
        }
    }

    fn render_frame_to_surface(
        &mut self,
        output: &wgpu::SurfaceTexture,
    ) -> Result<(), crate::RenderError> {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        if self.post_process_settings.mode == PostProcessMode::None {
            self.render_direct_frame(&mut encoder, &view)?;
        } else {
            self.render_post_processed_frame(&mut encoder, &view)?;
        }

        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    fn render_direct_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), crate::RenderError> {
        self.render_scene_to_view(encoder, view)
    }

    fn render_post_processed_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) -> Result<(), crate::RenderError> {
        if let Err(error) = self.ensure_post_process_target() {
            tracing::warn!("Failed to prepare post-process target: {error}");
            return self.render_scene_to_view(encoder, output_view);
        }

        let Some(target) = &mut self.post_process_target else {
            return self.render_scene_to_view(encoder, output_view);
        };

        self.post_process_pipeline
            .update_settings(&self.queue, &self.post_process_settings);
        match target.get_render_view() {
            Ok(target_view) => {
                let target_view = target_view.clone();
                self.render_scene_to_view(encoder, &target_view)?;
                let mut post_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Post Process Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: output_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                self.post_process_pipeline.render(&mut post_pass);
                Ok(())
            }
            Err(error) => {
                tracing::warn!("Failed to access post-process target view: {error}");
                self.render_scene_to_view(encoder, output_view)
            }
        }
    }

    fn ensure_post_process_target(&mut self) -> Result<(), crate::RenderError> {
        let size = (self.config.width.max(1), self.config.height.max(1));
        let target = self.post_process_target.get_or_insert(OffscreenTarget::new(
            &self.device,
            size,
            self.config.format,
        )?);
        target.resize(&self.device, size)?;
        self.post_process_pipeline
            .update_source_texture(&self.device, target.get_render_view()?);
        Ok(())
    }

    fn render_scene_to_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<(), crate::RenderError> {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_viewport(
            0.0,
            0.0,
            self.config.width as f32,
            self.config.height as f32,
            0.0,
            1.0,
        );
        if let Some(rect) = self.scene_clip_rect {
            render_pass.set_scissor_rect(rect.x, rect.y, rect.width, rect.height);
        }

        if self.tilemap_render_enabled {
            if self.tilemap_batches_below.is_empty() {
                self.tilemap_pipeline.render(&mut render_pass);
            } else {
                for texture_key in &self.tilemap_batches_below {
                    if let Some(pipeline) = self.tilemap_pipelines_by_texture.get(texture_key) {
                        pipeline.render(&mut render_pass);
                    }
                }
            }
        }
        self.world_underlay_pipeline.render(&mut render_pass);

        for batch in &self.sprite_draw_batches {
            match &batch.key {
                GpuSpriteBatchKey::Default => {
                    self.sprite_pipeline
                        .render_range(&mut render_pass, batch.start, batch.count);
                }
                GpuSpriteBatchKey::Textured(texture_path) => {
                    if let Some(pipeline) = self.sprite_pipelines_by_texture.get(texture_path) {
                        pipeline.render_range(&mut render_pass, batch.start, batch.count);
                    }
                }
            }
        }

        if self.tilemap_render_enabled {
            if self.tilemap_batches_above.is_empty() {
                self.overlay_tilemap_pipeline.render(&mut render_pass);
            } else {
                for texture_key in &self.tilemap_batches_above {
                    if let Some(pipeline) = self.tilemap_pipelines_by_texture.get(texture_key) {
                        pipeline.render(&mut render_pass);
                    }
                }
            }
        }
        self.debug_pipeline.render(&mut render_pass);
        self.ui_shape_pipeline.render(&mut render_pass);
        self.ui_debug_pipeline.render(&mut render_pass);
        self.text_renderer.render(&mut render_pass)?;
        Ok(())
    }

    fn refresh_ui_text_backgrounds(&mut self, backgrounds: &[TextBackgroundRect]) {
        self.ui_debug_pipeline.clear();
        for rect in backgrounds {
            self.ui_debug_pipeline.add_filled_rect(
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                rect.background_color,
            );
            if let Some(border_color) = rect.border_color {
                self.ui_debug_pipeline.add_rect(
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    border_color,
                );
            }
        }
        self.ui_debug_pipeline.update_camera(
            &self.queue,
            screen_space_projection(self.config.width as f32, self.config.height as f32),
        );
        self.ui_debug_pipeline.update_vertices(&self.device);
    }
}
