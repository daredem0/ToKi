#![doc = include_str!("../../../README.md")]
#![doc = "\n\n"]
#![doc = include_str!("../../../docs/SDD_SAD.md")]

pub mod graphics {
    pub mod image;
    pub mod vertex;
}

pub mod math {
    pub mod coordinates;
    pub mod projection;
}
pub mod errors;
pub use errors::CoreError;
pub mod pack;
pub mod project_assets;
pub mod project_runtime;
pub use project_runtime::TimingMode;

pub mod fonts;
pub mod menu;
pub mod sprite;
pub mod sprite_render;
pub mod text;
pub mod ui;

pub mod animation;
pub mod assets {
    pub mod atlas;
    pub mod object_sheet;
    pub mod tilemap;
}

pub mod camera;
pub use camera::{Camera, CameraController, CameraMode};

pub mod events;
pub use events::{EventHandler, EventQueue, GameEvent, GameUpdateResult};

pub mod game;
pub use game::{GameState, InputAction, InputKey, DEFAULT_TIMESTEP_MS};

pub mod timing;
pub use timing::{TimestepIterator, TimingSystem};

pub mod entity;
pub mod rules;
pub mod scene;
pub use scene::Scene;

pub mod scene_manager;
pub use scene_manager::SceneManager;

pub mod collision;
pub use collision::{CollisionBox, CollisionResult};

pub mod resources;
pub use resources::{ResourceError, ResourceManager};

pub mod serialization;

pub mod asset_cache;
pub use asset_cache::AssetCache;
