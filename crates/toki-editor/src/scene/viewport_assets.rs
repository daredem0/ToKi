use super::*;
use toki_core::graphics::image::{load_image_rgba8, DecodedImage};
use toki_core::palette::{recolor_indexed_image, resolve_palette, Palette4};
use toki_core::project_assets::normalize_asset_name;
use toki_core::sprite_render::{
    resolve_atlas_tile_frame, resolve_object_sheet_frame, resolve_sprite_render_requests,
    ResolvedSpriteVisual, SpriteAssetResolver, SpriteRenderMaterial, SpriteRenderRequest,
    SpriteResolveError, SpriteResolveFailure,
};

struct ViewportSpriteResolver<'a, 'b> {
    viewport: &'a mut SceneViewport,
    project_assets: &'b ProjectAssets,
    project_path: Option<&'b std::path::Path>,
}

impl SceneViewport {
    fn resolve_indexed_palette(
        &self,
        atlas: &AtlasMeta,
        palette_override: Option<&str>,
    ) -> Result<(String, Palette4), SpriteResolveError> {
        for palette_id in [
            self.indexed_palette_override.as_deref(),
            palette_override,
            atlas.palette.as_deref(),
            Some("gb_default"),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(palette) = resolve_palette(palette_id, &self.available_palettes) {
                return Ok((palette_id.to_string(), palette));
            }
        }

        Err(SpriteResolveError::AssetLoadFailed {
            asset_kind: "palette",
            asset_name: self
                .indexed_palette_override
                .as_deref()
                .or(palette_override)
                .or(atlas.palette.as_deref())
                .unwrap_or("gb_default")
                .to_string(),
            message: "palette id could not be resolved".to_string(),
        })
    }

    fn decoded_sprite_image(&mut self, texture_path: &std::path::Path) -> Result<DecodedImage> {
        if let Some(image) = self.decoded_sprite_images.get(texture_path) {
            return Ok(image.clone());
        }

        let image = load_image_rgba8(texture_path)?;
        self.decoded_sprite_images
            .insert(texture_path.to_path_buf(), image.clone());
        Ok(image)
    }

    fn recolored_sprite_image(
        &mut self,
        texture_path: &std::path::Path,
        cache_key: &str,
        palette: Palette4,
    ) -> Result<DecodedImage, SpriteResolveError> {
        if let Some(image) = self.recolored_sprite_images.get(cache_key) {
            return Ok(image.clone());
        }

        let decoded = self
            .decoded_sprite_image(texture_path)
            .map_err(|error| SpriteResolveError::AssetLoadFailed {
                asset_kind: "sprite_texture",
                asset_name: texture_path.display().to_string(),
                message: error.to_string(),
            })?;
        let recolored =
            recolor_indexed_image(&decoded, palette).map_err(|error| SpriteResolveError::AssetLoadFailed {
                asset_kind: "sprite_texture",
                asset_name: texture_path.display().to_string(),
                message: error.to_string(),
            })?;
        self.recolored_sprite_images
            .insert(cache_key.to_string(), recolored.clone());
        Ok(recolored)
    }

