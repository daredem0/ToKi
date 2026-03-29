use super::editor_ui::{CenterPanelTab, EditorRenderContext, EditorUI};
use crate::config::EditorConfig;
use crate::project::{Project, ProjectAssets};
use crate::scene::SceneViewport;
use crate::ui::inspector::InspectorSystem;
use crate::ui::panels::PanelSystem;
use std::any::Any;
use std::collections::HashMap;

pub(crate) struct EditorContextHost<'a> {
    pub scene_viewport: Option<&'a mut SceneViewport>,
    pub map_editor_viewport: Option<&'a mut SceneViewport>,
    pub project: Option<&'a mut Project>,
    pub project_assets: Option<&'a mut ProjectAssets>,
    pub available_map_names: Option<Vec<String>>,
    pub config: Option<&'a mut EditorConfig>,
    pub log_capture: Option<&'a crate::logging::LogCapture>,
    pub renderer: Option<&'a mut egui_wgpu::Renderer>,
}

impl<'a> EditorContextHost<'a> {
    pub fn from_render_context(render_ctx: EditorRenderContext<'a>) -> Self {
        Self {
            scene_viewport: render_ctx.scene_viewport,
            map_editor_viewport: render_ctx.map_editor_viewport,
            project: render_ctx.project,
            project_assets: render_ctx.project_assets,
            available_map_names: render_ctx.available_map_names,
            config: render_ctx.config,
            log_capture: render_ctx.log_capture,
            renderer: render_ctx.renderer,
        }
    }

}

pub(crate) trait EditorContext: Any {
    fn tab(&self) -> CenterPanelTab;

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    );

    fn render_inspector(
        &mut self,
        _shell: &mut EditorUI,
        _ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        _host: &mut EditorContextHost<'_>,
    ) -> bool {
        false
    }

    fn on_activate(&mut self, _shell: &mut EditorUI) {}

    fn on_deactivate(&mut self, _shell: &mut EditorUI) {}

    fn can_undo(&self, _shell: &EditorUI) -> bool {
        false
    }

    fn can_redo(&self, _shell: &EditorUI) -> bool {
        false
    }

    fn undo(&mut self, _shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        false
    }

    fn redo(&mut self, _shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Default)]
pub(crate) struct SceneViewportContext;
#[derive(Default)]
pub(crate) struct RuleGraphContext;
#[derive(Default)]
pub(crate) struct MapEditorContext;
#[derive(Default)]
pub(crate) struct MenuEditorContext;
#[derive(Default)]
pub(crate) struct DialogEditorContext;
#[derive(Default)]
pub(crate) struct SpriteEditorContext;
#[derive(Default)]
pub(crate) struct AnimationEditorContext;
#[derive(Default)]
pub(crate) struct EntityEditorContext;

struct NullEditorContext;

