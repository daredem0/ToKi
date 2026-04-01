//! Functions for applying entity property drafts to definitions and entities.

use super::super::InspectorSystem;
use super::types::EntityPropertyDraft;
use crate::editor_services::commands as editor_commands;
use crate::ui::editor_ui::EditorUI;
use crate::ui::undo_redo::EditorCommand;
use toki_core::entity::{
    decoration_collision_box, AiBehavior, AiComponent, CombatComponent, ControlRole, EntityId,
    InteractionComponent, MovementComponent, StaticObjectRenderDef, StoredEntity,
};

pub(super) const HEALTH_STAT_ID: &str = "health";
pub(super) const ATTACK_POWER_STAT_ID: &str = "attack_power";

impl InspectorSystem {
    pub(in super::super) fn apply_entity_property_draft_to_definition(
        definition: &mut toki_core::entity::EntityDefinition,
        draft: &EntityPropertyDraft,
    ) -> bool {
        let mut changed = false;

        let new_width = draft.size_x.clamp(1, u32::MAX as i64) as u32;
        let new_height = draft.size_y.clamp(1, u32::MAX as i64) as u32;
        if definition.rendering.size != [new_width, new_height] {
            definition.rendering.size = [new_width, new_height];
            changed = true;
        }

        changed |= apply_rendering_fields(definition, draft);
        changed |= apply_attribute_fields(definition, draft);
        changed |= apply_stat_fields(definition, draft);
        changed |= apply_collision_fields(definition, draft);
        changed |= apply_audio_fields(definition, draft);

        changed
    }

