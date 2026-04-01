use super::model::{AiConfig, ControlRole, EntityStats, MovementProfile, HEALTH_STAT_ID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct MovementComponent {
    pub speed: f32,
    #[serde(default)]
    pub movement_profile: MovementProfile,
    pub can_move: bool,
}

impl Default for MovementComponent {
    fn default() -> Self {
        Self {
            speed: 2.0,
            movement_profile: MovementProfile::default(),
            can_move: true,
        }
    }
}

impl MovementComponent {
    pub fn resolved_profile(&self, control_role: ControlRole) -> MovementProfile {
        self.movement_profile
            .resolved_for_control_role(control_role.resolved())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AiComponent {
    #[serde(default)]
    pub ai_config: AiConfig,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionComponent {
    #[serde(default)]
    pub interaction_reach: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CombatComponent {
    pub health: Option<u32>,
    #[serde(default, skip_serializing_if = "EntityStats::is_empty")]
    pub stats: EntityStats,
}

impl CombatComponent {
    pub fn ensure_health_stat(&mut self) {
        if let Some(health) = self.health {
            self.stats.ensure_stat(HEALTH_STAT_ID, health as i32);
        }
    }

    pub fn current_stat(&self, stat_id: &str) -> Option<i32> {
        self.stats.current(stat_id)
    }

    pub fn base_stat(&self, stat_id: &str) -> Option<i32> {
        self.stats.base(stat_id)
    }

    pub fn apply_stat_delta(&mut self, stat_id: &str, delta: i32) -> Option<i32> {
        let new_value = self.stats.apply_delta(stat_id, delta)?;
        if stat_id == HEALTH_STAT_ID {
            self.health = u32::try_from(new_value).ok();
        }
        Some(new_value)
    }
}
