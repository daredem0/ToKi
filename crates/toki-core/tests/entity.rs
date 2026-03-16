use glam::{IVec2, UVec2};
use toki_core::collision::CollisionBox;
use toki_core::entity::*;

fn test_definition(name: &str, category: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(100),
            stats: std::collections::HashMap::new(),
            speed: 2,
            solid: true,
            active: true,
            can_move: true,
            ai_behavior: if category == "creature" {
                AiBehavior::Wander
            } else {
                AiBehavior::None
            },
            movement_profile: if category == "human" {
                MovementProfile::PlayerWasd
            } else {
                MovementProfile::None
            },
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: "sfx_step".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["slime/idle_0".to_string()],
                frame_duration_ms: 150.0,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: vec!["test".to_string()],
    }
}

fn player_definition() -> EntityDefinition {
    let mut def = test_definition("player", "human");
    def.attributes.health = Some(100);
    def.attributes.speed = 2;
    def.attributes.solid = true;
    def.attributes.can_move = true;
    def
}

fn npc_definition(animation_name: &str) -> EntityDefinition {
    let mut def = test_definition("npc", "creature");
    def.attributes.health = Some(50);
    def.attributes.speed = 1;
    def.attributes.solid = true;
    def.attributes.can_move = false;
    def.animations.clips = vec![AnimationClipDef {
        state: "walk".to_string(),
        frame_tiles: vec![
            format!("{animation_name}/walk_0"),
            format!("{animation_name}/walk_1"),
            format!("{animation_name}/walk_2"),
            format!("{animation_name}/walk_3"),
        ],
        frame_duration_ms: 150.0,
        loop_mode: "loop".to_string(),
    }];
    def.animations.default_state = "walk".to_string();
    def
}

fn item_definition(item_name: &str) -> EntityDefinition {
    let mut def = test_definition("item", "item");
    def.attributes.health = None;
    def.attributes.solid = false;
    def.attributes.can_move = false;
    def.animations.atlas_name = "objects".to_string();
    def.animations.clips = vec![AnimationClipDef {
        state: "idle".to_string(),
        frame_tiles: vec![
            format!("{item_name}_0"),
            format!("{item_name}_1"),
            format!("{item_name}_2"),
            format!("{item_name}_3"),
        ],
        frame_duration_ms: 150.0,
        loop_mode: "loop".to_string(),
    }];
    def.animations.default_state = "idle".to_string();
    def
}

fn decoration_definition(decoration_name: &str) -> EntityDefinition {
    let mut def = test_definition("decoration", "building");
    def.attributes.health = None;
    def.attributes.solid = false;
    def.attributes.can_move = false;
    def.rendering.render_layer = -1;
    def.animations.atlas_name = "terrain".to_string();
    def.animations.clips = vec![AnimationClipDef {
        state: "idle".to_string(),
        frame_tiles: vec![
            format!("{decoration_name}_0"),
            format!("{decoration_name}_1"),
            format!("{decoration_name}_2"),
            format!("{decoration_name}_3"),
        ],
        frame_duration_ms: 150.0,
        loop_mode: "loop".to_string(),
    }];
    def.animations.default_state = "idle".to_string();
    def
}

trait DefinitionSpawnExt {
    fn spawn_player(&mut self, position: IVec2) -> EntityId;
    fn spawn_npc(&mut self, position: IVec2, animation_name: &str) -> EntityId;
    fn spawn_item(&mut self, position: IVec2, item_name: &str) -> EntityId;
    fn spawn_decoration(&mut self, position: IVec2, decoration_name: &str) -> EntityId;
}

impl DefinitionSpawnExt for EntityManager {
    fn spawn_player(&mut self, position: IVec2) -> EntityId {
        let id = self.next_entity_id_for_test();
        let mut entity = player_definition()
            .create_entity(position, id)
            .expect("player definition spawn should succeed");
        entity.control_role = ControlRole::PlayerCharacter;
        entity.entity_kind = EntityKind::Player;
        self.add_existing_entity(entity)
    }

    fn spawn_npc(&mut self, position: IVec2, animation_name: &str) -> EntityId {
        self.spawn_from_definition(&npc_definition(animation_name), position)
            .expect("npc definition spawn should succeed")
    }

    fn spawn_item(&mut self, position: IVec2, item_name: &str) -> EntityId {
        self.spawn_from_definition(&item_definition(item_name), position)
            .expect("item definition spawn should succeed")
    }

