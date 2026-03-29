use crate::project::Project;
use crate::ui::EditorUI;

pub fn load_into_ui(ui_state: &mut EditorUI, project: Option<&Project>) {
    let (graph_layouts, rule_graph_drafts, dialog_graph_layouts) = project
        .map(|project| {
            (
                project.metadata.editor.graph_layouts.clone(),
                project.metadata.editor.rule_graph_drafts.clone(),
                project.metadata.editor.dialog_graph_layouts.clone(),
            )
        })
        .unwrap_or_default();
    crate::ui::editor_ui::load_graph_layouts_from_project(ui_state, &graph_layouts);
    crate::ui::editor_ui::load_rule_graph_drafts_from_project(ui_state, &rule_graph_drafts);
    crate::ui::editor_ui::load_dialog_graph_layouts_from_project(ui_state, &dialog_graph_layouts);
}

pub fn persist_if_dirty(
    ui_state: &mut EditorUI,
    project: Option<&mut Project>,
    egui_ctx: &egui::Context,
) {
    if !crate::ui::editor_ui::is_graph_layout_dirty(ui_state)
        && !crate::ui::editor_ui::is_dialog_graph_layout_dirty(ui_state)
    {
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
        Ok(()) => {
            crate::ui::editor_ui::clear_graph_layout_dirty(ui_state);
            crate::ui::editor_ui::clear_dialog_graph_layout_dirty(ui_state);
        }
        Err(error) => tracing::warn!(
            "Failed to persist graph layout metadata to project metadata: {}",
            error
        ),
    }
}

pub fn copy_ui_into_project(ui_state: &EditorUI, project: &mut Project) {
    project.metadata.editor.graph_layouts =
        crate::ui::editor_ui::export_graph_layouts_for_project(ui_state);
    project.metadata.editor.rule_graph_drafts =
        crate::ui::editor_ui::export_rule_graph_drafts_for_project(ui_state);
    project.metadata.editor.dialog_graph_layouts =
        crate::ui::editor_ui::export_dialog_graph_layouts_for_project(ui_state);
}
