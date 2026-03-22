//! Default entity definitions and component values.

use std::collections::HashMap;

use toki_core::entity::{
    AiConfig, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityDefinition,
    MovementProfile, MovementSoundTrigger, PickupDef, PrimaryProjectileDef, RenderingDef,
};

/// Create default projectile definition
pub fn default_projectile_def() -> PrimaryProjectileDef {
    PrimaryProjectileDef {
        sheet: String::new(),
        object_name: String::new(),
        size: [8, 8],
        speed: 200,
        damage: 10,
        lifetime_ticks: 60,
        spawn_offset: [0, 0],
    }
}

/// Create default pickup definition
pub fn default_pickup_def() -> PickupDef {
    PickupDef {
        item_id: String::new(),
        count: 1,
    }
}

/// Create a default entity definition with sensible defaults
pub fn create_default_definition(
    name: &str,
    display_name: &str,
    category: &str,
) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string(),
        display_name: display_name.to_string(),
        description: String::new(),
        rendering: RenderingDef {
            size: [32, 32],
            render_layer: 0,
            visible: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: HashMap::new(),
            speed: 100.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 32,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::default(),
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size: [32, 32],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: String::new(),
            clips: Vec::new(),
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: Vec::new(),
    }
}
