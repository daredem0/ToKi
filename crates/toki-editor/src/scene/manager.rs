use anyhow::Result;
use std::path::Path;
use toki_core::assets::tilemap::TileMap;
use toki_core::{GameState, ResourceManager};

/// Manages the game scene data and state for editing
pub struct SceneManager {
    game_state: GameState,
    #[allow(dead_code)] // Will be used for tilemap/sprite rendering
    resources: ResourceManager,
    /// Current loaded tilemap (if any)
    tilemap: Option<TileMap>,
}

impl SceneManager {
    /// Create a new scene manager with empty game state
    pub fn new() -> Result<Self> {
        // Load resources (reuse from toki-runtime)
        let resources = ResourceManager::load_all()
            .map_err(|e| anyhow::anyhow!("Failed to load resources: {e}"))?;

        let game_state = GameState::new_empty();

        tracing::info!("Scene manager created successfully");
        Ok(Self {
            game_state,
            resources,
            tilemap: None,
        })
    }

    /// Create scene manager with provided game state
    pub fn with_game_state(game_state: GameState) -> Result<Self> {
        let mut manager = Self::new()?;
        manager.game_state = game_state;
        Ok(manager)
    }

    #[cfg(test)]
    pub fn with_game_state_and_resources(
        game_state: GameState,
        resources: ResourceManager,
    ) -> Self {
        Self {
            game_state,
            resources,
            tilemap: None,
        }
    }

    /// Get reference to game state
    pub fn game_state(&self) -> &GameState {
        &self.game_state
    }

    /// Get mutable reference to game state
    pub fn game_state_mut(&mut self) -> &mut GameState {
        &mut self.game_state
    }

    /// Get reference to resources
    #[allow(dead_code)] // Will be used for tilemap/sprite rendering
    pub fn resources(&self) -> &ResourceManager {
        &self.resources
    }

    /// Load a tilemap from file
    pub fn load_tilemap<P: AsRef<Path>>(&mut self, map_path: P) -> Result<()> {
        let tilemap = TileMap::load_from_file(&map_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tilemap: {}", e))?;

        // Validate the tilemap
        tilemap
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid tilemap: {}", e))?;

        self.tilemap = Some(tilemap);
        tracing::info!("Loaded tilemap from: {}", map_path.as_ref().display());
        Ok(())
    }

    /// Get reference to current tilemap
    pub fn tilemap(&self) -> Option<&TileMap> {
        self.tilemap.as_ref()
    }

    pub fn tilemap_mut(&mut self) -> Option<&mut TileMap> {
        self.tilemap.as_mut()
    }

    /// Set the current tilemap directly without loading from disk.
    pub fn set_tilemap(&mut self, tilemap: TileMap) -> Result<()> {
        tilemap
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid tilemap: {}", e))?;
        self.tilemap = Some(tilemap);
        tracing::info!("Set in-memory tilemap on scene manager");
        Ok(())
    }

    /// Clear the current tilemap
    pub fn clear_tilemap(&mut self) {
        self.tilemap = None;
        tracing::info!("Cleared tilemap from scene manager");
    }
}
