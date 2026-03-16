use std::path::{Path, PathBuf};

use toki_core::assets::object_sheet::ObjectSheetMeta;
use toki_core::{GameState, Scene};
use toki_render::RenderError;

use crate::systems::resources::resolve_project_resource_paths;
use crate::systems::{DecodedProjectCache, ResourceManager, RuntimeAssetLoadPlan};

use super::{App, RuntimeLaunchOptions};

impl App {
    pub(super) fn build_startup_state(
        launch_options: &RuntimeLaunchOptions,
    ) -> (
        ResourceManager,
        GameState,
        Option<tempfile::TempDir>,
        RuntimeAssetLoadPlan,
        DecodedProjectCache,
    ) {
        if let Some(pack_path) = &launch_options.pack_path {
            return Self::build_startup_state_from_pack(launch_options, pack_path).unwrap_or_else(
                |error| {
                    panic!(
                        "Failed to initialize runtime from pack '{}': {}",
                        pack_path.display(),
                        error
                    )
                },
            );
        }

        let mut decoded_project_cache = DecodedProjectCache::default();
        if let Some(project_path) = &launch_options.project_path {
            let scene = launch_options.scene_name.as_deref().and_then(|scene_name| {
                Self::load_project_scene_with_cache(
                    project_path,
                    scene_name,
                    &mut decoded_project_cache,
                )
                .ok()
            });

            let map_name = launch_options.map_name.clone().or_else(|| {
                scene
                    .as_ref()
                    .and_then(|loaded_scene| loaded_scene.maps.first().cloned())
            });

            match Self::load_project_resources_with_cache(
                project_path,
                launch_options.scene_name.as_deref(),
                map_name.as_deref(),
                &mut decoded_project_cache,
            ) {
                Ok((resources, asset_load_plan)) => {
                    let game_state = if let Some(scene) = scene {
                        Self::game_state_from_scene(scene)
                    } else {
                        Self::fallback_game_state()
                    };
                    return (
                        resources,
                        game_state,
                        None,
                        asset_load_plan,
                        decoded_project_cache,
                    );
                }
                Err(error) => {
                    tracing::error!(
                        "Failed to load project resources for '{}': {}",
                        project_path.display(),
                        error
                    );
                }
            }
        }

        match ResourceManager::load_all() {
            Ok(resources) => (
                resources,
                Self::fallback_game_state(),
                None,
                RuntimeAssetLoadPlan {
                    scene_name: launch_options.scene_name.clone(),
                    map_name: launch_options.map_name.clone(),
                    tilemap_texture_path: None,
                    sprite_texture_path: None,
                    preloaded_sfx_names: crate::systems::asset_loading::common_preloaded_sfx_names(
                    ),
                    stream_music: true,
                },
                decoded_project_cache,
            ),
            Err(error) => {
                panic!("Failed to initialize runtime resources: {error}");
            }
        }
    }

    pub(super) fn build_startup_state_from_pack(
        launch_options: &RuntimeLaunchOptions,
        pack_path: &Path,
    ) -> anyhow::Result<(
        ResourceManager,
        GameState,
        Option<tempfile::TempDir>,
        RuntimeAssetLoadPlan,
        DecodedProjectCache,
    )> {
        let mount = crate::pack::extract_pak_to_tempdir(pack_path)?;
        let mount_path = mount.path().to_path_buf();
        let mut decoded_project_cache = DecodedProjectCache::default();
        let scene = launch_options
            .scene_name
            .as_deref()
            .map(|scene_name| {
                Self::load_project_scene_with_cache(
                    &mount_path,
                    scene_name,
                    &mut decoded_project_cache,
                )
            })
            .transpose()
            .map_err(anyhow::Error::msg)?;
        let map_name = launch_options.map_name.clone().or_else(|| {
            scene
                .as_ref()
                .and_then(|loaded_scene| loaded_scene.maps.first().cloned())
        });
        let (resources, asset_load_plan) = Self::load_project_resources_with_cache(
            &mount_path,
            launch_options.scene_name.as_deref(),
            map_name.as_deref(),
            &mut decoded_project_cache,
        )?;
        let game_state = if let Some(scene) = scene {
            Self::game_state_from_scene(scene)
        } else {
            Self::fallback_game_state()
        };
        Ok((
            resources,
            game_state,
            Some(mount),
            asset_load_plan,
            decoded_project_cache,
        ))
    }

