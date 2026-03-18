use glam::{IVec2, UVec2};
use toki_core::animation::{AnimationClip, AnimationController, AnimationState, LoopMode};
use toki_core::entity::{
    AiBehavior, ControlRole, Entity, EntityAttributes, EntityKind, MovementProfile,
};
use toki_core::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger};
use toki_core::scene::Scene;

fn create_test_entity(id: u32, position: IVec2) -> Entity {
    let mut controller = AnimationController::new();
    let idle_clip = AnimationClip {
        state: AnimationState::Idle,
        atlas_name: "test_atlas".to_string(),
        frame_tile_names: vec!["idle_0".to_string()],
        frame_duration_ms: 300.0,
        loop_mode: LoopMode::Loop,
    };
    controller.add_clip(idle_clip);
    controller.play(AnimationState::Idle);

    Entity {
        id,
        position,
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Player,
        category: "human".to_string(),
        definition_name: None,
        control_role: ControlRole::PlayerCharacter,
        audio: toki_core::entity::EntityAudioSettings::default(),
        attributes: EntityAttributes {
            health: Some(100),
            stats: toki_core::entity::EntityStats::from_legacy_health(Some(100)),
            speed: 2.0,
            solid: true,
            visible: true,
            animation_controller: Some(controller),
            static_object_render: None,
            render_layer: 0,
            active: true,
            can_move: true,
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::PlayerWasd,
            primary_projectile: None,
            projectile: None,
            pickup: None,
            inventory: toki_core::entity::Inventory::default(),
            has_inventory: false,
        },
        collision_box: None,
    }
}

#[test]
fn test_scene_new() {
    let scene = Scene::new("test_scene".to_string());

    assert_eq!(scene.name, "test_scene");
    assert!(scene.description.is_none());
    assert!(scene.maps.is_empty());
    assert!(scene.entities.is_empty());
    assert!(scene.rules.rules.is_empty());
    assert!(scene.camera_position.is_none());
    assert!(scene.camera_scale.is_none());
}

#[test]
fn test_scene_with_maps() {
    let maps = vec!["map1".to_string(), "map2".to_string()];
    let scene = Scene::with_maps("test_scene".to_string(), maps.clone());

    assert_eq!(scene.name, "test_scene");
    assert!(scene.description.is_none());
    assert_eq!(scene.maps, maps);
    assert!(scene.entities.is_empty());
    assert!(scene.rules.rules.is_empty());
    assert!(scene.camera_position.is_none());
    assert!(scene.camera_scale.is_none());
}

#[test]
fn test_scene_add_entity() {
    let mut scene = Scene::new("test_scene".to_string());
    let entity = create_test_entity(1, IVec2::new(100, 200));

    let returned_id = scene.add_entity(entity);

    assert_eq!(returned_id, 1);
    assert_eq!(scene.entities.len(), 1);
    assert_eq!(scene.entities[0].id, 1);
    assert_eq!(scene.entities[0].position, IVec2::new(100, 200));
}

#[test]
fn test_scene_add_multiple_entities() {
    let mut scene = Scene::new("test_scene".to_string());

    let entity1 = create_test_entity(1, IVec2::new(100, 200));
    let entity2 = create_test_entity(2, IVec2::new(300, 400));

    scene.add_entity(entity1);
    scene.add_entity(entity2);

    assert_eq!(scene.entities.len(), 2);
    assert_eq!(scene.entities[0].id, 1);
    assert_eq!(scene.entities[1].id, 2);
}

#[test]
fn test_scene_remove_entity() {
    let mut scene = Scene::new("test_scene".to_string());

    let entity1 = create_test_entity(1, IVec2::new(100, 200));
    let entity2 = create_test_entity(2, IVec2::new(300, 400));

    scene.add_entity(entity1);
    scene.add_entity(entity2);

    let removed = scene.remove_entity(1);
    assert!(removed);
    assert_eq!(scene.entities.len(), 1);
    assert_eq!(scene.entities[0].id, 2);
}

#[test]
fn test_scene_remove_nonexistent_entity() {
    let mut scene = Scene::new("test_scene".to_string());
    let entity = create_test_entity(1, IVec2::new(100, 200));
    scene.add_entity(entity);

    let removed = scene.remove_entity(999);
    assert!(!removed);
    assert_eq!(scene.entities.len(), 1);
}

#[test]
fn test_scene_get_entity() {
    let mut scene = Scene::new("test_scene".to_string());
    let entity = create_test_entity(42, IVec2::new(100, 200));
    scene.add_entity(entity);

    let found = scene.get_entity(42);
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, 42);
    assert_eq!(found.unwrap().position, IVec2::new(100, 200));

    let not_found = scene.get_entity(999);
    assert!(not_found.is_none());
}

#[test]
fn test_scene_get_entity_mut() {
    let mut scene = Scene::new("test_scene".to_string());
    let entity = create_test_entity(42, IVec2::new(100, 200));
    scene.add_entity(entity);

    {
        let found = scene.get_entity_mut(42);
        assert!(found.is_some());
        found.unwrap().position = IVec2::new(500, 600);
    }

    let entity = scene.get_entity(42).unwrap();
    assert_eq!(entity.position, IVec2::new(500, 600));
}

