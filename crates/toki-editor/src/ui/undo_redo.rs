use super::editor_ui::{EditorUI, Selection};
use super::rule_graph::RuleGraph;
use crate::project::{Project, ProjectMetadata};
use crate::project::SceneGraphLayout;
use glam::IVec2;
use toki_core::entity::{Entity, EntityId};
use toki_core::menu::MenuSettings;
use toki_core::rules::RuleSet;
use toki_template_lowering::{
    apply_project_file_changes, revert_project_file_changes, ProjectFileChange,
};

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
    UpdateSceneRulesGraph(Box<UpdateSceneRulesGraphCommand>),
    UpdateMenuSettings(Box<UpdateMenuSettingsCommand>),
    ApplyProjectFileChanges(Box<ApplyProjectFileChangesCommand>),
    DeleteScene(Box<DeleteSceneCommand>),
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

    pub fn apply_project_file_changes(
        description: impl Into<String>,
        changes: Vec<ProjectFileChange>,
        selection_before: Option<Selection>,
        selection_after: Option<Selection>,
        project_metadata_before: Option<ProjectMetadata>,
        project_metadata_after: Option<ProjectMetadata>,
    ) -> Self {
        Self::ApplyProjectFileChanges(Box::new(ApplyProjectFileChangesCommand {
            description: description.into(),
            changes,
            selection_before,
            selection_after,
            project_metadata_before,
            project_metadata_after,
        }))
    }

    pub fn delete_scene(data: DeleteSceneCommandData) -> Self {
        Self::DeleteScene(Box::new(DeleteSceneCommand {
            removed_scene: data.removed_scene,
            active_scene_before: data.active_scene_before,
            active_scene_after: data.active_scene_after,
            selection_before: data.selection_before,
            selection_after: data.selection_after,
            changes: data.changes,
            project_metadata_before: data.project_metadata_before,
            project_metadata_after: data.project_metadata_after,
        }))
    }

    pub fn apply(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        match self {
            Self::AddEntity(command) => command.apply(ui_state),
            Self::RemoveEntity(command) => command.apply(ui_state),
            Self::MoveEntities(command) => command.apply(ui_state),
            Self::UpdateEntities(command) => command.apply(ui_state),
            Self::UpdateSceneRulesGraph(command) => command.apply(ui_state),
            Self::UpdateMenuSettings(command) => command.apply(project),
            Self::ApplyProjectFileChanges(command) => command.apply(ui_state, project),
            Self::DeleteScene(command) => command.apply(ui_state, project),
        }
    }

    pub fn undo(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        match self {
            Self::AddEntity(command) => command.undo(ui_state),
            Self::RemoveEntity(command) => command.undo(ui_state),
            Self::MoveEntities(command) => command.undo(ui_state),
            Self::UpdateEntities(command) => command.undo(ui_state),
            Self::UpdateSceneRulesGraph(command) => command.undo(ui_state),
            Self::UpdateMenuSettings(command) => command.undo(project),
            Self::ApplyProjectFileChanges(command) => command.undo(ui_state, project),
            Self::DeleteScene(command) => command.undo(ui_state, project),
        }
    }

    fn mark_post_apply(&self, ui_state: &mut EditorUI) {
        if matches!(
            self,
            Self::AddEntity(_)
                | Self::RemoveEntity(_)
                | Self::MoveEntities(_)
                | Self::UpdateEntities(_)
                | Self::UpdateSceneRulesGraph(_)
                | Self::ApplyProjectFileChanges(_)
                | Self::DeleteScene(_)
        ) {
            ui_state.scene_content_changed = true;
            ui_state.project.rescan_assets_requested = true;
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

#[derive(Debug, Clone)]
pub struct ApplyProjectFileChangesCommand {
    description: String,
    changes: Vec<ProjectFileChange>,
    selection_before: Option<Selection>,
    selection_after: Option<Selection>,
    project_metadata_before: Option<ProjectMetadata>,
    project_metadata_after: Option<ProjectMetadata>,
}

#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub index: usize,
    pub scene: toki_core::Scene,
}

#[derive(Debug, Clone)]
pub struct DeleteSceneCommand {
    removed_scene: SceneSnapshot,
    active_scene_before: Option<String>,
    active_scene_after: Option<String>,
    selection_before: Option<Selection>,
    selection_after: Option<Selection>,
    changes: Vec<ProjectFileChange>,
    project_metadata_before: Option<ProjectMetadata>,
    project_metadata_after: Option<ProjectMetadata>,
}

#[derive(Debug, Clone)]
pub struct DeleteSceneCommandData {
    pub removed_scene: SceneSnapshot,
    pub active_scene_before: Option<String>,
    pub active_scene_after: Option<String>,
    pub selection_before: Option<Selection>,
    pub selection_after: Option<Selection>,
    pub changes: Vec<ProjectFileChange>,
    pub project_metadata_before: Option<ProjectMetadata>,
    pub project_metadata_after: Option<ProjectMetadata>,
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

impl ApplyProjectFileChangesCommand {
    fn apply(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        let Some(project) = project else {
            return false;
        };

        if let Err(error) = apply_project_file_changes(&project.path, &self.changes) {
            tracing::error!(
                "Failed to apply project file change command '{}': {}",
                self.description,
                error
            );
            return false;
        }

        if let Some(metadata) = &self.project_metadata_after {
            project.metadata = metadata.clone();
        }
        apply_selection(ui_state, self.selection_after.clone());
        mark_project_dirty(project);
        true
    }

    fn undo(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        let Some(project) = project else {
            return false;
        };

        if let Err(error) = revert_project_file_changes(&project.path, &self.changes) {
            tracing::error!(
                "Failed to undo project file change command '{}': {}",
                self.description,
                error
            );
            return false;
        }

        if let Some(metadata) = &self.project_metadata_before {
            project.metadata = metadata.clone();
        }
        apply_selection(ui_state, self.selection_before.clone());
        mark_project_dirty(project);
        true
    }
}

impl DeleteSceneCommand {
    fn apply(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        let Some(project) = project else {
            return false;
        };
        if !ui_state
            .scenes
            .iter()
            .any(|scene| scene.name == self.removed_scene.scene.name)
        {
            return false;
        }
        if let Err(error) = apply_project_file_changes(&project.path, &self.changes) {
            tracing::error!("Failed to delete scene '{}': {}", self.removed_scene.scene.name, error);
            return false;
        }
        let Some(_) = remove_scene_snapshot(ui_state, &self.removed_scene.scene.name) else {
            return false;
        };
        if let Some(metadata) = &self.project_metadata_after {
            project.metadata = metadata.clone();
        }
        ui_state.active_scene = self.active_scene_after.clone();
        apply_selection(ui_state, self.selection_after.clone());
        mark_project_dirty(project);
        true
    }

    fn undo(&self, ui_state: &mut EditorUI, project: Option<&mut Project>) -> bool {
        let Some(project) = project else {
            return false;
        };
        if ui_state
            .scenes
            .iter()
            .any(|scene| scene.name == self.removed_scene.scene.name)
        {
            return false;
        }
        if let Err(error) = revert_project_file_changes(&project.path, &self.changes) {
            tracing::error!(
                "Failed to restore deleted scene '{}': {}",
                self.removed_scene.scene.name,
                error
            );
            return false;
        }
        restore_scene_snapshot(ui_state, &self.removed_scene);
        if let Some(metadata) = &self.project_metadata_before {
            project.metadata = metadata.clone();
        }
        ui_state.active_scene = self.active_scene_before.clone();
        apply_selection(ui_state, self.selection_before.clone());
        mark_project_dirty(project);
        true
    }
}

fn apply_selection(ui_state: &mut EditorUI, selection: Option<Selection>) {
    match selection {
        Some(selection) => ui_state.set_selection(selection),
        None => ui_state.clear_selection(),
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

fn remove_scene_snapshot(ui_state: &mut EditorUI, scene_name: &str) -> Option<SceneSnapshot> {
    let scene_index = ui_state.scenes.iter().position(|scene| scene.name == scene_name)?;
    let scene = ui_state.scenes.remove(scene_index);
    ui_state.graph.rule_graphs_by_scene.remove(scene_name);
    ui_state.graph.layouts_by_scene.remove(scene_name);
    Some(SceneSnapshot {
        index: scene_index,
        scene,
    })
}

fn restore_scene_snapshot(ui_state: &mut EditorUI, snapshot: &SceneSnapshot) {
    let insert_index = snapshot.index.min(ui_state.scenes.len());
    ui_state
        .scenes
        .insert(insert_index, snapshot.scene.clone());
}

#[cfg(test)]
#[path = "undo_redo_tests.rs"]
mod tests;
