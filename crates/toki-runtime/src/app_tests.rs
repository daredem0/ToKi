use super::{
    first_existing_path, App, RuntimeAudioMixOptions, RuntimeDisplayOptions, RuntimeLaunchOptions,
    RuntimeSplashOptions, SplashPolicy, COMMUNITY_SPLASH_VERSION_TEXT,
    SPLASH_BRANDING_VERSION_GAP_PX, SPLASH_TEXT_HORIZONTAL_PADDING_PX,
    SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER, SPLASH_VERSION_DEFAULT_SIZE_PX, SPLASH_VERSION_MIN_SIZE_PX,
};
use std::fs;
use std::io::{Seek, Write};
use std::path::PathBuf;
use std::time::Duration;
use toki_core::math::projection::ProjectionParameter;
use toki_core::menu::MenuSettings;
use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger};
use toki_core::text::{TextStyle, TextWeight};
use toki_core::{
    entity::{
        AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
        MovementProfile, MovementSoundTrigger, RenderingDef,
    },
    scene::{SceneAnchor, SceneAnchorFacing, SceneAnchorKind, ScenePlayerEntry},
    Scene,
};

fn make_unique_temp_dir() -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("toki_runtime_app_tests_{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

fn load_project_scene(project_path: &std::path::Path, scene_name: &str) -> Result<Scene, String> {
    let mut decoded_project_cache = super::DecodedProjectCache::default();
    App::load_project_scene_with_cache(project_path, scene_name, &mut decoded_project_cache)
}

fn write_player_definition(project_path: &std::path::Path, name: &str) {
    let definition = EntityDefinition {
        name: name.to_string(),
        display_name: "Player".to_string(),
        description: "Player definition".to_string(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 1,
            visible: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::from([
                ("health".to_string(), 100),
                ("attack_power".to_string(), 8),
            ]),
            speed: 2.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 0,
            ai_config: toki_core::entity::AiConfig::default(),
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: None,
            pickup: None,
            has_inventory: true,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 100,
            movement_sound_trigger: MovementSoundTrigger::AnimationLoop,
            movement_sound: "sfx_step".to_string(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![
                AnimationClipDef {
                    state: "idle_down".to_string(),
                    frame_tiles: vec!["idle".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
                AnimationClipDef {
                    state: "idle_right".to_string(),
                    frame_tiles: vec!["idle".to_string()],
                    frame_positions: None,
                    frame_duration_ms: 300.0,
                    frame_durations_ms: None,
                    loop_mode: "loop".to_string(),
                },
            ],
            default_state: "idle_down".to_string(),
        },
        category: "human".to_string(),
        tags: vec!["player".to_string()],
    };
    let entities_dir = project_path.join("entities");
    fs::create_dir_all(&entities_dir).expect("entities dir");
    fs::write(
        entities_dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&definition).expect("serialize definition"),
    )
    .expect("write definition");
}
fn write_test_pak(pak_path: &std::path::Path, entries: &[(String, Vec<u8>)]) {
    let mut file = fs::File::create(pak_path).expect("create pak");
    file.write_all(b"TOKIPAK1").expect("magic");
    file.write_all(&0u64.to_le_bytes())
        .expect("offset placeholder");
    file.write_all(&0u64.to_le_bytes())
        .expect("size placeholder");

    let mut manifest_entries = Vec::new();
    for (path, payload) in entries {
        let offset = file.stream_position().expect("offset");
        file.write_all(payload).expect("payload");
        manifest_entries.push(serde_json::json!({
            "path": path,
            "offset": offset,
            "size": payload.len(),
            "compression": "none"
        }));
    }

    let index_offset = file.stream_position().expect("index offset");
    let index_bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "version": 1,
        "entries": manifest_entries
    }))
    .expect("manifest");
    file.write_all(&index_bytes).expect("index");
    let index_size = index_bytes.len() as u64;
    file.seek(std::io::SeekFrom::Start(8)).expect("seek header");
    file.write_all(&index_offset.to_le_bytes())
        .expect("write offset");
    file.write_all(&index_size.to_le_bytes())
        .expect("write size");
}

#[test]
fn first_existing_path_returns_first_match() {
    let dir = make_unique_temp_dir();
    let missing = dir.join("missing.txt");
    let first = dir.join("a.txt");
    let second = dir.join("b.txt");
    fs::write(&first, "a").expect("first file write");
    fs::write(&second, "b").expect("second file write");

    let resolved = first_existing_path(&[missing, first.clone(), second.clone()]);
    assert_eq!(resolved, Some(first));
}

