use super::super::inspector_trait::{Inspector, InspectorContext};
use super::InspectorSystem;
use crate::project::Project;
use crate::ui::editor_ui::{EditorUI, Selection};
use toki_core::entity::EntityId;

/// Inspector for single or multi-entity selection.
pub struct EntityInspector {
    entity_id: EntityId,
}

impl EntityInspector {
    pub fn new(entity_id: EntityId) -> Self {
        Self { entity_id }
    }
}

impl Inspector for EntityInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        let mut entity_changed = false;

        if ctx.ui_state.has_multi_entity_selection() {
            ui.heading(format!(
                "👥 {} Entities",
                ctx.ui_state.selected_entity_ids().len()
            ));
            ui.separator();
            entity_changed = InspectorSystem::render_multi_scene_entity_editor(ui, ctx.ui_state);
        } else {
            ui.separator();
            ui.heading(format!("👤 Entity {}", self.entity_id));
            ui.separator();

            if let Some(scene_entity) =
                InspectorSystem::find_selected_scene_entity(ctx.ui_state, self.entity_id)
            {
                let mut draft = super::EntityPropertyDraft::from_entity(&scene_entity);
                if InspectorSystem::render_scene_entity_editor(ui, &mut draft, ctx.config) {
                    entity_changed = InspectorSystem::apply_entity_property_draft_with_undo(
                        ctx.ui_state,
                        self.entity_id,
                        &draft,
                    );
                }
            } else {
                ui.label("Runtime-only entity (read-only)");
                ui.separator();
                InspectorSystem::render_runtime_entity_read_only(
                    ui,
                    ctx.game_state,
                    self.entity_id,
                );
            }
        }

        if entity_changed {
            ctx.ui_state.scene_content_changed = true;
        }
        entity_changed
    }

    fn name(&self) -> &'static str {
        "Entity"
    }
}

/// Inspector for scene selection.
pub struct SceneInspector {
    scene_name: String,
}

impl SceneInspector {
    pub fn new(scene_name: String) -> Self {
        Self { scene_name }
    }
}

impl Inspector for SceneInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("🎬 {}", self.scene_name));
        ui.separator();

        if let Some(scene) = ctx.ui_state.get_scene(&self.scene_name) {
            ui.horizontal(|ui| {
                ui.label("Maps:");
                ui.label(format!("{}", scene.maps.len()));
            });

            ui.horizontal(|ui| {
                ui.label("Entities:");
                ui.label(format!("{}", scene.entities.len()));
            });

            ui.separator();
            ui.label("Scene Actions:");

            if ui.button("🗺 Add Map").clicked() {
                tracing::info!("Add Map to scene: {}", self.scene_name);
            }

            if ui.button("👤 Add Entity").clicked() {
                tracing::info!("Add Entity to scene: {}", self.scene_name);
            }

            if ui.button("Delete Scene").clicked() {
                let scene_is_empty = scene.maps.is_empty()
                    && scene.entities.is_empty()
                    && scene.rules.rules.is_empty();
                if scene_is_empty {
                    if let Some(project) = ctx.project.as_deref_mut() {
                        if let Ok(command) = build_delete_scene_command(
                            ctx.ui_state,
                            project,
                            &self.scene_name,
                        ) {
                            let _ = ctx.ui_state.execute_command_with_project(project, command);
                        }
                    }
                } else {
                    ctx.ui_state.project.pending_confirmation =
                        Some(super::super::editor_ui::EditorConfirmation::DeleteScene {
                            scene_name: self.scene_name.clone(),
                        });
                }
            }
        }

        // Rules editor section
        if let Some(scene_index) = ctx
            .ui_state
            .scenes
            .iter()
            .position(|scene| scene.name == self.scene_name)
        {
            ui.separator();
            let before_rules = ctx.ui_state.scenes[scene_index].rules.clone();
            let mut edited_rules = before_rules.clone();
            let rules_changed = InspectorSystem::render_scene_rules_editor(
                ui,
                &self.scene_name,
                &mut edited_rules,
                ctx.config,
            );
            if rules_changed && edited_rules != before_rules {
                use super::super::editor_ui::SceneRulesGraphCommandData;
                use super::super::rule_graph::RuleGraph;

                let before_graph = ctx.ui_state.rule_graph_for_scene(&self.scene_name).cloned();
                let after_graph = RuleGraph::from_rule_set(&edited_rules);
                let before_layout = ctx
                    .ui_state
                    .graph
                    .layouts_by_scene
                    .get(&self.scene_name)
                    .cloned();
                let (zoom, pan) = ctx.ui_state.graph_view_for_scene(&self.scene_name);
                let _ = ctx.ui_state.execute_scene_rules_graph_command(
                    &self.scene_name,
                    SceneRulesGraphCommandData {
                        before_rule_set: before_rules,
                        after_rule_set: edited_rules,
                        before_graph,
                        after_graph,
                        before_layout,
                        zoom,
                        pan,
                    },
                );
                return true;
            }
        }

        false
    }

    fn name(&self) -> &'static str {
        "Scene"
    }
}

