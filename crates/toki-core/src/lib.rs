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

pub mod assets {
    pub mod atlas;
    pub mod tilemap;
}

pub mod camera;
pub use camera::{Camera, CameraController, CameraMode};

pub mod game;
pub use game::{GameState, InputKey};

pub mod timing;
pub use timing::{TimestepIterator, TimingSystem};

pub mod entity;

pub mod collision;
pub use collision::{CollisionBox, CollisionResult};