#[test]
fn first_existing_path_returns_none_when_no_candidate_exists() {
    let dir = make_unique_temp_dir();
    let missing_a = dir.join("missing_a.txt");
    let missing_b = dir.join("missing_b.txt");
    let resolved = first_existing_path(&[missing_a, missing_b]);
    assert!(resolved.is_none());
}

#[test]
fn project_texture_paths_prefers_assets_sprites_files() {
    let project_dir = make_unique_temp_dir();
    let sprites_dir = project_dir.join("assets").join("sprites");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    let terrain = sprites_dir.join("terrain.png");
    let creatures = sprites_dir.join("creatures.png");
    fs::write(&terrain, "terrain").expect("terrain write");
    fs::write(&creatures, "creatures").expect("creatures write");

    let (tilemap_texture, sprite_texture) = App::project_texture_paths(&project_dir);
    assert_eq!(tilemap_texture, Some(terrain));
    assert_eq!(sprite_texture, Some(creatures));
}

#[test]
fn project_texture_paths_falls_back_to_assets_root() {
    let project_dir = make_unique_temp_dir();
    let assets_dir = project_dir.join("assets");
    fs::create_dir_all(&assets_dir).expect("assets dir");
    let terrain = assets_dir.join("terrain.png");
    let creatures = assets_dir.join("creatures.png");
    fs::write(&terrain, "terrain").expect("terrain write");
    fs::write(&creatures, "creatures").expect("creatures write");

    let (tilemap_texture, sprite_texture) = App::project_texture_paths(&project_dir);
    assert_eq!(tilemap_texture, Some(terrain));
    assert_eq!(sprite_texture, Some(creatures));
}

#[test]
fn load_project_scene_reads_valid_scene_file() {
    let project_dir = make_unique_temp_dir();
    let scenes_dir = project_dir.join("scenes");
    fs::create_dir_all(&scenes_dir).expect("scenes dir");

    let mut scene = Scene::new("Main Scene".to_string());
    scene.maps.push("main_map".to_string());
    scene.rules = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 1,
            once: false,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_start".to_string(),
            }],
        }],
    };
    let scene_json = serde_json::to_string_pretty(&scene).expect("scene should serialize to json");
    fs::write(scenes_dir.join("Main Scene.json"), scene_json).expect("scene write");

    let loaded =
        load_project_scene(&project_dir, "Main Scene").expect("scene should load from project");
    assert_eq!(loaded.name, "Main Scene");
    assert_eq!(loaded.maps, vec!["main_map".to_string()]);
    assert_eq!(loaded.rules, scene.rules);
}

#[test]
fn load_project_scene_uses_project_scene_mapping_from_project_toml() {
    let project_dir = make_unique_temp_dir();
    let mapped_dir = project_dir.join("scenes").join("nested");
    fs::create_dir_all(&mapped_dir).expect("mapped scene dir");
    fs::write(
        project_dir.join("project.toml"),
        "[scenes]\nTown = 'scenes/nested/town_entry.json'\n",
    )
    .expect("project file");

    let mut scene = Scene::new("Town".to_string());
    scene.maps.push("demo_map".to_string());
    fs::write(
        mapped_dir.join("town_entry.json"),
        serde_json::to_string_pretty(&scene).expect("serialize scene"),
    )
    .expect("write scene");

    let loaded = load_project_scene(&project_dir, "Town").expect("mapped scene should load");
    assert_eq!(loaded.name, "Town");
    assert_eq!(loaded.maps, vec!["demo_map".to_string()]);
}

#[test]
fn load_project_scene_returns_error_for_invalid_json() {
    let project_dir = make_unique_temp_dir();
    let scenes_dir = project_dir.join("scenes");
    fs::create_dir_all(&scenes_dir).expect("scenes dir");
    fs::write(scenes_dir.join("Broken.json"), "{ invalid json").expect("scene write");

    let error =
        load_project_scene(&project_dir, "Broken").expect_err("invalid scene json should fail");
    assert!(error.contains("Could not parse scene file"));
}

#[test]
fn load_project_scene_returns_error_for_missing_scene_file() {
    let project_dir = make_unique_temp_dir();
    fs::create_dir_all(project_dir.join("scenes")).expect("scenes dir");

    let error = load_project_scene(&project_dir, "DoesNotExist")
        .expect_err("missing scene file should fail");
    assert!(error.contains("Could not resolve scene file"));
}

