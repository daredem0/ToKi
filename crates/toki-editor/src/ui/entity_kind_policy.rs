use toki_core::entity::{
    runtime_entity_kind_for_category, AudioDef, CollisionDef, EntityKind, MovementComponent,
    MovementProfile, MovementSoundTrigger,
};

pub fn effective_kind_for_category(category: &str) -> EntityKind {
    runtime_entity_kind_for_category(category)
}

pub fn kind_supports_movement(kind: EntityKind) -> bool {
    matches!(kind, EntityKind::Player | EntityKind::Npc)
}

pub fn kind_supports_audio(kind: EntityKind) -> bool {
    matches!(kind, EntityKind::Player | EntityKind::Npc)
}

pub fn kind_supports_combat_defaults(kind: EntityKind) -> bool {
    matches!(kind, EntityKind::Player | EntityKind::Npc)
}

pub fn uses_decoration_collision_policy(kind: EntityKind) -> bool {
    kind == EntityKind::Decoration
}

pub fn default_audio_for_kind(kind: EntityKind) -> AudioDef {
    if kind_supports_audio(kind) {
        AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        }
    } else {
        AudioDef {
            footstep_trigger_distance: 0.0,
            hearing_radius: 0,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        }
    }
}

pub fn default_collision_for_kind(
    kind: EntityKind,
    size: [u32; 2],
    pickup_present: bool,
) -> CollisionDef {
    match kind {
        EntityKind::Decoration => CollisionDef {
            enabled: true,
            offset: [0, 0],
            size,
            trigger: false,
        },
        EntityKind::Item if pickup_present => CollisionDef {
            enabled: true,
            offset: [0, 0],
            size,
            trigger: true,
        },
        EntityKind::Item => CollisionDef {
            enabled: false,
            offset: [0, 0],
            size,
            trigger: false,
        },
        _ => CollisionDef {
            enabled: true,
            offset: [0, 0],
            size,
            trigger: false,
        },
    }
}

pub fn default_movement_for_kind(kind: EntityKind) -> Option<MovementComponent> {
    kind_supports_movement(kind).then_some(MovementComponent {
        speed: 100.0,
        movement_profile: match kind {
            EntityKind::Player => MovementProfile::PlayerWasd,
            EntityKind::Npc => MovementProfile::LegacyDefault,
            _ => MovementProfile::None,
        },
        can_move: true,
    })
}
