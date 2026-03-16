
use super::{can_entity_move_to_position, can_place_collision_box_at_position, CollisionBox};
use crate::assets::atlas::{AtlasMeta, TileInfo, TileProperties};
use crate::assets::tilemap::{MapObjectInstance, TileMap};
use crate::entity::{Entity, EntityAttributes, EntityKind};
use glam::{IVec2, UVec2};
use std::collections::HashMap;
use std::path::PathBuf;

fn collision_assets_with_center_solid_tile() -> (TileMap, AtlasMeta) {
    let mut tiles = HashMap::new();
    tiles.insert(
        "solid".to_string(),
        TileInfo {
            position: UVec2::new(0, 0),
            properties: TileProperties {
                solid: true,
                trigger: false,
            },
        },
    );
    tiles.insert(
        "floor".to_string(),
        TileInfo {
            position: UVec2::new(1, 0),
            properties: TileProperties {
                solid: false,
                trigger: false,
            },
        },
    );

    let atlas = AtlasMeta {
        image: PathBuf::from("test.png"),
        tile_size: UVec2::new(16, 16),
        tiles,
    };

    // 3x3 map with center tile solid
    let tilemap = TileMap {
        size: UVec2::new(3, 3),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("test_atlas.json"),
        tiles: vec![
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "solid".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
            "floor".to_string(),
        ],
        objects: vec![],
    };

    (tilemap, atlas)
}

fn solid_entity() -> Entity {
    Entity {
        id: 1,
        position: IVec2::ZERO,
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: Some("test".to_string()),
        control_role: crate::entity::ControlRole::None,
        audio: crate::entity::EntityAudioSettings::default(),
        attributes: EntityAttributes::default(),
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
    }
}

#[test]
fn can_place_collision_box_right_or_bottom_edge_touch_is_not_collision() {
    let (tilemap, atlas) = collision_assets_with_center_solid_tile();
    let collision_box = CollisionBox::solid_box(UVec2::new(16, 16));

    // Right edge touching solid tile's left edge at x=16
    assert!(can_place_collision_box_at_position(
        Some(&collision_box),
        IVec2::new(0, 16),
        &tilemap,
        &atlas,
    ));

    // Bottom edge touching solid tile's top edge at y=16
    assert!(can_place_collision_box_at_position(
        Some(&collision_box),
        IVec2::new(16, 0),
        &tilemap,
        &atlas,
    ));
}

#[test]
fn can_place_collision_box_detects_one_pixel_overlap_on_right_or_bottom() {
    let (tilemap, atlas) = collision_assets_with_center_solid_tile();
    let collision_box = CollisionBox::solid_box(UVec2::new(16, 16));

    // One pixel into solid tile from the left
    assert!(!can_place_collision_box_at_position(
        Some(&collision_box),
        IVec2::new(1, 16),
        &tilemap,
        &atlas,
    ));

    // One pixel into solid tile from above
    assert!(!can_place_collision_box_at_position(
        Some(&collision_box),
        IVec2::new(16, 1),
        &tilemap,
        &atlas,
    ));
}

#[test]
fn can_entity_move_to_position_right_or_bottom_edge_touch_is_not_collision() {
    let (tilemap, atlas) = collision_assets_with_center_solid_tile();
    let entity = solid_entity();

    assert!(can_entity_move_to_position(
        &entity,
        IVec2::new(0, 16),
        &tilemap,
        &atlas,
    ));
    assert!(can_entity_move_to_position(
        &entity,
        IVec2::new(16, 0),
        &tilemap,
        &atlas,
    ));
}

#[test]
fn can_place_collision_box_rejects_overlap_with_solid_map_object() {
    let (mut tilemap, atlas) = collision_assets_with_center_solid_tile();
    tilemap.objects.push(MapObjectInstance {
        sheet: PathBuf::from("fauna.json"),
        object_name: "bush".to_string(),
        position: UVec2::new(16, 16),
        size_px: UVec2::new(16, 16),
        visible: true,
        solid: true,
    });
    let collision_box = CollisionBox::solid_box(UVec2::new(16, 16));

    assert!(!can_place_collision_box_at_position(
        Some(&collision_box),
        IVec2::new(16, 16),
        &tilemap,
        &atlas,
    ));
}

#[test]
fn can_place_collision_box_ignores_non_solid_map_object() {
    let (mut tilemap, atlas) = collision_assets_with_center_solid_tile();
    tilemap.objects.push(MapObjectInstance {
        sheet: PathBuf::from("fauna.json"),
        object_name: "bush".to_string(),
        position: UVec2::new(0, 0),
        size_px: UVec2::new(16, 16),
        visible: true,
        solid: false,
    });
    let collision_box = CollisionBox::solid_box(UVec2::new(16, 16));

    assert!(can_place_collision_box_at_position(
        Some(&collision_box),
        IVec2::new(0, 0),
        &tilemap,
        &atlas,
    ));
}