#[test]
fn game_state_from_scene_uses_scene_data_without_fallback_entities() {
    let mut scene = Scene::new("Gameplay".to_string());
    scene.rules = RuleSet {
        rules: vec![Rule {
            id: "rule_1".to_string(),
            enabled: true,
            priority: 3,
            once: false,
            trigger: RuleTrigger::OnUpdate,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlayMusic {
                track_id: "lavandia".to_string(),
            }],
        }],
    };

    let game_state = App::game_state_from_scene(scene.clone());
    assert_eq!(
        game_state.scene_manager().active_scene_name(),
        Some("Gameplay")
    );
    assert_eq!(game_state.rules(), &scene.rules);
    assert_eq!(game_state.entity_manager().active_entities().len(), 0);
}

#[test]
fn fallback_game_state_spawns_player_and_npc() {
    let game_state = App::fallback_game_state();
    assert!(game_state.player_id().is_some(), "player should exist");
    assert_eq!(
        game_state.entity_manager().active_entities().len(),
        2,
        "fallback state should spawn player and one npc"
    );
}

#[test]
fn resolve_post_splash_sprite_texture_path_prefers_project_creatures_texture() {
    let project_dir = make_unique_temp_dir()
        .join("example_project")
        .join("MyGame");
    let sprites_dir = project_dir.join("assets").join("sprites");
    fs::create_dir_all(&sprites_dir).expect("sprites dir");
    let creatures_path = sprites_dir.join("creatures.png");
    fs::write(&creatures_path, "creatures").expect("creatures write");

    let options = RuntimeLaunchOptions {
        project_path: Some(project_dir),
        pack_path: None,
        scene_name: None,
        map_name: None,
        splash: RuntimeSplashOptions::default(),
        audio_mix: RuntimeAudioMixOptions::default(),
        display: RuntimeDisplayOptions::default(),
        transition: Default::default(),
        menu: MenuSettings::default(),
    };

    let resolved = App::resolve_post_splash_sprite_texture_path(&options, None);
    assert_eq!(resolved, Some(creatures_path));
}

#[test]
fn resolve_post_splash_sprite_texture_path_prefers_content_root_over_project_path() {
    let project_dir = make_unique_temp_dir()
        .join("example_project")
        .join("MyGame");
    let project_sprites_dir = project_dir.join("assets").join("sprites");
    fs::create_dir_all(&project_sprites_dir).expect("project sprites dir");
    fs::write(
        project_sprites_dir.join("creatures.png"),
        "project-creatures",
    )
    .expect("project sprite write");

    let mount_dir = make_unique_temp_dir().join("mount");
    let mount_sprites_dir = mount_dir.join("assets").join("sprites");
    fs::create_dir_all(&mount_sprites_dir).expect("mount sprites dir");
    let mount_creatures = mount_sprites_dir.join("creatures.png");
    fs::write(&mount_creatures, "mount-creatures").expect("mount sprite write");

    let options = RuntimeLaunchOptions {
        project_path: Some(project_dir),
        pack_path: None,
        scene_name: None,
        map_name: None,
        splash: RuntimeSplashOptions::default(),
        audio_mix: RuntimeAudioMixOptions::default(),
        display: RuntimeDisplayOptions::default(),
        transition: Default::default(),
        menu: MenuSettings::default(),
    };

    let resolved = App::resolve_post_splash_sprite_texture_path(&options, Some(&mount_dir));
    assert_eq!(resolved, Some(mount_creatures));
}

