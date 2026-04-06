use super::{runtime_entity_kind_for_category, EntityDefinition, EntityKind, StoredEntity};

struct ValidationInput<'a> {
    kind: EntityKind,
    has_movement: bool,
    has_ai: bool,
    has_combat: bool,
    has_pickup: bool,
    collision_trigger: bool,
    collision_enabled: bool,
    item_id: Option<&'a str>,
    item_count: Option<u32>,
}

fn collect_component_warnings(input: ValidationInput<'_>) -> Vec<String> {
    let mut warnings = Vec::new();
    let ValidationInput {
        kind,
        has_movement,
        has_ai,
        has_combat,
        has_pickup,
        collision_trigger,
        collision_enabled,
        item_id,
        item_count,
    } = input;

    if kind == EntityKind::Decoration {
        if has_movement {
            warnings.push("Decoration should not carry MovementComponent".to_string());
        }
        if has_ai {
            warnings.push("Decoration should not carry AiComponent".to_string());
        }
        if has_combat {
            warnings.push("Decoration should not carry CombatComponent".to_string());
        }
    }

    if kind == EntityKind::Item && has_ai {
        warnings.push("Item should not carry AiComponent".to_string());
    }
    if kind == EntityKind::Item && has_movement {
        warnings.push("Item should not carry MovementComponent".to_string());
    }

    if has_pickup && kind != EntityKind::Item {
        warnings.push("Pickup payload is only valid on EntityKind::Item".to_string());
    }

    if kind == EntityKind::Item && has_pickup {
        if item_id.is_none_or(|id| id.trim().is_empty()) {
            warnings.push("Item pickup is missing item_id".to_string());
        }
        if item_count.unwrap_or(0) == 0 {
            warnings.push("Item pickup count must be greater than 0".to_string());
        }
        if !collision_enabled || !collision_trigger {
            warnings.push("Pickup item should use trigger collision".to_string());
        }
    }

    warnings
}

fn collect_decoration_animation_warnings(definition: &EntityDefinition) -> Vec<String> {
    let mut warnings = Vec::new();
    let clips = &definition.animations.clips;

    if clips.is_empty() {
        return warnings;
    }

    if clips.len() != 1 {
        warnings.push("Animated decoration should define exactly one animation clip".to_string());
    }

    if definition.animations.default_state != "idle" {
        warnings.push("Animated decoration default_state should be 'idle'".to_string());
    }

    for clip in clips {
        if clip.state != "idle" {
            warnings.push("Animated decoration clips should use only the 'idle' state".to_string());
            break;
        }
    }

    warnings
}

pub fn validate_entity_definition_warnings(definition: &EntityDefinition) -> Vec<String> {
    let kind = runtime_entity_kind_for_category(&definition.category);
    let mut warnings = collect_component_warnings(ValidationInput {
        kind,
        has_movement: definition.components.movement.is_some(),
        has_ai: definition.components.ai.is_some(),
        has_combat: definition.components.combat.is_some(),
        has_pickup: definition.components.pickup.is_some(),
        collision_trigger: definition.collision.trigger,
        collision_enabled: definition.collision.enabled,
        item_id: definition
            .components
            .pickup
            .as_ref()
            .map(|pickup| pickup.item_id.as_str()),
        item_count: definition
            .components
            .pickup
            .as_ref()
            .map(|pickup| pickup.count),
    });

    if kind == EntityKind::Decoration {
        warnings.extend(collect_decoration_animation_warnings(definition));
    }

    warnings
}

