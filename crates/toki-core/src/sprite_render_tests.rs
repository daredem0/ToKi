use super::{
    collect_map_object_sprite_render_requests, resolve_sprite_render_request,
    resolve_sprite_render_requests, sort_sprite_render_requests, ResolvedSpriteVisual,
    SpriteAssetResolver, SpriteRenderOrigin, SpriteRenderRequest, SpriteRenderSize,
    SpriteResolveError, SpriteSortKey, SpriteVisualRef,
};
use crate::assets::tilemap::{MapObjectInstance, TileMap};
use crate::sprite::SpriteFrame;
use std::path::PathBuf;

#[derive(Default)]
struct FakeResolver;

impl SpriteAssetResolver for FakeResolver {
    fn resolve_atlas_tile(
        &mut self,
        atlas_name: &str,
        tile_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        if atlas_name == "missing" {
            return Err(SpriteResolveError::MissingAtlas {
                atlas_name: atlas_name.to_string(),
            });
        }
        if tile_name == "missing_tile" {
            return Err(SpriteResolveError::MissingAtlasTile {
                atlas_name: atlas_name.to_string(),
                tile_name: tile_name.to_string(),
            });
        }

        Ok(ResolvedSpriteVisual {
            frame: SpriteFrame {
                u0: 0.1,
                v0: 0.2,
                u1: 0.3,
                v1: 0.4,
            },
            intrinsic_size: glam::UVec2::new(16, 16),
            texture_path: Some(PathBuf::from(format!("{atlas_name}.png"))),
        })
    }

    fn resolve_object_sheet_object(
        &mut self,
        sheet_name: &str,
        object_name: &str,
    ) -> Result<ResolvedSpriteVisual, SpriteResolveError> {
        if sheet_name == "missing_sheet" {
            return Err(SpriteResolveError::MissingObjectSheet {
                sheet_name: sheet_name.to_string(),
            });
        }
        if object_name == "missing_object" {
            return Err(SpriteResolveError::MissingObject {
                sheet_name: sheet_name.to_string(),
                object_name: object_name.to_string(),
            });
        }

        Ok(ResolvedSpriteVisual {
            frame: SpriteFrame {
                u0: 0.5,
                v0: 0.6,
                u1: 0.7,
                v1: 0.8,
            },
            intrinsic_size: glam::UVec2::new(24, 12),
            texture_path: Some(PathBuf::from(format!("{sheet_name}.png"))),
        })
    }
}

#[test]
fn explicit_request_size_overrides_intrinsic_visual_size() {
    let request = SpriteRenderRequest {
        origin: SpriteRenderOrigin::AnimatedEntity(1),
        sort_key: SpriteSortKey {
            primary: 0,
            secondary: 0,
            sequence: 0,
        },
        visual: SpriteVisualRef::AtlasTile {
            atlas_name: "creatures".to_string(),
            tile_name: "slime/idle_0".to_string(),
        },
        position: glam::IVec2::new(10, 12),
        size: SpriteRenderSize::Explicit(glam::UVec2::new(32, 18)),
        flip_x: true,
    };

    let resolved =
        resolve_sprite_render_request(&mut FakeResolver, &request).expect("request should resolve");

    assert_eq!(resolved.size, glam::UVec2::new(32, 18));
    assert!(resolved.flip_x);
    assert_eq!(resolved.texture_path, Some(PathBuf::from("creatures.png")));
}

#[test]
fn intrinsic_request_size_uses_visual_intrinsic_size() {
    let request = SpriteRenderRequest {
        origin: SpriteRenderOrigin::MapObject {
            sheet_name: "items".to_string(),
            object_name: "coin".to_string(),
            position: glam::IVec2::new(0, 0),
        },
        sort_key: SpriteSortKey {
            primary: 3,
            secondary: 0,
            sequence: 0,
        },
        visual: SpriteVisualRef::ObjectSheetObject {
            sheet_name: "items".to_string(),
            object_name: "coin".to_string(),
        },
        position: glam::IVec2::new(24, 30),
        size: SpriteRenderSize::Intrinsic,
        flip_x: false,
    };

    let resolved =
        resolve_sprite_render_request(&mut FakeResolver, &request).expect("request should resolve");

    assert_eq!(resolved.size, glam::UVec2::new(24, 12));
    assert_eq!(resolved.position, glam::IVec2::new(24, 30));
}