pub(crate) fn build_delete_scene_command(
    ui_state: &EditorUI,
    project: &Project,
    scene_name: &str,
) -> Result<super::super::undo_redo::EditorCommand, String> {
    let Some(scene_index) = ui_state.scenes.iter().position(|scene| scene.name == scene_name) else {
        return Err(format!("scene '{scene_name}' not found"));
    };
    let removed_scene = super::super::undo_redo::SceneSnapshot {
        index: scene_index,
        scene: ui_state.scenes[scene_index].clone(),
    };
    let remaining_scene_names = ui_state
        .scenes
        .iter()
        .filter(|scene| scene.name != scene_name)
        .map(|scene| scene.name.clone())
        .collect::<Vec<_>>();
    let active_scene_before = ui_state.active_scene.clone();
    let active_scene_after = if active_scene_before.as_deref() == Some(scene_name) {
        remaining_scene_names.first().cloned()
    } else {
        active_scene_before.clone()
    };
    let selection_before = ui_state.selection.clone();
    let selection_after = active_scene_after
        .as_ref()
        .map(|scene_name| Selection::Scene(scene_name.clone()));

    let mut metadata_after = project.metadata.clone();
    metadata_after.scenes.remove(scene_name);
    metadata_after.editor.camera_settings.remove(scene_name);
    metadata_after.editor.graph_layouts.remove(scene_name);
    metadata_after.editor.rule_graph_drafts.remove(scene_name);
    if metadata_after.editor.last_scene.as_deref() == Some(scene_name) {
        metadata_after.editor.last_scene = active_scene_after.clone();
    }
    let project_file_before = std::fs::read_to_string(project.project_file_path())
        .map_err(|error| format!("failed to read project.toml: {error}"))?;
    let project_file_after = toml::to_string_pretty(&metadata_after)
        .map_err(|error| format!("failed to serialize project metadata: {error}"))?;
    let mut changes = Vec::new();

    if let Some(scene_relative_path) = try_resolve_scene_relative_path(project, scene_name)? {
        let scene_absolute_path = project.path.join(&scene_relative_path);
        let scene_before_contents = std::fs::read_to_string(&scene_absolute_path).map_err(|error| {
            format!(
                "failed to read scene file '{}': {error}",
                scene_absolute_path.display()
            )
        })?;
        changes.push(toki_template_lowering::ProjectFileChange {
            relative_path: scene_relative_path,
            before_contents: Some(scene_before_contents),
            after_contents: None,
        });
    }

    changes.push(toki_template_lowering::ProjectFileChange {
        relative_path: std::path::PathBuf::from("project.toml"),
        before_contents: Some(project_file_before),
        after_contents: Some(project_file_after),
    });

    Ok(super::super::undo_redo::EditorCommand::delete_scene(
        super::super::undo_redo::DeleteSceneCommandData {
            removed_scene,
            active_scene_before,
            active_scene_after,
            selection_before,
            selection_after,
            changes,
            project_metadata_before: Some(project.metadata.clone()),
            project_metadata_after: Some(metadata_after),
        },
    ))
}

fn try_resolve_scene_relative_path(
    project: &Project,
    scene_name: &str,
) -> Result<Option<std::path::PathBuf>, String> {
    if let Some(mapped_relative_path) = project.metadata.scenes.get(scene_name) {
        let mapped_relative_path = std::path::PathBuf::from(mapped_relative_path);
        if project.path.join(&mapped_relative_path).exists() {
            return Ok(Some(mapped_relative_path));
        }
    }

    let conventional_relative_path =
        std::path::PathBuf::from("scenes").join(format!("{scene_name}.json"));
    if project.path.join(&conventional_relative_path).exists() {
        return Ok(Some(conventional_relative_path));
    }

    let scenes_dir = project.path.join("scenes");
    if !scenes_dir.exists() {
        return Ok(None);
    }

    let matching_entry = std::fs::read_dir(&scenes_dir)
        .map_err(|error| format!("failed to read scenes directory '{}': {error}", scenes_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension().is_some_and(|extension| extension == "json")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem == scene_name)
        });

    let Some(matching_entry) = matching_entry else {
        return Ok(None);
    };

    matching_entry
        .strip_prefix(&project.path)
        .map(|relative_path| Some(relative_path.to_path_buf()))
        .map_err(|error| {
            format!(
                "failed to relativize scene path '{}' against project '{}': {error}",
                matching_entry.display(),
                project.path.display()
            )
        })
}

