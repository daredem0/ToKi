//! Rule target resolution.
//!
//! Contains logic for resolving rule targets to entity IDs.

use crate::entity::EntityId;
use crate::rules::{RuleTarget, TriggerContext};

use super::GameState;

impl GameState {
    /// Resolves a rule target to an entity ID using the current trigger context.
    ///
    /// # Architecture Note (for Phase 1.5B+ implementers)
    ///
    /// This method handles all `RuleTarget` variants:
    /// - `Player`: Returns the player entity ID
    /// - `Entity(id)`: Returns the specified entity ID directly
    /// - `TriggerSelf`: Returns `context.trigger_self` (the primary subject)
    /// - `TriggerOther`: Returns `context.trigger_other` (the secondary entity)
    /// - `RuleOwner`: Currently returns `None` for scene-owned rules.
    ///   When entity-owned rules are added, this should return the owning entity.
    ///
    /// Returns `None` if the target cannot be resolved (e.g., no player, context missing).
    pub(super) fn resolve_rule_target(
        &self,
        target: RuleTarget,
        context: &TriggerContext,
    ) -> Option<EntityId> {
        match target {
            RuleTarget::Player => self.player_id,
            RuleTarget::Entity(entity_id) => Some(entity_id),
            RuleTarget::TriggerSelf => context.trigger_self,
            RuleTarget::TriggerOther => context.trigger_other,
            // RuleOwner is only valid for entity-owned rules.
            // Currently all rules are scene-owned, so this returns None.
            // When entity-owned rules are added, pass the owner ID through context.
            RuleTarget::RuleOwner => None,
        }
    }
}
