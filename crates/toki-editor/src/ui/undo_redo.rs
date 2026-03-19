use super::editor_ui::EditorUI;
use super::rule_graph::RuleGraph;
use crate::project::Project;
use crate::project::SceneGraphLayout;
use glam::IVec2;
use toki_core::entity::{Entity, EntityId};
use toki_core::menu::MenuSettings;
use toki_core::rules::RuleSet;
use toki_core::Scene;

#[derive(Debug, Clone, Default)]
pub struct UndoRedoHistory {
    undo_stack: Vec<EditorCommand>,
    redo_stack: Vec<EditorCommand>,
}

impl UndoRedoHistory {
    pub fn execute(
        &mut self,
        command: EditorCommand,
        ui_state: &mut EditorUI,
        project: Option<&mut Project>,
    ) -> bool {
        if command.apply(ui_state, project) {
            self.undo_stack.push(command);
            self.redo_stack.clear();
            self.undo_stack
                .last()
                .expect("command just pushed")
                .mark_post_apply(ui_state);
            true
        } else {
            false
        }
    }

    pub fn undo(&mut self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        let Some(command) = self.undo_stack.pop() else {
            return false;
        };

        if command.undo(ui_state, project) {
            self.redo_stack.push(command);
            self.redo_stack
                .last()
                .expect("command just pushed")
                .mark_post_apply(ui_state);
            true
        } else {
            self.undo_stack.push(command);
            false
        }
    }

