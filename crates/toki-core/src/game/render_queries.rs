use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{EntityId, EntityManager};
use crate::sprite::SpriteFrame;
use crate::sprite_render::{
    SpriteRenderOrigin, SpriteRenderRequest, SpriteRenderSize, SpriteSortKey, SpriteVisualRef,
};

use super::{EntityHealthBar, GameState};

struct RenderQueryFacade<'a> {
    entity_manager: &'a EntityManager,
    player_id: Option<EntityId>,
    debug_collision_rendering: bool,
}

impl<'a> RenderQueryFacade<'a> {
    fn new(
        entity_manager: &'a EntityManager,
        player_id: Option<EntityId>,
        debug_collision_rendering: bool,
    ) -> Self {
        Self {
            entity_manager,
            player_id,
            debug_collision_rendering,
        }
    }

    fn entity_sprite_frame(
        &self,
        entity_id: EntityId,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> Option<SpriteFrame> {
        tracing::trace!(
            "Getting sprite frame for entity {} with texture size {}x{}",
            entity_id,
            texture_size.x,
            texture_size.y
        );

        if let Some(entity) = self.entity_manager.get_entity(entity_id) {
            tracing::trace!("Found entity {} for sprite frame lookup", entity_id);

            if let Some(animation_controller) = &entity.attributes.animation_controller {
                tracing::trace!("Entity {} has animation controller", entity_id);

                if let Ok(tile_name) = animation_controller.current_tile_name() {
                    tracing::trace!("Entity {} requesting tile: '{}'", entity_id, tile_name);

                    if let Some(uvs) = atlas.get_tile_uvs(&tile_name, texture_size) {
                        tracing::trace!(
                            "Found UVs for tile '{}': [{:.3}, {:.3}, {:.3}, {:.3}]",
                            tile_name,
                            uvs[0],
                            uvs[1],
                            uvs[2],
                            uvs[3]
                        );
                        return Some(SpriteFrame {
                            u0: uvs[0],
                            v0: uvs[1],
                            u1: uvs[2],
                            v1: uvs[3],
                        });
                    }

                    tracing::warn!(
                        "Tile '{}' not found in atlas for entity {}",
                        tile_name,
                        entity_id
                    );
                    tracing::trace!(
                        "Atlas contains tiles: {:?}",
                        atlas.tiles.keys().collect::<Vec<_>>()
                    );
                } else {
                    tracing::trace!(
                        "Entity {} animation controller failed to provide tile name",
                        entity_id
                    );
                }
            } else {
                tracing::trace!("Entity {} has no animation controller", entity_id);
            }
        } else {
            tracing::warn!("Entity {} not found when getting sprite frame", entity_id);
        }
        None
    }

    fn entity_current_atlas_name(&self, entity_id: EntityId) -> Option<String> {
        self.entity_manager
            .get_entity(entity_id)
            .and_then(|entity| entity.attributes.animation_controller.as_ref())
            .and_then(|controller| controller.current_atlas_name().ok())
    }

    fn entity_sprite_flip_x(&self, entity_id: EntityId) -> bool {
        self.entity_manager
            .get_entity(entity_id)
            .and_then(|entity| entity.attributes.animation_controller.as_ref())
            .map(|controller| GameState::animation_state_flip_x(controller.current_clip_state))
            .unwrap_or(false)
    }

    fn renderable_entities(&self) -> Vec<(EntityId, glam::IVec2, glam::UVec2)> {
        let active_entities = self.entity_manager.active_entities();
        tracing::trace!(
            "Checking {} active entities for renderability",
            active_entities.len()
        );

        let renderable = self
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                let entity = self.entity_manager.get_entity(entity_id)?;
                let is_visible = entity.attributes.visible;
                let has_animation = entity.attributes.animation_controller.is_some();

                tracing::trace!(
                    "Entity {}: visible={}, has_animation={}",
                    entity_id,
                    is_visible,
                    has_animation
                );

                if is_visible && has_animation {
                    tracing::trace!(
                        "Entity {} is renderable at ({}, {}) with size {}x{}",
                        entity_id,
                        entity.position.x,
                        entity.position.y,
                        entity.size.x,
                        entity.size.y
                    );
                    return Some((entity_id, entity.position, entity.size));
                }
                None
            })
            .collect::<Vec<_>>();

