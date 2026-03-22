//! Scene-related commands (delete scene, path resolution).

use crate::project::Project;
use crate::ui::editor_ui::{EditorUI, Selection};

/// Builds an undoable command to delete a scene from the project.
pub fn build_delete_scene_command(
    ui_state: &EditorUI,
    project: &Project,
    scene_name: &str,
) -> Result<crate::ui::undo_redo::EditorCommand, String> {
    let Some(scene_index) = ui_state
        .scenes
        .iter()
        .position(|scene| scene.name == scene_name)
    else {
        return Err(format!("scene '{scene_name}' not found"));
    };

    let removed_scene = crate::ui::undo_redo::SceneSnapshot {
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
        let scene_before_contents =
            std::fs::read_to_string(&scene_absolute_path).map_err(|error| {
                format!(
                    "failed to read scene file '{}': {error}",
                    scene_absolute_path.display()
                )
            })?;
        changes.push(crate::ui::undo_redo::ProjectFileChange {
            relative_path: scene_relative_path,
            before_contents: Some(scene_before_contents),
            after_contents: None,
        });
    }
    changes.push(crate::ui::undo_redo::ProjectFileChange {
        relative_path: std::path::PathBuf::from("project.toml"),
        before_contents: Some(project_file_before),
        after_contents: Some(project_file_after),
    });

    Ok(crate::ui::undo_redo::EditorCommand::delete_scene(
        crate::ui::undo_redo::DeleteSceneCommandData {
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
        .map_err(|error| {
            format!(
                "failed to read scenes directory '{}': {error}",
                scenes_dir.display()
            )
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
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