    pub fn redo(&mut self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        let Some(command) = self.redo_stack.pop() else {
            return false;
        };

        if command.apply(ui_state, project) {
            self.undo_stack.push(command);
            self.undo_stack
                .last()
                .expect("command just pushed")
                .mark_post_apply(ui_state);
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
}

#[derive(Debug, Clone)]
pub enum EditorCommand {
    AddEntity(Box<AddEntityCommand>),
    RemoveEntity(Box<RemoveEntityCommand>),
    MoveEntities(Box<MoveEntitiesCommand>),
    UpdateEntities(Box<UpdateEntitiesCommand>),
    UpdateScene(Box<UpdateSceneCommand>),
    UpdateSceneRulesGraph(Box<UpdateSceneRulesGraphCommand>),
    UpdateMenuSettings(Box<UpdateMenuSettingsCommand>),
}

impl EditorCommand {
    pub fn add_entity(scene_name: impl Into<String>, entity: Entity) -> Self {
        Self::AddEntity(Box::new(AddEntityCommand {
            scene_name: scene_name.into(),
            entity,
        }))
    }

    pub fn remove_entities(
        scene_name: impl Into<String>,
        removed_entities: Vec<IndexedEntity>,
    ) -> Self {
        Self::RemoveEntity(Box::new(RemoveEntityCommand {
            scene_name: scene_name.into(),
            removed_entities,
        }))
    }

    pub fn move_entities(
        scene_name: impl Into<String>,
        before_positions: Vec<EntityPosition>,
        after_positions: Vec<EntityPosition>,
    ) -> Self {
        Self::MoveEntities(Box::new(MoveEntitiesCommand {
            scene_name: scene_name.into(),
            before_positions,
            after_positions,
        }))
    }

    pub fn update_entities(
        scene_name: impl Into<String>,
        before_entities: Vec<Entity>,
        after_entities: Vec<Entity>,
    ) -> Self {
        Self::UpdateEntities(Box::new(UpdateEntitiesCommand {
            scene_name: scene_name.into(),
            before_entities,
            after_entities,
        }))
    }

    pub fn update_scene(scene_name: impl Into<String>, before: Scene, after: Scene) -> Self {
        Self::UpdateScene(Box::new(UpdateSceneCommand {
            scene_name: scene_name.into(),
            before,
            after,
        }))
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
        Self::UpdateSceneRulesGraph(Box::new(UpdateSceneRulesGraphCommand {
            scene_name: scene_name.into(),
            before_rule_set,
            after_rule_set,
            before_graph,
            after_graph,
            before_layout,
            after_layout,
        }))
    }

    pub fn update_menu_settings(before: MenuSettings, after: MenuSettings) -> Self {
        Self::UpdateMenuSettings(Box::new(UpdateMenuSettingsCommand { before, after }))
    }

    pub fn apply(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        match self {
            Self::AddEntity(command) => command.apply(ui_state),
            Self::RemoveEntity(command) => command.apply(ui_state),
            Self::MoveEntities(command) => command.apply(ui_state),
            Self::UpdateEntities(command) => command.apply(ui_state),
            Self::UpdateScene(command) => command.apply(ui_state),
            Self::UpdateSceneRulesGraph(command) => command.apply(ui_state),
            Self::UpdateMenuSettings(command) => command.apply(project),
        }
    }

    pub fn undo(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        match self {
            Self::AddEntity(command) => command.undo(ui_state),
            Self::RemoveEntity(command) => command.undo(ui_state),
            Self::MoveEntities(command) => command.undo(ui_state),
            Self::UpdateEntities(command) => command.undo(ui_state),
            Self::UpdateScene(command) => command.undo(ui_state),
            Self::UpdateSceneRulesGraph(command) => command.undo(ui_state),
            Self::UpdateMenuSettings(command) => command.undo(project),
        }
    }

    fn mark_post_apply(&self, ui_state: &mut EditorUI) {
        if matches!(
            self,
            Self::AddEntity(_)
                | Self::RemoveEntity(_)
                | Self::MoveEntities(_)
                | Self::UpdateEntities(_)
                | Self::UpdateScene(_)
                | Self::UpdateSceneRulesGraph(_)
        ) {
            ui_state.scene_content_changed = true;
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

#[derive(Debug, Clone)]
pub struct UpdateMenuSettingsCommand {
    before: MenuSettings,
    after: MenuSettings,
}

impl UpdateMenuSettingsCommand {
    fn apply(&self, project: Option<&mut Project>) -> bool {
        let Some(project) = project else {
            return false;
        };
        project.metadata.runtime.menu = self.after.clone();
        mark_project_dirty(project);
        true
    }

    fn undo(&self, project: Option<&mut Project>) -> bool {
        let Some(project) = project else {
            return false;
        };
        project.metadata.runtime.menu = self.before.clone();
        mark_project_dirty(project);
        true
    }
}

fn mark_project_dirty(project: &mut Project) {
    project.metadata.project.modified = chrono::Utc::now();
    project.is_dirty = true;
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
pub struct UpdateSceneCommand {
    scene_name: String,
    before: Scene,
    after: Scene,
}

impl UpdateSceneCommand {
    fn apply(&self, ui_state: &mut EditorUI) -> bool {
        apply_scene_snapshot(ui_state, &self.scene_name, &self.after)
    }

    fn undo(&self, ui_state: &mut EditorUI) -> bool {
        apply_scene_snapshot(ui_state, &self.scene_name, &self.before)
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
                .graph
                .rule_graphs_by_scene
                .insert(scene_name.to_string(), graph);
        }
        None => {
            ui_state.graph.rule_graphs_by_scene.remove(scene_name);
        }
    }

    match layout {
        Some(layout) => {
            ui_state
                .graph
                .layouts_by_scene
                .insert(scene_name.to_string(), layout);
        }
        None => {
            ui_state.graph.layouts_by_scene.remove(scene_name);
        }
    }
    ui_state.graph.layout_dirty = true;
    true
}

fn apply_scene_snapshot(ui_state: &mut EditorUI, scene_name: &str, snapshot: &Scene) -> bool {
    let Some(scene) = scene_mut(ui_state, scene_name) else {
        return false;
    };
    *scene = snapshot.clone();
    true
}

#[cfg(test)]
#[path = "undo_redo_tests.rs"]
mod tests;