/// Inspector for map selection (within a scene).
pub struct MapInspector {
    scene_name: String,
    map_name: String,
}

impl MapInspector {
    pub fn new(scene_name: String, map_name: String) -> Self {
        Self {
            scene_name,
            map_name,
        }
    }
}

impl Inspector for MapInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("🗺️ {}", self.map_name));
        ui.label(format!("Scene: {}", self.scene_name));
        ui.separator();

        InspectorSystem::render_map_details(
            ui,
            &self.map_name,
            ctx.config,
            Some(&self.scene_name),
            &mut ctx.ui_state.map.load_requested,
        );
        false
    }

    fn name(&self) -> &'static str {
        "Map"
    }
}

/// Inspector for standalone map (not in scene context).
pub struct StandaloneMapInspector {
    map_name: String,
}

impl StandaloneMapInspector {
    pub fn new(map_name: String) -> Self {
        Self { map_name }
    }
}

impl Inspector for StandaloneMapInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("🗺️ {}", self.map_name));
        ui.label("(Standalone map - not in scene)");
        ui.separator();

        InspectorSystem::render_map_details(
            ui,
            &self.map_name,
            ctx.config,
            None,
            &mut ctx.ui_state.map.load_requested,
        );
        false
    }

    fn name(&self) -> &'static str {
        "StandaloneMap"
    }
}

/// Inspector for entity definition from palette.
pub struct EntityDefinitionInspector {
    entity_name: String,
}

impl EntityDefinitionInspector {
    pub fn new(entity_name: String) -> Self {
        Self { entity_name }
    }
}

impl Inspector for EntityDefinitionInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("🤖 {}", self.entity_name));
        ui.label("Entity Definition");
        ui.separator();

        InspectorSystem::render_entity_definition_details(ui, &self.entity_name, ctx.config);
        false
    }

    fn name(&self) -> &'static str {
        "EntityDefinition"
    }
}

/// Inspector for rule graph node selection.
pub struct RuleGraphNodeInspector {
    scene_name: String,
    node_key: String,
}

impl RuleGraphNodeInspector {
    pub fn new(scene_name: String, node_key: String) -> Self {
        Self {
            scene_name,
            node_key,
        }
    }
}

impl Inspector for RuleGraphNodeInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading("🧩 Scene Rule Node");
        ui.label(format!("Scene: {}", self.scene_name));
        ui.monospace(&self.node_key);
        ui.separator();

        let changed = InspectorSystem::render_selected_rule_graph_node_editor(
            ui,
            ctx.ui_state,
            &self.scene_name,
            &self.node_key,
            ctx.config,
        );

        if changed {
            ctx.ui_state.scene_content_changed = true;
        }
        changed
    }

    fn name(&self) -> &'static str {
        "RuleGraphNode"
    }
}

/// Inspector for menu selections (placeholder).
pub struct MenuSelectionInspector;

impl Inspector for MenuSelectionInspector {
    fn render(&mut self, ui: &mut egui::Ui, _ctx: &mut InspectorContext<'_>) -> bool {
        ui.label("Menu selection available only in Menu Editor.");
        false
    }

    fn name(&self) -> &'static str {
        "MenuSelection"
    }
}

use super::super::inspector_trait::NoSelectionInspector;