    pub(super) fn load_atlas_for_tilemap(
        &mut self,
        atlas_name: &str,
        project_path: &std::path::Path,
    ) -> Result<AtlasMeta> {
        if let Some(cached_atlas) = &self.atlas_cache {
            return Ok(cached_atlas.clone());
        }

        let atlas_path = {
            let tilemaps_path = project_path
                .join("assets")
                .join("tilemaps")
                .join(atlas_name);
            if tilemaps_path.exists() {
                tilemaps_path
            } else {
                project_path.join("assets").join("sprites").join(atlas_name)
            }
        };

        let atlas = AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
            anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
        })?;

        tracing::trace!("Atlas image field contains: {:?}", atlas.image);
        if let Some(scene_renderer) = &mut self.scene_renderer {
            tracing::trace!("Scene renderer available, proceeding with texture load");
            let texture_path = atlas_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(&atlas.image);

            if texture_path.exists() {
                tracing::info!("Loading tilemap texture: {}", texture_path.display());
                scene_renderer
                    .load_tilemap_texture(texture_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load tilemap texture: {}", e))?;
                tracing::info!("Successfully loaded tilemap texture");
            } else {
                tracing::warn!("Tilemap texture not found: {}", texture_path.display());
            }
        }

        self.atlas_cache = Some(atlas.clone());
        tracing::info!("Loaded and cached atlas: {}", atlas_path.display());

        Ok(atlas)
    }

    pub(super) fn load_sprite_atlas_from_asset(
        &mut self,
        atlas_asset: &SpriteAtlasAsset,
        _project_path: Option<&std::path::Path>,
    ) -> Result<AtlasMeta> {
        let atlas_path = &atlas_asset.path;
        let atlas_key = atlas_path.to_string_lossy().to_string();

        if let Some(cached_atlas) = self.loaded_sprite_atlases.get(&atlas_key) {
            tracing::trace!("Using cached sprite atlas for: {}", atlas_path.display());
            return Ok(cached_atlas.clone());
        }

        tracing::info!("Loading sprite atlas from file: {}", atlas_path.display());

        let atlas = AtlasMeta::load_from_file(atlas_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load sprite atlas from '{}': {}",
                atlas_path.display(),
                e
            )
        })?;

        tracing::trace!(
            "Successfully loaded atlas metadata with {} tiles",
            atlas.tiles.len()
        );

        tracing::trace!("Sprite atlas image field contains: {:?}", atlas.image);
        if let Some(scene_renderer) = &mut self.scene_renderer {
            tracing::debug!("Scene renderer available, proceeding with sprite texture load");
            let texture_path = atlas_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(&atlas.image);

            tracing::trace!("Constructed texture path: {}", texture_path.display());

            if texture_path.exists() {
                tracing::info!("Loading sprite texture: {}", texture_path.display());
                scene_renderer
                    .load_sprite_texture(texture_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load sprite texture: {}", e))?;
                tracing::info!("Successfully loaded sprite texture from ProjectAssets");
            } else {
                tracing::error!("Sprite texture file not found: {}", texture_path.display());
                tracing::trace!("Atlas path parent: {:?}", atlas_path.parent());
                tracing::trace!("Atlas image field: {:?}", atlas.image);
            }
        } else {
            tracing::error!("Scene renderer not available - cannot load sprite texture");
        }

        self.loaded_sprite_atlases.insert(atlas_key, atlas.clone());
        tracing::trace!("Cached sprite atlas: {}", atlas_path.display());

        Ok(atlas)
    }

    pub(super) fn resolve_sprite_requests_into_instances(
        &mut self,
        project_assets: &ProjectAssets,
        project_path: Option<&std::path::Path>,
        requests: &[SpriteRenderRequest],
    ) -> (Vec<toki_render::SpriteInstance>, Vec<SpriteResolveFailure>) {
        let mut resolver = ViewportSpriteResolver {
            viewport: self,
            project_assets,
            project_path,
        };
        let (resolved, mut failures) = resolve_sprite_render_requests(&mut resolver, requests);
        let mut instances = Vec::with_capacity(resolved.len());
        for sprite in resolved {
            let (texture_path, texture_image, texture_cache_key) = match sprite.material {
                SpriteRenderMaterial::TrueColor => (sprite.texture_path, None, None),
                SpriteRenderMaterial::PaletteIndexed {
                    ref palette_id,
                    palette,
                } => {
                    let Some(texture_path) = sprite.texture_path.clone() else {
                        instances.push(toki_render::SpriteInstance {
                            frame: sprite.frame,
                            position: sprite.position,
                            size: sprite.size,
                            texture_path: None,
                            texture_image: None,
                            texture_cache_key: None,
                            flip_x: sprite.flip_x,
                        });
                        continue;
                    };

                    let cache_key = format!("{}#palette={}", texture_path.display(), palette_id);
                    match self.recolored_sprite_image(&texture_path, &cache_key, palette) {
                        Ok(image) => (Some(texture_path), Some(image), Some(cache_key)),
                        Err(error) => {
                            failures.push(SpriteResolveFailure {
                                origin: sprite.origin,
                                error,
                            });
                            continue;
                        }
                    }
                }
            };

            instances.push(toki_render::SpriteInstance {
                frame: sprite.frame,
                position: sprite.position,
                size: sprite.size,
                texture_path,
                texture_image,
                texture_cache_key,
                flip_x: sprite.flip_x,
            });
        }
        (instances, failures)
    }

    pub(super) fn load_object_sheet_from_asset(
        &mut self,
        object_sheet_asset: &ObjectSheetAsset,
    ) -> Result<ObjectSheetMeta> {
        let object_sheet_path = &object_sheet_asset.path;
        let object_sheet_key = object_sheet_path.to_string_lossy().to_string();

        if let Some(cached_object_sheet) = self.loaded_object_sheets.get(&object_sheet_key) {
            return Ok(cached_object_sheet.clone());
        }

        let object_sheet = ObjectSheetMeta::load_from_file(object_sheet_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load object sheet from '{}': {}",
                object_sheet_path.display(),
                e
            )
        })?;

        self.loaded_object_sheets
            .insert(object_sheet_key, object_sheet.clone());
        Ok(object_sheet)
    }
}

