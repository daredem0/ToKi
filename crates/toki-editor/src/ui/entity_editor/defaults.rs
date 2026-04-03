//! Default entity definitions and component values.

use toki_core::entity::{
    AnimationsDef, ComponentsDef, EntityDefinition, PickupDef, PrimaryProjectileDef, RenderingDef,
};

use crate::ui::entity_kind_policy::{
    default_audio_for_kind, default_collision_for_kind, default_grounding_for_kind,
    default_movement_for_kind,
    effective_kind_for_category,
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
    let kind = effective_kind_for_category(category);
    let size = [32, 32];
    EntityDefinition {
        name: name.to_string().into(),
        display_name: display_name.to_string(),
        description: String::new(),
        rendering: RenderingDef {
            size,
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: default_grounding_for_kind(kind, size),
        },
        solid: kind != toki_core::entity::EntityKind::Item,
        active: true,
        components: ComponentsDef {
            movement: default_movement_for_kind(kind),
            ai: None,
            interaction: None,
            combat: None,
            primary_projectile: None,
            pickup: None,
            inventory: None,
        },
        collision: default_collision_for_kind(kind, size, false),
        audio: default_audio_for_kind(kind),
        animations: AnimationsDef {
            atlas_name: String::new(),
            clips: Vec::new(),
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: Vec::new(),
    }
}
