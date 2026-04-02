use super::editor_ui::{
    AnimationEditorState, DialogEditorState, GraphEditorState, MapEditorState, PlacementState,
    SceneToolboxState, UiEditorState, ViewportCursorState,
};
use super::editor_ui::{CenterPanelTab, EditorUI};
use crate::config::EditorConfig;
use crate::project::{Project, ProjectAssets};
use crate::scene::SceneViewport;
use crate::ui::inspector::InspectorSystem;
use crate::ui::panels::PanelSystem;
use crate::ui::{entity_editor::EntityEditorState, sprite_editor::SpriteEditorState};
use std::any::Any;
use std::collections::HashMap;

pub(crate) struct EditorContextHost<'a> {
    pub scene_viewport: Option<&'a mut SceneViewport>,
    pub map_editor_viewport: Option<&'a mut SceneViewport>,
    pub project: Option<&'a mut Project>,
    pub project_assets: Option<&'a mut ProjectAssets>,
    pub available_map_names: Option<Vec<String>>,
    pub config: Option<&'a mut EditorConfig>,
    pub config_readonly: Option<&'a EditorConfig>,
    pub log_capture: Option<&'a crate::logging::LogCapture>,
    pub renderer: Option<&'a mut egui_wgpu::Renderer>,
}

pub(crate) trait EditorContext: Any {
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

    fn render_toolbox(
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

    fn prefers_local_undo_redo(&self) -> bool {
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
pub(crate) struct SceneViewportContext {
    pub placement: PlacementState,
    pub viewport_cursor: ViewportCursorState,
    pub toolbox: SceneToolboxState,
}

#[derive(Default)]
pub(crate) struct RuleGraphContext {
    pub graph: GraphEditorState,
}

#[derive(Default)]
pub(crate) struct MapEditorContext {
    pub map: MapEditorState,
}

#[derive(Default)]
pub(crate) struct MenuEditorContext;

#[derive(Default)]
pub(crate) struct DialogEditorContext {
    pub dialog: DialogEditorState,
}

#[derive(Default)]
pub(crate) struct UiEditorContext {
    pub ui: UiEditorState,
}

#[derive(Default)]
pub(crate) struct SpriteEditorContext {
    pub sprite: SpriteEditorState,
}

#[derive(Default)]
pub(crate) struct AnimationEditorContext {
    pub animation: AnimationEditorState,
}

#[derive(Default)]
pub(crate) struct EntityEditorContext {
    pub entity_editor: EntityEditorState,
}

impl SceneViewportContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::SceneViewport)
            .expect("scene viewport context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        ui.context_mut::<Self>(CenterPanelTab::SceneViewport)
            .expect("scene viewport context should always exist")
    }
}

impl RuleGraphContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::SceneGraph)
            .or_else(|| ui.context::<Self>(CenterPanelTab::SceneRules))
            .expect("rule graph context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        if ui.active_tab() == CenterPanelTab::SceneGraph
            || ui.active_tab() == CenterPanelTab::SceneRules
        {
            ui.context_mut::<Self>(ui.active_tab())
                .expect("rule graph context should always exist")
        } else if ui.context::<Self>(CenterPanelTab::SceneGraph).is_some() {
            ui.context_mut::<Self>(CenterPanelTab::SceneGraph)
                .expect("rule graph context should always exist")
        } else {
            ui.context_mut::<Self>(CenterPanelTab::SceneRules)
                .expect("rule graph context should always exist")
        }
    }
}

impl MapEditorContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::MapEditor)
            .expect("map editor context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        ui.context_mut::<Self>(CenterPanelTab::MapEditor)
            .expect("map editor context should always exist")
    }
}

impl DialogEditorContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::DialogEditor)
            .expect("dialog editor context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        ui.context_mut::<Self>(CenterPanelTab::DialogEditor)
            .expect("dialog editor context should always exist")
    }
}

impl SpriteEditorContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::SpriteEditor)
            .expect("sprite editor context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        ui.context_mut::<Self>(CenterPanelTab::SpriteEditor)
            .expect("sprite editor context should always exist")
    }
}