    pub(in super::super) fn find_selected_scene_entity(
        ui_state: &EditorUI,
        entity_id: EntityId,
    ) -> Option<StoredEntity> {
        let active_scene_name = ui_state.active_scene.clone()?;
        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == active_scene_name)?;
        scene.stored_entity(entity_id)
    }

    pub(in super::super) fn apply_entity_property_draft_with_undo(
        ui_state: &mut EditorUI,
        entity_id: EntityId,
        draft: &EntityPropertyDraft,
    ) -> bool {
        let Some(active_scene_name) = ui_state.active_scene.clone() else {
            return false;
        };
        let Some(scene_index) = ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == active_scene_name)
        else {
            return false;
        };
        let scene = &ui_state.scenes[scene_index];
        let Some(before) = scene.stored_entity(entity_id) else {
            return false;
        };
        let mut after = before.clone();
        let mut changed = Self::apply_entity_property_draft(&mut after, draft);

        let mut before_entities = vec![before];
        let mut after_entities = vec![after.clone()];

        if matches!(after.entity.control_role, ControlRole::PlayerCharacter) {
            for other in scene.entities() {
                if other.id == entity_id {
                    continue;
                }
                if matches!(other.effective_control_role(), ControlRole::PlayerCharacter) {
                    let Some(mut demoted) = scene.stored_entity(other.id) else {
                        continue;
                    };
                    demoted.entity.control_role = ControlRole::None;
                    let Some(original) = scene.stored_entity(other.id) else {
                        continue;
                    };
                    before_entities.push(original);
                    after_entities.push(demoted);
                    changed = true;
                }
            }
        }

        if !changed {
            return false;
        }

        editor_commands::execute(
            ui_state,
            EditorCommand::update_entities(active_scene_name, before_entities, after_entities),
        )
    }

    pub(in super::super) fn apply_entity_property_draft(
        stored: &mut StoredEntity,
        draft: &EntityPropertyDraft,
    ) -> bool {
        fn set_if_changed<T: PartialEq>(target: &mut T, value: T) -> bool {
            if *target != value {
                *target = value;
                true
            } else {
                false
            }
        }

        fn clamp_to_non_negative_u32(value: i64) -> u32 {
            value.clamp(0, u32::MAX as i64) as u32
        }

        fn clamp_to_min_one_u32(value: i64) -> u32 {
            value.clamp(1, u32::MAX as i64) as u32
        }

        let entity = &mut stored.entity;
        let mut changed = false;

        let new_position = glam::IVec2::new(draft.position_x, draft.position_y);
        changed |= set_if_changed(&mut entity.position, new_position);

        let new_size = glam::UVec2::new(
            clamp_to_min_one_u32(draft.size_x),
            clamp_to_min_one_u32(draft.size_y),
        );
        changed |= set_if_changed(&mut entity.size, new_size);
        let new_static_render = match (
            draft.static_object_sheet.as_ref(),
            draft.static_object_name.as_ref(),
        ) {
            (Some(sheet), Some(object_name)) => Some(StaticObjectRenderDef {
                sheet: sheet.clone(),
                object_name: object_name.clone(),
            }),
            _ => None,
        };
        changed |= set_if_changed(
            &mut entity.rendering.static_object_render,
            new_static_render,
        );

        changed |= set_if_changed(&mut entity.rendering.visible, draft.visible);
        changed |= set_if_changed(&mut entity.rendering.has_shadow, draft.has_shadow);
        let runtime_palette_override = {
            let trimmed = draft.palette_override.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        changed |= set_if_changed(
            &mut entity.rendering.palette_override,
            runtime_palette_override,
        );
        changed |= set_if_changed(&mut entity.active, draft.active);
        changed |= set_if_changed(&mut entity.solid, draft.solid);
        changed |= set_if_changed(&mut entity.control_role, draft.control_role);
        changed |= set_if_changed(
            &mut entity.audio.movement_sound_trigger,
            draft.movement_sound_trigger,
        );
        changed |= set_if_changed(
            &mut entity.audio.footstep_trigger_distance,
            draft.footstep_trigger_distance.max(0.0),
        );
        changed |= set_if_changed(&mut entity.audio.hearing_radius, draft.hearing_radius);
        let new_movement_sound = {
            let trimmed = draft.movement_sound.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };
        changed |= set_if_changed(&mut entity.audio.movement_sound, new_movement_sound);
        changed |= set_if_changed(
            &mut entity.rendering.render_layer,
            draft.render_layer,
        );
        changed |= set_if_changed(
            &mut entity.persistent_across_saves,
            draft.persistent_across_saves,
        );

        let new_health = if draft.health_enabled {
            Some(clamp_to_non_negative_u32(draft.health_value))
        } else {
            None
        };
        let new_attack_power = if draft.attack_power_enabled {
            Some(draft.attack_power_value.clamp(0, i32::MAX as i64) as i32)
        } else {
            None
        };

        let desired_interaction = if draft.interactable {
            Some(InteractionComponent {
                interaction_reach: draft.interaction_reach,
            })
        } else {
            None
        };
        changed |= set_if_changed(&mut stored.components.interaction, desired_interaction);

        let desired_movement = if draft.can_move
            || draft.speed > 0.0
            || draft.movement_profile != toki_core::entity::MovementProfile::LegacyDefault
        {
            Some(MovementComponent {
                speed: draft.speed.max(0.0) as f32,
                movement_profile: draft.movement_profile,
                can_move: draft.can_move,
            })
        } else {
            None
        };
        changed |= set_if_changed(&mut stored.components.movement, desired_movement);

        let desired_ai = if draft.ai_config.behavior != AiBehavior::None
            || draft.ai_config.detection_radius > 0
        {
            Some(AiComponent {
                ai_config: draft.ai_config,
            })
        } else {
            None
        };
        changed |= set_if_changed(&mut stored.components.ai, desired_ai);

        let desired_inventory = if draft.has_inventory {
            Some(
                stored
                    .components
                    .inventory
                    .clone()
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        changed |= set_if_changed(&mut stored.components.inventory, desired_inventory);

        let mut desired_combat = if draft.health_enabled || draft.attack_power_enabled {
            stored.components.combat.clone().unwrap_or_default()
        } else {
            CombatComponent::default()
        };
        desired_combat.health = new_health;
        changed |= Self::set_optional_runtime_stat(
            &mut desired_combat,
            HEALTH_STAT_ID,
            new_health.map(|value| value as i32),
        );
        changed |= Self::set_optional_runtime_stat(
            &mut desired_combat,
            ATTACK_POWER_STAT_ID,
            new_attack_power,
        );
        let desired_combat = if draft.health_enabled || draft.attack_power_enabled {
            Some(desired_combat)
        } else {
            None
        };
        changed |= set_if_changed(&mut stored.components.combat, desired_combat);

        if entity.rendering.static_object_render.is_some() {
            let new_collision_box =
                decoration_collision_box(entity.size, &entity.rendering.grounding, draft.solid);
            let collision_changed = match (&entity.collision_box, &new_collision_box) {
                (Some(current), Some(new_box)) => {
                    current.offset != new_box.offset
                        || current.size != new_box.size
                        || current.trigger != new_box.trigger
                }
                (None, None) => false,
                _ => true,
            };
            if collision_changed {
                entity.collision_box = new_collision_box;
                changed = true;
            }
        } else {
            changed |= apply_entity_collision(entity, draft);
        }

        changed
    }
}

fn apply_rendering_fields(
    definition: &mut toki_core::entity::EntityDefinition,
    draft: &EntityPropertyDraft,
) -> bool {
    let mut changed = false;

    if definition.rendering.render_layer != draft.render_layer {
        definition.rendering.render_layer = draft.render_layer;
        changed = true;
    }
    if definition.rendering.visible != draft.visible {
        definition.rendering.visible = draft.visible;
        changed = true;
    }
    if definition.rendering.has_shadow != draft.has_shadow {
        definition.rendering.has_shadow = draft.has_shadow;
        changed = true;
    }
    let definition_palette_override = {
        let trimmed = draft.palette_override.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };
    if definition.rendering.palette_override != definition_palette_override {
        definition.rendering.palette_override = definition_palette_override;
        changed = true;
    }

    changed
}

fn apply_attribute_fields(
    definition: &mut toki_core::entity::EntityDefinition,
    draft: &EntityPropertyDraft,
) -> bool {
    let mut changed = false;

    if definition.active != draft.active {
        definition.active = draft.active;
        changed = true;
    }
    if definition.solid != draft.solid {
        definition.solid = draft.solid;
        changed = true;
    }
    let desired_interaction = if draft.interactable {
        Some(InteractionComponent {
            interaction_reach: draft.interaction_reach,
        })
    } else {
        None
    };
    if definition.components.interaction != desired_interaction {
        definition.components.interaction = desired_interaction;
        changed = true;
    }
    let desired_movement = if draft.can_move
        || draft.speed > 0.0
        || draft.movement_profile != toki_core::entity::MovementProfile::LegacyDefault
    {
        Some(MovementComponent {
            speed: draft.speed.max(0.0) as f32,
            movement_profile: draft.movement_profile,
            can_move: draft.can_move,
        })
    } else {
        None
    };
    if definition.components.movement != desired_movement {
        definition.components.movement = desired_movement;
        changed = true;
    }
    let desired_ai = if draft.ai_config.behavior != AiBehavior::None
        || draft.ai_config.detection_radius > 0
    {
        Some(AiComponent {
            ai_config: draft.ai_config,
        })
    } else {
        None
    };
    if definition.components.ai != desired_ai {
        definition.components.ai = desired_ai;
        changed = true;
    }
    let desired_inventory = draft.has_inventory.then(Default::default);
    if definition.components.inventory != desired_inventory {
        definition.components.inventory = desired_inventory;
        changed = true;
    }

    changed
}

fn apply_stat_fields(
    definition: &mut toki_core::entity::EntityDefinition,
    draft: &EntityPropertyDraft,
) -> bool {
    let mut changed = false;

    let new_health = if draft.health_enabled {
        Some(draft.health_value.clamp(0, u32::MAX as i64) as u32)
    } else {
        None
    };
    let mut combat = definition.components.combat.clone().unwrap_or_default();
    if combat.health != new_health {
        combat.health = new_health;
        changed = true;
    }
    changed |= InspectorSystem::set_optional_definition_stat(
        &mut definition.components,
        HEALTH_STAT_ID,
        new_health.map(|value| value as i32),
    );
    changed |= InspectorSystem::set_optional_definition_stat(
        &mut definition.components,
        ATTACK_POWER_STAT_ID,
        if draft.attack_power_enabled {
            Some(draft.attack_power_value.clamp(0, i32::MAX as i64) as i32)
        } else {
            None
        },
    );
    definition.components.combat = if draft.health_enabled || draft.attack_power_enabled {
        Some(combat)
    } else {
        None
    };

    changed
}

fn apply_collision_fields(
    definition: &mut toki_core::entity::EntityDefinition,
    draft: &EntityPropertyDraft,
) -> bool {
    let mut changed = false;

    let new_collision_enabled = draft.collision.enabled;
    if definition.collision.enabled != new_collision_enabled {
        definition.collision.enabled = new_collision_enabled;
        changed = true;
    }
    let new_collision_offset = [draft.collision.offset_x, draft.collision.offset_y];
    if definition.collision.offset != new_collision_offset {
        definition.collision.offset = new_collision_offset;
        changed = true;
    }
    let new_collision_size = [
        draft.collision.size_x.clamp(1, u32::MAX as i64) as u32,
        draft.collision.size_y.clamp(1, u32::MAX as i64) as u32,
    ];
    if definition.collision.size != new_collision_size {
        definition.collision.size = new_collision_size;
        changed = true;
    }
    if definition.collision.trigger != draft.collision.trigger {
        definition.collision.trigger = draft.collision.trigger;
        changed = true;
    }

    changed
}

fn apply_audio_fields(
    definition: &mut toki_core::entity::EntityDefinition,
    draft: &EntityPropertyDraft,
) -> bool {
    let mut changed = false;

    if definition.audio.movement_sound_trigger != draft.movement_sound_trigger {
        definition.audio.movement_sound_trigger = draft.movement_sound_trigger;
        changed = true;
    }
    let new_footstep_distance = draft.footstep_trigger_distance.max(0.0);
    if (definition.audio.footstep_trigger_distance - new_footstep_distance).abs() > f32::EPSILON {
        definition.audio.footstep_trigger_distance = new_footstep_distance;
        changed = true;
    }
    if definition.audio.hearing_radius != draft.hearing_radius {
        definition.audio.hearing_radius = draft.hearing_radius;
        changed = true;
    }
    let new_movement_sound = draft.movement_sound.trim().to_string();
    if definition.audio.movement_sound != new_movement_sound {
        definition.audio.movement_sound = new_movement_sound;
        changed = true;
    }

    changed
}

fn apply_entity_collision(
    entity: &mut toki_core::entity::Entity,
    draft: &EntityPropertyDraft,
) -> bool {
    fn set_if_changed<T: PartialEq>(target: &mut T, value: T) -> bool {
        if *target != value {
            *target = value;
            true
        } else {
            false
        }
    }

    fn clamp_to_min_one_u32(value: i64) -> u32 {
        value.clamp(1, u32::MAX as i64) as u32
    }

    let mut changed = false;

    if draft.collision.enabled {
        if entity.collision_box.is_none() {
            entity.collision_box = Some(toki_core::collision::CollisionBox::solid_box(entity.size));
            changed = true;
        }

        if let Some(collision_box) = entity.collision_box.as_mut() {
            changed |= set_if_changed(
                &mut collision_box.offset,
                glam::IVec2::new(draft.collision.offset_x, draft.collision.offset_y),
            );
            changed |= set_if_changed(
                &mut collision_box.size,
                glam::UVec2::new(
                    clamp_to_min_one_u32(draft.collision.size_x),
                    clamp_to_min_one_u32(draft.collision.size_y),
                ),
            );
            changed |= set_if_changed(&mut collision_box.trigger, draft.collision.trigger);
        }
    } else if entity.collision_box.is_some() {
        entity.collision_box = None;
        changed = true;
    }

    changed
}