    fn spawn_decoration(&mut self, position: IVec2, decoration_name: &str) -> EntityId {
        self.spawn_from_definition(&decoration_definition(decoration_name), position)
            .expect("decoration definition spawn should succeed")
    }
}

trait EntityManagerTestExt {
    fn next_entity_id_for_test(&self) -> EntityId;
}

impl EntityManagerTestExt for EntityManager {
    fn next_entity_id_for_test(&self) -> EntityId {
        self.active_entities()
            .into_iter()
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
}

#[test]
fn test_entity_manager_creation() {
    let manager = EntityManager::new();

    assert_eq!(manager.get_player_id(), None);
    assert_eq!(manager.active_entities().len(), 0);
    assert_eq!(manager.entities_of_kind(&EntityKind::Player).len(), 0);
}

#[test]
fn test_spawn_player() {
    let mut manager = EntityManager::new();
    let position = IVec2::new(100, 50);

    let player_id = manager.spawn_player(position);

    // Check player was created correctly
    assert_eq!(manager.get_player_id(), Some(player_id));

    let player = manager.get_player().unwrap();
    assert_eq!(player.position, position);
    assert_eq!(player.entity_kind, EntityKind::Player);
    assert_eq!(
        player.effective_control_role(),
        ControlRole::PlayerCharacter
    );
    assert_eq!(player.attributes.health, Some(100));
    assert_eq!(player.attributes.speed, 2);
    assert!(player.attributes.active);
    assert!(player.attributes.can_move);

    // Check lookup tables
    assert_eq!(
        manager.entities_of_kind(&EntityKind::Player),
        vec![player_id]
    );
    assert_eq!(manager.active_entities(), vec![player_id]);
}

#[test]
fn test_add_existing_entity_tracks_explicit_player_character_role() {
    let mut manager = EntityManager::new();
    let entity = Entity {
        id: 11,
        position: IVec2::new(5, 6),
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: Some("slime".to_string()),
        control_role: ControlRole::PlayerCharacter,
        audio: EntityAudioSettings::default(),
        attributes: EntityAttributes {
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::PlayerWasd,
            ..EntityAttributes::default()
        },
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
    };

    let entity_id = manager.add_existing_entity(entity);

    assert_eq!(entity_id, 11);
    assert_eq!(manager.get_player_id(), Some(11));
    assert_eq!(
        manager
            .get_player()
            .expect("player-role entity should be tracked")
            .category,
        "creature"
    );
}

#[test]
fn test_add_existing_entity_seeds_generic_health_stat_from_legacy_health() {
    let mut manager = EntityManager::new();
    let entity = Entity {
        id: 13,
        position: IVec2::new(8, 9),
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: Some("slime".to_string()),
        control_role: ControlRole::None,
        audio: EntityAudioSettings::default(),
        attributes: EntityAttributes {
            health: Some(25),
            stats: EntityStats::default(),
            ai_behavior: AiBehavior::None,
            movement_profile: MovementProfile::None,
            ..EntityAttributes::default()
        },
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
    };

    let entity_id = manager.add_existing_entity(entity);
    let loaded = manager
        .get_entity(entity_id)
        .expect("existing entity should be stored");

    assert_eq!(loaded.attributes.health, Some(25));
    assert_eq!(loaded.attributes.current_stat(HEALTH_STAT_ID), Some(25));
    assert_eq!(loaded.attributes.base_stat(HEALTH_STAT_ID), Some(25));
}

#[test]
fn test_spawn_multiple_entities() {
    let mut manager = EntityManager::new();

    let player_id = manager.spawn_player(IVec2::new(0, 0));
    let npc_id = manager.spawn_npc(IVec2::new(10, 10), "guard");
    let item_id = manager.spawn_item(IVec2::new(20, 20), "coin");

    // Check all entities exist
    assert!(manager.get_entity(player_id).is_some());
    assert!(manager.get_entity(npc_id).is_some());
    assert!(manager.get_entity(item_id).is_some());

    // Check type-based queries
    assert_eq!(manager.entities_of_kind(&EntityKind::Player).len(), 1);
    assert_eq!(manager.entities_of_kind(&EntityKind::Npc).len(), 1);
    assert_eq!(manager.entities_of_kind(&EntityKind::Item).len(), 1);

    // Check active entities (all should be active by default)
    assert_eq!(manager.active_entities().len(), 3);
}

#[test]
fn test_despawn_entity() {
    let mut manager = EntityManager::new();

    let player_id = manager.spawn_player(IVec2::new(0, 0));
    let npc_id = manager.spawn_npc(IVec2::new(10, 10), "guard");

    // Despawn the NPC
    assert!(manager.despawn_entity(npc_id));

    // Check NPC is gone
    assert!(manager.get_entity(npc_id).is_none());
    assert_eq!(manager.entities_of_kind(&EntityKind::Npc).len(), 0);

    // Check player still exists
    assert!(manager.get_entity(player_id).is_some());
    assert_eq!(manager.get_player_id(), Some(player_id));

    // Try to despawn non-existent entity
    assert!(!manager.despawn_entity(999));
}

#[test]
fn test_despawn_player() {
    let mut manager = EntityManager::new();

    let player_id = manager.spawn_player(IVec2::new(0, 0));
    assert_eq!(manager.get_player_id(), Some(player_id));

    // Despawn player
    assert!(manager.despawn_entity(player_id));

    // Check player tracking is cleared
    assert_eq!(manager.get_player_id(), None);
    assert!(manager.get_player().is_none());
}

#[test]
fn test_entity_active_status() {
    let mut manager = EntityManager::new();

    let entity_id = manager.spawn_npc(IVec2::new(0, 0), "test");

    // Entity should be active by default
    assert_eq!(manager.active_entities(), vec![entity_id]);

    // Deactivate entity
    manager.set_entity_active(entity_id, false);
    assert_eq!(manager.active_entities().len(), 0);
    assert!(!manager.get_entity(entity_id).unwrap().attributes.active);

    // Reactivate entity
    manager.set_entity_active(entity_id, true);
    assert_eq!(manager.active_entities(), vec![entity_id]);
    assert!(manager.get_entity(entity_id).unwrap().attributes.active);
}

#[test]
fn test_visible_entities() {
    let mut manager = EntityManager::new();

    let visible_id = manager.spawn_player(IVec2::new(0, 0));
    let invisible_id = manager.spawn_npc(IVec2::new(10, 10), "hidden");

    // Make NPC invisible
    manager
        .get_entity_mut(invisible_id)
        .unwrap()
        .attributes
        .visible = false;

    let visible_entities = manager.visible_entities();
    assert_eq!(visible_entities.len(), 1);
    assert!(visible_entities.contains(&visible_id));
    assert!(!visible_entities.contains(&invisible_id));
}

#[test]
fn test_entity_attributes_defaults() {
    let attributes = EntityAttributes::default();

    assert_eq!(attributes.health, None);
    assert_eq!(attributes.speed, 2);
    assert!(attributes.solid);
    assert!(attributes.visible);
    assert!(attributes.active);
    assert!(attributes.can_move);
    assert_eq!(attributes.render_layer, 0);
    assert!(attributes.animation_controller.is_none());
}

#[test]
fn test_factory_method_differences() {
    let mut manager = EntityManager::new();

    let player_id = manager.spawn_player(IVec2::new(0, 0));
    let npc_id = manager.spawn_npc(IVec2::new(0, 0), "guard");
    let item_id = manager.spawn_item(IVec2::new(0, 0), "coin");
    let decoration_id = manager.spawn_decoration(IVec2::new(0, 0), "tree");

    let player = manager.get_entity(player_id).unwrap();
    let npc = manager.get_entity(npc_id).unwrap();
    let item = manager.get_entity(item_id).unwrap();
    let decoration = manager.get_entity(decoration_id).unwrap();

    // Check health differences
    assert_eq!(player.attributes.health, Some(100));
    assert_eq!(npc.attributes.health, Some(50));
    assert_eq!(item.attributes.health, None);
    assert_eq!(decoration.attributes.health, None);

    // Check speed differences
    assert_eq!(player.attributes.speed, 2);
    assert_eq!(npc.attributes.speed, 1);

    // Check movement differences
    assert!(player.attributes.can_move);
    assert!(!npc.attributes.can_move);
    assert!(!item.attributes.can_move);
    assert!(!decoration.attributes.can_move);

    // Check solid differences
    assert!(player.attributes.solid);
    assert!(npc.attributes.solid);
    assert!(!item.attributes.solid);
    assert!(!decoration.attributes.solid);

    // Check render layer differences
    assert_eq!(player.attributes.render_layer, 0);
    assert_eq!(decoration.attributes.render_layer, -1);
}

#[test]
fn test_entity_id_uniqueness() {
    let mut manager = EntityManager::new();

    let id1 = manager.spawn_player(IVec2::new(0, 0));
    let id2 = manager.spawn_npc(IVec2::new(10, 10), "guard");
    let id3 = manager.spawn_item(IVec2::new(20, 20), "coin");

    // All IDs should be unique
    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);

