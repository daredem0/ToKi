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
pub use camera::{Camera, CameraController, CameraMode, Entity};

pub mod game;
pub use game::{GameState, InputKey};
