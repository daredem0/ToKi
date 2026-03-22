//! Rule animation application.
//!
//! Contains logic for applying buffered animation commands.

use crate::animation::AnimationState;
use crate::entity::EntityId;

use super::GameState;

impl GameState {
    pub(in crate::game) fn apply_rule_animations(
        &mut self,
        pending_animations: Vec<(EntityId, AnimationState)>,
    ) {
        for (entity_id, state) in pending_animations {
            let Some(entity) = self.entity_manager.get_entity_mut(entity_id) else {
                continue;
            };
            let Some(animation_controller) = entity.attributes.animation_controller.as_mut() else {
                continue;
            };

            if animation_controller.current_clip_state != state {
                animation_controller.play(state);
            }
        }
    }
}