impl EditorContext for NullEditorContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::SceneViewport
    }

    fn render_center_panel(
        &mut self,
        _shell: &mut EditorUI,
        _ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _host: &mut EditorContextHost<'_>,
    ) {
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for SceneViewportContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::SceneViewport
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        PanelSystem::render_scene_viewport_tab(
            ui,
            shell,
            host.scene_viewport.as_deref_mut(),
            host.config.as_deref_mut(),
            host.renderer.as_deref_mut(),
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for RuleGraphContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::SceneGraph
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        PanelSystem::render_scene_graph(
            ui,
            shell,
            shell.active_tab() == CenterPanelTab::SceneRules,
            host.config.as_deref(),
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for MapEditorContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::MapEditor
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        PanelSystem::render_map_editor(
            ui,
            shell,
            host.map_editor_viewport.as_deref_mut(),
            host.available_map_names.take(),
            host.config.as_deref_mut(),
            host.renderer.as_deref_mut(),
        );
    }

    fn render_inspector(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_map_editor_command_palette(
            shell,
            ui,
            egui_ctx,
            host.config.as_deref(),
        );
        true
    }

    fn can_undo(&self, shell: &EditorUI) -> bool {
        shell.map.history.can_undo()
    }

    fn can_redo(&self, shell: &EditorUI) -> bool {
        shell.map.history.can_redo()
    }

    fn undo(&mut self, shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        let mut history = std::mem::take(&mut shell.map.history);
        let undone = history.undo(shell);
        shell.map.history = history;
        undone
    }

    fn redo(&mut self, shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        let mut history = std::mem::take(&mut shell.map.history);
        let redone = history.redo(shell);
        shell.map.history = history;
        redone
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for MenuEditorContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::MenuEditor
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        crate::ui::panels::menu_editor::render_menu_editor(ui, shell, host.project.as_deref_mut());
    }

    fn render_inspector(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_menu_editor_inspector(shell, ui, host.project.as_deref_mut());
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for DialogEditorContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::DialogEditor
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        crate::ui::panels::dialog_editor::render_dialog_editor(
            ui,
            shell,
            host.project_assets.as_deref_mut(),
            host.project.as_deref_mut(),
        );
    }

    fn render_inspector(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_dialog_editor_inspector(
            shell,
            ui,
            host.project.as_deref_mut(),
            host.project_assets.as_deref_mut(),
        );
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for SpriteEditorContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::SpriteEditor
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        crate::ui::panels::sprite_editor::render_sprite_editor(
            ui,
            shell,
            egui_ctx,
            host.project.as_deref_mut(),
        );
    }

    fn render_inspector(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        _host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_sprite_editor_inspector(shell, ui, egui_ctx);
        true
    }

    fn can_undo(&self, shell: &EditorUI) -> bool {
        shell.sprite.active().history.can_undo()
    }

    fn can_redo(&self, shell: &EditorUI) -> bool {
        shell.sprite.active().history.can_redo()
    }

    fn undo(&mut self, shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        shell.sprite.undo()
    }

    fn redo(&mut self, shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        shell.sprite.redo()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for AnimationEditorContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::AnimationEditor
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        crate::ui::panels::animation_editor::render_animation_editor(
            ui,
            shell,
            egui_ctx,
            host.project.as_deref_mut(),
        );
    }

    fn render_inspector(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        _host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_animation_editor_inspector(shell, ui);
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for EntityEditorContext {
    fn tab(&self) -> CenterPanelTab {
        CenterPanelTab::EntityEditor
    }

    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        crate::ui::panels::entity_editor::render_entity_editor(
            ui,
            shell,
            egui_ctx,
            host.project.as_deref_mut(),
        );
    }

    fn render_inspector(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        _host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_entity_editor_inspector(shell, ui);
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) fn default_active_context(tab: CenterPanelTab) -> Box<dyn EditorContext> {
    match tab {
        CenterPanelTab::SceneViewport => Box::new(SceneViewportContext::default()),
        CenterPanelTab::SceneGraph | CenterPanelTab::SceneRules => Box::new(RuleGraphContext::default()),
        CenterPanelTab::MapEditor => Box::new(MapEditorContext::default()),
        CenterPanelTab::MenuEditor => Box::new(MenuEditorContext),
        CenterPanelTab::DialogEditor => Box::new(DialogEditorContext::default()),
        CenterPanelTab::SpriteEditor => Box::new(SpriteEditorContext::default()),
        CenterPanelTab::AnimationEditor => Box::new(AnimationEditorContext::default()),
        CenterPanelTab::EntityEditor => Box::new(EntityEditorContext::default()),
    }
}

pub(crate) fn default_parked_contexts(
    active_tab: CenterPanelTab,
) -> HashMap<CenterPanelTab, Box<dyn EditorContext>> {
    let mut contexts = HashMap::new();
    for tab in [
        CenterPanelTab::SceneViewport,
        CenterPanelTab::SceneGraph,
        CenterPanelTab::MapEditor,
        CenterPanelTab::MenuEditor,
        CenterPanelTab::DialogEditor,
        CenterPanelTab::SpriteEditor,
        CenterPanelTab::AnimationEditor,
        CenterPanelTab::EntityEditor,
    ] {
        if tab != active_tab {
            contexts.insert(tab, default_active_context(tab));
        }
    }
    if active_tab != CenterPanelTab::SceneRules {
        let scene_rules_context = contexts
            .remove(&CenterPanelTab::SceneGraph)
            .unwrap_or_else(|| default_active_context(CenterPanelTab::SceneGraph));
        contexts.insert(
            CenterPanelTab::SceneRules,
            scene_rules_context,
        );
    }
    contexts
}

pub(crate) fn null_context() -> Box<dyn EditorContext> {
    Box::new(NullEditorContext)
}