impl SpriteAssetResolver for ViewportSpriteResolver<'_, '_> {
    fn resolve_atlas_tile(
        &mut self,
        atlas_name: &str,
        tile_name: &str,
        palette_override: Option<&str>,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        let atlas_name_clean = normalize_asset_name(atlas_name);
        let atlas_asset = self
            .project_assets
            .sprite_atlases
            .get(atlas_name_clean)
            .ok_or_else(|| SpriteResolveError::MissingAtlas {
                atlas_name: atlas_name.to_string(),
            })?;
        let atlas = self
            .viewport
            .load_sprite_atlas_from_asset(atlas_asset, self.project_path)
            .map_err(|error| SpriteResolveError::AssetLoadFailed {
                asset_kind: "sprite_atlas",
                asset_name: atlas_name.to_string(),
                message: error.to_string(),
            })?;
        let (frame, intrinsic_size) = resolve_atlas_tile_frame(&atlas, atlas_name, tile_name)?;
        let material = if atlas.is_palette_indexed() {
            let (palette_id, palette) = self
                .viewport
                .resolve_indexed_palette(&atlas, palette_override)?;
            SpriteRenderMaterial::PaletteIndexed {
                palette_id,
                palette,
            }
        } else {
            SpriteRenderMaterial::TrueColor
        };

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: atlas_asset
                .path
                .parent()
                .map(|parent| parent.join(&atlas.image)),
            material,
        })
    }

    fn resolve_object_sheet_object(
        &mut self,
        sheet_name: &str,
        object_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        let sheet_name_clean = normalize_asset_name(sheet_name);
        let object_sheet_asset = self
            .project_assets
            .object_sheets
            .get(sheet_name_clean)
            .ok_or_else(|| SpriteResolveError::MissingObjectSheet {
                sheet_name: sheet_name.to_string(),
            })?;
        let object_sheet = self
            .viewport
            .load_object_sheet_from_asset(object_sheet_asset)
            .map_err(|error| SpriteResolveError::AssetLoadFailed {
                asset_kind: "object_sheet",
                asset_name: sheet_name.to_string(),
                message: error.to_string(),
            })?;
        let (frame, intrinsic_size) =
            resolve_object_sheet_frame(&object_sheet, sheet_name, object_name)?;

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: object_sheet_asset
                .path
                .parent()
                .map(|parent| parent.join(&object_sheet.image)),
            material: SpriteRenderMaterial::TrueColor,
        })
    }
}
