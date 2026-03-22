use crate::project::Project;
use crate::ui::EditorUI;

pub fn load_into_ui(ui_state: &mut EditorUI, project: Option<&Project>) {
    let (graph_layouts, rule_graph_drafts) = project
        .map(|project| {
            (
                project.metadata.editor.graph_layouts.clone(),
                project.metadata.editor.rule_graph_drafts.clone(),
            )
        })
        .unwrap_or_default();
    ui_state.load_graph_layouts_from_project(&graph_layouts);
    ui_state.load_rule_graph_drafts_from_project(&rule_graph_drafts);
}

pub fn migrate_legacy_into_project(project: &mut Project) {
    let config_path = match std::env::current_dir() {
        Ok(dir) => dir.join("toki_editor_config.json"),
        Err(error) => {
            tracing::warn!(
                "Cannot determine current directory for legacy graph layout migration: {}",
                error
            );
            return;
        }
    };

    let raw_config = match std::fs::read_to_string(&config_path) {
        Ok(raw_config) => raw_config,
        Err(_) => return,
    };
    let mut config_json = match serde_json::from_str::<serde_json::Value>(&raw_config) {
        Ok(json) => json,
        Err(error) => {
            tracing::warn!(
                "Failed to parse config for legacy graph layout migration: {}",
                error
            );
            return;
        }
    };
    let Some(layouts_object) = config_json
        .get("graph_layouts")
        .and_then(|value| value.as_object())
        .cloned()
    else {
        return;
    };

    let project_key = project.path.to_string_lossy().to_string();
    let mut migrated_any = false;

    for (key, value) in layouts_object {
        let Some((entry_project_key, scene_name, node_key)) = parse_legacy_graph_layout_key(&key)
        else {
            continue;
        };
        let Some(position_values) = value.as_array() else {
            continue;
        };
        if position_values.len() != 2 {
            continue;
        }
        let Some(x) = position_values[0].as_f64() else {
            continue;
        };
        let Some(y) = position_values[1].as_f64() else {
            continue;
        };
        let position = [x as f32, y as f32];

        if entry_project_key == project_key {
            project
                .metadata
                .editor
                .graph_layouts
                .entry(scene_name)
                .or_default()
                .node_positions
                .insert(node_key, position);
            migrated_any = true;
        }
    }

    if migrated_any {
        if let Err(error) = project.save_metadata() {
            tracing::warn!(
                "Failed to persist migrated graph layout metadata: {}",
                error
            );
        }
        tracing::info!(
            "Migrated legacy scene graph layout entries from global config into project metadata"
        );
    }

    if let Some(config_object) = config_json.as_object_mut() {
        config_object.remove("graph_layouts");
        match serde_json::to_string_pretty(&config_json) {
            Ok(serialized) => {
                if let Err(error) = std::fs::write(&config_path, serialized) {
                    tracing::warn!(
                        "Failed to remove legacy graph layouts from config file: {}",
                        error
                    );
                }
            }
            Err(error) => {
                tracing::warn!(
                    "Failed to serialize config after removing legacy graph layouts: {}",
                    error
                );
            }
        }
    }
}

pub fn persist_if_dirty(
    ui_state: &mut EditorUI,
    project: Option<&mut Project>,
    egui_ctx: &egui::Context,
) {
    if !ui_state.is_graph_layout_dirty() {
        return;
    }
    if egui_ctx.input(|input| input.pointer.any_down()) {
        return;
    }

    let Some(project) = project else {
        return;
    };

    copy_ui_into_project(ui_state, project);
    match project.save_metadata() {
        Ok(()) => ui_state.clear_graph_layout_dirty(),
        Err(error) => tracing::warn!(
            "Failed to persist scene graph layout to project metadata: {}",
            error
        ),
    }
}

pub fn copy_ui_into_project(ui_state: &EditorUI, project: &mut Project) {
    project.metadata.editor.graph_layouts = ui_state.export_graph_layouts_for_project();
    project.metadata.editor.rule_graph_drafts = ui_state.export_rule_graph_drafts_for_project();
}

pub(crate) fn parse_legacy_graph_layout_key(key: &str) -> Option<(String, String, String)> {
    let mut parts = key.rsplitn(3, "::");
    let node_key = parts.next()?.to_string();
    let scene_name = parts.next()?.to_string();
    let project_key = parts.next()?.to_string();
    Some((project_key, scene_name, node_key))
}