#[test]
fn batch_resolution_collects_failures_without_stopping() {
    let requests = vec![
        SpriteRenderRequest {
            origin: SpriteRenderOrigin::AnimatedEntity(1),
            sort_key: SpriteSortKey {
                primary: 0,
                secondary: 0,
                sequence: 0,
            },
            visual: SpriteVisualRef::AtlasTile {
                atlas_name: "creatures".to_string(),
                tile_name: "slime/idle_0".to_string(),
            },
            position: glam::IVec2::ZERO,
            size: SpriteRenderSize::Explicit(glam::UVec2::new(16, 16)),
            flip_x: false,
        },
        SpriteRenderRequest {
            origin: SpriteRenderOrigin::Projectile(2),
            sort_key: SpriteSortKey {
                primary: 2,
                secondary: 0,
                sequence: 0,
            },
            visual: SpriteVisualRef::ObjectSheetObject {
                sheet_name: "missing_sheet".to_string(),
                object_name: "rock".to_string(),
            },
            position: glam::IVec2::ZERO,
            size: SpriteRenderSize::Explicit(glam::UVec2::new(8, 8)),
            flip_x: false,
        },
    ];

    let (resolved, failures) = resolve_sprite_render_requests(&mut FakeResolver, &requests);

    assert_eq!(resolved.len(), 1);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].origin, SpriteRenderOrigin::Projectile(2));
    assert_eq!(
        failures[0].error,
        SpriteResolveError::MissingObjectSheet {
            sheet_name: "missing_sheet".to_string(),
        }
    );
}

#[test]
fn map_object_request_collection_uses_intrinsic_size_and_sheet_stem() {
    let tilemap = TileMap {
        size: glam::UVec2::new(1, 1),
        tile_size: glam::UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        tiles: vec!["grass".to_string()],
        objects: vec![MapObjectInstance {
            sheet: PathBuf::from("assets/sprites/items.json"),
            object_name: "coin".to_string(),
            position: glam::UVec2::new(32, 48),
            size_px: glam::UVec2::new(99, 99),
            visible: true,
            solid: false,
        }],
    };

    let requests = collect_map_object_sprite_render_requests(&tilemap);
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].visual,
        SpriteVisualRef::ObjectSheetObject {
            sheet_name: "items".to_string(),
            object_name: "coin".to_string(),
        }
    );
    assert_eq!(requests[0].size, SpriteRenderSize::Intrinsic);
}

#[test]
fn sort_requests_orders_by_shared_sort_key() {
    let mut requests = vec![
        SpriteRenderRequest {
            origin: SpriteRenderOrigin::AnimatedEntity(1),
            sort_key: SpriteSortKey {
                primary: 2,
                secondary: 0,
                sequence: 3,
            },
            visual: SpriteVisualRef::AtlasTile {
                atlas_name: "creatures".to_string(),
                tile_name: "slime".to_string(),
            },
            position: glam::IVec2::ZERO,
            size: SpriteRenderSize::Explicit(glam::UVec2::new(16, 16)),
            flip_x: false,
        },
        SpriteRenderRequest {
            origin: SpriteRenderOrigin::AnimatedEntity(2),
            sort_key: SpriteSortKey {
                primary: 0,
                secondary: 0,
                sequence: 1,
            },
            visual: SpriteVisualRef::AtlasTile {
                atlas_name: "creatures".to_string(),
                tile_name: "player".to_string(),
            },
            position: glam::IVec2::ZERO,
            size: SpriteRenderSize::Explicit(glam::UVec2::new(16, 16)),
            flip_x: false,
        },
    ];

    sort_sprite_render_requests(&mut requests);

    assert_eq!(requests[0].origin, SpriteRenderOrigin::AnimatedEntity(2));
    assert_eq!(requests[1].origin, SpriteRenderOrigin::AnimatedEntity(1));
}
