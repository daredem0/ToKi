pub mod graphics {
    pub mod image;
    pub mod vertex;
}

pub mod math {
    pub mod projection;
}
pub mod errors;
pub use errors::CoreError;

pub mod sprite;

pub mod animation;
pub mod assets {
    pub mod atlas;
    pub mod tilemap;
}

pub mod camera;
pub use camera::{Camera, CameraController, CameraMode};

pub mod events;
pub use events::{EventHandler, EventQueue, GameEvent, GameUpdateResult};

pub mod game;
pub use game::{GameState, InputKey};

pub mod timing;
pub use timing::{TimestepIterator, TimingSystem};

pub mod entity;
pub mod scene;
pub use scene::Scene;

pub mod scene_manager;
pub use scene_manager::SceneManager;

pub mod collision;
pub use collision::{CollisionBox, CollisionResult};

pub mod resources;
pub use resources::{ResourceError, ResourceManager};

pub mod serialization;
