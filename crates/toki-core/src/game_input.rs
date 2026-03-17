use super::{GameState, InputAction, InputKey};
use crate::entity::{EntityId, MovementProfile};
use crate::rules::RuleKey;
use std::collections::HashSet;

impl GameState {
    pub fn clear_runtime_inputs(&mut self) {
        self.keys_held.clear();
        self.profile_keys_held.clear();
        self.profile_actions_held.clear();
        self.pending_profile_actions.clear();
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
        let mut held_keys = self.keys_held.iter().copied().collect::<HashSet<_>>();
        if let Some(profile_keys) = self.profile_keys_held.get(&movement_profile) {
            held_keys.extend(profile_keys.iter().copied());
        }
        let mut held_keys = held_keys.into_iter().collect::<Vec<_>>();
        held_keys.sort_by_key(|key| Self::input_key_order(*key));
        held_keys
    }

    pub(super) fn all_held_keys(&self) -> Vec<InputKey> {
        let mut held_keys = self.keys_held.clone();
        for profile_keys in self.profile_keys_held.values() {
            held_keys.extend(profile_keys.iter().copied());
        }
        let mut held_keys = held_keys.into_iter().collect::<Vec<_>>();
        held_keys.sort_by_key(|key| Self::input_key_order(*key));
        held_keys
    }

    /// Handle key press events
    pub fn handle_key_press(&mut self, key: InputKey) {
        match key {
            InputKey::DebugToggle => {
                self.debug_collision_rendering = !self.debug_collision_rendering;
                tracing::info!(
                    "Debug collision rendering: {}",
                    self.debug_collision_rendering
                );
            }
            _ => {
                self.keys_held.insert(key);
            }
        }
    }

    /// Handle key release events
    pub fn handle_key_release(&mut self, key: InputKey) {
        self.keys_held.remove(&key);
    }

    /// Handle profile-scoped movement key press events.
    pub fn handle_profile_key_press(&mut self, profile: MovementProfile, key: InputKey) {
        if matches!(key, InputKey::DebugToggle) {
            self.handle_key_press(key);
            return;
        }
        self.profile_keys_held
            .entry(profile)
            .or_default()
            .insert(key);
    }

    /// Handle profile-scoped movement key release events.
    pub fn handle_profile_key_release(&mut self, profile: MovementProfile, key: InputKey) {
        if let Some(keys) = self.profile_keys_held.get_mut(&profile) {
            keys.remove(&key);
            if keys.is_empty() {
                self.profile_keys_held.remove(&profile);
            }
        }
    }

    /// Handle profile-scoped action press events.
    pub fn handle_profile_action_press(&mut self, profile: MovementProfile, action: InputAction) {
        let held_actions = self.profile_actions_held.entry(profile).or_default();
        if held_actions.insert(action) {
            self.pending_profile_actions
                .entry(profile)
                .or_default()
                .insert(action);
        }
    }

    /// Handle profile-scoped action release events.
    pub fn handle_profile_action_release(&mut self, profile: MovementProfile, action: InputAction) {
        if let Some(actions) = self.profile_actions_held.get_mut(&profile) {
            actions.remove(&action);
            if actions.is_empty() {
                self.profile_actions_held.remove(&profile);
            }
        }
    }

    pub(super) fn input_key_order(key: InputKey) -> u8 {
        match key {
            InputKey::Up => 0,
            InputKey::Down => 1,
            InputKey::Left => 2,
            InputKey::Right => 3,
            InputKey::DebugToggle => 4,
        }
    }

    pub(super) fn to_rule_key(key: InputKey) -> RuleKey {
        match key {
            InputKey::Up => RuleKey::Up,
            InputKey::Down => RuleKey::Down,
            InputKey::Left => RuleKey::Left,
            InputKey::Right => RuleKey::Right,
            InputKey::DebugToggle => RuleKey::DebugToggle,
        }
    }

    pub(super) fn to_input_key(key: RuleKey) -> InputKey {
        match key {
            RuleKey::Up => InputKey::Up,
            RuleKey::Down => InputKey::Down,
            RuleKey::Left => InputKey::Left,
            RuleKey::Right => InputKey::Right,
            RuleKey::DebugToggle => InputKey::DebugToggle,
        }
    }
}
