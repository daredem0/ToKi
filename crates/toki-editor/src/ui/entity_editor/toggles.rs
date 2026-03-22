//! Component toggle state for entity editor.

use toki_core::entity::{AiBehavior, AudioDef, EntityDefinition};

/// Tracks which optional components are enabled for the current entity.
#[derive(Debug, Clone, Default)]
pub struct ComponentToggles {
    pub health_enabled: bool,
    pub inventory_enabled: bool,
    pub projectile_enabled: bool,
    pub pickup_enabled: bool,
    pub ai_enabled: bool,
    pub collision_enabled: bool,
    pub audio_enabled: bool,
}

impl ComponentToggles {
    /// Create toggles from an EntityDefinition, detecting which components are active
    pub fn from_definition(def: &EntityDefinition) -> Self {
        Self {
            health_enabled: def.attributes.health.is_some(),
            inventory_enabled: def.attributes.has_inventory,
            projectile_enabled: def.attributes.primary_projectile.is_some(),
            pickup_enabled: def.attributes.pickup.is_some(),
            ai_enabled: def.attributes.ai_config.behavior != AiBehavior::None,
            collision_enabled: def.collision.enabled,
            audio_enabled: Self::has_audio_config(&def.audio),
        }
    }

    /// Check if audio has any meaningful configuration
    fn has_audio_config(audio: &AudioDef) -> bool {
        !audio.movement_sound.is_empty() || audio.collision_sound.is_some()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn enabled_count(&self) -> usize {
        [
            self.health_enabled,
            self.inventory_enabled,
            self.projectile_enabled,
            self.pickup_enabled,
            self.ai_enabled,
            self.collision_enabled,
            self.audio_enabled,
        ]
        .iter()
        .filter(|&&b| b)
        .count()
    }

}