#[test]
fn build_startup_state_loads_resources_and_scene_from_pack_mount() {
    let temp = tempfile::tempdir().expect("temp dir");
    let pack_path = temp.path().join("game.toki.pak");

    let mut scene = Scene::new("Main Scene".to_string());
    scene.maps.push("demo_map".to_string());
    let scene_json = serde_json::to_vec_pretty(&scene).expect("scene json");

    let creatures_atlas = br#"{
  "image": "creatures.png",
  "tile_size": [16, 16],
  "tiles": {
    "idle": { "position": [0, 0], "properties": { "solid": false } }
  }
}"#
    .to_vec();

    let terrain_atlas_path = PathBuf::from("terrain.json");
    let map_json = format!(
        r#"{{
  "size": [1, 1],
  "tile_size": [16, 16],
  "atlas": "{}",
  "tiles": ["floor"]
}}"#,
        terrain_atlas_path.display()
    )
    .into_bytes();

    let terrain_atlas = br#"{
  "image": "terrain.png",
  "tile_size": [16, 16],
  "tiles": {
    "floor": { "position": [0, 0], "properties": { "solid": false } }
  }
}"#
    .to_vec();

    write_test_pak(
        &pack_path,
        &[
            ("scenes/Main Scene.json".to_string(), scene_json),
            ("assets/sprites/creatures.json".to_string(), creatures_atlas),
            ("assets/tilemaps/demo_map.json".to_string(), map_json),
            ("assets/tilemaps/terrain.json".to_string(), terrain_atlas),
        ],
    );

    let launch_options = RuntimeLaunchOptions {
        project_path: None,
        pack_path: Some(pack_path),
        scene_name: Some("Main Scene".to_string()),
        map_name: None,
        splash: RuntimeSplashOptions::default(),
        audio_mix: RuntimeAudioMixOptions::default(),
        display: RuntimeDisplayOptions::default(),
        transition: Default::default(),
        menu: MenuSettings::default(),
    };

    let (resources, game_state, pack_mount, asset_load_plan, _) =
        App::build_startup_state(&launch_options);

    assert!(pack_mount.is_some(), "pack mount should be retained");
    assert_eq!(
        game_state.scene_manager().active_scene_name(),
        Some("Main Scene")
    );
    assert_eq!(game_state.entity_manager().active_entities().len(), 0);
    assert_eq!(resources.get_tilemap().size, glam::UVec2::new(1, 1));
    assert_eq!(resources.get_tilemap().atlas, terrain_atlas_path);
    assert_eq!(asset_load_plan.map_name.as_deref(), Some("demo_map"));
}

#[test]
fn build_startup_state_uses_scene_player_entry_and_preloads_all_scenes() {
    let project_dir = make_unique_temp_dir();
    fs::create_dir_all(project_dir.join("assets").join("sprites")).expect("sprites dir");
    fs::create_dir_all(project_dir.join("assets").join("tilemaps")).expect("tilemaps dir");
    fs::create_dir_all(project_dir.join("scenes").join("custom")).expect("custom scenes dir");
    write_player_definition(&project_dir, "player");

    fs::write(
        project_dir.join("project.toml"),
        "[scenes]\nMain = 'scenes/custom/main_scene.json'\nSecond = 'scenes/Second.json'\n",
    )
    .expect("project");

    let main_scene = Scene {
        name: "Main".to_string(),
        description: None,
        maps: vec!["demo_map".to_string()],
        entities: vec![],
        rules: RuleSet::default(),
        camera_position: None,
        camera_scale: None,
        background_music_track_id: None,
        anchors: vec![SceneAnchor {
            id: "entry".to_string(),
            kind: SceneAnchorKind::SpawnPoint,
            position: glam::IVec2::new(48, 64),
            facing: Some(SceneAnchorFacing::Right),
        }],
        player_entry: Some(ScenePlayerEntry {
            entity_definition_name: "player".to_string(),
            spawn_point_id: "entry".to_string(),
        }),
    };
    fs::write(
        project_dir
            .join("scenes")
            .join("custom")
            .join("main_scene.json"),
        serde_json::to_string_pretty(&main_scene).expect("serialize main scene"),
    )
    .expect("write main scene");

    let second_scene = Scene::new("Second".to_string());
    fs::write(
        project_dir.join("scenes").join("Second.json"),
        serde_json::to_string_pretty(&second_scene).expect("serialize second scene"),
    )
    .expect("write second scene");

    fs::write(
        project_dir
            .join("assets")
            .join("sprites")
            .join("creatures.json"),
        r#"{
  "image": "creatures.png",
  "tile_size": [16, 16],
  "tiles": { "idle": { "position": [0, 0], "properties": { "solid": false } } }
}"#,
    )
    .expect("creatures atlas");
    fs::write(
        project_dir
            .join("assets")
            .join("tilemaps")
            .join("demo_map.json"),
        r#"{
  "size": [1, 1],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": ["floor"]
}"#,
    )
    .expect("tilemap");
    fs::write(
        project_dir
            .join("assets")
            .join("tilemaps")
            .join("terrain.json"),
        r#"{
  "image": "terrain.png",
  "tile_size": [16, 16],
  "tiles": { "floor": { "position": [0, 0], "properties": { "solid": false } } }
}"#,
    )
    .expect("terrain atlas");

    let launch_options = RuntimeLaunchOptions {
        project_path: Some(project_dir.clone()),
        pack_path: None,
        scene_name: Some("Main".to_string()),
        map_name: None,
        splash: RuntimeSplashOptions::default(),
        audio_mix: RuntimeAudioMixOptions::default(),
        display: RuntimeDisplayOptions::default(),
        transition: Default::default(),
        menu: MenuSettings::default(),
    };

    let (_resources, game_state, _pack_mount, asset_load_plan, _) =
        App::build_startup_state(&launch_options);

    assert_eq!(game_state.scene_manager().active_scene_name(), Some("Main"));
    assert!(game_state.scene_manager().has_scene("Second"));
    assert_eq!(game_state.player_position(), glam::IVec2::new(48, 64));
    assert_eq!(
        game_state
            .player_entity()
            .expect("scene player should exist")
            .definition_name
            .as_deref(),
        Some("player")
    );
    assert_eq!(asset_load_plan.map_name.as_deref(), Some("demo_map"));
}

