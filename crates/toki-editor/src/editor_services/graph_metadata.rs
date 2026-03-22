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
