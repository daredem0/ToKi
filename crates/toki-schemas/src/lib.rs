//! Canonical JSON schema payloads used across ToKi tooling.

pub const SCENE_SCHEMA: &str = include_str!("../schemas/scene.json");
pub const ENTITY_SCHEMA: &str = include_str!("../schemas/entity.json");
pub const ATLAS_SCHEMA: &str = include_str!("../schemas/atlas.json");
pub const MAP_SCHEMA: &str = include_str!("../schemas/map.json");

pub const SCHEMA_FILES: [(&str, &str); 4] = [
    ("scene", SCENE_SCHEMA),
    ("entity", ENTITY_SCHEMA),
    ("atlas", ATLAS_SCHEMA),
    ("map", MAP_SCHEMA),
];
