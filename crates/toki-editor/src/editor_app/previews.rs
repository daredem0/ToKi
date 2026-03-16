use super::*;

impl EditorApp {
    pub(super) fn load_preview_sprite_frame_static(
        entity_def_name: &str,
        project_path: &std::path::Path,
        project_assets: &crate::project::ProjectAssets,
    ) -> Option<toki_core::sprite::SpriteFrame> {
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

        let atlas_name = &entity_def.animations.atlas_name;
        let atlas_name_clean = atlas_name.strip_suffix(".json").unwrap_or(atlas_name);
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
                    return Some(toki_core::sprite::SpriteFrame {
                        u0: uvs[0],
                        v0: uvs[1],
                        u1: uvs[2],
                        v1: uvs[3],
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