#[test]
fn build_startup_state_tolerates_stale_scene_manifest_paths() {
    let project_dir = make_unique_temp_dir();
    fs::create_dir_all(project_dir.join("assets").join("sprites")).expect("sprites dir");
    fs::create_dir_all(project_dir.join("assets").join("tilemaps")).expect("tilemaps dir");
    fs::create_dir_all(project_dir.join("scenes")).expect("scenes dir");
    write_player_definition(&project_dir, "player");

    fs::write(
        project_dir.join("project.toml"),
        "[scenes]\n\"Main Scene\" = 'scenes/mainscene.json'\n",
    )
    .expect("project");

    let startup_scene = Scene {
        name: "Main Scene".to_string(),
        description: None,
        maps: vec!["demo_map".to_string()],
        entities: vec![],
        rules: RuleSet::default(),
        camera_position: None,
        camera_scale: None,
        background_music_track_id: None,
        anchors: vec![SceneAnchor {
            id: "entry".to_string(),
            kind: SceneAnchorKind::SpawnPoint,
            position: glam::IVec2::new(32, 48),
            facing: Some(SceneAnchorFacing::Down),
        }],
        player_entry: Some(ScenePlayerEntry {
            entity_definition_name: "player".to_string(),
            spawn_point_id: "entry".to_string(),
        }),
    };
    fs::write(
        project_dir.join("scenes").join("Main Scene.json"),
        serde_json::to_string_pretty(&startup_scene).expect("serialize scene"),
    )
    .expect("scene");

    fs::write(
        project_dir
            .join("assets")
            .join("sprites")
            .join("creatures.json"),
        r#"{
  "image": "creatures.png",
  "tile_size": [16, 16],
  "tiles": { "idle": { "position": [0, 0], "properties": { "solid": false } } }
}"#,
    )
    .expect("creatures atlas");
    fs::write(
        project_dir
            .join("assets")
            .join("tilemaps")
            .join("demo_map.json"),
        r#"{
  "size": [1, 1],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": ["floor"]
}"#,
    )
    .expect("tilemap");
    fs::write(
        project_dir
            .join("assets")
            .join("tilemaps")
            .join("terrain.json"),
        r#"{
  "image": "terrain.png",
  "tile_size": [16, 16],
  "tiles": { "floor": { "position": [0, 0], "properties": { "solid": false } } }
}"#,
    )
    .expect("terrain atlas");

    let launch_options = RuntimeLaunchOptions {
        project_path: Some(project_dir),
        pack_path: None,
        scene_name: Some("Main Scene".to_string()),
        map_name: None,
        splash: RuntimeSplashOptions::default(),
        audio_mix: RuntimeAudioMixOptions::default(),
        display: RuntimeDisplayOptions::default(),
        transition: Default::default(),
        menu: MenuSettings::default(),
    };

    let (_resources, game_state, _mount, asset_load_plan, _) =
        App::build_startup_state(&launch_options);

    assert_eq!(
        game_state.scene_manager().active_scene_name(),
        Some("Main Scene")
    );
    assert_eq!(game_state.player_position(), glam::IVec2::new(32, 48));
    assert_eq!(asset_load_plan.map_name.as_deref(), Some("demo_map"));
}

