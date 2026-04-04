use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TexturePipelineKind {
    Tilemap,
    Sprite,
}

struct TextureLoadConfig<'a> {
    kind: TexturePipelineKind,
    source: TextureSource<'a>,
}

impl GpuState {
    pub fn load_tilemap_texture(
        &mut self,
        texture_path: PathBuf,
    ) -> Result<(), crate::RenderError> {
        self.load_texture_pipeline(TextureLoadConfig {
            kind: TexturePipelineKind::Tilemap,
            source: TextureSource::path(texture_path.as_path()),
        })?;
        self.overlay_tilemap_pipeline = TilemapPipeline::new(
            &self.device,
            &self.queue,
            self.config.format,
            TextureSource::path(texture_path.as_path()),
        )?;
        Ok(())
    }

    pub fn load_tilemap_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        self.load_texture_pipeline(TextureLoadConfig {
            kind: TexturePipelineKind::Tilemap,
            source: TextureSource::rgba8(image),
        })?;
        self.overlay_tilemap_pipeline = TilemapPipeline::new(
            &self.device,
            &self.queue,
            self.config.format,
            TextureSource::rgba8(image),
        )?;
        Ok(())
    }

    pub fn load_sprite_texture(&mut self, texture_path: PathBuf) -> Result<(), crate::RenderError> {
        self.load_texture_pipeline(TextureLoadConfig {
            kind: TexturePipelineKind::Sprite,
            source: TextureSource::path(texture_path.as_path()),
        })
    }

    pub fn load_sprite_texture_rgba8(
        &mut self,
        image: &DecodedImage,
    ) -> Result<(), crate::RenderError> {
        self.load_texture_pipeline(TextureLoadConfig {
            kind: TexturePipelineKind::Sprite,
            source: TextureSource::rgba8(image),
        })
    }

    fn load_texture_pipeline(
        &mut self,
        config: TextureLoadConfig<'_>,
    ) -> Result<(), crate::RenderError> {
        match config.kind {
            TexturePipelineKind::Tilemap => {
                self.tilemap_pipeline = TilemapPipeline::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                    config.source,
                )?;
            }
            TexturePipelineKind::Sprite => {
                self.sprite_pipeline = SpritePipeline::new(
                    &self.device,
                    &self.queue,
                    self.config.format,
                    config.source,
                )?;
                self.sprite_pipelines_by_texture.clear();
            }
        }
        Ok(())
    }
}
