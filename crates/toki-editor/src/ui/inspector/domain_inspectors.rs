use super::super::inspector_trait::{Inspector, InspectorContext};
use super::InspectorSystem;
use toki_core::entity::{ControlRole, EntityId};
use toki_core::scene::{SceneAnchorFacing, SceneAnchorKind, ScenePlayerEntry};

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

    fn scene_has_authored_player_entity(scene: &toki_core::Scene) -> bool {
        scene
            .entities
            .iter()
            .any(|entity| entity.control_role == ControlRole::PlayerCharacter)
    }

    fn render_scene_player_entry_editor(
        ui: &mut egui::Ui,
        scene_name: &str,
        scene: &toki_core::Scene,
        ctx: &mut InspectorContext<'_>,
    ) -> bool {
        ui.label("Scene Player Entry:");

        let entity_definition_names = ctx
            .project
            .as_ref()
            .map(|project| {
                InspectorSystem::discover_entity_definition_names(
                    project.path.join("entities").as_path(),
                )
            })
            .unwrap_or_default();
        let spawn_point_ids = scene
            .anchors
            .iter()
            .filter(|anchor| anchor.kind == SceneAnchorKind::SpawnPoint)
            .map(|anchor| anchor.id.clone())
            .collect::<Vec<_>>();
        let has_authored_player_entity = Self::scene_has_authored_player_entity(scene);

        if has_authored_player_entity {
            ui.colored_label(
                egui::Color32::YELLOW,
                "This scene already contains a placed player entity. Scene Player Entry preview stays disabled until that authored player entity is removed.",
            );
        }

        match scene.player_entry.clone() {
            Some(current_entry) => {
                let mut edited_entry = current_entry.clone();
                let mut entry_changed = false;

                ui.horizontal(|ui| {
                    ui.label("Entity Definition:");
                    egui::ComboBox::from_id_salt(("scene_player_entity_definition", scene_name))
                        .selected_text(edited_entry.entity_definition_name.as_str())
                        .show_ui(ui, |ui| {
                            for entity_definition_name in &entity_definition_names {
                                entry_changed |= ui
                                    .selectable_value(
                                        &mut edited_entry.entity_definition_name,
                                        entity_definition_name.clone(),
                                        entity_definition_name,
                                    )
                                    .changed();
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Spawn Point:");
                    egui::ComboBox::from_id_salt(("scene_player_spawn_point", scene_name))
                        .selected_text(edited_entry.spawn_point_id.as_str())
                        .show_ui(ui, |ui| {
                            for spawn_point_id in &spawn_point_ids {
                                entry_changed |= ui
                                    .selectable_value(
                                        &mut edited_entry.spawn_point_id,
                                        spawn_point_id.clone(),
                                        spawn_point_id,
                                    )
                                    .changed();
                            }
                        });
                });

                if entry_changed {
                    let before_scene = scene.clone();
                    let mut after_scene = before_scene.clone();
                    after_scene.player_entry = Some(edited_entry);
                    if ctx.ui_state.execute_command(
                        crate::ui::undo_redo::EditorCommand::update_scene(
                            scene_name.to_string(),
                            before_scene,
                            after_scene,
                        ),
                    ) {
                        return true;
                    }
                }

                if entity_definition_names.is_empty() {
                    ui.label("No entity definitions found in this project.");
                }
                if spawn_point_ids.is_empty() {
                    ui.label("Add a spawn point before assigning a scene player entry.");
                }

                if ui.button("➖ Remove Scene Player Entry").clicked() {
                    let before_scene = scene.clone();
                    let mut after_scene = before_scene.clone();
                    after_scene.player_entry = None;
                    if ctx.ui_state.execute_command(
                        crate::ui::undo_redo::EditorCommand::update_scene(
                            scene_name.to_string(),
                            before_scene,
                            after_scene,
                        ),
                    ) {
                        if matches!(
                            ctx.ui_state.selection,
                            Some(crate::ui::editor_ui::Selection::ScenePlayerEntry(ref selected_scene))
                                if selected_scene == scene_name
                        ) {
                            ctx.ui_state
                                .set_selection(crate::ui::editor_ui::Selection::Scene(
                                    scene_name.to_string(),
                                ));
                        }
                        return true;
                    }
                }
            }
            None => {
                if entity_definition_names.is_empty() {
                    ui.label("No entity definitions found in this project.");
                }
                if spawn_point_ids.is_empty() {
                    ui.label("Add a spawn point before creating a scene player entry.");
                }

                let can_add_scene_player_entry =
                    !entity_definition_names.is_empty() && !spawn_point_ids.is_empty();
                if ui
                    .add_enabled(
                        can_add_scene_player_entry,
                        egui::Button::new("➕ Add Scene Player Entry"),
                    )
                    .clicked()
                {
                    let before_scene = scene.clone();
                    let mut after_scene = before_scene.clone();
                    after_scene.player_entry = Some(ScenePlayerEntry {
                        entity_definition_name: entity_definition_names[0].clone(),
                        spawn_point_id: spawn_point_ids[0].clone(),
                    });
                    if ctx.ui_state.execute_command(
                        crate::ui::undo_redo::EditorCommand::update_scene(
                            scene_name.to_string(),
                            before_scene,
                            after_scene,
                        ),
                    ) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

impl Inspector for SceneInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("🎬 {}", self.scene_name));
        ui.separator();

        if let Some(scene) = ctx.ui_state.get_scene(&self.scene_name).cloned() {
            ui.horizontal(|ui| {
                ui.label("Maps:");
                ui.label(format!("{}", scene.maps.len()));
            });

            ui.horizontal(|ui| {
                ui.label("Entities:");
                ui.label(format!("{}", scene.entities.len()));
            });

            ui.separator();

            let mut background_music_track_id =
                scene.background_music_track_id.clone().unwrap_or_default();
            let mut background_music_changed = false;
            ui.horizontal(|ui| {
                ui.label("Background Music:");
                background_music_changed |= ui
                    .text_edit_singleline(&mut background_music_track_id)
                    .changed();
            });
            if background_music_changed {
                let before_scene = scene.clone();
                let mut after_scene = before_scene.clone();
                after_scene.background_music_track_id =
                    (!background_music_track_id.trim().is_empty())
                        .then(|| background_music_track_id.trim().to_string());
                if ctx
                    .ui_state
                    .execute_command(crate::ui::undo_redo::EditorCommand::update_scene(
                        self.scene_name.clone(),
                        before_scene,
                        after_scene,
                    ))
                {
                    return true;
                }
            }

            ui.separator();
            if Self::render_scene_player_entry_editor(ui, &self.scene_name, &scene, ctx) {
                return true;
            }

            ui.separator();
            ui.label("Scene Actions:");

            if ui.button("🗺 Add Map").clicked() {
                tracing::info!("Add Map to scene: {}", self.scene_name);
            }

            if ui.button("👤 Add Entity").clicked() {
                tracing::info!("Add Entity to scene: {}", self.scene_name);
            }

            ui.separator();
            ui.label("Scene Anchors:");

            for anchor in &scene.anchors {
                let selected = matches!(
                    ctx.ui_state.selection,
                    Some(crate::ui::editor_ui::Selection::SceneAnchor {
                        ref scene_name,
                        ref anchor_id
                    }) if scene_name == &self.scene_name && anchor_id == &anchor.id
                );
                if ui
                    .selectable_label(
                        selected,
                        format!(
                            "{} ({:?}) @ {}, {}",
                            anchor.id, anchor.kind, anchor.position.x, anchor.position.y
                        ),
                    )
                    .clicked()
                {
                    ctx.ui_state
                        .set_selection(crate::ui::editor_ui::Selection::SceneAnchor {
                            scene_name: self.scene_name.clone(),
                            anchor_id: anchor.id.clone(),
                        });
                }
            }

            if ui.button("➕ Place Spawn Point").clicked() {
                let next_id = crate::ui::interactions::PlacementInteraction::next_scene_anchor_id(
                    &scene.anchors,
                    SceneAnchorKind::SpawnPoint,
                );
                ctx.ui_state.enter_scene_anchor_placement_mode(
                    crate::ui::editor_ui::SceneAnchorPlacementDraft {
                        kind: SceneAnchorKind::SpawnPoint,
                        suggested_id: next_id,
                    },
                );
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
                &ctx.ui_state.scenes,
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

pub struct SceneAnchorInspector {
    scene_name: String,
    anchor_id: String,
}

impl SceneAnchorInspector {
    pub fn new(scene_name: String, anchor_id: String) -> Self {
        Self {
            scene_name,
            anchor_id,
        }
    }
}

impl Inspector for SceneAnchorInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading(format!("📍 {}", self.anchor_id));
        ui.label(format!("Scene: {}", self.scene_name));
        ui.separator();

        let Some(before_scene) = ctx.ui_state.get_scene(&self.scene_name).cloned() else {
            ui.label("Scene not found.");
            return false;
        };
        let Some(anchor_index) = before_scene
            .anchors
            .iter()
            .position(|anchor| anchor.id == self.anchor_id)
        else {
            ui.label("Selected anchor no longer exists.");
            return false;
        };

        let mut edited_anchor = before_scene.anchors[anchor_index].clone();
        let mut changed = false;
        let mut delete_requested = false;

        ui.horizontal(|ui| {
            ui.label("Id:");
            changed |= ui.text_edit_singleline(&mut edited_anchor.id).changed();
        });

        ui.horizontal(|ui| {
            ui.label("Kind:");
            egui::ComboBox::from_id_salt(("scene_anchor_kind", &self.scene_name, &self.anchor_id))
                .selected_text(match edited_anchor.kind {
                    SceneAnchorKind::SpawnPoint => "SpawnPoint",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(
                            &mut edited_anchor.kind,
                            SceneAnchorKind::SpawnPoint,
                            "SpawnPoint",
                        )
                        .changed();
                });
        });

        ui.horizontal(|ui| {
            ui.label("Position:");
            changed |= ui
                .add(egui::DragValue::new(&mut edited_anchor.position.x).speed(1.0))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut edited_anchor.position.y).speed(1.0))
                .changed();
        });

        ui.horizontal(|ui| {
            ui.label("Facing:");
            let mut facing = edited_anchor.facing;
            egui::ComboBox::from_id_salt((
                "scene_anchor_facing",
                &self.scene_name,
                &self.anchor_id,
            ))
            .selected_text(match facing {
                None => "<none>",
                Some(SceneAnchorFacing::Up) => "Up",
                Some(SceneAnchorFacing::Down) => "Down",
                Some(SceneAnchorFacing::Left) => "Left",
                Some(SceneAnchorFacing::Right) => "Right",
            })
            .show_ui(ui, |ui| {
                changed |= ui.selectable_value(&mut facing, None, "<none>").changed();
                changed |= ui
                    .selectable_value(&mut facing, Some(SceneAnchorFacing::Up), "Up")
                    .changed();
                changed |= ui
                    .selectable_value(&mut facing, Some(SceneAnchorFacing::Down), "Down")
                    .changed();
                changed |= ui
                    .selectable_value(&mut facing, Some(SceneAnchorFacing::Left), "Left")
                    .changed();
                changed |= ui
                    .selectable_value(&mut facing, Some(SceneAnchorFacing::Right), "Right")
                    .changed();
            });
            edited_anchor.facing = facing;
        });

        if before_scene
            .anchors
            .iter()
            .enumerate()
            .any(|(index, anchor)| index != anchor_index && anchor.id == edited_anchor.id)
        {
            ui.colored_label(
                egui::Color32::from_rgb(255, 210, 80),
                "Anchor id must be unique within the scene.",
            );
            changed = false;
        }

        if ui.button("🗑 Delete Anchor").clicked() {
            delete_requested = true;
        }

        if delete_requested {
            let mut after_scene = before_scene.clone();
            after_scene.anchors.remove(anchor_index);
            if ctx
                .ui_state
                .execute_command(crate::ui::undo_redo::EditorCommand::update_scene(
                    self.scene_name.clone(),
                    before_scene,
                    after_scene,
                ))
            {
                ctx.ui_state
                    .set_selection(crate::ui::editor_ui::Selection::Scene(
                        self.scene_name.clone(),
                    ));
                return true;
            }
            return false;
        }

        if changed {
            let mut after_scene = before_scene.clone();
            after_scene.anchors[anchor_index] = edited_anchor.clone();
            if ctx
                .ui_state
                .execute_command(crate::ui::undo_redo::EditorCommand::update_scene(
                    self.scene_name.clone(),
                    before_scene,
                    after_scene,
                ))
            {
                self.anchor_id = edited_anchor.id.clone();
                ctx.ui_state
                    .set_selection(crate::ui::editor_ui::Selection::SceneAnchor {
                        scene_name: self.scene_name.clone(),
                        anchor_id: edited_anchor.id,
                    });
                return true;
            }
        }

        false
    }

    fn name(&self) -> &'static str {
        "SceneAnchor"
    }
}

pub struct ScenePlayerEntryInspector {
    scene_name: String,
}

impl ScenePlayerEntryInspector {
    pub fn new(scene_name: String) -> Self {
        Self { scene_name }
    }
}

impl Inspector for ScenePlayerEntryInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading("🧍 Scene Player");
        ui.label(format!("Scene: {}", self.scene_name));
        ui.separator();

        let Some(scene) = ctx.ui_state.get_scene(&self.scene_name).cloned() else {
            ui.label("Scene not found.");
            return false;
        };

        let changed =
            SceneInspector::render_scene_player_entry_editor(ui, &self.scene_name, &scene, ctx);

        if changed {
            return true;
        }

        if let Some(player_entry) = scene.player_entry.as_ref() {
            ui.separator();
            ui.heading(format!("🤖 {}", player_entry.entity_definition_name));
            ui.label("Player Entity Definition");
            ui.separator();

            if let Some(config) = ctx.config {
                if let Some(project_path) = config.current_project_path() {
                    let entity_file = project_path
                        .join("entities")
                        .join(format!("{}.json", player_entry.entity_definition_name));
                    match std::fs::read_to_string(&entity_file) {
                        Ok(content) => match serde_json::from_str::<
                            toki_core::entity::EntityDefinition,
                        >(&content)
                        {
                            Ok(mut definition) => {
                                let mut draft =
                                    super::EntityPropertyDraft::from_entity_definition(&definition);
                                if InspectorSystem::render_entity_definition_property_editor(
                                    ui, &mut draft, ctx.config,
                                ) && InspectorSystem::apply_entity_property_draft_to_definition(
                                        &mut definition,
                                        &draft,
                                    )
                                {
                                    if let Err(err) = InspectorSystem::save_entity_definition(
                                        &definition,
                                        &entity_file,
                                    ) {
                                        ui.colored_label(egui::Color32::RED, err);
                                    } else {
                                        ctx.ui_state.scene_content_changed = true;
                                        return true;
                                    }
                                }
                            }
                            Err(err) => {
                                ui.colored_label(
                                    egui::Color32::RED,
                                    format!("Failed to parse entity definition: {err}"),
                                );
                            }
                        },
                        Err(err) => {
                            ui.colored_label(
                                egui::Color32::RED,
                                format!("Failed to read entity definition: {err}"),
                            );
                        }
                    }
                }
            }
        }

        false
    }

    fn name(&self) -> &'static str {
        "ScenePlayerEntry"
    }
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

use super::super::editor_ui::Selection;
use super::super::inspector_trait::NoSelectionInspector;

/// Creates the appropriate inspector for the given selection.
/// This factory replaces the large match statement in render_selection_inspector_contents.
pub fn create_inspector_for_selection(selection: Option<&Selection>) -> Box<dyn Inspector> {
    match selection {
        Some(Selection::Scene(scene_name)) => Box::new(SceneInspector::new(scene_name.clone())),
        Some(Selection::ScenePlayerEntry(scene_name)) => {
            Box::new(ScenePlayerEntryInspector::new(scene_name.clone()))
        }
        Some(Selection::SceneAnchor {
            scene_name,
            anchor_id,
        }) => Box::new(SceneAnchorInspector::new(
            scene_name.clone(),
            anchor_id.clone(),
        )),
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
    fn create_inspector_for_scene_player_entry_returns_scene_player_entry_inspector() {
        let selection = Selection::ScenePlayerEntry("TestScene".to_string());
        let inspector = create_inspector_for_selection(Some(&selection));
        assert_eq!(inspector.name(), "ScenePlayerEntry");
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
    fn scene_player_entry_inspector_has_correct_name() {
        let inspector = ScenePlayerEntryInspector::new("TestScene".to_string());
        assert_eq!(inspector.name(), "ScenePlayerEntry");
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
}
