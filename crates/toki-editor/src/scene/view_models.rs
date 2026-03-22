#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneSummaryView {
    pub map_count: usize,
    pub entity_count: usize,
    pub anchor_count: usize,
    pub rule_count: usize,
}

impl SceneSummaryView {
    pub fn from_scene(scene: &toki_core::Scene) -> Self {
        Self {
            map_count: scene.maps.len(),
            entity_count: scene.entities.len(),
            anchor_count: scene.anchors.len(),
            rule_count: scene.rules.rules.len(),
        }
    }
}
