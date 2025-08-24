use glam::{IVec2, UVec2};
use toki_core::entity::*;

#[test]
fn test_entity_manager_creation() {
    let manager = EntityManager::new();

    assert_eq!(manager.get_player_id(), None);
    assert_eq!(manager.active_entities().len(), 0);
    assert_eq!(manager.entities_of_type(&EntityType::Player).len(), 0);
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
    assert_eq!(player.entity_type, EntityType::Player);
    assert_eq!(player.attributes.health, Some(100));
    assert_eq!(player.attributes.speed, 2);
    assert!(player.attributes.active);
    assert!(player.attributes.can_move);

    // Check lookup tables
    assert_eq!(
        manager.entities_of_type(&EntityType::Player),
        vec![player_id]
    );
    assert_eq!(manager.active_entities(), vec![player_id]);
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
    assert_eq!(manager.entities_of_type(&EntityType::Player).len(), 1);
    assert_eq!(manager.entities_of_type(&EntityType::Npc).len(), 1);
    assert_eq!(manager.entities_of_type(&EntityType::Item).len(), 1);

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
    assert_eq!(manager.entities_of_type(&EntityType::Npc).len(), 0);

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

    // Only the second player should be tracked as THE player
    assert_eq!(manager.get_player_id(), Some(second_player));
    assert_ne!(manager.get_player_id(), Some(first_player));

    // But both should exist as entities
    assert!(manager.get_entity(first_player).is_some());
    assert!(manager.get_entity(second_player).is_some());

    // Both should be in the Player type list
    let players = manager.entities_of_type(&EntityType::Player);
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
