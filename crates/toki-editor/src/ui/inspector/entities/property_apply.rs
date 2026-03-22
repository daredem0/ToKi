//! Functions for applying entity property drafts to definitions and entities.

use super::super::InspectorSystem;
use super::types::EntityPropertyDraft;
use crate::editor_services::commands as editor_commands;
use crate::ui::editor_ui::EditorUI;
use crate::ui::undo_redo::EditorCommand;
use toki_core::entity::{ControlRole, EntityId};

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
    ) -> Option<toki_core::entity::Entity> {
        let active_scene_name = ui_state.active_scene.clone()?;
        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == active_scene_name)?;
        scene
            .entities
            .iter()
            .find(|entity| entity.id == entity_id)
            .cloned()
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
        let Some(entity_index) = ui_state.scenes[scene_index]
            .entities
            .iter()
            .position(|entity| entity.id == entity_id)
        else {
            return false;
        };

        let before = ui_state.scenes[scene_index].entities[entity_index].clone();
        let mut after = before.clone();
        let mut changed = Self::apply_entity_property_draft(&mut after, draft);

        let mut before_entities = vec![before];
        let mut after_entities = vec![after.clone()];

        if matches!(after.control_role, ControlRole::PlayerCharacter) {
            for other in ui_state.scenes[scene_index].entities.iter() {
                if other.id == entity_id {
                    continue;
                }
                if matches!(other.effective_control_role(), ControlRole::PlayerCharacter) {
                    let mut demoted = other.clone();
                    demoted.control_role = ControlRole::None;
                    before_entities.push(other.clone());
                    after_entities.push(demoted);
                    changed = true;
                }
            }
        }

        if !changed {
            return false;
        }

        editor_commands::execute(ui_state, EditorCommand::update_entities(
            active_scene_name,
            before_entities,
            after_entities,
        ))
    }

    pub(in super::super) fn apply_entity_property_draft(
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

        fn clamp_to_non_negative_u32(value: i64) -> u32 {
            value.clamp(0, u32::MAX as i64) as u32
        }

        fn clamp_to_min_one_u32(value: i64) -> u32 {
            value.clamp(1, u32::MAX as i64) as u32
        }

        let mut changed = false;

        let new_position = glam::IVec2::new(draft.position_x, draft.position_y);
        changed |= set_if_changed(&mut entity.position, new_position);

        let new_size = glam::UVec2::new(
            clamp_to_min_one_u32(draft.size_x),
            clamp_to_min_one_u32(draft.size_y),
        );
        changed |= set_if_changed(&mut entity.size, new_size);

        changed |= set_if_changed(&mut entity.attributes.visible, draft.visible);
        changed |= set_if_changed(&mut entity.attributes.has_shadow, draft.has_shadow);
        changed |= set_if_changed(&mut entity.attributes.active, draft.active);
        changed |= set_if_changed(&mut entity.attributes.solid, draft.solid);
        changed |= set_if_changed(&mut entity.attributes.interactable, draft.interactable);
        changed |= set_if_changed(
            &mut entity.attributes.interaction_reach,
            draft.interaction_reach,
        );
        changed |= set_if_changed(&mut entity.attributes.can_move, draft.can_move);
        changed |= set_if_changed(&mut entity.control_role, draft.control_role);
        changed |= set_if_changed(&mut entity.attributes.ai_config, draft.ai_config);
        changed |= set_if_changed(
            &mut entity.attributes.movement_profile,
            draft.movement_profile,
        );
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
        changed |= set_if_changed(&mut entity.attributes.has_inventory, draft.has_inventory);
        changed |= set_if_changed(&mut entity.attributes.speed, draft.speed.max(0.0) as f32);
        changed |= set_if_changed(&mut entity.attributes.render_layer, draft.render_layer);

        let new_health = if draft.health_enabled {
            Some(clamp_to_non_negative_u32(draft.health_value))
        } else {
            None
        };
        changed |= set_if_changed(&mut entity.attributes.health, new_health);
        changed |= Self::set_optional_runtime_stat(
            &mut entity.attributes,
            HEALTH_STAT_ID,
            new_health.map(|value| value as i32),
        );

        let new_attack_power = if draft.attack_power_enabled {
            Some(draft.attack_power_value.clamp(0, i32::MAX as i64) as i32)
        } else {
            None
        };
        changed |= Self::set_optional_runtime_stat(
            &mut entity.attributes,
            ATTACK_POWER_STAT_ID,
            new_attack_power,
        );

        changed |= apply_entity_collision(entity, draft);

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

    changed
}

fn apply_attribute_fields(
    definition: &mut toki_core::entity::EntityDefinition,
    draft: &EntityPropertyDraft,
) -> bool {
    let mut changed = false;

    if definition.attributes.active != draft.active {
        definition.attributes.active = draft.active;
        changed = true;
    }
    if definition.attributes.solid != draft.solid {
        definition.attributes.solid = draft.solid;
        changed = true;
    }
    if definition.attributes.interactable != draft.interactable {
        definition.attributes.interactable = draft.interactable;
        changed = true;
    }
    if definition.attributes.interaction_reach != draft.interaction_reach {
        definition.attributes.interaction_reach = draft.interaction_reach;
        changed = true;
    }
    if definition.attributes.can_move != draft.can_move {
        definition.attributes.can_move = draft.can_move;
        changed = true;
    }
    if definition.attributes.ai_config != draft.ai_config {
        definition.attributes.ai_config = draft.ai_config;
        changed = true;
    }
    if definition.attributes.movement_profile != draft.movement_profile {
        definition.attributes.movement_profile = draft.movement_profile;
        changed = true;
    }
    let new_speed = draft.speed.max(0.0) as f32;
    if (definition.attributes.speed - new_speed).abs() > f32::EPSILON {
        definition.attributes.speed = new_speed;
        changed = true;
    }
    if definition.attributes.has_inventory != draft.has_inventory {
        definition.attributes.has_inventory = draft.has_inventory;
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
    if definition.attributes.health != new_health {
        definition.attributes.health = new_health;
        changed = true;
    }
    changed |= InspectorSystem::set_optional_definition_stat(
        &mut definition.attributes,
        HEALTH_STAT_ID,
        new_health.map(|value| value as i32),
    );
    changed |= InspectorSystem::set_optional_definition_stat(
        &mut definition.attributes,
        ATTACK_POWER_STAT_ID,
        if draft.attack_power_enabled {
            Some(draft.attack_power_value.clamp(0, i32::MAX as i64) as i32)
        } else {
            None
        },
    );

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
