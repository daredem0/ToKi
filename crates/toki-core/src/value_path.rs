use crate::entity::{EntityId, EntityManager, HEALTH_STAT_ID};
use crate::flags::GameFlags;
use crate::rules::TriggerContext;
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValuePathRoot {
    Flags,
    Player,
    SelfEntity,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValuePathAccessor {
    Flag(String),
    Health,
    MaxHealth,
    Active,
    Kind,
    Stat(String),
    InventoryCount(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePath {
    root: ValuePathRoot,
    accessor: ValuePathAccessor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedValue {
    Bool(bool),
    Int(i32),
    String(String),
}

impl ResolvedValue {
    pub fn into_flag_value(self) -> crate::FlagValue {
        match self {
            Self::Bool(value) => crate::FlagValue::Bool(value),
            Self::Int(value) => crate::FlagValue::Int(value),
            Self::String(value) => crate::FlagValue::String(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolvedValueRef<'a> {
    Bool(bool),
    Int(i32),
    String(Cow<'a, str>),
}

impl<'a> ResolvedValueRef<'a> {
    pub(crate) fn into_owned(self) -> ResolvedValue {
        match self {
            Self::Bool(value) => ResolvedValue::Bool(value),
            Self::Int(value) => ResolvedValue::Int(value),
            Self::String(value) => ResolvedValue::String(value.into_owned()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ValuePathContext<'a, 'b> {
    pub entity_manager: &'a EntityManager,
    pub game_flags: &'a GameFlags,
    pub player_id: Option<EntityId>,
    pub trigger_context: &'b TriggerContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValuePathError {
    #[error("path must not be empty")]
    Empty,
    #[error("unknown path root '{0}'")]
    UnknownRoot(String),
    #[error("invalid path '{path}': {reason}")]
    InvalidStructure { path: String, reason: String },
    #[error("path '{0}' could not be resolved")]
    Unresolved(String),
}

impl ValuePath {
    pub fn parse(path: &str) -> Result<Self, ValuePathError> {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            return Err(ValuePathError::Empty);
        }
        let segments = trimmed.split('.').collect::<Vec<_>>();
        if segments.is_empty() {
            return Err(ValuePathError::Empty);
        }
        let root = match segments[0] {
            "flags" => ValuePathRoot::Flags,
            "player" => ValuePathRoot::Player,
            "self" => ValuePathRoot::SelfEntity,
            "target" => ValuePathRoot::Target,
            other => return Err(ValuePathError::UnknownRoot(other.to_string())),
        };

        let accessor = match root {
            ValuePathRoot::Flags => {
                if segments.len() != 2 {
                    return Err(ValuePathError::InvalidStructure {
                        path: trimmed.to_string(),
                        reason: "flags paths must be 'flags.<id>'".to_string(),
                    });
                }
                ValuePathAccessor::Flag(segments[1].to_string())
            }
            ValuePathRoot::Player | ValuePathRoot::SelfEntity | ValuePathRoot::Target => {
                parse_entity_accessor(trimmed, &segments)?
            }
        };

        Ok(Self { root, accessor })
    }

    pub fn root(&self) -> &ValuePathRoot {
        &self.root
    }

    pub fn accessor(&self) -> &ValuePathAccessor {
        &self.accessor
    }

    pub fn resolve(
        &self,
        context: ValuePathContext<'_, '_>,
    ) -> Result<ResolvedValue, ValuePathError> {
        self.resolve_borrowed(context).map(ResolvedValueRef::into_owned)
    }

    pub(crate) fn resolve_borrowed<'a>(
        &self,
        context: ValuePathContext<'a, '_>,
    ) -> Result<ResolvedValueRef<'a>, ValuePathError> {
        match (&self.root, &self.accessor) {
            (ValuePathRoot::Flags, ValuePathAccessor::Flag(flag)) => context
                .game_flags
                .get(flag)
                .map(|value| match value {
                    crate::FlagValue::Bool(value) => ResolvedValueRef::Bool(*value),
                    crate::FlagValue::Int(value) => ResolvedValueRef::Int(*value),
                    crate::FlagValue::String(value) => {
                        ResolvedValueRef::String(Cow::Borrowed(value.as_str()))
                    }
                })
                .ok_or_else(|| ValuePathError::Unresolved(format!("flags.{flag}"))),
            (root, accessor) => {
                let entity_id = match root {
                    ValuePathRoot::Player => context.player_id,
                    ValuePathRoot::SelfEntity => context.trigger_context.trigger_self,
                    ValuePathRoot::Target => context.trigger_context.trigger_other,
                    ValuePathRoot::Flags => None,
                }
                .ok_or_else(|| ValuePathError::Unresolved(self.to_string()))?;

                let entity = context
                    .entity_manager
                    .get_entity(entity_id)
                    .ok_or_else(|| ValuePathError::Unresolved(self.to_string()))?;

                match accessor {
                    ValuePathAccessor::Health => context
                        .entity_manager
                        .combat(entity_id)
                        .and_then(|combat| combat.current_stat(HEALTH_STAT_ID))
                        .map(ResolvedValueRef::Int)
                        .ok_or_else(|| ValuePathError::Unresolved(self.to_string())),
                    ValuePathAccessor::MaxHealth => context
                        .entity_manager
                        .combat(entity_id)
                        .and_then(|combat| {
                            combat
                                .base_stat(HEALTH_STAT_ID)
                                .or_else(|| combat.current_stat(HEALTH_STAT_ID))
                        })
                        .map(ResolvedValueRef::Int)
                        .ok_or_else(|| ValuePathError::Unresolved(self.to_string())),
                    ValuePathAccessor::Active => Ok(ResolvedValueRef::Bool(entity.active)),
                    ValuePathAccessor::Kind => Ok(ResolvedValueRef::String(Cow::Owned(format!(
                        "{:?}",
                        entity.entity_kind
                    )))),
                    ValuePathAccessor::Stat(stat) => context
                        .entity_manager
                        .combat(entity_id)
                        .and_then(|combat| combat.current_stat(stat))
                        .map(ResolvedValueRef::Int)
                        .ok_or_else(|| ValuePathError::Unresolved(self.to_string())),
                    ValuePathAccessor::InventoryCount(item_id) => context
                        .entity_manager
                        .storage()
                        .components()
                        .inventory(entity_id)
                        .map(|inventory| {
                            ResolvedValueRef::Int(inventory.item_count(item_id) as i32)
                        })
                        .ok_or_else(|| ValuePathError::Unresolved(self.to_string())),
                    ValuePathAccessor::Flag(_) => {
                        unreachable!("flag access only valid on flags root")
                    }
                }
            }
        }
    }
}

impl std::fmt::Display for ValuePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.root, &self.accessor) {
            (ValuePathRoot::Flags, ValuePathAccessor::Flag(flag)) => write!(f, "flags.{flag}"),
            (root, ValuePathAccessor::Health) => write!(f, "{}.health", entity_root_label(root)),
            (root, ValuePathAccessor::MaxHealth) => {
                write!(f, "{}.max_health", entity_root_label(root))
            }
            (root, ValuePathAccessor::Active) => write!(f, "{}.active", entity_root_label(root)),
            (root, ValuePathAccessor::Kind) => write!(f, "{}.kind", entity_root_label(root)),
            (root, ValuePathAccessor::Stat(stat)) => {
                write!(f, "{}.stats.{stat}", entity_root_label(root))
            }
            (root, ValuePathAccessor::InventoryCount(item_id)) => {
                write!(f, "{}.inventory.{item_id}", entity_root_label(root))
            }
            (_, ValuePathAccessor::Flag(flag)) => {
                write!(f, "{}.flags.{flag}", entity_root_label(&self.root))
            }
        }
    }
}

fn parse_entity_accessor(
    path: &str,
    segments: &[&str],
) -> Result<ValuePathAccessor, ValuePathError> {
    match segments {
        [_, "health"] => Ok(ValuePathAccessor::Health),
        [_, "max_health"] => Ok(ValuePathAccessor::MaxHealth),
        [_, "active"] => Ok(ValuePathAccessor::Active),
        [_, "kind"] => Ok(ValuePathAccessor::Kind),
        [_, "stats", stat] if !stat.trim().is_empty() => {
            Ok(ValuePathAccessor::Stat((*stat).to_string()))
        }
        [_, "inventory", item_id] if !item_id.trim().is_empty() => {
            Ok(ValuePathAccessor::InventoryCount((*item_id).to_string()))
        }
        _ => Err(ValuePathError::InvalidStructure {
            path: path.to_string(),
            reason: "entity paths must be '<root>.health', '<root>.max_health', '<root>.active', '<root>.kind', '<root>.stats.<id>', or '<root>.inventory.<item_id>'".to_string(),
        }),
    }
}

fn entity_root_label(root: &ValuePathRoot) -> &'static str {
    match root {
        ValuePathRoot::Flags => "flags",
        ValuePathRoot::Player => "player",
        ValuePathRoot::SelfEntity => "self",
        ValuePathRoot::Target => "target",
    }
}

#[cfg(test)]
mod tests {
    use super::{ResolvedValue, ValuePath, ValuePathAccessor, ValuePathContext, ValuePathRoot};
    use crate::entity::HEALTH_STAT_ID;
    use crate::flags::{FlagValue, GameFlags};
    use crate::game::SceneSystem;
    use crate::rules::TriggerContext;
    use glam::IVec2;

    #[test]
    fn parse_flag_path() {
        let path = ValuePath::parse("flags.quest_stage").expect("flag path should parse");
        assert_eq!(path.root(), &ValuePathRoot::Flags);
        assert_eq!(
            path.accessor(),
            &ValuePathAccessor::Flag("quest_stage".to_string())
        );
    }

    #[test]
    fn parse_entity_stat_path() {
        let path = ValuePath::parse("self.stats.attack").expect("stat path should parse");
        assert_eq!(path.root(), &ValuePathRoot::SelfEntity);
        assert_eq!(
            path.accessor(),
            &ValuePathAccessor::Stat("attack".to_string())
        );
    }

    #[test]
    fn reject_invalid_flag_shape() {
        let error = ValuePath::parse("flags.a.b").expect_err("invalid path should fail");
        assert!(matches!(
            error,
            super::ValuePathError::InvalidStructure { .. }
        ));
    }

    #[test]
    fn resolve_flag_and_entity_values() {
        let mut flags = GameFlags::default();
        flags.set("coins", FlagValue::Int(7));
        let mut state = crate::GameState::new_empty();
        let entity_id = SceneSystem::spawn_player_at(&mut state, IVec2::new(0, 0));
        state
            .world_mut()
            .entity_manager_mut()
            .storage_mut()
            .components_mut()
            .ensure_inventory(entity_id)
            .add_item("potion", 3);

        let trigger = TriggerContext::with_pair(entity_id, entity_id);
        let context = ValuePathContext {
            entity_manager: state.world().entity_manager(),
            game_flags: &flags,
            player_id: Some(entity_id),
            trigger_context: &trigger,
        };

        assert_eq!(
            ValuePath::parse("flags.coins")
                .expect("flag path")
                .resolve(context)
                .expect("flag should resolve"),
            ResolvedValue::Int(7)
        );
        assert_eq!(
            ValuePath::parse("player.inventory.potion")
                .expect("inventory path")
                .resolve(context)
                .expect("inventory count should resolve"),
            ResolvedValue::Int(3)
        );
    }

    #[test]
    fn unresolved_target_reports_error() {
        let state = crate::GameState::new_empty();
        let flags = GameFlags::default();
        let context = ValuePathContext {
            entity_manager: state.world().entity_manager(),
            game_flags: &flags,
            player_id: None,
            trigger_context: &TriggerContext::empty(),
        };
        let error = ValuePath::parse("target.health")
            .expect("target path should parse")
            .resolve(context)
            .expect_err("missing target should fail");
        assert!(matches!(error, super::ValuePathError::Unresolved(_)));
    }

    #[test]
    fn resolve_health_uses_current_health_stat() {
        let mut state = crate::GameState::new_empty();
        let player_id = SceneSystem::spawn_player_at(&mut state, IVec2::new(0, 0));
        state
            .world_mut()
            .entity_manager_mut()
            .combat_mut(player_id)
            .expect("player should exist")
            .stats
            .current
            .insert(HEALTH_STAT_ID.to_string(), 12);
        let flags = GameFlags::default();
        let trigger = TriggerContext::with_self_only(player_id);
        let context = ValuePathContext {
            entity_manager: state.world().entity_manager(),
            game_flags: &flags,
            player_id: Some(player_id),
            trigger_context: &trigger,
        };
        assert_eq!(
            ValuePath::parse("player.health")
                .expect("health path should parse")
                .resolve(context)
                .expect("health should resolve"),
            ResolvedValue::Int(12)
        );
    }

    #[test]
    fn resolve_max_health_uses_base_health_stat() {
        let mut state = crate::GameState::new_empty();
        let player_id = SceneSystem::spawn_player_at(&mut state, IVec2::new(0, 0));
        let player = state
            .world_mut()
            .entity_manager_mut()
            .combat_mut(player_id)
            .expect("player should exist");
        player.stats.base.insert(HEALTH_STAT_ID.to_string(), 25);
        player.stats.current.insert(HEALTH_STAT_ID.to_string(), 12);

        let flags = GameFlags::default();
        let context = ValuePathContext {
            entity_manager: state.world().entity_manager(),
            game_flags: &flags,
            player_id: Some(player_id),
            trigger_context: &TriggerContext::empty(),
        };

        assert_eq!(
            ValuePath::parse("player.max_health")
                .expect("max health path should parse")
                .resolve(context)
                .expect("max health should resolve"),
            ResolvedValue::Int(25)
        );
    }
}
