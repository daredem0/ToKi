use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{EntityId, EntityManager};
use crate::sprite::SpriteFrame;
use crate::sprite_render::{
    SpriteRenderOrigin, SpriteRenderRequest, SpriteRenderSize, SpriteSortKey, SpriteVisualRef,
};
use std::collections::HashSet;

use super::{EntityHealthBar, GameState};

#[derive(Debug, Clone, PartialEq)]
pub struct GroundShadow {
    pub position: glam::Vec2,
    pub size: glam::Vec2,
    pub color: [f32; 4],
}

pub struct RenderQueryService<'a> {
    entity_manager: &'a EntityManager,
    player_id: Option<EntityId>,
    debug_collision_rendering: bool,
}

impl<'a> RenderQueryService<'a> {
    pub fn new(
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

    pub fn entity_sprite_frame(
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

            if let Some(animation_controller) = &entity.attributes.rendering.animation_controller {
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

    pub fn entity_current_atlas_name(&self, entity_id: EntityId) -> Option<String> {
        self.entity_manager
            .get_entity(entity_id)
            .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
            .and_then(|controller| controller.current_atlas_name().ok())
    }

    pub fn entity_sprite_flip_x(&self, entity_id: EntityId) -> bool {
        self.entity_manager
            .get_entity(entity_id)
            .and_then(|entity| entity.attributes.rendering.animation_controller.as_ref())
            .map(|controller| GameState::animation_state_flip_x(controller.current_clip_state))
            .unwrap_or(false)
    }

    pub fn renderable_entities(&self) -> Vec<(EntityId, glam::IVec2, glam::UVec2)> {
        let active_entities = self.entity_manager.active_entities();
        let projectile_ids = self
            .entity_manager
            .storage()
            .components()
            .projectile_ids()
            .collect::<HashSet<_>>();
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
                let is_visible = entity.attributes.rendering.visible;
                let is_renderable = entity.attributes.rendering.animation_controller.is_some()
                    || entity.attributes.rendering.static_object_render.is_some()
                    || projectile_ids.contains(&entity_id);

                tracing::trace!(
                    "Entity {}: visible={}, is_renderable={}",
                    entity_id,
                    is_visible,
                    is_renderable
                );

                if is_visible && is_renderable {
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

    pub fn entity_health_bars(&self) -> Vec<EntityHealthBar> {
        self.entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                let entity = self.entity_manager.get_entity(entity_id)?;
                if !entity.attributes.rendering.visible || !entity.attributes.behavior.active {
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

    pub fn entity_ground_shadows(&self) -> Vec<GroundShadow> {
        let projectile_ids = self
            .entity_manager
            .storage()
            .components()
            .projectile_ids()
            .collect::<HashSet<_>>();
        self.entity_manager
            .active_entities_iter()
            .filter_map(|entity_id| {
                let entity = self.entity_manager.get_entity(entity_id)?;
                if !entity.attributes.rendering.visible || !entity.attributes.behavior.active {
                    return None;
                }
                if !entity.attributes.rendering.has_shadow {
                    return None;
                }
                if projectile_ids.contains(&entity_id) {
                    return None;
                }
                if entity.attributes.rendering.animation_controller.is_none()
                    && entity.attributes.rendering.static_object_render.is_none()
                {
                    return None;
                }

                let (footprint_pos, footprint_size) = entity.footprint_world_bounds();
                let ground_origin = entity.resolved_ground_origin();
                let footprint_width = footprint_size.x.max(1) as f32;
                let footprint_height = footprint_size.y.max(1) as f32;
                let sprite_width = entity.size.x.max(1) as f32;
                let shadow_width = if footprint_width >= sprite_width * 0.9 {
                    (sprite_width * 0.8).round().max(4.0)
                } else {
                    (footprint_width + 4.0)
                        .max((sprite_width * 0.55).round())
                        .min((sprite_width * 0.8).round())
                        .round()
                        .max(4.0)
                };
                let shadow_height = (footprint_height * 0.5 + 1.0).round().clamp(2.0, 4.0);
                let shadow_x = ground_origin.x as f32 - shadow_width * 0.5;
                let shadow_y = footprint_pos.y as f32 + footprint_height - shadow_height;

                Some(GroundShadow {
                    position: glam::Vec2::new(shadow_x, shadow_y),
                    size: glam::Vec2::new(shadow_width, shadow_height),
                    color: [0.0, 0.0, 0.0, 0.28],
                })
            })
            .collect()
    }

    pub fn sprite_render_requests(&self) -> Vec<SpriteRenderRequest> {
        let mut requests = Vec::new();
        let mut animated_sequence = 0_u32;
        let mut static_sequence = 0_u32;
        let mut projectile_sequence = 0_u32;

        for entity_id in self.entity_manager.active_entities_iter() {
            let Some(entity) = self.entity_manager.get_entity(entity_id) else {
                continue;
            };
            if !entity.attributes.rendering.visible || !entity.attributes.behavior.active {
                continue;
            }

            if let Some(animation_controller) = &entity.attributes.rendering.animation_controller {
                let Ok(atlas_name) = animation_controller.current_atlas_name() else {
                    continue;
                };
                let Ok(tile_name) = animation_controller.current_tile_name() else {
                    continue;
                };

                requests.push(SpriteRenderRequest {
                    origin: SpriteRenderOrigin::AnimatedEntity(entity_id),
                    sort_key: SpriteSortKey {
                        primary: entity.ground_contact_y(),
                        secondary: entity.attributes.rendering.render_layer,
                        sequence: animated_sequence,
                    },
                    visual: SpriteVisualRef::AtlasTile {
                        atlas_name,
                        tile_name,
                    },
                    position: entity.position,
                    size: SpriteRenderSize::Explicit(entity.size),
                    palette_override: entity.attributes.rendering.palette_override.clone(),
                    flip_x: GameState::animation_state_flip_x(
                        animation_controller.current_clip_state,
                    ),
                });
                animated_sequence += 1;
                continue;
            }

            if let Some(static_render) = &entity.attributes.rendering.static_object_render {
                requests.push(SpriteRenderRequest {
                    origin: SpriteRenderOrigin::StaticEntity(entity_id),
                    sort_key: SpriteSortKey {
                        primary: entity.ground_contact_y(),
                        secondary: entity.attributes.rendering.render_layer,
                        sequence: static_sequence,
                    },
                    visual: SpriteVisualRef::ObjectSheetObject {
                        sheet_name: static_render.sheet.clone(),
                        object_name: static_render.object_name.clone(),
                    },
                    position: entity.position,
                    size: SpriteRenderSize::Explicit(entity.size),
                    palette_override: None,
                    flip_x: false,
                });
                static_sequence += 1;
                continue;
            }

            if let Some(projectile) = self
                .entity_manager
                .storage()
                .components()
                .projectile(entity_id)
            {
                requests.push(SpriteRenderRequest {
                    origin: SpriteRenderOrigin::Projectile(entity_id),
                    sort_key: SpriteSortKey {
                        primary: entity.ground_contact_y(),
                        secondary: entity.attributes.rendering.render_layer,
                        sequence: projectile_sequence,
                    },
                    visual: SpriteVisualRef::ObjectSheetObject {
                        sheet_name: projectile.sheet.clone(),
                        object_name: projectile.object_name.clone(),
                    },
                    position: entity.position,
                    size: SpriteRenderSize::Explicit(entity.size),
                    palette_override: None,
                    flip_x: false,
                });
                projectile_sequence += 1;
            }
        }

        requests
    }

    pub fn current_sprite_frame(&self, atlas: &AtlasMeta, texture_size: glam::UVec2) -> SpriteFrame {
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

    pub fn player_position(&self) -> glam::IVec2 {
        self.player_id
            .and_then(|player_id| self.entity_manager.get_entity(player_id))
            .map(|player| player.position)
            .unwrap_or(glam::IVec2::ZERO)
    }

    pub fn entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
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

    pub fn solid_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)> {
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

    pub fn trigger_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)> {
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

#[cfg(test)]
mod tests {
    use super::{GroundShadow, RenderQueryService};
    use crate::entity::{
        EntityAttributes, EntityBehavior, EntityFootprint, EntityGameplay, EntityGrounding,
        EntityManager, EntityRendering, EntityStats, ProjectileState, StaticObjectRenderDef,
    };

    fn entity_attributes() -> EntityAttributes {
        EntityAttributes::default()
    }

    #[test]
    fn health_bar_queries_filter_invisible_and_inactive_entities() {
        let mut entity_manager = EntityManager::new();
        let visible_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(4, 8),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                gameplay: EntityGameplay {
                    health: Some(20),
                    stats: EntityStats::from_legacy_health(Some(20)),
                    ..EntityGameplay::default()
                },
                ..entity_attributes()
            },
        );
        let hidden_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(20, 8),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                gameplay: EntityGameplay {
                    health: Some(20),
                    stats: EntityStats::from_legacy_health(Some(20)),
                    ..EntityGameplay::default()
                },
                rendering: EntityRendering {
                    visible: false,
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );
        let inactive_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(36, 8),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                gameplay: EntityGameplay {
                    health: Some(20),
                    stats: EntityStats::from_legacy_health(Some(20)),
                    ..EntityGameplay::default()
                },
                behavior: EntityBehavior {
                    active: false,
                    ..EntityBehavior::default()
                },
                ..entity_attributes()
            },
        );

        let facade = RenderQueryService::new(&entity_manager, None, false);
        let bars = facade.entity_health_bars();

        assert_eq!(bars.len(), 1);
        assert_eq!(bars[0].entity_id, visible_id);
        assert_ne!(bars[0].entity_id, hidden_id);
        assert_ne!(bars[0].entity_id, inactive_id);
    }

    #[test]
    fn ground_shadow_queries_filter_non_renderable_entities() {
        let mut entity_manager = EntityManager::new();
        entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(10, 20),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "crate".to_string(),
                    }),
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );
        entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(30, 20),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "hidden".to_string(),
                    }),
                    visible: false,
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );
        entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(50, 20),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "inactive".to_string(),
                    }),
                    has_shadow: false,
                    ..EntityRendering::default()
                },
                behavior: EntityBehavior {
                    active: false,
                    ..EntityBehavior::default()
                },
                ..entity_attributes()
            },
        );
        let projectile_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Projectile,
            glam::IVec2::new(70, 20),
            glam::UVec2::new(8, 8),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "bullet".to_string(),
                    }),
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );
        entity_manager.storage_mut().components_mut().set_projectile(
            projectile_id,
            Some(ProjectileState {
                sheet: "objects".to_string(),
                object_name: "bullet".to_string(),
                size: [8, 8],
                velocity: [1, 0],
                remaining_ticks: 10,
                damage: 1,
                owner_id: None,
            }),
        );

        let facade = RenderQueryService::new(&entity_manager, None, false);
        let shadows = facade.entity_ground_shadows();

        assert_eq!(shadows.len(), 1);
    }

    #[test]
    fn ground_shadow_queries_project_flattened_shadow_at_entity_base() {
        let mut entity_manager = EntityManager::new();
        entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(10, 20),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "crate".to_string(),
                    }),
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );

        let facade = RenderQueryService::new(&entity_manager, None, false);
        let shadows = facade.entity_ground_shadows();

        assert_eq!(
            shadows,
            vec![GroundShadow {
                position: glam::Vec2::new(11.5, 32.0),
                size: glam::Vec2::new(13.0, 4.0),
                color: [0.0, 0.0, 0.0, 0.28],
            }]
        );
    }

    #[test]
    fn ground_shadow_queries_follow_narrow_footprint_instead_of_full_sprite_width() {
        let mut entity_manager = EntityManager::new();
        entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(10, 20),
            glam::UVec2::new(32, 48),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "tree".to_string(),
                    }),
                    grounding: EntityGrounding {
                        origin: Some([16, 47]),
                        footprint: Some(EntityFootprint::new([8, 40], [16, 8])),
                    },
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );

        let facade = RenderQueryService::new(&entity_manager, None, false);
        let shadows = facade.entity_ground_shadows();

        assert_eq!(
            shadows,
            vec![GroundShadow {
                position: glam::Vec2::new(16.0, 64.0),
                size: glam::Vec2::new(20.0, 4.0),
                color: [0.0, 0.0, 0.0, 0.28],
            }]
        );
    }

    #[test]
    fn ground_shadow_queries_keep_small_footprint_entities_readable() {
        let mut entity_manager = EntityManager::new();
        entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(10, 20),
            glam::UVec2::new(16, 16),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "player".to_string(),
                    }),
                    grounding: EntityGrounding {
                        origin: Some([8, 15]),
                        footprint: Some(EntityFootprint::new([4, 12], [8, 4])),
                    },
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );

        let facade = RenderQueryService::new(&entity_manager, None, false);
        let shadows = facade.entity_ground_shadows();

        assert_eq!(
            shadows,
            vec![GroundShadow {
                position: glam::Vec2::new(12.0, 33.0),
                size: glam::Vec2::new(12.0, 3.0),
                color: [0.0, 0.0, 0.0, 0.28],
            }]
        );
    }

    #[test]
    fn sprite_render_requests_sort_entities_by_ground_contact_y() {
        let mut entity_manager = EntityManager::new();
        let lower_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(0, 0),
            glam::UVec2::new(32, 48),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "lower".to_string(),
                    }),
                    grounding: EntityGrounding {
                        origin: Some([16, 47]),
                        footprint: Some(EntityFootprint::new([8, 40], [16, 8])),
                    },
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );
        let upper_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Npc,
            glam::IVec2::new(0, 24),
            glam::UVec2::new(32, 48),
            EntityAttributes {
                rendering: EntityRendering {
                    static_object_render: Some(StaticObjectRenderDef {
                        sheet: "objects".to_string(),
                        object_name: "upper".to_string(),
                    }),
                    grounding: EntityGrounding {
                        origin: Some([16, 47]),
                        footprint: Some(EntityFootprint::new([8, 40], [16, 8])),
                    },
                    ..EntityRendering::default()
                },
                ..entity_attributes()
            },
        );

        let facade = RenderQueryService::new(&entity_manager, None, false);
        let mut requests = facade.sprite_render_requests();
        crate::sprite_render::sort_sprite_render_requests(&mut requests);

        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0].origin,
            crate::sprite_render::SpriteRenderOrigin::StaticEntity(lower_id)
        );
        assert_eq!(
            requests[1].origin,
            crate::sprite_render::SpriteRenderOrigin::StaticEntity(upper_id)
        );
        assert!(requests[0].sort_key.primary < requests[1].sort_key.primary);
    }
}
