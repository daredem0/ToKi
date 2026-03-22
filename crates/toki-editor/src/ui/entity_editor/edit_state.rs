//! Entity edit state for the entity editor.

use std::collections::HashMap;
use std::path::PathBuf;

use toki_core::entity::{AiBehavior, AiConfig, EntityDefinition};

use super::defaults::{default_pickup_def, default_projectile_def};
use super::toggles::ComponentToggles;

/// State for editing an entity definition.
#[derive(Debug, Clone)]
pub struct EntityEditState {
    /// The entity definition being edited
    pub definition: EntityDefinition,
    /// Path to the definition file
    pub file_path: PathBuf,
    /// Which optional components are enabled
    pub toggles: ComponentToggles,
    /// Tags as editable comma-separated string
    pub tags_input: String,
    /// Whether changes have been made
    pub dirty: bool,
    /// Validation errors by field
    pub validation_errors: HashMap<String, String>,
}

impl EntityEditState {
    /// Create edit state from a loaded entity definition
    pub fn from_definition(def: EntityDefinition, file_path: PathBuf) -> Self {
        let toggles = ComponentToggles::from_definition(&def);
        let tags_input = def.tags.join(", ");
        Self {
            definition: def,
            file_path,
            toggles,
            tags_input,
            dirty: false,
            validation_errors: HashMap::new(),
        }
    }

    /// Mark the entity as modified
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Sync tags from the input string back to the definition
    pub fn sync_tags(&mut self) {
        self.definition.tags = self
            .tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    /// Toggle health component on/off
    pub fn toggle_health(&mut self) {
        self.toggles.health_enabled = !self.toggles.health_enabled;
        self.definition.attributes.health = if self.toggles.health_enabled {
            Some(100)
        } else {
            None
        };
        self.mark_dirty();
    }

    /// Toggle inventory component on/off
    pub fn toggle_inventory(&mut self) {
        self.toggles.inventory_enabled = !self.toggles.inventory_enabled;
        self.definition.attributes.has_inventory = self.toggles.inventory_enabled;
        self.mark_dirty();
    }

    /// Toggle projectile component on/off
    pub fn toggle_projectile(&mut self) {
        self.toggles.projectile_enabled = !self.toggles.projectile_enabled;
        self.definition.attributes.primary_projectile = if self.toggles.projectile_enabled {
            Some(default_projectile_def())
        } else {
            None
        };
        self.mark_dirty();
    }

    /// Toggle pickup component on/off
    pub fn toggle_pickup(&mut self) {
        self.toggles.pickup_enabled = !self.toggles.pickup_enabled;
        self.definition.attributes.pickup = if self.toggles.pickup_enabled {
            Some(default_pickup_def())
        } else {
            None
        };
        self.mark_dirty();
    }

    /// Toggle AI component on/off
    pub fn toggle_ai(&mut self) {
        self.toggles.ai_enabled = !self.toggles.ai_enabled;
        self.definition.attributes.ai_config = if self.toggles.ai_enabled {
            AiConfig {
                behavior: AiBehavior::Wander,
                detection_radius: 128,
            }
        } else {
            AiConfig::default()
        };
        self.mark_dirty();
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn toggle_collision(&mut self) {
        self.toggles.collision_enabled = !self.toggles.collision_enabled;
        self.definition.collision.enabled = self.toggles.collision_enabled;
        self.mark_dirty();
    }

    /// Toggle audio component on/off
    pub fn toggle_audio(&mut self) {
        self.toggles.audio_enabled = !self.toggles.audio_enabled;
        if !self.toggles.audio_enabled {
            self.definition.audio.movement_sound.clear();
            self.definition.audio.collision_sound = None;
        }
        self.mark_dirty();
    }

    /// Validate the current entity definition
    pub fn validate(&mut self) -> bool {
        self.validation_errors.clear();
        self.validate_name();
        self.validate_display_name();
        self.validate_size();
        self.validate_health();
        self.validate_collision_size();
        self.validation_errors.is_empty()
    }

    fn validate_name(&mut self) {
        let name = self.definition.name.trim();
        if name.is_empty() {
            self.validation_errors
                .insert("name".to_string(), "Name is required".to_string());
        } else if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.validation_errors.insert(
                "name".to_string(),
                "Name must contain only letters, numbers, and underscores".to_string(),
            );
        } else if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            self.validation_errors.insert(
                "name".to_string(),
                "Name must not start with a number".to_string(),
            );
        }
    }

    fn validate_display_name(&mut self) {
        if self.definition.display_name.trim().is_empty() {
            self.validation_errors.insert(
                "display_name".to_string(),
                "Display name is required".to_string(),
            );
        }
    }

    fn validate_size(&mut self) {
        if self.definition.rendering.size[0] == 0 || self.definition.rendering.size[1] == 0 {
            self.validation_errors.insert(
                "size".to_string(),
                "Size must be greater than zero".to_string(),
            );
        }
    }

    fn validate_health(&mut self) {
        if self.toggles.health_enabled {
            if let Some(health) = self.definition.attributes.health {
                if health == 0 {
                    self.validation_errors.insert(
                        "health".to_string(),
                        "Health must be greater than zero".to_string(),
                    );
                }
            }
        }
    }

    fn validate_collision_size(&mut self) {
        if self.toggles.collision_enabled
            && (self.definition.collision.size[0] == 0 || self.definition.collision.size[1] == 0)
        {
            self.validation_errors.insert(
                "collision_size".to_string(),
                "Collision size must be greater than zero".to_string(),
            );
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn has_error(&self, field: &str) -> bool {
        self.validation_errors.contains_key(field)
    }

    /// Get the validation error for a field
    pub fn get_error(&self, field: &str) -> Option<&String> {
        self.validation_errors.get(field)
    }
}
