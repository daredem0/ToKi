use crate::config::EditorConfig;
use crate::editor_grid::GridInteraction;
use crate::editor_types::PlacementPreviewVisual;
use crate::project::ProjectAssets;
use crate::scene::viewport::{DragPreviewSprite, OverlayLineInstance, OverlaySpriteInstance};
use toki_core::assets::tilemap::TileMap;
use toki_core::entity::{ControlRole, Entity};
use toki_core::project_assets::normalize_asset_name;
use toki_core::Scene;

pub struct SceneAnchorOverlayRequest<'a> {
    pub active_scene_name: Option<&'a str>,
    pub scenes: &'a [Scene],
    pub dragged_anchor: Option<(&'a str, &'a str)>,
    pub preview_position: Option<glam::Vec2>,
    pub preview_valid: bool,
    pub draft_active: bool,
}

pub fn cached_preview_sprite_frame(
    preview_sprite_frames: &mut std::collections::HashMap<
        (std::path::PathBuf, String),
        Option<PlacementPreviewVisual>,
    >,
    entity_def_name: &str,
    project_path: &std::path::Path,
    project_assets: &ProjectAssets,
) -> Option<PlacementPreviewVisual> {
    let cache_key = (project_path.to_path_buf(), entity_def_name.to_string());
    let cached = preview_sprite_frames.entry(cache_key).or_insert_with(|| {
        load_preview_sprite_frame(entity_def_name, project_path, project_assets)
    });
    cached.clone()
}

pub fn load_preview_sprite_frame(
    entity_def_name: &str,
    project_path: &std::path::Path,
    project_assets: &ProjectAssets,
) -> Option<PlacementPreviewVisual> {
    tracing::info!(
        "Loading preview sprite frame for entity '{}' (one-time cache)",
        entity_def_name
    );

    let entity_file = project_path
        .join("entities")
        .join(format!("{}.json", entity_def_name));
    if !entity_file.exists() {
        tracing::warn!(
            "Entity definition file not found for preview: {:?}",
            entity_file
        );
        return None;
    }

    let entity_def = match std::fs::read_to_string(&entity_file).and_then(|content| {
        serde_json::from_str::<toki_core::entity::EntityDefinition>(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }) {
        Ok(def) => def,
        Err(error) => {
            tracing::warn!("Failed to load entity definition for preview: {}", error);
            return None;
        }
    };

    if let Some(static_object) = &entity_def.rendering.static_object {
        let sheet_name = normalize_asset_name(&static_object.sheet);
        let object_sheet_asset = project_assets.object_sheets.get(sheet_name)?;
        let object_sheet = match toki_core::assets::object_sheet::ObjectSheetMeta::load_from_file(
            &object_sheet_asset.path,
        ) {
            Ok(sheet) => sheet,
            Err(error) => {
                tracing::warn!("Failed to load object sheet for preview: {}", error);
                return None;
            }
        };
        let texture_size = object_sheet.image_size().unwrap_or(glam::UVec2::new(16, 16));
        let Some(uvs) = object_sheet.get_object_uvs(&static_object.object_name, texture_size) else {
            tracing::warn!(
                "Failed to get UV coordinates for object '{}' in preview",
                static_object.object_name
            );
            return None;
        };

        return Some(PlacementPreviewVisual {
            frame: toki_core::sprite::SpriteFrame {
                u0: uvs[0],
                v0: uvs[1],
                u1: uvs[2],
                v1: uvs[3],
            },
            texture_path: object_sheet_asset
                .path
                .parent()
                .map(|parent| parent.join(&object_sheet.image)),
            size: glam::UVec2::new(entity_def.rendering.size[0], entity_def.rendering.size[1]),
        });
    }

    let atlas_name_clean = normalize_asset_name(&entity_def.animations.atlas_name);
    let atlas_asset = project_assets.sprite_atlases.get(atlas_name_clean)?;
    let sprite_atlas = match toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_asset.path)
    {
        Ok(atlas) => atlas,
        Err(error) => {
            tracing::warn!("Failed to load sprite atlas for preview: {}", error);
            return None;
        }
    };

    let sprite_texture_size = sprite_atlas.image_size().unwrap_or(glam::UVec2::new(64, 16));

    if let Some(clip_def) = entity_def.animations.clips.first() {
        if let Some(first_tile_name) = clip_def.frame_tiles.first() {
            if let Some(uvs) = sprite_atlas.get_tile_uvs(first_tile_name, sprite_texture_size) {
                return Some(PlacementPreviewVisual {
                    frame: toki_core::sprite::SpriteFrame {
                        u0: uvs[0],
                        v0: uvs[1],
                        u1: uvs[2],
                        v1: uvs[3],
                    },
                    texture_path: atlas_asset
                        .path
                        .parent()
                        .map(|parent| parent.join(&sprite_atlas.image)),
                    size: glam::UVec2::new(
                        entity_def.rendering.size[0],
                        entity_def.rendering.size[1],
                    ),
                });
            }

            tracing::warn!(
                "Failed to get UV coordinates for tile '{}' in preview",
                first_tile_name
            );
        } else {
            tracing::warn!("No frame tiles found in first animation clip for preview");
        }
    } else {
        tracing::warn!("No animation clips found for preview");
    }

    None
}

