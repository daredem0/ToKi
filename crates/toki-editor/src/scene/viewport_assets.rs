use super::*;
use toki_core::sprite_render::{
    resolve_atlas_tile_frame, resolve_object_sheet_frame, resolve_sprite_render_requests,
    ResolvedSpriteVisual, SpriteAssetResolver, SpriteRenderRequest, SpriteResolveError,
    SpriteResolveFailure,
};

struct ViewportSpriteResolver<'a, 'b> {
    viewport: &'a mut SceneViewport,
    project_assets: &'b ProjectAssets,
    project_path: Option<&'b std::path::Path>,
}

impl SceneViewport {
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
        let (resolved, failures) = resolve_sprite_render_requests(&mut resolver, requests);
        let instances = resolved
            .into_iter()
            .map(|sprite| toki_render::SpriteInstance {
                frame: sprite.frame,
                position: sprite.position,
                size: sprite.size,
                texture_path: sprite.texture_path,
                flip_x: sprite.flip_x,
            })
            .collect();
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
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        let atlas_name_clean = atlas_name.strip_suffix(".json").unwrap_or(atlas_name);
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

        Ok(ResolvedSpriteVisual {
            frame,
            intrinsic_size,
            texture_path: atlas_asset
                .path
                .parent()
                .map(|parent| parent.join(&atlas.image)),
        })
    }

    fn resolve_object_sheet_object(
        &mut self,
        sheet_name: &str,
        object_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        let sheet_name_clean = sheet_name.strip_suffix(".json").unwrap_or(sheet_name);
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
        })
    }
}