#[test]
fn app_defers_scene_switch_until_fade_out_completes_then_fades_back_in() {
    #[derive(Default)]
    struct FakeAudioSink;

    impl super::app_transition::TransitionAudioSink for FakeAudioSink {
        fn play_background_music_in_channel(
            &mut self,
            _channel: &str,
            _track_id: &str,
            _volume: f32,
        ) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }

        fn set_channel_volume_percent(&mut self, _channel: &str, _percent: u8) {}

        fn stop_channel(&mut self, _channel: &str) {}
    }

    let project_dir = make_unique_temp_dir();
    fs::create_dir_all(project_dir.join("assets").join("sprites")).expect("sprites dir");
    fs::create_dir_all(project_dir.join("assets").join("tilemaps")).expect("tilemaps dir");
    fs::create_dir_all(project_dir.join("scenes")).expect("scenes dir");
    write_player_definition(&project_dir, "player");

    fs::write(
        project_dir.join("project.toml"),
        "[scenes]\nMain = 'scenes/Main.json'\nSecond = 'scenes/Second.json'\n",
    )
    .expect("project");

    let main_scene = Scene {
        name: "Main".to_string(),
        description: None,
        maps: vec!["demo_map".to_string()],
        entities: vec![],
        rules: RuleSet {
            rules: vec![Rule {
                id: "switch_to_second".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnUpdate,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::SwitchScene {
                    scene_name: "Second".to_string(),
                    spawn_point_id: "entry_b".to_string(),
                }],
            }],
        },
        camera_position: None,
        camera_scale: None,
        background_music_track_id: None,
        anchors: vec![SceneAnchor {
            id: "entry_a".to_string(),
            kind: SceneAnchorKind::SpawnPoint,
            position: glam::IVec2::new(16, 16),
            facing: Some(SceneAnchorFacing::Down),
        }],
        player_entry: Some(ScenePlayerEntry {
            entity_definition_name: "player".to_string(),
            spawn_point_id: "entry_a".to_string(),
        }),
    };
    fs::write(
        project_dir.join("scenes").join("Main.json"),
        serde_json::to_string_pretty(&main_scene).expect("serialize main scene"),
    )
    .expect("write main scene");

    let second_scene = Scene {
        name: "Second".to_string(),
        description: None,
        maps: vec!["demo_map".to_string()],
        entities: vec![],
        rules: RuleSet::default(),
        camera_position: None,
        camera_scale: None,
        background_music_track_id: None,
        anchors: vec![SceneAnchor {
            id: "entry_b".to_string(),
            kind: SceneAnchorKind::SpawnPoint,
            position: glam::IVec2::new(96, 48),
            facing: Some(SceneAnchorFacing::Right),
        }],
        player_entry: Some(ScenePlayerEntry {
            entity_definition_name: "player".to_string(),
            spawn_point_id: "entry_b".to_string(),
        }),
    };
    fs::write(
        project_dir.join("scenes").join("Second.json"),
        serde_json::to_string_pretty(&second_scene).expect("serialize second scene"),
    )
    .expect("write second scene");

    fs::write(
        project_dir
            .join("assets")
            .join("sprites")
            .join("creatures.json"),
        r#"{
  "image": "creatures.png",
  "tile_size": [16, 16],
  "tiles": { "idle": { "position": [0, 0], "properties": { "solid": false } } }
}"#,
    )
    .expect("creatures atlas");
    fs::write(
        project_dir
            .join("assets")
            .join("tilemaps")
            .join("demo_map.json"),
        r#"{
  "size": [1, 1],
  "tile_size": [16, 16],
  "atlas": "terrain.json",
  "tiles": ["floor"]
}"#,
    )
    .expect("tilemap");
    fs::write(
        project_dir
            .join("assets")
            .join("tilemaps")
            .join("terrain.json"),
        r#"{
  "image": "terrain.png",
  "tile_size": [16, 16],
  "tiles": { "floor": { "position": [0, 0], "properties": { "solid": false } } }
}"#,
    )
    .expect("terrain atlas");

    let launch_options = RuntimeLaunchOptions {
        project_path: Some(project_dir),
        pack_path: None,
        scene_name: Some("Main".to_string()),
        map_name: None,
        splash: RuntimeSplashOptions::default(),
        audio_mix: RuntimeAudioMixOptions::default(),
        display: RuntimeDisplayOptions::default(),
        transition: super::RuntimeTransitionOptions {
            fade_duration_ms: 100,
        },
        menu: MenuSettings::default(),
    };
    let (resources, mut game_state, _mount, _asset_load_plan, _) =
        App::build_startup_state(&launch_options);
    let mut transition =
        super::app_transition::SceneTransitionController::new(launch_options.transition.clone());
    let mut audio = FakeAudioSink;

    assert_eq!(game_state.scene_manager().active_scene_name(), Some("Main"));

    let result = game_state.update(
        glam::UVec2::new(16, 16),
        resources.get_tilemap(),
        resources.get_terrain_atlas(),
    );
    let request = result
        .scene_switch_request
        .expect("rule should emit scene-switch request");
    let target_track = game_state
        .scene_manager()
        .get_scene(&request.scene_name)
        .and_then(|scene| scene.background_music_track_id.clone());
    assert!(transition.request_scene_switch(request, target_track));

    assert!(matches!(
        transition.advance(50, &mut audio, launch_options.audio_mix.music_percent),
        super::app_transition::TransitionAdvance::None
    ));
    assert_eq!(game_state.scene_manager().active_scene_name(), Some("Main"));
    assert!(transition.fade_alpha() > 0.0);

    let request = match transition.advance(60, &mut audio, launch_options.audio_mix.music_percent) {
        super::app_transition::TransitionAdvance::ReadyToSwap(request) => request,
        other => panic!("expected ready-to-swap transition action, got {other:?}"),
    };
    game_state
        .transition_to_scene(&request.scene_name, &request.spawn_point_id)
        .expect("scene transition should apply after fade-out");
    transition
        .complete_scene_switch(
            &mut audio,
            true,
            game_state
                .active_scene()
                .and_then(|scene| scene.background_music_track_id.as_deref()),
        )
        .expect("fade-in should start");

    assert_eq!(
        game_state.scene_manager().active_scene_name(),
        Some("Second")
    );
    assert_eq!(game_state.player_position(), glam::IVec2::new(96, 48));
    assert!(transition.is_active());

    let _ = transition.advance(100, &mut audio, launch_options.audio_mix.music_percent);
    assert!(!transition.is_active());
    assert_eq!(transition.fade_alpha(), 0.0);
}