/// Creates the appropriate inspector for the given selection.
/// This factory replaces the large match statement in render_selection_inspector_contents.
pub fn create_inspector_for_selection(selection: Option<&Selection>) -> Box<dyn Inspector> {
    match selection {
        Some(Selection::Scene(scene_name)) => Box::new(SceneInspector::new(scene_name.clone())),
        Some(Selection::RuleGraphNode {
            scene_name,
            node_key,
        }) => Box::new(RuleGraphNodeInspector::new(
            scene_name.clone(),
            node_key.clone(),
        )),
        Some(Selection::Map(scene_name, map_name)) => {
            Box::new(MapInspector::new(scene_name.clone(), map_name.clone()))
        }
        Some(Selection::Entity(entity_id)) => Box::new(EntityInspector::new(*entity_id)),
        Some(Selection::StandaloneMap(map_name)) => {
            Box::new(StandaloneMapInspector::new(map_name.clone()))
        }
        Some(Selection::EntityDefinition(entity_name)) => {
            Box::new(EntityDefinitionInspector::new(entity_name.clone()))
        }
        Some(Selection::MenuScreen(_))
        | Some(Selection::MenuDialog(_))
        | Some(Selection::MenuEntry { .. }) => Box::new(MenuSelectionInspector),
        None => Box::new(NoSelectionInspector),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn create_inspector_for_none_returns_no_selection_inspector() {
        let inspector = create_inspector_for_selection(None);
        assert_eq!(inspector.name(), "NoSelection");
    }

    #[test]
    fn create_inspector_for_scene_returns_scene_inspector() {
        let selection = Selection::Scene("TestScene".to_string());
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "Scene");
    }

    #[test]
    fn create_inspector_for_entity_returns_entity_inspector() {
        let selection = Selection::Entity(42);
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "Entity");
    }

    #[test]
    fn create_inspector_for_map_returns_map_inspector() {
        let selection = Selection::Map("Scene1".to_string(), "map1".to_string());
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "Map");
    }

    #[test]
    fn create_inspector_for_standalone_map_returns_standalone_map_inspector() {
        let selection = Selection::StandaloneMap("standalone".to_string());
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "StandaloneMap");
    }

    #[test]
    fn create_inspector_for_entity_definition_returns_entity_definition_inspector() {
        let selection = Selection::EntityDefinition("player".to_string());
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "EntityDefinition");
    }

    #[test]
    fn create_inspector_for_rule_graph_node_returns_rule_graph_node_inspector() {
        let selection = Selection::RuleGraphNode {
            scene_name: "Scene1".to_string(),
            node_key: "node1".to_string(),
        };
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "RuleGraphNode");
    }

    #[test]
    fn create_inspector_for_menu_selection_returns_menu_selection_inspector() {
        let selection = Selection::MenuScreen("main".to_string());
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "MenuSelection");
    }

    #[test]
    fn entity_inspector_has_correct_name() {
        let inspector = EntityInspector::new(1);
        assert_eq!(inspector.name(), "Entity");
    }

    #[test]
    fn scene_inspector_has_correct_name() {
        let inspector = SceneInspector::new("TestScene".to_string());
        assert_eq!(inspector.name(), "Scene");
    }

    #[test]
    fn map_inspector_has_correct_name() {
        let inspector = MapInspector::new("Scene1".to_string(), "map1".to_string());
        assert_eq!(inspector.name(), "Map");
    }

    #[test]
    fn standalone_map_inspector_has_correct_name() {
        let inspector = StandaloneMapInspector::new("standalone".to_string());
        assert_eq!(inspector.name(), "StandaloneMap");
    }

    #[test]
    fn entity_definition_inspector_has_correct_name() {
        let inspector = EntityDefinitionInspector::new("player".to_string());
        assert_eq!(inspector.name(), "EntityDefinition");
    }

    #[test]
    fn rule_graph_node_inspector_has_correct_name() {
        let inspector = RuleGraphNodeInspector::new("Scene1".to_string(), "node1".to_string());
        assert_eq!(inspector.name(), "RuleGraphNode");
    }

    #[test]
    fn menu_selection_inspector_has_correct_name() {
        let inspector = MenuSelectionInspector;
        assert_eq!(inspector.name(), "MenuSelection");
    }

    #[test]
    fn build_delete_scene_command_resolves_existing_scene_file_when_metadata_path_is_stale() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let project_root = temp_dir.path().to_path_buf();
        std::fs::create_dir_all(project_root.join("scenes")).expect("scenes dir should exist");
        std::fs::write(
            project_root.join("scenes").join("Main Scene.json"),
            "{\n  \"name\": \"Main Scene\"\n}\n",
        )
        .expect("scene file should write");

        let mut project = Project::new("Demo".to_string(), project_root);
        project
            .metadata
            .scenes
            .insert("Main Scene".to_string(), "scenes/mainscene.json".to_string());
        std::fs::write(
            project.project_file_path(),
            toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
        )
        .expect("project metadata should write");

        let mut ui_state = EditorUI::new();
        ui_state.scenes = vec![toki_core::Scene::new("Main Scene".to_string())];
        ui_state.active_scene = Some("Main Scene".to_string());
        ui_state.set_selection(Selection::Scene("Main Scene".to_string()));

        let command = build_delete_scene_command(&ui_state, &project, "Main Scene");
        assert!(command.is_ok());
    }

    #[test]
    fn build_delete_scene_command_allows_scene_without_backing_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir should exist");
        let project_root = temp_dir.path().to_path_buf();

        let project = Project::new("Demo".to_string(), project_root);
        std::fs::write(
            project.project_file_path(),
            toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
        )
        .expect("project metadata should write");

        let mut ui_state = EditorUI::new();
        ui_state.scenes = vec![toki_core::Scene::new("Scene 3".to_string())];
        ui_state.active_scene = Some("Scene 3".to_string());
        ui_state.set_selection(Selection::Scene("Scene 3".to_string()));

        let command = build_delete_scene_command(&ui_state, &project, "Scene 3");
        assert!(command.is_ok());
    }
}
