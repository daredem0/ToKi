use super::input_state::InputStateEffect;
use super::{GameState, InputAction, InputKey, RuntimeState};
use crate::entity::{EntityId, MovementProfile};
use crate::rules::RuleKey;

pub struct InputSystem;

impl InputSystem {
    pub fn clear(runtime: &mut RuntimeState) {
        runtime.input.clear();
    }

    pub fn handle_key_press(runtime: &mut RuntimeState, key: InputKey) {
        match runtime.input.handle_key_press(key) {
            InputStateEffect::ToggleDebugCollisionRendering => {
                runtime.debug_collision_rendering = !runtime.debug_collision_rendering;
                tracing::info!(
                    "Debug collision rendering: {}",
                    runtime.debug_collision_rendering
                );
            }
            InputStateEffect::None => {}
        }
    }

    pub fn handle_key_release(runtime: &mut RuntimeState, key: InputKey) {
        runtime.input.handle_key_release(key);
    }

    pub fn handle_profile_key_press(
        runtime: &mut RuntimeState,
        profile: MovementProfile,
        key: InputKey,
    ) {
        runtime.input.handle_profile_key_press(profile, key);
    }

    pub fn handle_profile_key_release(
        runtime: &mut RuntimeState,
        profile: MovementProfile,
        key: InputKey,
    ) {
        runtime.input.handle_profile_key_release(profile, key);
    }

    pub fn handle_profile_action_press(
        runtime: &mut RuntimeState,
        profile: MovementProfile,
        action: InputAction,
    ) {
        runtime.input.handle_profile_action_press(profile, action);
    }

    pub fn handle_profile_action_release(
        runtime: &mut RuntimeState,
        profile: MovementProfile,
        action: InputAction,
    ) {
        runtime.input.handle_profile_action_release(profile, action);
    }
}

impl GameState {
    pub(super) fn controlled_input_entity_ids(&self) -> Vec<EntityId> {
        let mut entity_ids = self
            .world
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                let entity = self.world.entity_manager.get_entity(entity_id)?;
                if matches!(
                    entity.effective_movement_profile(self.world.entity_manager.movement(entity_id)),
                    MovementProfile::PlayerWasd
                ) {
                    Some(entity_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        entity_ids.sort_unstable();
        entity_ids
    }

    pub(super) fn held_keys_for_profile(&self, movement_profile: MovementProfile) -> Vec<InputKey> {
        self.runtime.input.held_keys_for_profile(movement_profile)
    }

    pub(super) fn all_held_keys(&self) -> Vec<InputKey> {
        self.runtime.input.all_held_keys()
    }

    pub(super) fn take_pending_profile_actions(
        &mut self,
    ) -> std::collections::HashMap<MovementProfile, std::collections::HashSet<InputAction>> {
        self.runtime.input.take_pending_profile_actions()
    }
    pub(super) fn to_rule_key(key: InputKey) -> RuleKey {
        match key {
            InputKey::Up => RuleKey::Up,
            InputKey::Down => RuleKey::Down,
            InputKey::Left => RuleKey::Left,
            InputKey::Right => RuleKey::Right,
            InputKey::DebugToggle => RuleKey::DebugToggle,
            InputKey::Interact => RuleKey::Interact,
            InputKey::AttackPrimary => RuleKey::AttackPrimary,
            InputKey::AttackSecondary => RuleKey::AttackSecondary,
            InputKey::Inventory => RuleKey::Inventory,
            InputKey::Pause => RuleKey::Pause,
        }
    }

    pub(super) fn to_input_key(key: RuleKey) -> InputKey {
        match key {
            RuleKey::Up => InputKey::Up,
            RuleKey::Down => InputKey::Down,
            RuleKey::Left => InputKey::Left,
            RuleKey::Right => InputKey::Right,
            RuleKey::DebugToggle => InputKey::DebugToggle,
            RuleKey::Interact => InputKey::Interact,
            RuleKey::AttackPrimary => InputKey::AttackPrimary,
            RuleKey::AttackSecondary => InputKey::AttackSecondary,
            RuleKey::Inventory => InputKey::Inventory,
            RuleKey::Pause => InputKey::Pause,
        }
    }
}
