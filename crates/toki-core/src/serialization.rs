use crate::entity::{Entity, EntityManager};
use crate::game::GameState;
use serde_json;
use std::fs;

pub fn save_entity_to_file(entity: &Entity, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(entity)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_entity_from_file(path: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let entity: Entity = serde_json::from_str(&json)?;
    Ok(entity)
}

pub fn save_scene(
    entity_manager: &EntityManager,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(entity_manager)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_scene(path: &str) -> Result<EntityManager, Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let entity_manager: EntityManager = serde_json::from_str(&json)?;
    Ok(entity_manager)
}

pub fn save_game(game_state: &GameState, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(game_state)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_game(path: &str) -> Result<GameState, Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let game_state: GameState = serde_json::from_str(&json)?;
    Ok(game_state)
}
