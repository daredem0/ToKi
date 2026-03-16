use super::*;

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

    pub(super) fn build_map_object_sprite_instance(
        &mut self,
        project_assets: &ProjectAssets,
        object: &MapObjectInstance,
    ) -> Option<toki_render::SpriteInstance> {
        let sheet_name = object
            .sheet
            .file_stem()
            .and_then(|name| name.to_str())
            .or_else(|| object.sheet.to_str())?;
        let object_sheet_asset = project_assets.object_sheets.get(sheet_name)?;
        let object_sheet = self.load_object_sheet_from_asset(object_sheet_asset).ok()?;
        let texture_size = object_sheet.image_size()?;
        let rect = object_sheet.get_object_rect(&object.object_name)?;
        let uv_rect = object_sheet.get_object_uvs(&object.object_name, texture_size)?;

        Some(toki_render::SpriteInstance {
            frame: toki_core::sprite::SpriteFrame {
                u0: uv_rect[0],
                v0: uv_rect[1],
                u1: uv_rect[2],
                v1: uv_rect[3],
            },
            position: object.position.as_ivec2(),
            size: glam::UVec2::new(rect[2], rect[3]),
            texture_path: object_sheet_asset
                .path
                .parent()
                .map(|parent| parent.join(&object_sheet.image)),
            flip_x: false,
        })
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
