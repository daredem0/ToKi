use super::*;
use crate::config::EditorConfig;
use crate::ui::editor_ui::PlacementPreviewVisual;
use crate::ui::interactions::GridInteraction;
use toki_core::project_assets::normalize_asset_name;

impl EditorApp {
    fn scene_anchor_cross_lines(
        world_position: glam::Vec2,
        tilemap: Option<&toki_core::assets::tilemap::TileMap>,
        config: Option<&EditorConfig>,
        color: [f32; 4],
    ) -> Vec<crate::scene::viewport::OverlayLineInstance> {
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
            crate::scene::viewport::OverlayLineInstance {
                start: top_left,
                end: bottom_right,
                thickness: 1.0,
                color,
            },
            crate::scene::viewport::OverlayLineInstance {
                start: top_right,
                end: bottom_left,
                thickness: 1.0,
                color,
            },
        ]
    }

    pub(super) fn build_scene_anchor_overlay_lines(
        ui_state: &crate::ui::EditorUI,
        tilemap: Option<&toki_core::assets::tilemap::TileMap>,
        config: Option<&EditorConfig>,
    ) -> Vec<crate::scene::viewport::OverlayLineInstance> {
        let Some(active_scene_name) = ui_state.active_scene.as_ref() else {
            return Vec::new();
        };
        let Some(scene) = ui_state
            .scenes
            .iter()
            .find(|scene| &scene.name == active_scene_name)
        else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        let dragged_anchor_id = ui_state
            .placement
            .scene_anchor_move_drag
            .as_ref()
            .filter(|drag| drag.scene_name == scene.name)
            .map(|drag| drag.anchor.id.as_str());

        for anchor in &scene.anchors {
            if dragged_anchor_id == Some(anchor.id.as_str()) {
                continue;
            }
            lines.extend(Self::scene_anchor_cross_lines(
                anchor.position.as_vec2(),
                tilemap,
                config,
                [0.1882, 0.5176, 1.0, 1.0],
            ));
        }

        if ui_state.placement.scene_anchor_move_drag.is_some() {
            if let Some(preview_position) = ui_state.placement.preview_position {
                lines.extend(Self::scene_anchor_cross_lines(
                    preview_position,
                    tilemap,
                    config,
                    [0.1882, 0.5176, 1.0, 1.0],
                ));
            }
            return lines;
        }

        if ui_state.placement.scene_anchor_draft().is_some() {
            if let Some(preview_position) = ui_state.placement.preview_position {
                let color = if ui_state.placement.preview_valid.unwrap_or(true) {
                    [0.1882, 0.5176, 1.0, 1.0]
                } else {
                    [1.0, 0.0, 0.0, 1.0]
                };
                lines.extend(Self::scene_anchor_cross_lines(
                    preview_position,
                    tilemap,
                    config,
                    color,
                ));
            }
        }

        lines
    }

    pub(super) fn load_preview_sprite_frame_static(
        entity_def_name: &str,
        project_path: &std::path::Path,
        project_assets: &crate::project::ProjectAssets,
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
            let object_sheet =
                match toki_core::assets::object_sheet::ObjectSheetMeta::load_from_file(
                    &object_sheet_asset.path,
                ) {
                    Ok(sheet) => sheet,
                    Err(error) => {
                        tracing::warn!("Failed to load object sheet for preview: {}", error);
                        return None;
                    }
                };
            let texture_size = object_sheet
                .image_size()
                .unwrap_or(glam::UVec2::new(16, 16));
            let Some(uvs) = object_sheet.get_object_uvs(&static_object.object_name, texture_size)
            else {
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

        let atlas_name = &entity_def.animations.atlas_name;
        let atlas_name_clean = normalize_asset_name(atlas_name);
        let atlas_asset = project_assets.sprite_atlases.get(atlas_name_clean)?;

        let sprite_atlas =
            match toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_asset.path) {
                Ok(atlas) => atlas,
                Err(error) => {
                    tracing::warn!("Failed to load sprite atlas for preview: {}", error);
                    return None;
                }
            };

        let sprite_texture_size = sprite_atlas
            .image_size()
            .unwrap_or(glam::UVec2::new(64, 16));

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

    pub(super) fn build_drag_preview_sprites(
        drag_state: &crate::ui::editor_ui::EntityMoveDragState,
        preview_position: glam::Vec2,
        tilemap: Option<&toki_core::assets::tilemap::TileMap>,
        terrain_atlas: Option<&toki_core::assets::atlas::AtlasMeta>,
    ) -> Vec<DragPreviewSprite> {
        let anchor_preview = glam::IVec2::new(
            preview_position.x.floor() as i32,
            preview_position.y.floor() as i32,
        );
        let delta = anchor_preview - drag_state.entity.position;

        drag_state
            .dragged_entities
            .iter()
            .map(|entity| {
                let world_position = entity.position + delta;
                let is_valid = match (tilemap, terrain_atlas) {
                    (Some(tilemap), Some(terrain_atlas)) => {
                        toki_core::collision::can_entity_move_to_position(
                            entity,
                            world_position,
                            tilemap,
                            terrain_atlas,
                        )
                    }
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
}
