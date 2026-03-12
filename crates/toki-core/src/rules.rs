use serde::{Deserialize, Serialize};

use crate::animation::AnimationState;
use crate::entity::EntityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleTrigger {
    OnStart,
    OnUpdate,
    OnKey { key: RuleKey },
    OnCollision,
    OnTrigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleKey {
    Up,
    Down,
    Left,
    Right,
    DebugToggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleCondition {
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSoundChannel {
    Movement,
    Collision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleTarget {
    Player,
    Entity(EntityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSpawnEntityType {
    PlayerLikeNpc,
    Npc,
    Item,
    Decoration,
    Trigger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleAction {
    PlaySound {
        channel: RuleSoundChannel,
        sound_id: String,
    },
    PlayMusic {
        track_id: String,
    },
    PlayAnimation {
        target: RuleTarget,
        state: AnimationState,
    },
    SetVelocity {
        target: RuleTarget,
        velocity: [i32; 2],
    },
    Spawn {
        entity_type: RuleSpawnEntityType,
        position: [i32; 2],
    },
    DestroySelf {
        target: RuleTarget,
    },
    /// Runtime placeholder until scene-switch plumbing is integrated end-to-end.
    SwitchScene {
        scene_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub once: bool,
    pub trigger: RuleTrigger,
    #[serde(default)]
    pub conditions: Vec<RuleCondition>,
    #[serde(default)]
    pub actions: Vec<RuleAction>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSet {
    #[serde(default)]
    pub rules: Vec<Rule>,
}

fn default_true() -> bool {
    true
}