        tracing::trace!(
            "Found {} renderable entities out of {} active entities",
            renderable.len(),
            active_entities.len()
        );
        renderable
    }

    fn entity_health_bars(&self) -> Vec<EntityHealthBar> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                let entity = self.entity_manager.get_entity(entity_id)?;
                if !entity.attributes.visible || !entity.attributes.active {
                    return None;
                }

                let current = entity
                    .attributes
                    .current_stat(crate::entity::HEALTH_STAT_ID)?;
                let max = entity
                    .attributes
                    .base_stat(crate::entity::HEALTH_STAT_ID)
                    .or(Some(current))
                    .filter(|value| *value > 0)?;

                Some(EntityHealthBar {
                    entity_id,
                    position: entity.position,
                    size: entity.size,
                    current: current.clamp(0, max),
                    max,
                })
            })
            .collect()
    }

    fn sprite_render_requests(&self) -> Vec<SpriteRenderRequest> {
        let mut requests = Vec::new();
        let mut animated_sequence = 0_u32;
        let mut static_sequence = 0_u32;
        let mut projectile_sequence = 0_u32;

        for entity_id in self.entity_manager.active_entities_iter() {
            let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                continue;
            };
            if !entity.attributes.visible || !entity.attributes.active {
                continue;
            }

            if let Some(animation_controller) = &entity.attributes.animation_controller {
                let Ok(atlas_name) = animation_controller.current_atlas_name() else {
                    continue;
                };
                let Ok(tile_name) = animation_controller.current_tile_name() else {
                    continue;
                };

                requests.push(SpriteRenderRequest {
                    origin: SpriteRenderOrigin::AnimatedEntity(entity_id),
                    sort_key: SpriteSortKey {
                        primary: 0,
                        secondary: entity.attributes.render_layer,
                        sequence: animated_sequence,
                    },
                    visual: SpriteVisualRef::AtlasTile {
                        atlas_name,
                        tile_name,
                    },
                    position: entity.position,
                    size: SpriteRenderSize::Explicit(entity.size),
                    flip_x: GameState::animation_state_flip_x(
                        animation_controller.current_clip_state,
                    ),
                });
                animated_sequence += 1;
                continue;
            }

            if let Some(static_render) = &entity.attributes.static_object_render {
                requests.push(SpriteRenderRequest {
                    origin: SpriteRenderOrigin::StaticEntity(entity_id),
                    sort_key: SpriteSortKey {
                        primary: 1,
                        secondary: entity.attributes.render_layer,
                        sequence: static_sequence,
                    },
                    visual: SpriteVisualRef::ObjectSheetObject {
                        sheet_name: static_render.sheet.clone(),
                        object_name: static_render.object_name.clone(),
                    },
                    position: entity.position,
                    size: SpriteRenderSize::Explicit(entity.size),
                    flip_x: false,
                });
                static_sequence += 1;
                continue;
            }

            if let Some(projectile) = &entity.attributes.projectile {
                requests.push(SpriteRenderRequest {
                    origin: SpriteRenderOrigin::Projectile(entity_id),
                    sort_key: SpriteSortKey {
                        primary: 2,
                        secondary: entity.attributes.render_layer,
                        sequence: projectile_sequence,
                    },
                    visual: SpriteVisualRef::ObjectSheetObject {
                        sheet_name: projectile.sheet.clone(),
                        object_name: projectile.object_name.clone(),
                    },
                    position: entity.position,
                    size: SpriteRenderSize::Explicit(entity.size),
                    flip_x: false,
                });
                projectile_sequence += 1;
            }
        }

        requests
    }

    fn current_sprite_frame(&self, atlas: &AtlasMeta, texture_size: glam::UVec2) -> SpriteFrame {
        if let Some(player_id) = self.player_id {
            if let Some(frame) = self.entity_sprite_frame(player_id, atlas, texture_size) {
                return frame;
            }
        }

        SpriteFrame {
            u0: 0.0,
            v0: 0.0,
            u1: 0.25,
            v1: 1.0,
        }
    }

    fn player_position(&self) -> glam::IVec2 {
        self.player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|player| player.position)
            .unwrap_or(glam::IVec2::ZERO)
    }

    fn entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut boxes = Vec::new();
        for entity_id in self.entity_manager.active_entities_iter() {
            if let Some(entity) = self.entity_manager.get_entity(entity_id) {
                if let Some(collision_box) = &entity.collision_box {
                    let (world_pos, size) = collision_box.world_bounds(entity.position);
                    boxes.push((world_pos, size, collision_box.trigger));
                }
            }
        }
        boxes
    }

    fn solid_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut solid_tiles = Vec::new();
        for y in 0..tilemap.size.y {
            for x in 0..tilemap.size.x {
                if let Ok(is_solid) = tilemap.is_tile_solid_at(atlas, x, y) {
                    if is_solid {
                        solid_tiles.push((x, y));
                    }
                }
            }
        }
        solid_tiles
    }

    fn trigger_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)> {
        if !self.debug_collision_rendering {
            return Vec::new();
        }

        let mut trigger_tiles = Vec::new();
        for y in 0..tilemap.size.y {
            for x in 0..tilemap.size.x {
                if let Ok(tile_name) = tilemap.get_tile_name(x, y) {
                    if atlas.is_tile_trigger(tile_name) {
                        trigger_tiles.push((x, y));
                    }
                }
            }
        }
        trigger_tiles
    }
}