impl UiEditorContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::UiEditor)
            .expect("ui editor context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        ui.context_mut::<Self>(CenterPanelTab::UiEditor)
            .expect("ui editor context should always exist")
    }
}

impl AnimationEditorContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::AnimationEditor)
            .expect("animation editor context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        ui.context_mut::<Self>(CenterPanelTab::AnimationEditor)
            .expect("animation editor context should always exist")
    }
}

impl EntityEditorContext {
    pub(crate) fn state(ui: &EditorUI) -> &Self {
        ui.context::<Self>(CenterPanelTab::EntityEditor)
            .expect("entity editor context should always exist")
    }

    pub(crate) fn state_mut(ui: &mut EditorUI) -> &mut Self {
        ui.context_mut::<Self>(CenterPanelTab::EntityEditor)
            .expect("entity editor context should always exist")
    }
}

pub(crate) fn scene_viewport_context(ui: &EditorUI) -> &SceneViewportContext {
    SceneViewportContext::state(ui)
}

pub(crate) fn scene_viewport_context_mut(ui: &mut EditorUI) -> &mut SceneViewportContext {
    SceneViewportContext::state_mut(ui)
}

pub(crate) fn graph_state(ui: &EditorUI) -> &GraphEditorState {
    &RuleGraphContext::state(ui).graph
}

pub(crate) fn graph_state_mut(ui: &mut EditorUI) -> &mut GraphEditorState {
    &mut RuleGraphContext::state_mut(ui).graph
}

pub(crate) fn map_state(ui: &EditorUI) -> &MapEditorState {
    &MapEditorContext::state(ui).map
}

pub(crate) fn map_state_mut(ui: &mut EditorUI) -> &mut MapEditorState {
    &mut MapEditorContext::state_mut(ui).map
}

pub(crate) fn dialog_state(ui: &EditorUI) -> &DialogEditorState {
    &DialogEditorContext::state(ui).dialog
}

pub(crate) fn dialog_state_mut(ui: &mut EditorUI) -> &mut DialogEditorState {
    &mut DialogEditorContext::state_mut(ui).dialog
}

pub(crate) fn sprite_state(ui: &EditorUI) -> &SpriteEditorState {
    &SpriteEditorContext::state(ui).sprite
}

pub(crate) fn ui_editor_state(ui: &EditorUI) -> &UiEditorContext {
    UiEditorContext::state(ui)
}

pub(crate) fn ui_editor_state_mut(ui: &mut EditorUI) -> &mut UiEditorContext {
    UiEditorContext::state_mut(ui)
}

pub(crate) fn sprite_state_mut(ui: &mut EditorUI) -> &mut SpriteEditorState {
    &mut SpriteEditorContext::state_mut(ui).sprite
}

pub(crate) fn animation_state(ui: &EditorUI) -> &AnimationEditorState {
    &AnimationEditorContext::state(ui).animation
}

pub(crate) fn animation_state_mut(ui: &mut EditorUI) -> &mut AnimationEditorState {
    &mut AnimationEditorContext::state_mut(ui).animation
}

pub(crate) fn entity_editor_state(ui: &EditorUI) -> &EntityEditorState {
    &EntityEditorContext::state(ui).entity_editor
}

pub(crate) fn entity_editor_state_mut(ui: &mut EditorUI) -> &mut EntityEditorState {
    &mut EntityEditorContext::state_mut(ui).entity_editor
}

struct NullEditorContext;

impl EditorContext for NullEditorContext {
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
    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        let EditorContextHost {
            scene_viewport,
            config,
            renderer,
            ..
        } = host;
        PanelSystem::render_scene_viewport_tab(
            ui,
            shell,
            scene_viewport.as_deref_mut(),
            config.as_deref_mut(),
            renderer.as_deref_mut(),
        );
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render_toolbox(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        host: &mut EditorContextHost<'_>,
    ) -> bool {
        let EditorContextHost {
            project,
            project_assets,
            config,
            config_readonly,
            scene_viewport,
            ..
        } = host;
        let config = config_readonly.as_ref().copied().or(config.as_deref());
        super::inspector::InspectorSystem::render_scene_viewport_toolbox(
            shell,
            ui,
            egui_ctx,
            project.as_deref_mut(),
            project_assets.as_deref_mut(),
            config,
            scene_viewport.as_deref_mut(),
        );
        true
    }
}

