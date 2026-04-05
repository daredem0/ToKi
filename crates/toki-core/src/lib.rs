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
pub(crate) mod io;
pub use errors::CoreError;
pub mod ids;
pub use ids::{DialogId, EntityDefName, SceneId, UiLayoutId, UiWidgetId};
pub mod cache_utils;
pub mod dialog;
pub mod dialog_runtime;
pub mod expression;
pub mod flags;
pub mod indexed_presentation;
pub mod pack;
pub mod palette;
pub mod project_assets;
pub mod project_content;
pub mod project_runtime;
pub use project_runtime::{SceneTransitionEffect, TimingMode};

pub mod fonts;
pub mod menu;
pub mod sprite;
pub mod sprite_render;
pub mod text;
pub mod ui;
pub mod ui_layout;
pub use ui_layout::{
    UiBinding, UiController, UiLayoutAsset, UiLayoutEngine, UiRequest, UiTheme, UiWidgetNode,
};

pub mod animation;
pub mod value_path;
pub mod assets {
    pub mod animated_tile;
    pub mod atlas;
    pub mod autotile;
    pub mod object_sheet;
    pub mod tile_animation;
    pub mod tilemap;
    pub mod tileset;

    #[cfg(test)]
    #[path = "animated_tile_tests.rs"]
    mod animated_tile_tests;

    #[cfg(test)]
    #[path = "autotile_tests.rs"]
    mod autotile_tests;

    #[cfg(test)]
    #[path = "tilemap_tests.rs"]
    mod tilemap_tests;
}

pub mod camera;
pub use camera::{Camera, CameraController, CameraMode};

pub mod events;
pub use events::{
    DialogStartRequest, EventHandler, EventQueue, GameEvent, GameUpdateResult, PersistenceRequest,
    SceneSwitchRequest,
};

pub mod ai;
pub use ai::AiSystem;

pub mod game;
pub use game::{GameState, InputAction, InputKey, RestoreError, DEFAULT_TIMESTEP_MS};

pub mod timing;
pub use flags::{FlagValue, GameFlags};
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
pub use serialization::{
    list_save_slot_metadata, load_save_data, load_save_data_from_slot, read_save_slot_metadata,
    save_game_to_slot, save_save_data, save_slot_file_path, PersistedSceneEntityState, SaveData,
    SaveSlotMetadata, SavedCameraState, MAX_SAVE_SLOTS, SAVE_DATA_VERSION,
};

pub mod asset_cache;
pub use asset_cache::AssetCache;
