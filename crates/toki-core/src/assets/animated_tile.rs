use crate::animation::LoopMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TileLoopMode {
    #[default]
    Loop,
    PingPong,
}

impl From<TileLoopMode> for LoopMode {
    fn from(mode: TileLoopMode) -> Self {
        match mode {
            TileLoopMode::Loop => LoopMode::Loop,
            TileLoopMode::PingPong => LoopMode::PingPong,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimatedTileDef {
    pub frames: Vec<String>,
    pub frame_duration_ms: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_durations_ms: Option<Vec<f32>>,
    #[serde(default)]
    pub loop_mode: TileLoopMode,
}

impl AnimatedTileDef {
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn frame_duration_at(&self, index: usize) -> f32 {
        self.frame_durations_ms
            .as_ref()
            .and_then(|durations| durations.get(index).copied())
            .unwrap_or(self.frame_duration_ms)
    }
}
