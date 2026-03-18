use std::path::PathBuf;

use crate::assets::tilemap::TileMap;
use crate::entity::EntityId;
use crate::sprite::SpriteFrame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpriteVisualRef {
    AtlasTile {
        atlas_name: String,
        tile_name: String,
    },
    ObjectSheetObject {
        sheet_name: String,
        object_name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteRenderSize {
    Explicit(glam::UVec2),
    Intrinsic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpriteRenderOrigin {
    AnimatedEntity(EntityId),
    StaticEntity(EntityId),
    Projectile(EntityId),
    MapObject {
        sheet_name: String,
        object_name: String,
        position: glam::IVec2,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpriteSortKey {
    pub primary: i32,
    pub secondary: i32,
    pub sequence: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteRenderRequest {
    pub origin: SpriteRenderOrigin,
    pub sort_key: SpriteSortKey,
    pub visual: SpriteVisualRef,
    pub position: glam::IVec2,
    pub size: SpriteRenderSize,
    pub flip_x: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedSpriteVisual {
    pub frame: SpriteFrame,
    pub intrinsic_size: glam::UVec2,
    pub texture_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ResolvedSpriteRenderInstance {
    pub origin: SpriteRenderOrigin,
    pub sort_key: SpriteSortKey,
    pub frame: SpriteFrame,
    pub position: glam::IVec2,
    pub size: glam::UVec2,
    pub texture_path: Option<PathBuf>,
    pub flip_x: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpriteResolveError {
    MissingAtlas {
        atlas_name: String,
    },
    MissingAtlasTile {
        atlas_name: String,
        tile_name: String,
    },
    MissingObjectSheet {
        sheet_name: String,
    },
    MissingObject {
        sheet_name: String,
        object_name: String,
    },
    AssetLoadFailed {
        asset_kind: &'static str,
        asset_name: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteResolveFailure {
    pub origin: SpriteRenderOrigin,
    pub error: SpriteResolveError,
}

pub trait SpriteAssetResolver {
    fn resolve_atlas_tile(
        &mut self,
        atlas_name: &str,
        tile_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError>;

    fn resolve_object_sheet_object(
        &mut self,
        sheet_name: &str,
        object_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError>;
}

pub fn sort_sprite_render_requests(requests: &mut [SpriteRenderRequest]) {
    requests.sort_by_key(|request| request.sort_key);
}

pub fn resolve_sprite_render_request(
    resolver: &mut impl SpriteAssetResolver,
    request: &SpriteRenderRequest,
) -> Result<ResolvedSpriteRenderInstance, SpriteResolveError> {
    let visual = match &request.visual {
        SpriteVisualRef::AtlasTile {
            atlas_name,
            tile_name,
        } => resolver.resolve_atlas_tile(atlas_name, tile_name)?,
        SpriteVisualRef::ObjectSheetObject {
            sheet_name,
            object_name,
        } => resolver.resolve_object_sheet_object(sheet_name, object_name)?,
    };

    Ok(ResolvedSpriteRenderInstance {
        origin: request.origin.clone(),
        sort_key: request.sort_key,
        frame: visual.frame,
        position: request.position,
        size: match request.size {
            SpriteRenderSize::Explicit(size) => size,
            SpriteRenderSize::Intrinsic => visual.intrinsic_size,
        },
        texture_path: visual.texture_path,
        flip_x: request.flip_x,
    })
}

pub fn resolve_sprite_render_requests(
    resolver: &mut impl SpriteAssetResolver,
    requests: &[SpriteRenderRequest],
) -> (Vec<ResolvedSpriteRenderInstance>, Vec<SpriteResolveFailure>) {
    let mut resolved = Vec::with_capacity(requests.len());
    let mut failures = Vec::new();

    for request in requests {
        match resolve_sprite_render_request(resolver, request) {
            Ok(instance) => resolved.push(instance),
            Err(error) => failures.push(SpriteResolveFailure {
                origin: request.origin.clone(),
                error,
            }),
        }
    }

    (resolved, failures)
}

pub fn collect_map_object_sprite_render_requests(tilemap: &TileMap) -> Vec<SpriteRenderRequest> {
    tilemap
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.visible)
        .filter_map(|(index, object)| {
            let sheet_name = object
                .sheet
                .file_stem()
                .and_then(|name| name.to_str())
                .or_else(|| object.sheet.to_str())?
                .to_string();
            let object_name = object.object_name.clone();
            Some(SpriteRenderRequest {
                origin: SpriteRenderOrigin::MapObject {
                    sheet_name: sheet_name.clone(),
                    object_name: object_name.clone(),
                    position: object.position.as_ivec2(),
                },
                sort_key: SpriteSortKey {
                    primary: 3,
                    secondary: 0,
                    sequence: index as u32,
                },
                visual: SpriteVisualRef::ObjectSheetObject {
                    sheet_name,
                    object_name,
                },
                position: object.position.as_ivec2(),
                size: SpriteRenderSize::Intrinsic,
                flip_x: false,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "sprite_render_tests.rs"]
mod tests;
