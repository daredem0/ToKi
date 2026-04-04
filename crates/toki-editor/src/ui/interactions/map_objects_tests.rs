use super::MapObjectInteraction;
use glam::{IVec2, UVec2};
use std::path::PathBuf;
use toki_core::assets::tilemap::{TileLayer, TileMap};
use toki_core::entity::{build_decoration_entity, DecorationSpec};

fn sample_tilemap() -> TileMap {
    TileMap {
        size: UVec2::new(4, 4),
        tile_size: UVec2::new(16, 16),
        atlas: PathBuf::from("terrain.json"),
        layers: vec![TileLayer::new("ground", vec!["grass".to_string(); 16])],
    }
}

fn sample_decoration(id: u32, position: IVec2, visible: bool) -> toki_core::entity::Entity {
    let mut entity = build_decoration_entity(
        id,
        DecorationSpec::new(
            position,
            UVec2::new(16, 16),
            "fauna.json",
            format!("object_{id}"),
        ),
    );
    entity.rendering.visible = visible;
    entity
}

#[test]
fn object_anchor_at_world_snaps_to_tile_grid() {
    let tilemap = sample_tilemap();

    assert_eq!(
        MapObjectInteraction::object_anchor_at_world(&tilemap, glam::Vec2::new(23.9, 31.9)),
        Some(UVec2::new(16, 16))
    );
}

#[test]
fn object_entity_at_world_prefers_topmost_visible_decoration() {
    let first = sample_decoration(1, IVec2::new(16, 16), true);
    let second = sample_decoration(2, IVec2::new(16, 16), true);

    assert_eq!(
        MapObjectInteraction::object_entity_at_world(
            [&first, &second],
            glam::Vec2::new(20.0, 20.0)
        ),
        Some(2)
    );
}

#[test]
fn object_entity_at_world_ignores_invisible_decorations() {
    let hidden = sample_decoration(7, IVec2::new(16, 16), false);

    assert_eq!(
        MapObjectInteraction::object_entity_at_world([&hidden], glam::Vec2::new(20.0, 20.0)),
        None
    );
}