#[test]
fn build_startup_state_from_pack_returns_error_when_required_assets_are_missing() {
    let temp = tempfile::tempdir().expect("temp dir");
    let pack_path = temp.path().join("broken.toki.pak");

    let mut scene = Scene::new("Main Scene".to_string());
    scene.maps.push("demo_map".to_string());
    let scene_json = serde_json::to_vec_pretty(&scene).expect("scene json");

    write_test_pak(
        &pack_path,
        &[
            ("scenes/Main Scene.json".to_string(), scene_json),
            ("assets/tilemaps/demo_map.json".to_string(), b"{}".to_vec()),
        ],
    );

    let launch_options = RuntimeLaunchOptions {
        project_path: None,
        pack_path: Some(pack_path.clone()),
        scene_name: Some("Main Scene".to_string()),
        map_name: None,
        splash: RuntimeSplashOptions::default(),
        audio_mix: RuntimeAudioMixOptions::default(),
        display: RuntimeDisplayOptions::default(),
        transition: Default::default(),
        menu: MenuSettings::default(),
    };

    let error = App::build_startup_state_from_pack(&launch_options, &pack_path)
        .expect_err("missing pack assets should fail startup");
    let text = error.to_string();
    assert!(
        text.contains("atlas") || text.contains("resources") || text.contains("Core error"),
        "unexpected error: {error}"
    );
}

#[test]
fn community_splash_policy_forces_branding_on() {
    let requested = RuntimeSplashOptions {
        duration_ms: 1200,
        show_branding: false,
    };
    let resolved = SplashPolicy::Community.resolve(&requested);
    assert!(resolved.show_branding);
}

#[test]
fn community_splash_policy_clamps_duration_bounds() {
    let below_min = SplashPolicy::Community.resolve(&RuntimeSplashOptions {
        duration_ms: 200,
        show_branding: true,
    });
    assert_eq!(
        below_min.duration,
        Duration::from_millis(super::COMMUNITY_SPLASH_MIN_DURATION_MS)
    );

    let above_max = SplashPolicy::Community.resolve(&RuntimeSplashOptions {
        duration_ms: super::COMMUNITY_SPLASH_MAX_DURATION_MS + 1,
        show_branding: true,
    });
    assert_eq!(
        above_max.duration,
        Duration::from_millis(super::COMMUNITY_SPLASH_MAX_DURATION_MS)
    );
}

