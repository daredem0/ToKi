use super::*;
use crate::editor_sprite_preview::{
    load_texture_preview_image, resolve_indexed_preview_palette, texture_preview_cache_key,
};
use toki_core::cache_utils::clone_cached_or_load;
use toki_core::graphics::image::{load_image_rgba8, DecodedImage};
use toki_core::palette::{recolor_indexed_image, Palette};
use toki_core::project_assets::normalize_asset_name;
use toki_core::sprite_render::{
    resolve_atlas_tile_frame, resolve_object_sheet_frame, resolve_sprite_render_requests,
    sort_sprite_render_requests, ResolvedSpriteVisual, SpriteAssetResolver, SpriteRenderMaterial,
    SpriteRenderRequest, SpriteResolveError, SpriteResolveFailure,
};

struct ViewportSpriteResolver<'a, 'b> {
    viewport: &'a mut SceneViewport,
    project_assets: &'b ProjectAssets,
    project_path: Option<&'b std::path::Path>,
}

fn load_cached_string_keyed<T: Clone>(
    cached: Option<T>,
    key: &str,
    loader: impl FnOnce() -> Result<T>,
    store: impl FnOnce(String, T),
) -> Result<T> {
    clone_cached_or_load(cached, loader, |value| store(key.to_string(), value))
}

impl SceneViewport {
    fn ensure_tilemap_texture_loaded(
        &mut self,
        atlas: &AtlasMeta,
        atlas_path: &std::path::Path,
    ) -> Result<()> {
        let Some(scene_renderer) = &mut self.scene_renderer else {
            return Ok(());
        };

        let texture_path = atlas_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(&atlas.image);
        if !texture_path.exists() {
            tracing::warn!("Tilemap texture not found: {}", texture_path.display());
            return Ok(());
        }

        let resolved_palette = resolve_indexed_preview_palette(
            atlas.color_mode,
            &self.available_palettes,
            self.indexed_palette_override.as_deref(),
            None,
            atlas.palette.as_deref(),
        )
        .map_err(|error| anyhow::anyhow!(error))?
        .map(|(palette_id, _)| palette_id);

        let cache_key =
            texture_preview_cache_key(&texture_path, atlas.color_mode, resolved_palette.as_deref());
        if self.tilemap_texture_cache_key.as_deref() == Some(cache_key.as_str()) {
            return Ok(());
        }

        if atlas.is_palette_indexed() {
            let (image, _) = load_texture_preview_image(
                &texture_path,
                atlas.color_mode,
                &self.available_palettes,
                self.indexed_palette_override.as_deref(),
                None,
                atlas.palette.as_deref(),
            )
            .map_err(|error| anyhow::anyhow!(error))?;
            scene_renderer
                .load_tilemap_texture_rgba8(&image)
                .map_err(|e| anyhow::anyhow!("Failed to load recolored tilemap texture: {}", e))?;
        } else {
            scene_renderer
                .load_tilemap_texture(texture_path.clone())
                .map_err(|e| anyhow::anyhow!("Failed to load tilemap texture: {}", e))?;
        }
        self.tilemap_texture_cache_key = Some(cache_key);
        tracing::info!("Loaded tilemap texture: {}", texture_path.display());
        Ok(())
    }

    fn decoded_sprite_image(&mut self, texture_path: &std::path::Path) -> Result<DecodedImage> {
        Ok(clone_cached_or_load(
            self.decoded_sprite_images.get(texture_path).cloned(),
            || load_image_rgba8(texture_path),
            |image| {
                self.decoded_sprite_images
                    .insert(texture_path.to_path_buf(), image);
            },
        )?)
    }

