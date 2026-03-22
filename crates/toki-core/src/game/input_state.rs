use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::entity::MovementProfile;

use super::{InputAction, InputKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputStateEffect {
    None,
    ToggleDebugCollisionRendering,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct InputRuntimeState {
    #[serde(default)]
    keys_held: HashSet<InputKey>,
    #[serde(skip, default)]
    profile_keys_held: HashMap<MovementProfile, HashSet<InputKey>>,
    #[serde(skip, default)]
    profile_actions_held: HashMap<MovementProfile, HashSet<InputAction>>,
    #[serde(skip, default)]
    pending_profile_actions: HashMap<MovementProfile, HashSet<InputAction>>,
}

impl InputRuntimeState {
    pub(super) fn clear(&mut self) {
        self.keys_held.clear();
        self.profile_keys_held.clear();
        self.profile_actions_held.clear();
        self.pending_profile_actions.clear();
    }

    pub(super) fn held_keys_for_profile(&self, movement_profile: MovementProfile) -> Vec<InputKey> {
        let mut held_keys = self.keys_held.iter().copied().collect::<HashSet<_>>();
        if let Some(profile_keys) = self.profile_keys_held.get(&movement_profile) {
            held_keys.extend(profile_keys.iter().copied());
        }
        let mut held_keys = held_keys.into_iter().collect::<Vec<_>>();
        held_keys.sort_by_key(|key| input_key_order(*key));
        held_keys
    }

    pub(super) fn all_held_keys(&self) -> Vec<InputKey> {
        let mut held_keys = self.keys_held.clone();
        for profile_keys in self.profile_keys_held.values() {
            held_keys.extend(profile_keys.iter().copied());
        }
        let mut held_keys = held_keys.into_iter().collect::<Vec<_>>();
        held_keys.sort_by_key(|key| input_key_order(*key));
        held_keys
    }

    pub(super) fn handle_key_press(&mut self, key: InputKey) -> InputStateEffect {
        match key {
            InputKey::DebugToggle => InputStateEffect::ToggleDebugCollisionRendering,
            _ => {
                self.keys_held.insert(key);
                InputStateEffect::None
            }
        }
    }

    pub(super) fn handle_key_release(&mut self, key: InputKey) {
        self.keys_held.remove(&key);
    }

    pub(super) fn handle_profile_key_press(&mut self, profile: MovementProfile, key: InputKey) {
        self.profile_keys_held
            .entry(profile)
            .or_default()
            .insert(key);
    }

    pub(super) fn handle_profile_key_release(&mut self, profile: MovementProfile, key: InputKey) {
        if let Some(keys) = self.profile_keys_held.get_mut(&profile) {
            keys.remove(&key);
            if keys.is_empty() {
                self.profile_keys_held.remove(&profile);
            }
        }
    }

    pub(super) fn handle_profile_action_press(
        &mut self,
        profile: MovementProfile,
        action: InputAction,
    ) {
        let held_actions = self.profile_actions_held.entry(profile).or_default();
        if held_actions.insert(action) {
            self.pending_profile_actions
                .entry(profile)
                .or_default()
                .insert(action);
        }
    }

    pub(super) fn handle_profile_action_release(
        &mut self,
        profile: MovementProfile,
        action: InputAction,
    ) {
        if let Some(actions) = self.profile_actions_held.get_mut(&profile) {
            actions.remove(&action);
            if actions.is_empty() {
                self.profile_actions_held.remove(&profile);
            }
        }
    }

    pub(super) fn take_pending_profile_actions(
        &mut self,
    ) -> HashMap<MovementProfile, HashSet<InputAction>> {
        std::mem::take(&mut self.pending_profile_actions)
    }
}

pub(super) fn input_key_order(key: InputKey) -> u8 {
    match key {
        InputKey::Up => 0,
        InputKey::Down => 1,
        InputKey::Left => 2,
        InputKey::Right => 3,
        InputKey::DebugToggle => 4,
        InputKey::Interact => 5,
        InputKey::AttackPrimary => 6,
        InputKey::AttackSecondary => 7,
        InputKey::Inventory => 8,
        InputKey::Pause => 9,
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::MovementProfile;

    use super::{InputRuntimeState, InputStateEffect};
    use crate::game::{InputAction, InputKey};

    #[test]
    fn input_runtime_state_merges_global_and_profile_keys_in_stable_order() {
        let mut state = InputRuntimeState::default();
        state.handle_key_press(InputKey::Right);
        state.handle_profile_key_press(MovementProfile::PlayerWasd, InputKey::Up);

        assert_eq!(
            state.held_keys_for_profile(MovementProfile::PlayerWasd),
            vec![InputKey::Up, InputKey::Right]
        );
    }

    #[test]
    fn profile_action_press_is_edge_triggered_until_release() {
        let mut state = InputRuntimeState::default();
        state.handle_profile_action_press(MovementProfile::PlayerWasd, InputAction::Primary);
        state.handle_profile_action_press(MovementProfile::PlayerWasd, InputAction::Primary);

        let pending = state.take_pending_profile_actions();
        assert_eq!(pending.len(), 1);
        assert!(pending[&MovementProfile::PlayerWasd].contains(&InputAction::Primary));
        assert!(state.take_pending_profile_actions().is_empty());

        state.handle_profile_action_release(MovementProfile::PlayerWasd, InputAction::Primary);
        state.handle_profile_action_press(MovementProfile::PlayerWasd, InputAction::Primary);
        assert_eq!(state.take_pending_profile_actions().len(), 1);
    }

    #[test]
    fn debug_toggle_returns_effect_instead_of_mutating_runtime_state_directly() {
        let mut state = InputRuntimeState::default();
        let effect = state.handle_key_press(InputKey::DebugToggle);

        assert_eq!(effect, InputStateEffect::ToggleDebugCollisionRendering);
        assert!(state.all_held_keys().is_empty());
    }
}