#[test]
fn test_scene_add_map() {
    let mut scene = Scene::new("test_scene".to_string());

    scene.add_map("map1".to_string());
    assert_eq!(scene.maps.len(), 1);
    assert!(scene.maps.contains(&"map1".to_string()));

    // Adding same map again should not duplicate
    scene.add_map("map1".to_string());
    assert_eq!(scene.maps.len(), 1);

    // Adding different map should work
    scene.add_map("map2".to_string());
    assert_eq!(scene.maps.len(), 2);
    assert!(scene.maps.contains(&"map2".to_string()));
}

#[test]
fn test_scene_remove_map() {
    let mut scene = Scene::with_maps(
        "test_scene".to_string(),
        vec!["map1".to_string(), "map2".to_string(), "map3".to_string()],
    );

    let removed = scene.remove_map("map2");
    assert!(removed);
    assert_eq!(scene.maps.len(), 2);
    assert!(!scene.maps.contains(&"map2".to_string()));
    assert!(scene.maps.contains(&"map1".to_string()));
    assert!(scene.maps.contains(&"map3".to_string()));
}

#[test]
fn test_scene_remove_nonexistent_map() {
    let mut scene = Scene::with_maps("test_scene".to_string(), vec!["map1".to_string()]);

    let removed = scene.remove_map("nonexistent");
    assert!(!removed);
    assert_eq!(scene.maps.len(), 1);
}

#[test]
fn test_scene_has_map() {
    let scene = Scene::with_maps(
        "test_scene".to_string(),
        vec!["map1".to_string(), "map2".to_string()],
    );

    assert!(scene.has_map("map1"));
    assert!(scene.has_map("map2"));
    assert!(!scene.has_map("nonexistent"));
}

#[test]
fn test_scene_serialization() {
    let mut scene = Scene::new("test_scene".to_string());
    scene.description = Some("A test scene".to_string());
    scene.add_map("test_map".to_string());
    scene.camera_position = Some(IVec2::new(100, 200));
    scene.camera_scale = Some(2);

    let entity = create_test_entity(1, IVec2::new(50, 75));
    scene.add_entity(entity);

    // Test serialization round-trip
    let json = serde_json::to_string_pretty(&scene).unwrap();
    let deserialized: Scene = serde_json::from_str(&json).unwrap();

    // Check that important fields are preserved
    assert_eq!(scene.name, deserialized.name);
    assert_eq!(scene.description, deserialized.description);
    assert_eq!(scene.maps, deserialized.maps);
    assert_eq!(scene.camera_position, deserialized.camera_position);
    assert_eq!(scene.camera_scale, deserialized.camera_scale);
    assert_eq!(scene.entities.len(), deserialized.entities.len());
    assert_eq!(scene.entities[0].id, deserialized.entities[0].id);
    assert_eq!(
        scene.entities[0].position,
        deserialized.entities[0].position
    );
    assert_eq!(
        scene.entities[0].control_role,
        deserialized.entities[0].control_role
    );
}

#[test]
fn test_scene_clone() {
    let mut scene = Scene::new("test_scene".to_string());
    scene.description = Some("A test scene".to_string());
    scene.add_map("test_map".to_string());

    let entity = create_test_entity(1, IVec2::new(50, 75));
    scene.add_entity(entity);

    let cloned_scene = scene.clone();

    assert_eq!(scene.name, cloned_scene.name);
    assert_eq!(scene.description, cloned_scene.description);
    assert_eq!(scene.maps, cloned_scene.maps);
    assert_eq!(scene.entities.len(), cloned_scene.entities.len());
    assert_eq!(scene.entities[0].id, cloned_scene.entities[0].id);
}

#[test]
fn test_scene_empty_operations() {
    let mut scene = Scene::new("empty_scene".to_string());

    // Test operations on empty scene
    assert!(scene.get_entity(1).is_none());
    assert!(scene.get_entity_mut(1).is_none());
    assert!(!scene.remove_entity(1));
    assert!(!scene.remove_map("nonexistent"));
    assert!(!scene.has_map("nonexistent"));
}

#[test]
fn test_scene_with_camera_settings() {
    let mut scene = Scene::new("camera_scene".to_string());

    // Test camera position and scale
    scene.camera_position = Some(IVec2::new(1000, 2000));
    scene.camera_scale = Some(4);

    assert_eq!(scene.camera_position, Some(IVec2::new(1000, 2000)));
    assert_eq!(scene.camera_scale, Some(4));

    // Test serialization with camera settings
    let json = serde_json::to_string(&scene).unwrap();
    let deserialized: Scene = serde_json::from_str(&json).unwrap();

    assert_eq!(scene.camera_position, deserialized.camera_position);
    assert_eq!(scene.camera_scale, deserialized.camera_scale);
}

#[test]
fn test_scene_rules_serialization_roundtrip() {
    let mut scene = Scene::new("rule_scene".to_string());
    scene.rules = RuleSet {
        rules: vec![Rule {
            id: "scene_rule".to_string(),
            enabled: true,
            priority: 7,
            once: true,
            trigger: RuleTrigger::OnStart,
            conditions: vec![RuleCondition::Always],
            actions: vec![RuleAction::PlaySound {
                channel: RuleSoundChannel::Movement,
                sound_id: "sfx_scene_start".to_string(),
            }],
        }],
    };

    let json = serde_json::to_string_pretty(&scene).unwrap();
    let deserialized: Scene = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.rules.rules.len(), 1);
    assert_eq!(deserialized.rules.rules[0].id, "scene_rule");
    assert_eq!(deserialized.rules.rules[0].trigger, RuleTrigger::OnStart);
}