    // IDs should start from 1
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_multiple_players_not_allowed() {
    let mut manager = EntityManager::new();

    let first_player = manager.spawn_player(IVec2::new(0, 0));
    let second_player = manager.spawn_player(IVec2::new(100, 100));

    // The first explicit player-character stays tracked until a higher-level scene/editor
    // workflow deliberately reassigns the role.
    assert_eq!(manager.get_player_id(), Some(first_player));
    assert_ne!(manager.get_player_id(), Some(second_player));

    // But both should exist as entities
    assert!(manager.get_entity(first_player).is_some());
    assert!(manager.get_entity(second_player).is_some());

    // Both should be in the Player type list
    let players = manager.entities_of_kind(&EntityKind::Player);
    assert_eq!(players.len(), 2);
    assert!(players.contains(&first_player));
    assert!(players.contains(&second_player));
}

#[test]
fn test_entity_position_and_size() {
    let mut manager = EntityManager::new();

    let position = IVec2::new(50, 75);
    let entity_id = manager.spawn_player(position);

    let entity = manager.get_entity(entity_id).unwrap();
    assert_eq!(entity.position, position);
    assert_eq!(entity.size, UVec2::new(16, 16)); // Standard Game Boy sprite size

    // Test mutability
    let entity_mut = manager.get_entity_mut(entity_id).unwrap();
    entity_mut.position = IVec2::new(100, 200);

    let entity = manager.get_entity(entity_id).unwrap();
    assert_eq!(entity.position, IVec2::new(100, 200));
}

#[test]
fn test_spawn_from_definition_sets_definition_name_without_assigning_player_role() {
    let mut manager = EntityManager::new();
    let definition = test_definition("player", "human");

    let entity_id = manager
        .spawn_from_definition(&definition, IVec2::new(12, 34))
        .expect("definition spawn should succeed");

    let entity = manager.get_entity(entity_id).expect("entity should exist");
    assert_eq!(entity.definition_name.as_deref(), Some("player"));
    assert_eq!(entity.entity_kind, EntityKind::Npc);
    assert_eq!(entity.position, IVec2::new(12, 34));
    assert_eq!(entity.effective_control_role(), ControlRole::None);
    assert_eq!(manager.get_player_id(), None);
}

#[test]
fn test_spawn_from_definition_registers_audio_component() {
    let mut manager = EntityManager::new();
    let definition = test_definition("audio_player", "human");

    let entity_id = manager
        .spawn_from_definition(&definition, IVec2::new(0, 0))
        .expect("definition spawn should succeed");

    let audio = manager
        .audio_component(entity_id)
        .expect("audio component should be registered");
    assert_eq!(audio.footstep_distance_accumulator, 0.0);
    assert_eq!(audio.footstep_trigger_distance, 32.0);
    assert!(!audio.last_collision_state);
    assert_eq!(audio.movement_sound.as_deref(), Some("sfx_step"));
    assert_eq!(audio.collision_sound.as_deref(), Some("sfx_hit2"));
}

#[test]
fn test_add_existing_entity_uses_scene_audio_settings_for_component() {
    let mut manager = EntityManager::new();
    let mut entity = test_definition("audio_override", "creature")
        .create_entity(IVec2::new(4, 8), 77)
        .expect("definition entity should be created");
    entity.audio.footstep_trigger_distance = 7.5;
    entity.audio.movement_sound = Some("sfx_custom_step".to_string());

    let entity_id = manager.add_existing_entity(entity);

    let audio = manager
        .audio_component(entity_id)
        .expect("audio component should be initialized from scene entity");
    assert_eq!(audio.footstep_trigger_distance, 7.5);
    assert_eq!(audio.movement_sound.as_deref(), Some("sfx_custom_step"));
    assert_eq!(audio.footstep_distance_accumulator, 0.0);
}
