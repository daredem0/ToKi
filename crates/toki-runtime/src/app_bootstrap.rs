use std::path::{Path, PathBuf};

use anyhow::Context;
use toki_core::dialog::DialogTree;
use toki_core::entity::EntityDefinition;
use toki_core::project_assets::{
    discover_project_dialog_paths, discover_project_entity_definition_paths,
    discover_project_scene_paths, first_existing_path, resolve_project_scene_path,
};
use toki_core::project_content::{
    build_game_state_from_project_content as shared_build_game_state_from_project_content,
    build_game_state_from_scene as shared_build_game_state_from_scene,
};
use toki_core::{GameState, Scene};
use toki_render::RenderError;

use crate::systems::{DecodedProjectCache, ResourceManager, RuntimeAssetLoadPlan};

use super::{App, RuntimeLaunchOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupErrorPolicy {
    Lenient,
    Strict,
}

struct ResolvedStartupRoot {
    root_path: PathBuf,
    pack_mount: Option<tempfile::TempDir>,
}

impl ResolvedStartupRoot {
    fn project(path: PathBuf) -> Self {
        Self {
            root_path: path,
            pack_mount: None,
        }
    }

    fn pack(mount: tempfile::TempDir) -> Self {
        let root_path = mount.path().to_path_buf();
        Self {
            root_path,
            pack_mount: Some(mount),
        }
    }
}

#[derive(Debug)]
struct PreloadedProjectContent {
    scenes: Vec<Scene>,
    entity_definitions: Vec<EntityDefinition>,
    dialogs: Vec<DialogTree>,
}

struct StartupBundle {
    resources: ResourceManager,
    game_state: GameState,
    dialogs: Vec<DialogTree>,
    pack_mount: Option<tempfile::TempDir>,
    asset_load_plan: RuntimeAssetLoadPlan,
    decoded_project_cache: DecodedProjectCache,
}

type StartupStateParts = (
    ResourceManager,
    GameState,
    Vec<DialogTree>,
    Option<tempfile::TempDir>,
    RuntimeAssetLoadPlan,
    DecodedProjectCache,
);

impl StartupBundle {
    fn into_parts(self) -> StartupStateParts {
        (
            self.resources,
            self.game_state,
            self.dialogs,
            self.pack_mount,
            self.asset_load_plan,
            self.decoded_project_cache,
        )
    }
}

struct StartupCoordinator<'a> {
    launch_options: &'a RuntimeLaunchOptions,
}

impl<'a> StartupCoordinator<'a> {
    fn new(launch_options: &'a RuntimeLaunchOptions) -> Self {
        Self { launch_options }
    }

    fn load_with_policy<T>(
        error_policy: StartupErrorPolicy,
        project_path: &Path,
        content_kind: &str,
        result: anyhow::Result<Vec<T>>,
    ) -> anyhow::Result<Vec<T>> {
        match result {
            Ok(items) => Ok(items),
            Err(error) => match error_policy {
                StartupErrorPolicy::Strict => Err(error),
                StartupErrorPolicy::Lenient => {
                    tracing::error!(
                        "Failed to preload {} from '{}': {}",
                        content_kind,
                        project_path.display(),
                        error
                    );
                    Ok(Vec::new())
                }
            },
        }
    }

    fn build(&self) -> StartupBundle {
        if let Some(pack_path) = &self.launch_options.pack_path {
            return self.build_from_pack(pack_path).unwrap_or_else(|error| {
                panic!(
                    "Failed to initialize runtime from pack '{}': {}",
                    pack_path.display(),
                    error
                )
            });
        }

        let mut decoded_project_cache = DecodedProjectCache::default();
        if let Some(project_path) = &self.launch_options.project_path {
            tracing::info!("Loading project from: {}", project_path.display());
            let startup_root = ResolvedStartupRoot::project(project_path.clone());
            match self.build_from_content_root(
                startup_root,
                StartupErrorPolicy::Lenient,
                &mut decoded_project_cache,
            ) {
                Ok(bundle) => return bundle,
                Err(error) => {
                    tracing::error!(
                        "Failed to initialize runtime from project '{}': {}",
                        project_path.display(),
                        error
                    );
                }
            }
        }

        match ResourceManager::load_all() {
            Ok(resources) => StartupBundle {
                resources,
                game_state: App::fallback_game_state(),
                dialogs: Vec::new(),
                pack_mount: None,
                asset_load_plan: RuntimeAssetLoadPlan {
                    scene_name: self.launch_options.scene_name.clone(),
                    map_name: self.launch_options.map_name.clone(),
                    tilemap_texture_path: None,
                    sprite_texture_path: None,
                    preloaded_sfx_names: crate::systems::asset_loading::common_preloaded_sfx_names(
                    ),
                    stream_music: true,
                },
                decoded_project_cache,
            },
            Err(error) => {
                panic!("Failed to initialize runtime resources: {error}");
            }
        }
    }