pub fn build_scene_player_overlay_sprites(
    active_scene_name: Option<&str>,
    scenes: &[Scene],
    project_path: &std::path::Path,
    project_assets: &ProjectAssets,
    preview_cache: &mut std::collections::HashMap<
        (std::path::PathBuf, String),
        Option<PlacementPreviewVisual>,
    >,
) -> Vec<OverlaySpriteInstance> {
    let Some(active_scene_name) = active_scene_name else {
        return Vec::new();
    };
    let Some(scene) = scenes.iter().find(|scene| scene.name == active_scene_name) else {
        return Vec::new();
    };
    let Some(player_entry) = scene.player_entry.as_ref() else {
        return Vec::new();
    };

    if scene
        .entities
        .iter()
        .any(|entity| entity.control_role == ControlRole::PlayerCharacter)
    {
        return Vec::new();
    }

    let Some(spawn_point) = scene.get_anchor(&player_entry.spawn_point_id) else {
        return Vec::new();
    };
    let Some(visual) = cached_preview_sprite_frame(
        preview_cache,
        &player_entry.entity_definition_name,
        project_path,
        project_assets,
    ) else {
        return Vec::new();
    };

    vec![OverlaySpriteInstance {
        world_position: spawn_point.position,
        visual,
    }]
}

pub fn build_scene_anchor_overlay_lines(
    request: SceneAnchorOverlayRequest<'_>,
    tilemap: Option<&TileMap>,
    config: Option<&EditorConfig>,
) -> Vec<OverlayLineInstance> {
    let Some(active_scene_name) = request.active_scene_name else {
        return Vec::new();
    };
    let Some(scene) = request
        .scenes
        .iter()
        .find(|scene| scene.name == active_scene_name)
    else {
        return Vec::new();
    };

    let mut lines = Vec::new();

    for anchor in &scene.anchors {
        if request.dragged_anchor.is_some_and(|(scene_name, anchor_id)| {
            scene_name == scene.name && anchor_id == anchor.id
        }) {
            continue;
        }
        lines.extend(scene_anchor_cross_lines(
            anchor.position.as_vec2(),
            tilemap,
            config,
            [0.1882, 0.5176, 1.0, 1.0],
        ));
    }

    if request.dragged_anchor.is_some() {
        if let Some(preview_position) = request.preview_position {
            lines.extend(scene_anchor_cross_lines(
                preview_position,
                tilemap,
                config,
                [0.1882, 0.5176, 1.0, 1.0],
            ));
        }
        return lines;
    }

    if request.draft_active {
        if let Some(preview_position) = request.preview_position {
            let color = if request.preview_valid {
                [0.1882, 0.5176, 1.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };
            lines.extend(scene_anchor_cross_lines(
                preview_position,
                tilemap,
                config,
                color,
            ));
        }
    }

    lines
}

pub fn build_drag_preview_sprites(
    dragged_entities: &[Entity],
    anchor_entity_position: glam::IVec2,
    preview_position: glam::Vec2,
    tilemap: Option<&TileMap>,
    terrain_atlas: Option<&toki_core::assets::atlas::AtlasMeta>,
) -> Vec<DragPreviewSprite> {
    let anchor_preview = glam::IVec2::new(
        preview_position.x.floor() as i32,
        preview_position.y.floor() as i32,
    );
    let delta = anchor_preview - anchor_entity_position;

    dragged_entities
        .iter()
        .map(|entity| {
            let world_position = entity.position + delta;
            let is_valid = match (tilemap, terrain_atlas) {
                (Some(tilemap), Some(terrain_atlas)) => toki_core::collision::can_entity_move_to_position(
                    entity,
                    world_position,
                    tilemap,
                    terrain_atlas,
                ),
                _ => true,
            };

            DragPreviewSprite {
                entity_id: entity.id,
                world_position,
                is_valid,
            }
        })
        .collect()
}

fn scene_anchor_cross_lines(
    world_position: glam::Vec2,
    tilemap: Option<&TileMap>,
    config: Option<&EditorConfig>,
    color: [f32; 4],
) -> Vec<OverlayLineInstance> {
    let pose = GridInteraction::placement_pose(world_position, tilemap, config);
    let Some(size) = GridInteraction::effective_grid_size(tilemap, config) else {
        return Vec::new();
    };
    let origin = pose.world_origin;
    let size = size.as_vec2();
    let top_left = origin;
    let top_right = origin + glam::Vec2::new(size.x, 0.0);
    let bottom_left = origin + glam::Vec2::new(0.0, size.y);
    let bottom_right = origin + size;

    vec![
        OverlayLineInstance {
            start: top_left,
            end: bottom_right,
            thickness: 1.0,
            color,
        },
        OverlayLineInstance {
            start: top_right,
            end: bottom_left,
            thickness: 1.0,
            color,
        },
    ]
}
