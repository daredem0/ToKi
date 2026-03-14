use super::editor_ui::EditorUI;
use super::rule_graph::RuleGraph;
use crate::project::SceneGraphLayout;
use glam::IVec2;
use toki_core::entity::{Entity, EntityId};
use toki_core::rules::RuleSet;

#[derive(Debug, Clone, Default)]
pub struct UndoRedoHistory {
    undo_stack: Vec<EditorCommand>,
    redo_stack: Vec<EditorCommand>,
}

impl UndoRedoHistory {
    pub fn execute(&mut self, command: EditorCommand, ui_state: &mut EditorUI) -> bool {
        if command.apply(ui_state) {
            self.undo_stack.push(command);
            self.redo_stack.clear();
            ui_state.scene_content_changed = true;
            true
        } else {
            false
        }
    }

    pub fn undo(&mut self, ui_state: &mut EditorUI) -> bool {
        let Some(command) = self.undo_stack.pop() else {
            return false;
        };

        if command.undo(ui_state) {
            self.redo_stack.push(command);
            ui_state.scene_content_changed = true;
            true
        } else {
            self.undo_stack.push(command);
            false
        }
    }

    pub fn redo(&mut self, ui_state: &mut EditorUI) -> bool {
        let Some(command) = self.redo_stack.pop() else {
            return false;
        };

        if command.apply(ui_state) {
            self.undo_stack.push(command);
            ui_state.scene_content_changed = true;
            true
        } else {
            self.redo_stack.push(command);
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    #[cfg(test)]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    #[cfg(test)]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }
}

#[derive(Debug, Clone)]
pub enum EditorCommand {
    AddEntity(AddEntityCommand),
    RemoveEntity(RemoveEntityCommand),
    MoveEntities(MoveEntitiesCommand),
    UpdateEntities(UpdateEntitiesCommand),
    UpdateSceneRulesGraph(UpdateSceneRulesGraphCommand),
}

impl EditorCommand {
    pub fn add_entity(scene_name: impl Into<String>, entity: Entity) -> Self {
        Self::AddEntity(AddEntityCommand {
            scene_name: scene_name.into(),
            entity,
        })
    }

    pub fn remove_entities(
        scene_name: impl Into<String>,
        removed_entities: Vec<IndexedEntity>,
    ) -> Self {
        Self::RemoveEntity(RemoveEntityCommand {
            scene_name: scene_name.into(),
            removed_entities,
        })
    }

    pub fn move_entities(
        scene_name: impl Into<String>,
        before_positions: Vec<EntityPosition>,
        after_positions: Vec<EntityPosition>,
    ) -> Self {
        Self::MoveEntities(MoveEntitiesCommand {
            scene_name: scene_name.into(),
            before_positions,
            after_positions,
        })
    }

    pub fn update_entities(
        scene_name: impl Into<String>,
        before_entities: Vec<Entity>,
        after_entities: Vec<Entity>,
    ) -> Self {
        Self::UpdateEntities(UpdateEntitiesCommand {
            scene_name: scene_name.into(),
            before_entities,
            after_entities,
        })
    }

    pub fn update_scene_rules_graph(
        scene_name: impl Into<String>,
        before_rule_set: RuleSet,
        after_rule_set: RuleSet,
        before_graph: Option<RuleGraph>,
        after_graph: Option<RuleGraph>,
        before_layout: Option<SceneGraphLayout>,
        after_layout: Option<SceneGraphLayout>,
    ) -> Self {
        Self::UpdateSceneRulesGraph(UpdateSceneRulesGraphCommand {
            scene_name: scene_name.into(),
            before_rule_set,
            after_rule_set,
            before_graph,
            after_graph,
            before_layout,
            after_layout,
        })
    }

    pub fn apply(&self, ui_state: &mut EditorUI) -> bool {
        match self {
            Self::AddEntity(command) => command.apply(ui_state),
            Self::RemoveEntity(command) => command.apply(ui_state),
            Self::MoveEntities(command) => command.apply(ui_state),
            Self::UpdateEntities(command) => command.apply(ui_state),
            Self::UpdateSceneRulesGraph(command) => command.apply(ui_state),
        }
    }

    pub fn undo(&self, ui_state: &mut EditorUI) -> bool {
        match self {
            Self::AddEntity(command) => command.undo(ui_state),
            Self::RemoveEntity(command) => command.undo(ui_state),
            Self::MoveEntities(command) => command.undo(ui_state),
            Self::UpdateEntities(command) => command.undo(ui_state),
            Self::UpdateSceneRulesGraph(command) => command.undo(ui_state),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexedEntity {
    pub index: usize,
    pub entity: Entity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityPosition {
    pub id: EntityId,
    pub position: IVec2,
}

impl EntityPosition {
    pub fn new(id: EntityId, position: IVec2) -> Self {
        Self { id, position }
    }
}

#[derive(Debug, Clone)]
pub struct AddEntityCommand {
    scene_name: String,
    entity: Entity,
}

impl AddEntityCommand {
    fn apply(&self, ui_state: &mut EditorUI) -> bool {
        let Some(scene) = scene_mut(ui_state, &self.scene_name) else {
            return false;
        };
        if scene
            .entities
            .iter()
            .any(|existing| existing.id == self.entity.id)
        {
            return false;
        }
        scene.entities.push(self.entity.clone());
        true
    }

    fn undo(&self, ui_state: &mut EditorUI) -> bool {
        let Some(scene) = scene_mut(ui_state, &self.scene_name) else {
            return false;
        };
        let Some(index) = scene
            .entities
            .iter()
            .position(|existing| existing.id == self.entity.id)
        else {
            return false;
        };
        scene.entities.remove(index);
        true
    }
}

#[derive(Debug, Clone)]
pub struct RemoveEntityCommand {
    scene_name: String,
    removed_entities: Vec<IndexedEntity>,
}

impl RemoveEntityCommand {
    fn apply(&self, ui_state: &mut EditorUI) -> bool {
        let Some(scene) = scene_mut(ui_state, &self.scene_name) else {
            return false;
        };

        let mut changed = false;
        for removed in &self.removed_entities {
            if let Some(index) = scene
                .entities
                .iter()
                .position(|entity| entity.id == removed.entity.id)
            {
                scene.entities.remove(index);
                changed = true;
            }
        }
        changed
    }

    fn undo(&self, ui_state: &mut EditorUI) -> bool {
        let Some(scene) = scene_mut(ui_state, &self.scene_name) else {
            return false;
        };

        let mut to_restore = self.removed_entities.clone();
        to_restore.sort_by_key(|entry| entry.index);

        let mut changed = false;
        for removed in to_restore {
            if scene
                .entities
                .iter()
                .any(|entity| entity.id == removed.entity.id)
            {
                continue;
            }
            let insert_index = removed.index.min(scene.entities.len());
            scene.entities.insert(insert_index, removed.entity);
            changed = true;
        }
        changed
    }
}

#[derive(Debug, Clone)]
pub struct MoveEntitiesCommand {
    scene_name: String,
    before_positions: Vec<EntityPosition>,
    after_positions: Vec<EntityPosition>,
}

impl MoveEntitiesCommand {
    fn apply(&self, ui_state: &mut EditorUI) -> bool {
        apply_entity_positions(ui_state, &self.scene_name, &self.after_positions)
    }

    fn undo(&self, ui_state: &mut EditorUI) -> bool {
        apply_entity_positions(ui_state, &self.scene_name, &self.before_positions)
    }
}

#[derive(Debug, Clone)]
pub struct UpdateEntitiesCommand {
    scene_name: String,
    before_entities: Vec<Entity>,
    after_entities: Vec<Entity>,
}

impl UpdateEntitiesCommand {
    fn apply(&self, ui_state: &mut EditorUI) -> bool {
        apply_entity_snapshots(ui_state, &self.scene_name, &self.after_entities)
    }

    fn undo(&self, ui_state: &mut EditorUI) -> bool {
        apply_entity_snapshots(ui_state, &self.scene_name, &self.before_entities)
    }
}

#[derive(Debug, Clone)]
pub struct UpdateSceneRulesGraphCommand {
    scene_name: String,
    before_rule_set: RuleSet,
    after_rule_set: RuleSet,
    before_graph: Option<RuleGraph>,
    after_graph: Option<RuleGraph>,
    before_layout: Option<SceneGraphLayout>,
    after_layout: Option<SceneGraphLayout>,
}

impl UpdateSceneRulesGraphCommand {
    fn apply(&self, ui_state: &mut EditorUI) -> bool {
        apply_scene_rules_graph_snapshot(
            ui_state,
            &self.scene_name,
            &self.after_rule_set,
            self.after_graph.clone(),
            self.after_layout.clone(),
        )
    }

    fn undo(&self, ui_state: &mut EditorUI) -> bool {
        apply_scene_rules_graph_snapshot(
            ui_state,
            &self.scene_name,
            &self.before_rule_set,
            self.before_graph.clone(),
            self.before_layout.clone(),
        )
    }
}

fn scene_mut<'a>(ui_state: &'a mut EditorUI, scene_name: &str) -> Option<&'a mut toki_core::Scene> {
    ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == scene_name)
}

fn apply_entity_positions(
    ui_state: &mut EditorUI,
    scene_name: &str,
    positions: &[EntityPosition],
) -> bool {
    let Some(scene) = scene_mut(ui_state, scene_name) else {
        return false;
    };

    let mut changed = false;
    for target in positions {
        if let Some(entity) = scene
            .entities
            .iter_mut()
            .find(|entity| entity.id == target.id)
        {
            if entity.position != target.position {
                entity.position = target.position;
                changed = true;
            }
        }
    }
    changed
}

fn apply_entity_snapshots(ui_state: &mut EditorUI, scene_name: &str, snapshots: &[Entity]) -> bool {
    let Some(scene) = scene_mut(ui_state, scene_name) else {
        return false;
    };

    let mut changed = false;
    for snapshot in snapshots {
        if let Some(entity) = scene
            .entities
            .iter_mut()
            .find(|entity| entity.id == snapshot.id)
        {
            *entity = snapshot.clone();
            changed = true;
        }
    }
    changed
}

fn apply_scene_rules_graph_snapshot(
    ui_state: &mut EditorUI,
    scene_name: &str,
    rule_set: &RuleSet,
    graph: Option<RuleGraph>,
    layout: Option<SceneGraphLayout>,
) -> bool {
    let Some(scene_index) = ui_state
        .scenes
        .iter()
        .position(|scene| scene.name == scene_name)
    else {
        return false;
    };
    ui_state.scenes[scene_index].rules = rule_set.clone();

    match graph {
        Some(graph) => {
            ui_state
                .rule_graphs_by_scene
                .insert(scene_name.to_string(), graph);
        }
        None => {
            ui_state.rule_graphs_by_scene.remove(scene_name);
        }
    }

    match layout {
        Some(layout) => {
            ui_state
                .graph_layouts_by_scene
                .insert(scene_name.to_string(), layout);
        }
        None => {
            ui_state.graph_layouts_by_scene.remove(scene_name);
        }
    }
    ui_state.graph_layout_dirty = true;
    true
}

#[cfg(test)]
mod tests {
    use super::{EditorCommand, EntityPosition, IndexedEntity, UndoRedoHistory};
    use crate::project::SceneGraphLayout;
    use crate::ui::rule_graph::RuleGraph;
    use crate::ui::EditorUI;
    use glam::{IVec2, UVec2};
    use toki_core::entity::{Entity, EntityAttributes, EntityType};
    use toki_core::rules::{
        Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger,
    };

    fn sample_entity(id: u32, position: IVec2) -> Entity {
        Entity {
            id,
            position,
            size: UVec2::new(16, 16),
            entity_type: EntityType::Npc,
            definition_name: Some("npc".to_string()),
            attributes: EntityAttributes::default(),
            collision_box: None,
        }
    }

    fn main_scene_entities(ui_state: &EditorUI) -> Vec<Entity> {
        ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist")
            .entities
            .clone()
    }

    #[test]
    fn execute_clears_redo_stack_when_new_command_is_applied() {
        let mut ui_state = EditorUI::new();
        let mut history = UndoRedoHistory::default();

        assert!(history.execute(
            EditorCommand::add_entity("Main Scene", sample_entity(1, IVec2::new(1, 1))),
            &mut ui_state
        ));
        assert!(history.can_undo());
        assert!(!history.can_redo());

        assert!(history.undo(&mut ui_state));
        assert!(history.can_redo());

        assert!(history.execute(
            EditorCommand::add_entity("Main Scene", sample_entity(2, IVec2::new(2, 2))),
            &mut ui_state
        ));
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.undo_depth(), 1);
        assert_eq!(history.redo_depth(), 0);
    }

    #[test]
    fn add_entity_command_supports_undo_and_redo() {
        let mut ui_state = EditorUI::new();
        let mut history = UndoRedoHistory::default();

        let command = EditorCommand::add_entity("Main Scene", sample_entity(7, IVec2::new(4, 8)));
        assert!(history.execute(command, &mut ui_state));
        assert_eq!(main_scene_entities(&ui_state).len(), 1);

        assert!(history.undo(&mut ui_state));
        assert!(main_scene_entities(&ui_state).is_empty());

        assert!(history.redo(&mut ui_state));
        assert_eq!(main_scene_entities(&ui_state).len(), 1);
    }

    #[test]
    fn move_entities_command_round_trips_positions() {
        let mut ui_state = EditorUI::new();
        let mut history = UndoRedoHistory::default();

        let first = sample_entity(1, IVec2::new(10, 20));
        let second = sample_entity(2, IVec2::new(30, 40));
        ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist")
            .entities
            .extend([first, second]);

        let command = EditorCommand::move_entities(
            "Main Scene",
            vec![
                EntityPosition::new(1, IVec2::new(10, 20)),
                EntityPosition::new(2, IVec2::new(30, 40)),
            ],
            vec![
                EntityPosition::new(1, IVec2::new(15, 17)),
                EntityPosition::new(2, IVec2::new(35, 37)),
            ],
        );
        assert!(history.execute(command, &mut ui_state));

        let entities = main_scene_entities(&ui_state);
        assert_eq!(
            entities
                .iter()
                .find(|entity| entity.id == 1)
                .expect("first entity should exist")
                .position,
            IVec2::new(15, 17)
        );
        assert_eq!(
            entities
                .iter()
                .find(|entity| entity.id == 2)
                .expect("second entity should exist")
                .position,
            IVec2::new(35, 37)
        );

        assert!(history.undo(&mut ui_state));
        let entities = main_scene_entities(&ui_state);
        assert_eq!(
            entities
                .iter()
                .find(|entity| entity.id == 1)
                .expect("first entity should exist")
                .position,
            IVec2::new(10, 20)
        );
        assert_eq!(
            entities
                .iter()
                .find(|entity| entity.id == 2)
                .expect("second entity should exist")
                .position,
            IVec2::new(30, 40)
        );
    }

    #[test]
    fn update_entities_command_restores_previous_state_on_undo() {
        let mut ui_state = EditorUI::new();
        let mut history = UndoRedoHistory::default();

        let before = sample_entity(42, IVec2::new(5, 5));
        ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist")
            .entities
            .push(before.clone());

        let mut after = before.clone();
        after.attributes.visible = false;
        after.attributes.render_layer = 9;

        let command =
            EditorCommand::update_entities("Main Scene", vec![before.clone()], vec![after.clone()]);
        assert!(history.execute(command, &mut ui_state));

        let entity = main_scene_entities(&ui_state)
            .into_iter()
            .find(|entity| entity.id == 42)
            .expect("entity should exist");
        assert!(!entity.attributes.visible);
        assert_eq!(entity.attributes.render_layer, 9);

        assert!(history.undo(&mut ui_state));
        let entity = main_scene_entities(&ui_state)
            .into_iter()
            .find(|entity| entity.id == 42)
            .expect("entity should exist");
        assert!(entity.attributes.visible);
        assert_eq!(entity.attributes.render_layer, 0);
    }

    #[test]
    fn remove_entities_command_restores_original_order_on_undo() {
        let mut ui_state = EditorUI::new();
        let mut history = UndoRedoHistory::default();

        let entities = vec![
            sample_entity(1, IVec2::new(0, 0)),
            sample_entity(2, IVec2::new(16, 0)),
            sample_entity(3, IVec2::new(32, 0)),
        ];
        ui_state
            .scenes
            .iter_mut()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist")
            .entities
            .extend(entities.clone());

        let command = EditorCommand::remove_entities(
            "Main Scene",
            vec![
                IndexedEntity {
                    index: 0,
                    entity: entities[0].clone(),
                },
                IndexedEntity {
                    index: 2,
                    entity: entities[2].clone(),
                },
            ],
        );
        assert!(history.execute(command, &mut ui_state));
        let ids = main_scene_entities(&ui_state)
            .into_iter()
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![2]);

        assert!(history.undo(&mut ui_state));
        let ids = main_scene_entities(&ui_state)
            .into_iter()
            .map(|entity| entity.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn execute_noop_command_does_not_affect_history() {
        let mut ui_state = EditorUI::new();
        let mut history = UndoRedoHistory::default();

        let command =
            EditorCommand::add_entity("Missing Scene", sample_entity(1, IVec2::new(0, 0)));
        assert!(!history.execute(command, &mut ui_state));
        assert_eq!(history.undo_depth(), 0);
        assert_eq!(history.redo_depth(), 0);
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn update_scene_rules_graph_command_round_trips_rules_graph_and_layout() {
        let mut ui_state = EditorUI::new();
        let mut history = UndoRedoHistory::default();

        let before_rule_set = RuleSet::default();
        let after_rule_set = RuleSet {
            rules: vec![Rule {
                id: "rule_1".to_string(),
                enabled: true,
                priority: 0,
                once: false,
                trigger: RuleTrigger::OnStart,
                conditions: vec![RuleCondition::Always],
                actions: vec![RuleAction::PlaySound {
                    channel: RuleSoundChannel::Movement,
                    sound_id: "sfx_start".to_string(),
                }],
            }],
        };

        let before_graph = None;
        let after_graph = Some(RuleGraph::from_rule_set(&after_rule_set));
        let before_layout = None;
        let mut after_layout = SceneGraphLayout::default();
        after_layout
            .node_positions
            .insert("rule_1::trigger".to_string(), [12.0, 34.0]);
        after_layout.zoom = 1.25;
        after_layout.pan = [32.0, 48.0];

        let command = EditorCommand::update_scene_rules_graph(
            "Main Scene",
            before_rule_set.clone(),
            after_rule_set.clone(),
            before_graph,
            after_graph.clone(),
            before_layout,
            Some(after_layout.clone()),
        );
        assert!(history.execute(command, &mut ui_state));

        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist");
        assert_eq!(scene.rules, after_rule_set);
        assert_eq!(
            ui_state.rule_graph_for_scene("Main Scene"),
            after_graph.as_ref()
        );
        let layout = ui_state
            .graph_layouts_by_scene
            .get("Main Scene")
            .expect("graph layout should exist");
        assert_eq!(layout.node_positions, after_layout.node_positions);
        assert_eq!(layout.zoom, after_layout.zoom);
        assert_eq!(layout.pan, after_layout.pan);

        assert!(history.undo(&mut ui_state));
        let scene = ui_state
            .scenes
            .iter()
            .find(|scene| scene.name == "Main Scene")
            .expect("main scene should exist");
        assert_eq!(scene.rules, before_rule_set);
        assert!(ui_state.rule_graph_for_scene("Main Scene").is_none());
        assert!(!ui_state.graph_layouts_by_scene.contains_key("Main Scene"));
    }
}