    pub(super) fn load_project_resources_with_cache(
        project_path: &Path,
        scene_name: Option<&str>,
        map_name: Option<&str>,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> Result<(ResourceManager, RuntimeAssetLoadPlan), RenderError> {
        let resolved = resolve_project_resource_paths(project_path, map_name)?;
        let tilemap = decoded_project_cache.load_tilemap_from_path(&resolved.tilemap_path)?;
        tilemap.validate()?;
        let terrain_atlas =
            decoded_project_cache.load_atlas_from_path(&resolved.terrain_atlas_path)?;
        let mut sprite_atlases = std::collections::HashMap::new();
        let mut sprite_texture_paths = std::collections::HashMap::new();
        let mut object_sheets = std::collections::HashMap::new();
        let mut object_texture_paths = std::collections::HashMap::new();
        for atlas_path in &resolved.sprite_atlas_paths {
            let atlas = decoded_project_cache.load_atlas_from_path(atlas_path)?;
            let texture_path = crate::systems::resources::resolve_atlas_texture_path(atlas_path)?;
            if let Some(file_name) = atlas_path.file_name().and_then(|name| name.to_str()) {
                sprite_atlases.insert(file_name.to_string(), atlas.clone());
                sprite_texture_paths.insert(file_name.to_string(), texture_path.clone());
            }
            if let Some(stem) = atlas_path.file_stem().and_then(|name| name.to_str()) {
                sprite_atlases.insert(stem.to_string(), atlas);
                sprite_texture_paths.insert(stem.to_string(), texture_path);
            }
        }
        for object_sheet_path in &resolved.object_sheet_paths {
            let object_sheet = ObjectSheetMeta::load_from_file(object_sheet_path)?;
            let texture_path =
                crate::systems::resources::resolve_object_sheet_texture_path(object_sheet_path)?;
            if let Some(file_name) = object_sheet_path.file_name().and_then(|name| name.to_str()) {
                object_sheets.insert(file_name.to_string(), object_sheet.clone());
                object_texture_paths.insert(file_name.to_string(), texture_path.clone());
            }
            if let Some(stem) = object_sheet_path.file_stem().and_then(|name| name.to_str()) {
                object_sheets.insert(stem.to_string(), object_sheet);
                object_texture_paths.insert(stem.to_string(), texture_path);
            }
        }
        let resources = ResourceManager::from_preloaded(
            terrain_atlas,
            sprite_atlases,
            sprite_texture_paths,
            object_sheets,
            object_texture_paths,
            tilemap,
        );
        let asset_load_plan = RuntimeAssetLoadPlan::from_resolved_paths(
            scene_name.map(str::to_string),
            map_name.map(str::to_string),
            &resolved,
        );
        Ok((resources, asset_load_plan))
    }

    pub(super) fn load_project_scene_with_cache(
        project_path: &Path,
        scene_name: &str,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> Result<Scene, String> {
        let scene_path = project_path
            .join("scenes")
            .join(format!("{scene_name}.json"));
        decoded_project_cache.load_scene_from_path(&scene_path)
    }

    pub(super) fn game_state_from_scene(scene: Scene) -> GameState {
        let scene_name = scene.name.clone();
        let mut game_state = GameState::new_empty();
        game_state.add_scene(scene);
        if let Err(error) = game_state.load_scene(&scene_name) {
            tracing::error!(
                "Failed to load startup scene '{}' into game state: {}",
                scene_name,
                error
            );
            return Self::fallback_game_state();
        }
        game_state
    }

    pub(super) fn fallback_game_state() -> GameState {
        let mut game_state = GameState::new_empty();
        let _player_id = game_state.spawn_player_at(glam::IVec2::new(80, 72));
        let _npc_id = game_state.spawn_player_like_npc(glam::IVec2::new(120, 72));
        game_state
    }

    pub(super) fn project_texture_paths(project_path: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
        let tilemap_texture = first_existing_path(&[
            project_path
                .join("assets")
                .join("sprites")
                .join("terrain.png"),
            project_path.join("assets").join("terrain.png"),
        ]);
        let sprite_texture = first_existing_path(&[
            project_path
                .join("assets")
                .join("sprites")
                .join("creatures.png"),
            project_path.join("assets").join("creatures.png"),
        ]);
        (tilemap_texture, sprite_texture)
    }
}

pub(super) fn first_existing_path(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}