    fn build_from_pack(&self, pack_path: &Path) -> anyhow::Result<StartupBundle> {
        let mount = crate::pack::extract_pak_to_tempdir(pack_path)?;
        let startup_root = ResolvedStartupRoot::pack(mount);
        let mut decoded_project_cache = DecodedProjectCache::default();
        self.build_from_content_root(
            startup_root,
            StartupErrorPolicy::Strict,
            &mut decoded_project_cache,
        )
        .map_err(anyhow::Error::msg)
    }

    fn build_from_content_root(
        &self,
        startup_root: ResolvedStartupRoot,
        error_policy: StartupErrorPolicy,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> anyhow::Result<StartupBundle> {
        let project_path = startup_root.root_path.clone();
        let preloaded =
            self.preload_project_content(&project_path, error_policy, decoded_project_cache)?;
        let scene = self
            .resolve_startup_scene(&preloaded.scenes, self.launch_options.scene_name.as_deref());
        let map_name = self.resolve_startup_map_name(scene.as_ref());
        let (resources, asset_load_plan) = App::load_project_resources_with_cache(
            &project_path,
            self.launch_options.scene_name.as_deref(),
            map_name.as_deref(),
            decoded_project_cache,
        )
        .context("failed to load project resources")?;
        let mut resources = resources;
        if self
            .launch_options
            .display
            .indexed_palette_override
            .is_some()
        {
            resources.set_indexed_palette_override(
                self.launch_options.display.indexed_palette_override.clone(),
            );
        }
        let game_state = if let Some(scene_name) = self.launch_options.scene_name.as_deref() {
            App::game_state_from_project_content(
                preloaded.scenes,
                preloaded.entity_definitions,
                scene_name,
                &self.launch_options.flags.declarations,
            )
        } else {
            App::fallback_game_state()
        };

        Ok(StartupBundle {
            resources,
            game_state,
            dialogs: preloaded.dialogs,
            pack_mount: startup_root.pack_mount,
            asset_load_plan,
            decoded_project_cache: std::mem::take(decoded_project_cache),
        })
    }

    fn preload_project_content(
        &self,
        project_path: &Path,
        error_policy: StartupErrorPolicy,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> anyhow::Result<PreloadedProjectContent> {
        let scenes = Self::load_with_policy(
            error_policy,
            project_path,
            "project scenes",
            App::load_all_project_scenes_with_cache(project_path, decoded_project_cache),
        )?;

        let entity_definitions = Self::load_with_policy(
            error_policy,
            project_path,
            "entity definitions",
            App::load_project_entity_definitions_with_cache(project_path, decoded_project_cache),
        )?;

        let dialogs = Self::load_with_policy(
            error_policy,
            project_path,
            "dialogs",
            App::load_project_dialogs_with_cache(project_path, decoded_project_cache),
        )?;

        Ok(PreloadedProjectContent {
            scenes,
            entity_definitions,
            dialogs,
        })
    }

    fn resolve_startup_scene(&self, scenes: &[Scene], scene_name: Option<&str>) -> Option<Scene> {
        scene_name.and_then(|scene_name| {
            scenes
                .iter()
                .find(|scene| scene.name == scene_name)
                .cloned()
        })
    }

    fn resolve_startup_map_name(&self, scene: Option<&Scene>) -> Option<String> {
        self.launch_options
            .map_name
            .clone()
            .or_else(|| scene.and_then(|loaded_scene| loaded_scene.maps.first().cloned()))
    }
}

impl App {
    pub(super) fn build_startup_state(launch_options: &RuntimeLaunchOptions) -> StartupStateParts {
        StartupCoordinator::new(launch_options).build().into_parts()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn build_startup_state_from_pack(
        launch_options: &RuntimeLaunchOptions,
        pack_path: &Path,
    ) -> anyhow::Result<StartupStateParts> {
        Ok(StartupCoordinator::new(launch_options)
            .build_from_pack(pack_path)?
            .into_parts())
    }

    pub(super) fn load_project_resources_with_cache(
        project_path: &Path,
        scene_name: Option<&str>,
        map_name: Option<&str>,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> Result<(ResourceManager, RuntimeAssetLoadPlan), RenderError> {
        let (resources, resolved) = ResourceManager::load_for_project_with_cache(
            project_path,
            map_name,
            decoded_project_cache,
        )?;
        let asset_load_plan = RuntimeAssetLoadPlan::from_resolved_paths(
            scene_name.map(str::to_string),
            map_name.map(str::to_string),
            &resolved,
        );
        Ok((resources, asset_load_plan))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn load_project_scene_with_cache(
        project_path: &Path,
        scene_name: &str,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> anyhow::Result<Scene> {
        let scene_path = resolve_project_scene_path(project_path, scene_name)
            .with_context(|| format!("could not resolve scene file for '{scene_name}'"))?;
        decoded_project_cache
            .load_scene_from_path(&scene_path)
            .map_err(anyhow::Error::msg)
    }

    pub(super) fn load_all_project_scenes_with_cache(
        project_path: &Path,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> anyhow::Result<Vec<Scene>> {
        let scene_paths = discover_project_scene_paths(project_path).with_context(|| {
            format!(
                "failed to discover scenes under '{}'",
                project_path.display()
            )
        })?;
        let mut scenes = Vec::new();
        for (_, path) in scene_paths {
            scenes.push(
                decoded_project_cache
                    .load_scene_from_path(&path)
                    .map_err(anyhow::Error::msg)?,
            );
        }
        scenes.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(scenes)
    }

    pub(super) fn load_project_entity_definitions_with_cache(
        project_path: &Path,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> anyhow::Result<Vec<EntityDefinition>> {
        let definition_paths = discover_project_entity_definition_paths(project_path)
            .with_context(|| {
                format!(
                    "failed to discover entity definitions under '{}'",
                    project_path.display()
                )
            })?;
        let mut definitions = Vec::new();
        for path in definition_paths {
            definitions.push(
                decoded_project_cache
                    .load_entity_definition_from_path(&path)
                    .map_err(anyhow::Error::msg)?,
            );
        }
        definitions.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(definitions)
    }

    pub(super) fn load_project_dialogs_with_cache(
        project_path: &Path,
        decoded_project_cache: &mut DecodedProjectCache,
    ) -> anyhow::Result<Vec<DialogTree>> {
        let dialog_paths = discover_project_dialog_paths(project_path).with_context(|| {
            format!(
                "failed to discover dialogs under '{}'",
                project_path.display()
            )
        })?;
        let mut dialogs = Vec::new();
        for path in dialog_paths {
            dialogs.push(
                decoded_project_cache
                    .load_dialog_from_path(&path)
                    .map_err(anyhow::Error::msg)?,
            );
        }
        dialogs.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(dialogs)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn game_state_from_scene(scene: Scene) -> GameState {
        let scene_name = scene.name.clone();
        match shared_build_game_state_from_scene(scene, std::iter::empty()) {
            Ok(game_state) => game_state,
            Err(error) => {
                tracing::error!(
                    "Failed to load startup scene '{}' into game state: {}",
                    scene_name,
                    error
                );
                Self::fallback_game_state()
            }
        }
    }

    pub(super) fn game_state_from_project_content(
        scenes: Vec<Scene>,
        entity_definitions: Vec<EntityDefinition>,
        startup_scene_name: &str,
        flag_declarations: &[toki_core::project_runtime::ProjectFlagDefinition],
    ) -> GameState {
        match shared_build_game_state_from_project_content(
            scenes,
            entity_definitions,
            startup_scene_name,
        ) {
            Ok(mut game_state) => {
                game_state.apply_flag_defaults(flag_declarations);
                game_state
            }
            Err(error) => {
                tracing::error!(
                    "Failed to load startup scene '{}' into game state: {}",
                    startup_scene_name,
                    error
                );
                Self::fallback_game_state()
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use toki_core::entity::{
        AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition, MovementProfile,
        MovementSoundTrigger, RenderingDef,
    };
    use toki_core::FlagValue;

    fn write_minimal_entity_definition(project_path: &Path, name: &str) {
        let entities_dir = project_path.join("entities");
        fs::create_dir_all(&entities_dir).expect("entities dir");
        let definition = EntityDefinition {
            name: name.to_string(),
            display_name: name.to_string(),
            description: String::new(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                has_shadow: true,
                palette_override: None,
                static_object: None,
                grounding: Default::default(),
            },
            attributes: AttributesDef {
                health: Some(100),
                stats: std::collections::HashMap::new(),
                speed: 1.0,
                solid: true,
                active: true,
                can_move: true,
                interactable: false,
                interaction_reach: 0,
                ai_config: toki_core::entity::AiConfig::default(),
                movement_profile: MovementProfile::None,
                primary_projectile: None,
                pickup: None,
                has_inventory: false,
            },
            collision: CollisionDef {
                enabled: false,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
            },
            audio: AudioDef {
                footstep_trigger_distance: 16.0,
                hearing_radius: 0,
                movement_sound_trigger: MovementSoundTrigger::Distance,
                movement_sound: String::new(),
                collision_sound: None,
            },
            animations: AnimationsDef {
                atlas_name: "creatures".to_string(),
                clips: Vec::new(),
                default_state: "idle".to_string(),
            },
            category: "test".to_string(),
            tags: Vec::new(),
        };
        fs::write(
            entities_dir.join(format!("{name}.json")),
            serde_json::to_string_pretty(&definition).expect("serialize"),
        )
        .expect("write entity definition");
    }

    #[test]
    fn game_state_from_project_content_applies_flag_defaults() {
        let scene = Scene::new("Main".to_string());
        let game_state = App::game_state_from_project_content(
            vec![scene],
            Vec::new(),
            "Main",
            &[toki_core::project_runtime::ProjectFlagDefinition {
                id: "quest_done".to_string(),
                default_value: FlagValue::Bool(true),
            }],
        );

        assert_eq!(game_state.flag("quest_done"), Some(&FlagValue::Bool(true)));
    }

    #[test]
    fn preload_project_content_is_lenient_for_unpacked_projects() {
        let temp = tempfile::tempdir().expect("tempdir");
        let scenes_dir = temp.path().join("scenes");
        fs::create_dir_all(&scenes_dir).expect("scenes dir");
        fs::write(scenes_dir.join("broken.json"), "{not valid json").expect("write broken scene");
        write_minimal_entity_definition(temp.path(), "player");

        let launch_options = RuntimeLaunchOptions {
            project_path: Some(temp.path().to_path_buf()),
            ..RuntimeLaunchOptions::default()
        };
        let coordinator = StartupCoordinator::new(&launch_options);
        let mut cache = DecodedProjectCache::default();

        let content = coordinator
            .preload_project_content(temp.path(), StartupErrorPolicy::Lenient, &mut cache)
            .expect("lenient preload should not fail");

        assert!(content.scenes.is_empty(), "broken scenes should be ignored");
        assert_eq!(content.entity_definitions.len(), 1);
        assert_eq!(content.entity_definitions[0].name, "player");
    }

    #[test]
    fn preload_project_content_is_strict_for_pack_like_startup() {
        let temp = tempfile::tempdir().expect("tempdir");
        let scenes_dir = temp.path().join("scenes");
        fs::create_dir_all(&scenes_dir).expect("scenes dir");
        fs::write(scenes_dir.join("broken.json"), "{not valid json").expect("write broken scene");

        let launch_options = RuntimeLaunchOptions::default();
        let coordinator = StartupCoordinator::new(&launch_options);
        let mut cache = DecodedProjectCache::default();

        let error = coordinator
            .preload_project_content(temp.path(), StartupErrorPolicy::Strict, &mut cache)
            .expect_err("strict preload should fail");
        let error = error.to_string();
        assert!(error.contains("broken.json") || error.contains("Failed to parse scene"));
    }
}
