use glam::{IVec2, UVec2};
use toki_core::collision::CollisionBox;
use toki_core::entity::*;

fn test_definition(name: &str, category: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.into(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        solid: true,
        active: true,
        components: ComponentsDef {
            movement: Some(MovementComponent {
                speed: 2.0,
                movement_profile: if category == "human" {
                    MovementProfile::PlayerWasd
                } else {
                    MovementProfile::None
                },
                can_move: true,
            }),
            ai: (category == "creature").then_some(AiComponent {
                ai_config: AiConfig::from_legacy_behavior(AiBehavior::Wander),
            }),
            combat: Some(CombatComponent {
                health: Some(100),
                stats: EntityStats::from_legacy_health(Some(100)),
            }),
            primary_projectile: None,
            pickup: None,
            inventory: None,
            ..Default::default()
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
                frame_positions: None,
                frame_duration_ms: 150.0,
                frame_durations_ms: None,
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
    def.components.combat = Some(CombatComponent {
        health: Some(100),
        stats: EntityStats::from_legacy_health(Some(100)),
    });
    def.components
        .movement
        .get_or_insert_with(MovementComponent::default)
        .speed = 2.0;
    def.solid = true;
    def.components
        .movement
        .get_or_insert_with(MovementComponent::default)
        .can_move = true;
    def
}

fn npc_definition(animation_name: &str) -> EntityDefinition {
    let mut def = test_definition("npc", "creature");
    def.components.combat = Some(CombatComponent {
        health: Some(50),
        stats: EntityStats::from_legacy_health(Some(50)),
    });
    def.components
        .movement
        .get_or_insert_with(MovementComponent::default)
        .speed = 1.0;
    def.solid = true;
    def.components
        .movement
        .get_or_insert_with(MovementComponent::default)
        .can_move = false;
    def.animations.clips = vec![AnimationClipDef {
        state: "walk".to_string(),
        frame_tiles: vec![
            format!("{animation_name}/walk_0"),
            format!("{animation_name}/walk_1"),
            format!("{animation_name}/walk_2"),
            format!("{animation_name}/walk_3"),
        ],
        frame_positions: None,
        frame_duration_ms: 150.0,
        frame_durations_ms: None,
        loop_mode: "loop".to_string(),
    }];
    def.animations.default_state = "walk".to_string();
    def
}

fn item_definition(item_name: &str) -> EntityDefinition {
    let mut def = test_definition("item", "item");
    def.components.combat = None;
    def.solid = false;
    def.components
        .movement
        .get_or_insert_with(MovementComponent::default)
        .can_move = false;
    def.animations.atlas_name = "objects".to_string();
    def.animations.clips = vec![AnimationClipDef {
        state: "idle".to_string(),
        frame_tiles: vec![
            format!("{item_name}_0"),
            format!("{item_name}_1"),
            format!("{item_name}_2"),
            format!("{item_name}_3"),
        ],
        frame_positions: None,
        frame_duration_ms: 150.0,
        frame_durations_ms: None,
        loop_mode: "loop".to_string(),
    }];
    def.animations.default_state = "idle".to_string();
    def
}

fn decoration_definition(decoration_name: &str) -> EntityDefinition {
    let mut def = test_definition("decoration", "building");
    def.components.combat = None;
    def.solid = false;
    def.components
        .movement
        .get_or_insert_with(MovementComponent::default)
        .can_move = false;
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
        frame_positions: None,
        frame_duration_ms: 150.0,
        frame_durations_ms: None,
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
        let id = self
            .spawn_from_definition(&player_definition(), position)
            .expect("player definition spawn should succeed");
        assert!(self.set_control_role(id, ControlRole::PlayerCharacter));
        id
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

#[test]
fn test_entity_manager_creation() {
    let manager = EntityManager::new();

    assert_eq!(manager.get_player_id(), None);
    assert_eq!(manager.active_entities().len(), 0);
    assert_eq!(manager.entities_of_kind(&EntityKind::Player).len(), 0);
}

#[test]
fn test_entity_interaction_bounds_prefers_collision_box() {
    let mut entity = player_definition()
        .create_entity(IVec2::new(10, 20), 1)
        .expect("player definition spawn should succeed");
    entity.collision_box = Some(CollisionBox::new(IVec2::new(4, 5), UVec2::new(6, 7), false));

    let (position, size) = entity.interaction_bounds();

    assert_eq!(position, IVec2::new(14, 25));
    assert_eq!(size, UVec2::new(6, 7));
}

#[test]
fn build_decoration_entity_uses_grounding_footprint_for_collision_box() {
    let entity = build_decoration_entity(
        7,
        DecorationSpec {
            position: IVec2::new(32, 48),
            size: UVec2::new(16, 24),
            sheet: "fauna.json".to_string(),
            object_name: "bush".to_string(),
            grounding: EntityGrounding {
                origin: Some([8, 23]),
                footprint: Some(EntityFootprint::new([4, 16], [8, 8])),
            },
            visible: true,
            solid: true,
        },
    );

    let collision_box = entity
        .collision_box
        .as_ref()
        .expect("solid decoration should have a collision box");
    assert_eq!(collision_box.offset, IVec2::new(4, 16));
    assert_eq!(collision_box.size, UVec2::new(8, 8));
    assert!(!collision_box.trigger);
}

#[test]
fn test_entity_interaction_bounds_falls_back_to_entity_rect_without_collision_box() {
    let mut entity = player_definition()
        .create_entity(IVec2::new(10, 20), 1)
        .expect("player definition spawn should succeed");
    entity.collision_box = None;

    let (position, size) = entity.interaction_bounds();

    assert_eq!(position, IVec2::new(10, 20));
    assert_eq!(size, UVec2::new(16, 16));
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
    assert_eq!(
        manager
            .combat(player_id)
            .expect("player combat should exist")
            .health,
        Some(100)
    );
    assert_eq!(
        manager
            .movement(player_id)
            .expect("player movement should exist")
            .speed,
        2.0
    );
    assert!(player.active);
    assert!(
        manager
            .movement(player_id)
            .expect("player movement should exist")
            .can_move
    );

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
        definition_name: Some("slime".into()),
        persistent_across_saves: false,
        control_role: ControlRole::PlayerCharacter,
        audio: EntityAudioSettings::default(),
        rendering: EntityRendering::default(),
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
        solid: true,
        active: true,
        movement_accumulator: glam::Vec2::ZERO,
        tags: Vec::new(),
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
        definition_name: Some("slime".into()),
        persistent_across_saves: false,
        control_role: ControlRole::None,
        audio: EntityAudioSettings::default(),
        rendering: EntityRendering::default(),
        collision_box: Some(CollisionBox::solid_box(UVec2::new(16, 16))),
        solid: true,
        active: true,
        movement_accumulator: glam::Vec2::ZERO,
        tags: Vec::new(),
    };

    let entity_id = manager.add_existing_stored_entity(StoredEntity::new(
        entity,
        OptionalEntityComponents {
            movement: Some(MovementComponent {
                speed: 2.0,
                movement_profile: MovementProfile::None,
                can_move: true,
            }),
            ai: Some(AiComponent {
                ai_config: AiConfig::default(),
            }),
            combat: Some(CombatComponent {
                health: Some(25),
                stats: EntityStats::default(),
            }),
            ..Default::default()
        },
    ));
    let _loaded = manager
        .get_entity(entity_id)
        .expect("existing entity should be stored");

    assert_eq!(
        manager.combat(entity_id).and_then(|combat| combat.health),
        Some(25)
    );
    assert_eq!(
        manager
            .combat(entity_id)
            .and_then(|combat| combat.current_stat(HEALTH_STAT_ID)),
        Some(25)
    );
    assert_eq!(
        manager
            .combat(entity_id)
            .and_then(|combat| combat.base_stat(HEALTH_STAT_ID)),
        Some(25)
    );
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
    assert!(!manager.get_entity(entity_id).unwrap().active);

    // Reactivate entity
    manager.set_entity_active(entity_id, true);
    assert_eq!(manager.active_entities(), vec![entity_id]);
    assert!(manager.get_entity(entity_id).unwrap().active);
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
        .rendering
        .visible = false;

    let visible_entities = manager.visible_entities();
    assert_eq!(visible_entities.len(), 1);
    assert!(visible_entities.contains(&visible_id));
    assert!(!visible_entities.contains(&invisible_id));
}

#[test]
fn test_entity_defaults() {
    let entity = Entity {
        id: 1,
        position: IVec2::ZERO,
        size: UVec2::new(16, 16),
        entity_kind: EntityKind::Npc,
        category: "creature".to_string(),
        definition_name: None,
        persistent_across_saves: false,
        control_role: ControlRole::LegacyDefault,
        audio: EntityAudioSettings::default(),
        rendering: EntityRendering::default(),
        collision_box: None,
        solid: true,
        active: true,
        movement_accumulator: glam::Vec2::ZERO,
        tags: Vec::new(),
    };

    assert!(entity.solid);
    assert!(entity.rendering.visible);
    assert!(entity.active);
    assert_eq!(entity.rendering.render_layer, 0);
    assert!(entity.rendering.animation_controller.is_none());
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
    assert_eq!(
        manager.combat(player_id).and_then(|combat| combat.health),
        Some(100)
    );
    assert_eq!(
        manager.combat(npc_id).and_then(|combat| combat.health),
        Some(50)
    );
    assert_eq!(
        manager.combat(item_id).and_then(|combat| combat.health),
        None
    );
    assert_eq!(
        manager
            .combat(decoration_id)
            .and_then(|combat| combat.health),
        None
    );

    // Check speed differences
    assert_eq!(
        manager.movement(player_id).expect("player movement").speed,
        2.0
    );
    assert_eq!(manager.movement(npc_id).expect("npc movement").speed, 1.0);

    // Check movement differences
    assert!(
        manager
            .movement(player_id)
            .expect("player movement")
            .can_move
    );
    assert!(!manager.movement(npc_id).expect("npc movement").can_move);
    assert!(!manager.movement(item_id).expect("item movement").can_move);
    assert!(
        !manager
            .movement(decoration_id)
            .expect("deco movement")
            .can_move
    );

    // Check solid differences
    assert!(player.solid);
    assert!(npc.solid);
    assert!(!item.solid);
    assert!(!decoration.solid);

    // Check render layer differences
    assert_eq!(player.rendering.render_layer, 0);
    assert_eq!(decoration.rendering.render_layer, -1);
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

    // The most recently assigned explicit player-character becomes the tracked player.
    assert_eq!(manager.get_player_id(), Some(second_player));
    assert_ne!(manager.get_player_id(), Some(first_player));

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
fn clone_entity_preserves_indices_and_sparse_components() {
    let mut manager = EntityManager::new();
    let source_id = manager.spawn_player(IVec2::new(8, 12));
    manager.storage_mut().components_mut().set_pickup(
        source_id,
        Some(PickupDef {
            item_id: "coin".to_string(),
            count: 2,
        }),
    );

    let cloned_id = manager
        .clone_entity(source_id, IVec2::new(40, 60))
        .expect("clone should succeed");

    assert_eq!(manager.get_player_id(), Some(source_id));
    assert!(manager
        .entities_of_kind(&EntityKind::Player)
        .contains(&cloned_id));
    assert_eq!(
        manager
            .storage()
            .components()
            .pickup(cloned_id)
            .expect("clone pickup should exist")
            .item_id,
        "coin"
    );
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
    assert_eq!(entity.entity_kind, EntityKind::Player);
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
        .storage()
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
        .storage()
        .audio_component(entity_id)
        .expect("audio component should be initialized from scene entity");
    assert_eq!(audio.footstep_trigger_distance, 7.5);
    assert_eq!(audio.movement_sound.as_deref(), Some("sfx_custom_step"));
    assert_eq!(audio.footstep_distance_accumulator, 0.0);
}