pub fn validate_stored_entity_warnings(stored: &StoredEntity) -> Vec<String> {
    collect_component_warnings(ValidationInput {
        kind: stored.entity.entity_kind,
        has_movement: stored.components.movement.is_some(),
        has_ai: stored.components.ai.is_some(),
        has_combat: stored.components.combat.is_some(),
        has_pickup: stored.components.pickup.is_some(),
        collision_trigger: stored
            .entity
            .collision_box
            .as_ref()
            .is_some_and(|collision| collision.trigger),
        collision_enabled: stored.entity.collision_box.is_some(),
        item_id: stored
            .components
            .pickup
            .as_ref()
            .map(|pickup| pickup.item_id.as_str()),
        item_count: stored.components.pickup.as_ref().map(|pickup| pickup.count),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{
        AnimationClipDef, AnimationsDef, AudioDef, CollisionDef, ComponentsDef, EntityDefinition,
        RenderingDef, StaticObjectRenderDef,
    };

    fn base_definition(category: &str) -> EntityDefinition {
        EntityDefinition {
            name: "test".to_string().into(),
            display_name: "Test".to_string(),
            description: String::new(),
            rendering: RenderingDef {
                size: [16, 16],
                render_layer: 0,
                visible: true,
                has_shadow: true,
                has_drop_shadow: false,
                palette_override: None,
                static_object: Some(StaticObjectRenderDef {
                    sheet: "items".to_string(),
                    object_name: "coin".to_string(),
                }),
                grounding: Default::default(),
            },
            solid: false,
            active: true,
            components: ComponentsDef::default(),
            collision: CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: true,
            },
            audio: AudioDef {
                footstep_trigger_distance: 0.0,
                hearing_radius: 0,
                movement_sound_trigger: Default::default(),
                movement_sound: String::new(),
                collision_sound: None,
            },
            animations: AnimationsDef {
                atlas_name: String::new(),
                clips: Vec::new(),
                default_state: String::new(),
            },
            category: category.to_string(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn validation_warns_for_item_with_movement_and_missing_pickup_payload() {
        let mut definition = base_definition("item");
        definition.components.movement = Some(Default::default());
        definition.components.pickup = Some(crate::entity::PickupDef {
            item_id: String::new(),
            count: 0,
        });

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings.iter().any(|w| w.contains("MovementComponent")));
        assert!(warnings.iter().any(|w| w.contains("missing item_id")));
        assert!(warnings.iter().any(|w| w.contains("greater than 0")));
    }

    #[test]
    fn validation_warns_for_non_item_pickup_payload() {
        let mut definition = base_definition("decoration");
        definition.components.pickup = Some(crate::entity::PickupDef {
            item_id: "coin".to_string(),
            count: 1,
        });

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings
            .iter()
            .any(|w| w.contains("only valid on EntityKind::Item")));
    }

    #[test]
    fn validation_warns_for_item_pickup_without_trigger_collision() {
        let mut definition = base_definition("item");
        definition.components.pickup = Some(crate::entity::PickupDef {
            item_id: "coin".to_string(),
            count: 1,
        });
        definition.collision.trigger = false;

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings
            .iter()
            .any(|w| w.contains("Pickup item should use trigger collision")));
    }

    #[test]
    fn validation_accepts_static_decoration_without_animation_clips() {
        let definition = base_definition("decoration");

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(!warnings.iter().any(|w| w.contains("Animated decoration")));
    }

    #[test]
    fn validation_accepts_single_idle_decoration_animation() {
        let mut definition = base_definition("decoration");
        definition.rendering.static_object = None;
        definition.animations.clips = vec![AnimationClipDef {
            state: "idle".to_string(),
            frame_tiles: vec!["torch/idle_a".to_string()],
            frame_positions: None,
            frame_duration_ms: 120.0,
            frame_durations_ms: None,
            loop_mode: "loop".to_string(),
        }];
        definition.animations.default_state = "idle".to_string();

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(!warnings.iter().any(|w| w.contains("Animated decoration")));
    }

    #[test]
    fn decoration_with_movement_ai_combat_warns_for_each() {
        let mut definition = base_definition("decoration");
        definition.components.movement = Some(Default::default());
        definition.components.ai = Some(Default::default());
        definition.components.combat = Some(Default::default());

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings.iter().any(|w| w.contains("MovementComponent")));
        assert!(warnings.iter().any(|w| w.contains("AiComponent")));
        assert!(warnings.iter().any(|w| w.contains("CombatComponent")));
    }

    #[test]
    fn item_with_ai_warns() {
        let mut definition = base_definition("item");
        definition.components.ai = Some(Default::default());

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings.iter().any(|w| w.contains("AiComponent")));
    }

    #[test]
    fn valid_item_pickup_produces_no_warnings() {
        let mut definition = base_definition("item");
        definition.components.pickup = Some(crate::entity::PickupDef {
            item_id: "coin".to_string(),
            count: 1,
        });
        // base_definition already sets collision.enabled=true, trigger=true

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    }

    #[test]
    fn item_pickup_with_whitespace_only_id_warns() {
        let mut definition = base_definition("item");
        definition.components.pickup = Some(crate::entity::PickupDef {
            item_id: "   ".to_string(),
            count: 1,
        });

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings.iter().any(|w| w.contains("missing item_id")));
    }

    #[test]
    fn validation_warns_for_non_idle_or_multiple_decoration_animations() {
        let mut definition = base_definition("decoration");
        definition.rendering.static_object = None;
        definition.animations.clips = vec![
            AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["torch/idle_a".to_string()],
                frame_positions: None,
                frame_duration_ms: 120.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            },
            AnimationClipDef {
                state: "walk_down".to_string(),
                frame_tiles: vec!["torch/walk_a".to_string()],
                frame_positions: None,
                frame_duration_ms: 120.0,
                frame_durations_ms: None,
                loop_mode: "loop".to_string(),
            },
        ];
        definition.animations.default_state = "walk_down".to_string();

        let warnings = validate_entity_definition_warnings(&definition);
        assert!(warnings
            .iter()
            .any(|w| w.contains("exactly one animation clip")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("default_state should be 'idle'")));
        assert!(warnings.iter().any(|w| w.contains("only the 'idle' state")));
    }
}