impl GameState {
    fn render_query_facade(&self) -> RenderQueryFacade<'_> {
        RenderQueryFacade::new(
            &self.entity_manager,
            self.player_id,
            self.debug_collision_rendering,
        )
    }

    /// Get sprite frame for a specific entity
    pub fn get_entity_sprite_frame(
        &self,
        entity_id: EntityId,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> Option<SpriteFrame> {
        self.render_query_facade()
            .entity_sprite_frame(entity_id, atlas, texture_size)
    }

    pub fn get_entity_current_atlas_name(&self, entity_id: EntityId) -> Option<String> {
        self.render_query_facade().entity_current_atlas_name(entity_id)
    }

    pub fn get_entity_sprite_flip_x(&self, entity_id: EntityId) -> bool {
        self.render_query_facade().entity_sprite_flip_x(entity_id)
    }

    /// Get all renderable entities (entities that are visible and have animation controllers)
    pub fn get_renderable_entities(&self) -> Vec<(EntityId, glam::IVec2, glam::UVec2)> {
        self.render_query_facade().renderable_entities()
    }

    /// Get world-space health bar data for visible, active entities with health stats.
    pub fn get_entity_health_bars(&self) -> Vec<EntityHealthBar> {
        self.render_query_facade().entity_health_bars()
    }

    pub fn get_sprite_render_requests(&self) -> Vec<SpriteRenderRequest> {
        self.render_query_facade().sprite_render_requests()
    }

    /// Get the current player sprite frame for rendering with proper atlas lookup.
    pub fn current_sprite_frame(
        &self,
        atlas: &AtlasMeta,
        texture_size: glam::UVec2,
    ) -> SpriteFrame {
        self.render_query_facade()
            .current_sprite_frame(atlas, texture_size)
    }

    /// Get player position for rendering
    pub fn player_position(&self) -> glam::IVec2 {
        self.render_query_facade().player_position()
    }

    /// Check if debug collision rendering is enabled
    pub fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.debug_collision_rendering
    }

    /// Get entity collision boxes for debug rendering
    /// Returns Vec of (position, size, is_trigger) tuples
    pub fn get_entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
        self.render_query_facade().entity_collision_boxes()
    }

    /// Get solid tile positions for debug rendering
    /// Returns Vec of (tile_x, tile_y) coordinates of solid tiles
    pub fn get_solid_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        self.render_query_facade()
            .solid_tile_positions(tilemap, atlas)
    }

    /// Get trigger tile positions for debug rendering
    /// Returns Vec of (tile_x, tile_y) coordinates of trigger tiles
    pub fn get_trigger_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        self.render_query_facade()
            .trigger_tile_positions(tilemap, atlas)
    }
}

#[cfg(test)]
mod tests {
    use super::RenderQueryFacade;
    use crate::entity::{EntityManager, EntityStats};

    #[test]
    fn health_bar_queries_filter_invisible_and_inactive_entities() {
        let mut entity_manager = EntityManager::new();
        let visible_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(4, 8),
            glam::UVec2::new(16, 16),
            crate::entity::EntityAttributes {
                health: Some(20),
                stats: EntityStats::from_legacy_health(Some(20)),
                ..crate::entity::EntityAttributes::default()
            },
        );
        let hidden_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(20, 8),
            glam::UVec2::new(16, 16),
            crate::entity::EntityAttributes {
                health: Some(20),
                stats: EntityStats::from_legacy_health(Some(20)),
                visible: false,
                ..crate::entity::EntityAttributes::default()
            },
        );
        let inactive_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(36, 8),
            glam::UVec2::new(16, 16),
            crate::entity::EntityAttributes {
                health: Some(20),
                stats: EntityStats::from_legacy_health(Some(20)),
                active: false,
                ..crate::entity::EntityAttributes::default()
            },
        );

        let facade = RenderQueryFacade::new(&entity_manager, None, false);
        let bars = facade.entity_health_bars();

        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].entity_id, visible_id);
        assert_ne!(bars[0].entity_id, hidden_id);
        assert_ne!(bars[0].entity_id, inactive_id);
    }
}