#[test]
fn centered_logo_origin_matches_previous_default_layout() {
    let view = App::projection_view_size(ProjectionParameter {
        width: 160,
        height: 144,
        desired_width: 160,
        desired_height: 144,
    });
    let origin = App::centered_logo_origin_for_view(view, glam::UVec2::new(128, 108));
    assert_eq!(origin, glam::IVec2::new(16, 18));
}

#[test]
fn centered_logo_origin_is_centered_for_wide_window() {
    let view = App::projection_view_size(ProjectionParameter {
        width: 320,
        height: 144,
        desired_width: 160,
        desired_height: 144,
    });
    let origin = App::centered_logo_origin_for_view(view, glam::UVec2::new(128, 108));
    assert_eq!(origin, glam::IVec2::new(96, 18));
}

#[test]
fn centered_logo_origin_is_centered_for_tall_window() {
    let view = App::projection_view_size(ProjectionParameter {
        width: 160,
        height: 320,
        desired_width: 160,
        desired_height: 144,
    });
    let origin = App::centered_logo_origin_for_view(view, glam::UVec2::new(128, 108));
    assert_eq!(origin, glam::IVec2::new(16, 106));
}

#[test]
fn splash_version_text_is_positioned_below_branding_text() {
    let branding_style = TextStyle {
        font_family: "Sans".to_string(),
        size_px: 16.0,
        weight: TextWeight::Bold,
        ..TextStyle::default()
    };
    let version_style = TextStyle {
        font_family: "Sans".to_string(),
        size_px: 11.0,
        weight: TextWeight::Normal,
        ..TextStyle::default()
    };
    let view_size = glam::Vec2::new(320.0, 240.0);
    let logo_size = glam::UVec2::new(128, 108);
    let logo_origin = App::centered_logo_origin_for_view(view_size, logo_size);

    let (branding_position, version_position) = App::splash_branding_positions(
        view_size,
        true,
        logo_origin,
        logo_size,
        &branding_style,
        &version_style,
    );

    let branding_bottom =
        branding_position.y + branding_style.size_px * SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER;
    assert!(
        version_position.y >= branding_bottom + SPLASH_BRANDING_VERSION_GAP_PX,
        "version text should be below branding text: branding_bottom={}, version_y={}",
        branding_bottom,
        version_position.y
    );
}

#[test]
fn splash_branding_block_fits_in_default_runtime_view() {
    let branding_style = TextStyle {
        font_family: "Sans".to_string(),
        size_px: 16.0,
        weight: TextWeight::Bold,
        ..TextStyle::default()
    };
    let version_style = TextStyle {
        font_family: "Sans".to_string(),
        size_px: 11.0,
        weight: TextWeight::Normal,
        ..TextStyle::default()
    };
    let view_size = glam::Vec2::new(160.0, 144.0);
    let logo_size = glam::UVec2::new(128, 108);
    let logo_origin = App::centered_logo_origin_for_view(view_size, logo_size);

    let (branding_position, version_position) = App::splash_branding_positions(
        view_size,
        true,
        logo_origin,
        logo_size,
        &branding_style,
        &version_style,
    );

    let branding_bottom =
        branding_position.y + branding_style.size_px * SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER;
    let version_bottom =
        version_position.y + version_style.size_px * SPLASH_TEXT_LINE_HEIGHT_MULTIPLIER;
    assert!(version_position.y >= branding_bottom + SPLASH_BRANDING_VERSION_GAP_PX);
    assert!(version_bottom <= view_size.y - 4.0);
}

#[test]
fn splash_version_style_shrinks_long_version_to_fit_default_view_width() {
    let style = App::fitted_splash_version_style(160.0, COMMUNITY_SPLASH_VERSION_TEXT);
    let estimated_width =
        COMMUNITY_SPLASH_VERSION_TEXT.chars().count() as f32 * style.size_px * 0.55;
    assert!(style.size_px <= SPLASH_VERSION_DEFAULT_SIZE_PX);
    assert!(style.size_px >= SPLASH_VERSION_MIN_SIZE_PX);
    assert!(estimated_width <= 160.0 - SPLASH_TEXT_HORIZONTAL_PADDING_PX);
}
