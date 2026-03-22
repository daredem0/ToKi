use super::input_state::InputStateEffect;
use super::{GameState, InputAction, InputKey};
use crate::entity::{EntityId, MovementProfile};
use crate::rules::RuleKey;

impl GameState {
    pub fn clear_runtime_inputs(&mut self) {
        self.input_state.clear();
    }

    pub(super) fn controlled_input_entity_ids(&self) -> Vec<EntityId> {
        let mut entity_ids = self
            .entity_manager
            .active_entities()
            .iter()
            .filter_map(|&entity_id| {
                let entity = self.entity_manager.get_entity(entity_id)?;
                if matches!(
                    Self::effective_movement_profile(entity),
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
        self.input_state.held_keys_for_profile(movement_profile)
    }

    pub(super) fn all_held_keys(&self) -> Vec<InputKey> {
        self.input_state.all_held_keys()
    }

    /// Handle key press events
    pub fn handle_key_press(&mut self, key: InputKey) {
        match self.input_state.handle_key_press(key) {
            InputStateEffect::ToggleDebugCollisionRendering => {
                self.debug_collision_rendering = !self.debug_collision_rendering;
                tracing::info!(
                    "Debug collision rendering: {}",
                    self.debug_collision_rendering
                );
            }
            InputStateEffect::None => {}
        }
    }

    /// Handle key release events
    pub fn handle_key_release(&mut self, key: InputKey) {
        self.input_state.handle_key_release(key);
    }

    /// Handle profile-scoped movement key press events.
    pub fn handle_profile_key_press(&mut self, profile: MovementProfile, key: InputKey) {
        if matches!(key, InputKey::DebugToggle) {
            self.handle_key_press(key);
            return;
        }
        self.input_state.handle_profile_key_press(profile, key);
    }

    /// Handle profile-scoped movement key release events.
    pub fn handle_profile_key_release(&mut self, profile: MovementProfile, key: InputKey) {
        self.input_state.handle_profile_key_release(profile, key);
    }

    /// Handle profile-scoped action press events.
    pub fn handle_profile_action_press(&mut self, profile: MovementProfile, action: InputAction) {
        self.input_state
            .handle_profile_action_press(profile, action);
    }

    /// Handle profile-scoped action release events.
    pub fn handle_profile_action_release(&mut self, profile: MovementProfile, action: InputAction) {
        self.input_state
            .handle_profile_action_release(profile, action);
    }

    pub(super) fn take_pending_profile_actions(
        &mut self,
    ) -> std::collections::HashMap<MovementProfile, std::collections::HashSet<InputAction>> {
        self.input_state.take_pending_profile_actions()
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