impl EditorContext for RuleGraphContext {
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
            host.config_readonly
                .as_ref()
                .copied()
                .or(host.config.as_deref()),
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
    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        let EditorContextHost {
            map_editor_viewport,
            available_map_names,
            config,
            renderer,
            ..
        } = host;
        PanelSystem::render_map_editor(
            ui,
            shell,
            map_editor_viewport.as_deref_mut(),
            available_map_names.take(),
            config.as_deref_mut(),
            renderer.as_deref_mut(),
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
        let config = host
            .config_readonly
            .as_ref()
            .copied()
            .or(host.config.as_deref());
        InspectorSystem::render_map_editor_inspector(shell, ui, egui_ctx, config);
        true
    }

    fn render_toolbox(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        host: &mut EditorContextHost<'_>,
    ) -> bool {
        let config = host
            .config_readonly
            .as_ref()
            .copied()
            .or(host.config.as_deref());
        InspectorSystem::render_map_editor_toolbox(shell, ui, egui_ctx, config);
        true
    }

    fn can_undo(&self, _shell: &EditorUI) -> bool {
        self.map.history.can_undo()
    }

    fn can_redo(&self, _shell: &EditorUI) -> bool {
        self.map.history.can_redo()
    }

    fn prefers_local_undo_redo(&self) -> bool {
        true
    }

    fn undo(&mut self, _shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        let mut history = std::mem::take(&mut self.map.history);
        let undone = history.undo(&mut self.map);
        self.map.history = history;
        undone
    }

    fn redo(&mut self, _shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        let mut history = std::mem::take(&mut self.map.history);
        let redone = history.redo(&mut self.map);
        self.map.history = history;
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

impl EditorContext for UiEditorContext {
    fn render_center_panel(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        host: &mut EditorContextHost<'_>,
    ) {
        crate::ui::panels::ui_editor::render_ui_editor(
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
        InspectorSystem::render_ui_editor_inspector(
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

    fn render_toolbox(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        _host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_sprite_editor_toolbox(shell, ui, egui_ctx);
        true
    }

    fn can_undo(&self, _shell: &EditorUI) -> bool {
        self.sprite.active().history.can_undo()
    }

    fn can_redo(&self, _shell: &EditorUI) -> bool {
        self.sprite.active().history.can_redo()
    }

    fn prefers_local_undo_redo(&self) -> bool {
        true
    }

    fn undo(&mut self, _shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        self.sprite.undo()
    }

    fn redo(&mut self, _shell: &mut EditorUI, _project: Option<&mut Project>) -> bool {
        self.sprite.redo()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl EditorContext for AnimationEditorContext {
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

    fn render_toolbox(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        _host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_animation_editor_toolbox(shell, ui);
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

    fn render_toolbox(
        &mut self,
        shell: &mut EditorUI,
        ui: &mut egui::Ui,
        _egui_ctx: &egui::Context,
        _game_state: Option<&toki_core::GameState>,
        _host: &mut EditorContextHost<'_>,
    ) -> bool {
        InspectorSystem::render_entity_editor_toolbox(shell, ui);
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
        CenterPanelTab::SceneGraph | CenterPanelTab::SceneRules => {
            Box::new(RuleGraphContext::default())
        }
        CenterPanelTab::MapEditor => Box::new(MapEditorContext::default()),
        CenterPanelTab::MenuEditor => Box::new(MenuEditorContext),
        CenterPanelTab::DialogEditor => Box::new(DialogEditorContext::default()),
        CenterPanelTab::UiEditor => Box::new(UiEditorContext::default()),
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
        CenterPanelTab::UiEditor,
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
        contexts.insert(CenterPanelTab::SceneRules, scene_rules_context);
    }
    contexts
}

pub(crate) fn null_context() -> Box<dyn EditorContext> {
    Box::new(NullEditorContext)
}
