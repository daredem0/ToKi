use toki_core::entity::{
    runtime_entity_kind_for_category, AudioDef, CollisionDef, EntityFootprint, EntityGrounding,
    EntityKind, MovementComponent, MovementProfile, MovementSoundTrigger,
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

pub fn default_grounding_for_kind(kind: EntityKind, size: [u32; 2]) -> EntityGrounding {
    let footprint = match kind {
        EntityKind::Decoration => bottom_centered_footprint(size, 3, 4, 1, 4),
        EntityKind::Player
        | EntityKind::Npc
        | EntityKind::Item
        | EntityKind::Trigger
        | EntityKind::Projectile => bottom_centered_footprint(size, 1, 2, 1, 4),
    };
    EntityGrounding {
        origin: None,
        footprint: Some(footprint),
    }
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
    let footprint = default_grounding_for_kind(kind, size)
        .footprint
        .expect("default grounding footprint should be present");
    match kind {
        EntityKind::Decoration => CollisionDef {
            enabled: true,
            offset: footprint.offset,
            size: footprint.size,
            trigger: false,
        },
        EntityKind::Item if pickup_present => CollisionDef {
            enabled: true,
            offset: footprint.offset,
            size: footprint.size,
            trigger: true,
        },
        EntityKind::Item => CollisionDef {
            enabled: false,
            offset: footprint.offset,
            size: footprint.size,
            trigger: false,
        },
        _ => CollisionDef {
            enabled: true,
            offset: footprint.offset,
            size: footprint.size,
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

fn bottom_centered_footprint(
    size: [u32; 2],
    width_numerator: u32,
    width_denominator: u32,
    height_numerator: u32,
    height_denominator: u32,
) -> EntityFootprint {
    let width = scaled_dimension(size[0], width_numerator, width_denominator);
    let height = scaled_dimension(size[1], height_numerator, height_denominator);
    let offset_x = ((size[0].saturating_sub(width)) / 2) as i32;
    let offset_y = size[1].saturating_sub(height) as i32;
    EntityFootprint::new([offset_x, offset_y], [width, height])
}

fn scaled_dimension(value: u32, numerator: u32, denominator: u32) -> u32 {
    let scaled = value.saturating_mul(numerator) / denominator.max(1);
    scaled.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_defaults_use_bottom_centered_half_width_footprint() {
        let grounding = default_grounding_for_kind(EntityKind::Npc, [16, 16]);
        let collision = default_collision_for_kind(EntityKind::Npc, [16, 16], false);

        assert_eq!(
            grounding.footprint,
            Some(EntityFootprint::new([4, 12], [8, 4]))
        );
        assert_eq!(collision.offset, [4, 12]);
        assert_eq!(collision.size, [8, 4]);
    }

    #[test]
    fn decoration_defaults_use_wider_bottom_strip_footprint() {
        let grounding = default_grounding_for_kind(EntityKind::Decoration, [64, 64]);
        let collision = default_collision_for_kind(EntityKind::Decoration, [64, 64], false);

        assert_eq!(
            grounding.footprint,
            Some(EntityFootprint::new([8, 48], [48, 16]))
        );
        assert_eq!(collision.offset, [8, 48]);
        assert_eq!(collision.size, [48, 16]);
    }

    #[test]
    fn pickup_defaults_keep_trigger_semantics_but_use_grounded_footprint() {
        let collision = default_collision_for_kind(EntityKind::Item, [16, 16], true);

        assert!(collision.enabled);
        assert!(collision.trigger);
        assert_eq!(collision.offset, [4, 12]);
        assert_eq!(collision.size, [8, 4]);
    }
}