    fn recolored_sprite_image(
        &mut self,
        texture_path: &std::path::Path,
        cache_key: &str,
        palette: &Palette,
    ) -> Result<DecodedImage, SpriteResolveError> {
        if let Some(image) = self.recolored_sprite_images.get(cache_key) {
            return Ok(image.clone());
        }

        let decoded = self.decoded_sprite_image(texture_path).map_err(|error| {
            SpriteResolveError::AssetLoadFailed {
                asset_kind: "sprite_texture",
                asset_name: texture_path.display().to_string(),
                message: error.to_string(),
            }
        })?;
        let recolored = recolor_indexed_image(&decoded, palette).map_err(|error| {
            SpriteResolveError::AssetLoadFailed {
                asset_kind: "sprite_texture",
                asset_name: texture_path.display().to_string(),
                message: error.to_string(),
            }
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

        let atlas = if self.atlas_cache_path.as_deref() == Some(atlas_path.as_path()) {
            if let Some(atlas) = self.atlas_cache.clone() {
                atlas
            } else {
                let atlas = AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
                    anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
                })?;
                self.atlas_cache = Some(atlas.clone());
                self.atlas_cache_path = Some(atlas_path.clone());
                atlas
            }
        } else {
            let atlas = AtlasMeta::load_from_file(&atlas_path).map_err(|e| {
                anyhow::anyhow!("Failed to load atlas '{}': {}", atlas_path.display(), e)
            })?;
            self.atlas_cache = Some(atlas.clone());
            self.atlas_cache_path = Some(atlas_path.clone());
            self.tilemap_texture_cache_key = None;
            atlas
        };

        tracing::trace!("Atlas image field contains: {:?}", atlas.image);
        self.ensure_tilemap_texture_loaded(&atlas, &atlas_path)?;
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

        load_cached_string_keyed(
            self.loaded_sprite_atlases.get(&atlas_key).cloned(),
            &atlas_key,
            || {
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
                    tracing::debug!(
                        "Scene renderer available, proceeding with sprite texture load"
                    );
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
                        tracing::error!(
                            "Sprite texture file not found: {}",
                            texture_path.display()
                        );
                        tracing::trace!("Atlas path parent: {:?}", atlas_path.parent());
                        tracing::trace!("Atlas image field: {:?}", atlas.image);
                    }
                } else {
                    tracing::error!("Scene renderer not available - cannot load sprite texture");
                }
                tracing::trace!("Cached sprite atlas: {}", atlas_path.display());
                Ok(atlas)
            },
            |key, atlas| {
                self.loaded_sprite_atlases.insert(key, atlas);
            },
        )
    }

    pub(super) fn resolve_sprite_requests_into_instances(
        &mut self,
        project_assets: &ProjectAssets,
        project_path: Option<&std::path::Path>,
        requests: &[SpriteRenderRequest],
    ) -> (Vec<toki_render::SpriteInstance>, Vec<SpriteResolveFailure>) {
        let mut sorted_requests = requests.to_vec();
        sort_sprite_render_requests(&mut sorted_requests);
        let mut resolver = ViewportSpriteResolver {
            viewport: self,
            project_assets,
            project_path,
        };
        let (resolved, mut failures) =
            resolve_sprite_render_requests(&mut resolver, &sorted_requests);
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
                    match self.recolored_sprite_image(&texture_path, &cache_key, &palette) {
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

        load_cached_string_keyed(
            self.loaded_object_sheets.get(&object_sheet_key).cloned(),
            &object_sheet_key,
            || {
                ObjectSheetMeta::load_from_file(object_sheet_path).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to load object sheet from '{}': {}",
                        object_sheet_path.display(),
                        e
                    )
                })
            },
            |key, object_sheet| {
                self.loaded_object_sheets.insert(key, object_sheet);
            },
        )
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
            let Some((palette_id, palette)) = resolve_indexed_preview_palette(
                atlas.color_mode,
                &self.viewport.available_palettes,
                self.viewport.indexed_palette_override.as_deref(),
                palette_override,
                atlas.palette.as_deref(),
            )
            .map_err(|message| SpriteResolveError::AssetLoadFailed {
                asset_kind: "palette",
                asset_name: self
                    .viewport
                    .indexed_palette_override
                    .as_deref()
                    .or(palette_override)
                    .or(atlas.palette.as_deref())
                    .unwrap_or("gb_default")
                    .to_string(),
                message,
            })?
            else {
                unreachable!("palette indexed atlas must resolve to a palette");
            };
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

        let material = if object_sheet.is_palette_indexed() {
            resolve_indexed_preview_palette(
                toki_core::assets::atlas::ColorMode::PaletteIndexed,
                &self.viewport.available_palettes,
                self.viewport.indexed_palette_override.as_deref(),
                None,
                object_sheet.palette.as_deref(),
            )
            .ok()
            .flatten()
            .map(
                |(palette_id, palette)| SpriteRenderMaterial::PaletteIndexed {
                    palette_id,
                    palette,
                },
            )
            .unwrap_or(SpriteRenderMaterial::TrueColor)
        } else {
            SpriteRenderMaterial::TrueColor
        };

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: object_sheet_asset
                .path
                .parent()
                .map(|parent| parent.join(&object_sheet.image)),
            material,
        })
    }
}
