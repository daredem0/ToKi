use toki_core::game::RuleSystem;
use toki_core::menu::{
    build_dialog_layout, build_menu_layout, compose_dialog_ui, compose_menu_ui, MenuAppearance,
    MenuDialogLayout, MenuInput, MenuLayout,
};
use toki_core::ui::UiCommand;
use toki_core::DialogStartRequest;

use super::App;

impl App {
    pub(super) fn runtime_ui_scale_factor(&self) -> f32 {
        if self.rendering.has_gpu() {
            self.rendering
                .viewport_presentation()
                .runtime_ui_scale_factor()
        } else {
            1.0
        }
    }

    pub(super) fn scaled_runtime_menu_appearance(
        &self,
        appearance: &MenuAppearance,
    ) -> MenuAppearance {
        let scale = self.runtime_ui_scale_factor();
        let mut scaled = appearance.clone();
        scaled.font_size_px = ((scaled.font_size_px as f32 * scale).round().max(3.0)) as u16;
        scaled.title_spacing_px =
            ((scaled.title_spacing_px as f32 * scale).round().max(2.0)) as u16;
        scaled.button_spacing_px =
            ((scaled.button_spacing_px as f32 * scale).round().max(2.0)) as u16;
        scaled.footer_spacing_px =
            ((scaled.footer_spacing_px as f32 * scale).round().max(2.0)) as u16;
        scaled.border_thickness_px =
            ((scaled.border_thickness_px as f32 * scale).round().max(1.0)) as u16;
        scaled.dialog_speaker_text.font_size_px = ((scaled.dialog_speaker_text.font_size_px as f32
            * scale)
            .round()
            .max(3.0)) as u16;
        scaled.dialog_body_text.font_size_px = ((scaled.dialog_body_text.font_size_px as f32
            * scale)
            .round()
            .max(3.0)) as u16;
        scaled
    }

    fn runtime_menu_viewport(&self) -> glam::Vec2 {
        if self.rendering.has_gpu() {
            self.rendering.viewport_surface_size()
        } else {
            let size = self.camera_system.viewport_size();
            glam::Vec2::new(size.x as f32, size.y as f32)
        }
    }

    fn runtime_menu_position(&self, position: glam::Vec2) -> Option<glam::Vec2> {
        if self.rendering.has_gpu() {
            self.rendering.surface_to_viewport_position(position)
        } else {
            Some(position)
        }
    }

    fn close_runtime_menu(&mut self) {
        self.menu_system.close();
        self.runtime_overlay = None;
        self.game_system.clear_runtime_inputs();
    }

    pub(super) fn is_dialog_open(&self) -> bool {
        self.dialog_system.is_open()
    }

    pub(super) fn is_menu_open(&self) -> bool {
        self.menu_system.is_open()
    }

    pub(super) fn should_gate_gameplay_for_menu(&self) -> bool {
        should_gate_gameplay(
            self.dialog_system.is_open(),
            self.dialog_system.active_dialog_gates_gameplay(),
            self.menu_system.is_open(),
            self.menu_system.settings().gate_gameplay_when_open,
        )
    }

    pub(super) fn open_pause_menu(&mut self) {
        self.menu_system.open_pause_root();
        self.runtime_overlay = None;
        self.game_system.clear_runtime_inputs();
    }

    pub(super) fn handle_menu_input(&mut self, input: MenuInput) {
        if self.handle_dialog_input(input) {
            return;
        }
        if self.handle_runtime_overlay_input(input) {
            return;
        }
        if let Some(command) = self.menu_system.handle_input(input) {
            self.apply_menu_command(command);
        }
    }

    pub(super) fn handle_menu_pointer_click(&mut self, position: glam::Vec2) -> bool {
        let viewport = self.runtime_menu_viewport();
        let Some(position) = self.runtime_menu_position(position) else {
            return false;
        };

        if self.runtime_overlay.is_some() {
            return self.handle_runtime_overlay_pointer_click(position, viewport);
        }

        if self.dialog_system.is_open() {
            let Some(dialog_view) = self.dialog_system.current_view() else {
                return false;
            };
            let appearance = self
                .scaled_runtime_menu_appearance(narrative_dialog_appearance(&self.launch_options));
            let layout = build_dialog_layout(&dialog_view, &appearance, viewport);
            if let Some(entry_index) = dialog_entry_at_position(&layout, position) {
                let result = self
                    .dialog_system
                    .activate_entry(entry_index, &self.game_system.game_state);
                self.apply_dialog_advance_result(result);
                return true;
            }
            return false;
        }

        if self.runtime_overlay.is_some() || !self.menu_system.is_open() {
            return false;
        }

        if let Some(dialog_view) = self.menu_system.current_dialog_view() {
            let appearance =
                self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);
            let layout = build_dialog_layout(&dialog_view, &appearance, viewport);
            if let Some(entry_index) = dialog_entry_at_position(&layout, position) {
                if let Some(command) = self.menu_system.activate_dialog_entry(entry_index) {
                    self.apply_menu_command(command);
                }
                return true;
            }
            return false;
        }

        let inventory = self.game_system.game_state.player_inventory_entries();
        let Some(menu_view) = self.menu_system.current_view(&inventory) else {
            return false;
        };
        let appearance =
            self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);
        let layout = build_menu_layout(&menu_view, &appearance, viewport);
        if let Some(entry_index) = menu_entry_at_position(&layout, position) {
            if self
                .menu_system
                .select_screen_view_entry(&inventory, entry_index)
            {
                if let Some(command) = self.menu_system.handle_input(MenuInput::Confirm) {
                    self.apply_menu_command(command);
                }
                return true;
            }
        }
        false
    }

    pub(super) fn handle_menu_pointer_hover(&mut self, position: glam::Vec2) -> bool {
        let viewport = self.runtime_menu_viewport();
        let Some(position) = self.runtime_menu_position(position) else {
            return false;
        };

        if self.runtime_overlay.is_some() {
            return self.handle_runtime_overlay_pointer_hover(position, viewport);
        }

        if self.dialog_system.is_open() {
            let Some(dialog_view) = self.dialog_system.current_view() else {
                return false;
            };
            let appearance = self
                .scaled_runtime_menu_appearance(narrative_dialog_appearance(&self.launch_options));
            let layout = build_dialog_layout(&dialog_view, &appearance, viewport);
            if let Some(entry_index) = dialog_entry_at_position(&layout, position) {
                self.dialog_system.select_entry(entry_index);
                return true;
            }
            return false;
        }

        if self.runtime_overlay.is_some() || !self.menu_system.is_open() {
            return false;
        }

        if let Some(dialog_view) = self.menu_system.current_dialog_view() {
            let appearance =
                self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);
            let layout = build_dialog_layout(&dialog_view, &appearance, viewport);
            if let Some(entry_index) = dialog_entry_at_position(&layout, position) {
                self.menu_system.select_dialog_entry(entry_index);
                return true;
            }
            return false;
        }

        let inventory = self.game_system.game_state.player_inventory_entries();
        let Some(menu_view) = self.menu_system.current_view(&inventory) else {
            return false;
        };
        let appearance =
            self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);
        let layout = build_menu_layout(&menu_view, &appearance, viewport);
        menu_entry_at_position(&layout, position).is_some_and(|entry_index| {
            self.menu_system
                .select_screen_view_entry(&inventory, entry_index)
        })
    }

    pub(super) fn handle_menu_pointer_drag(&mut self, position: glam::Vec2) -> bool {
        let viewport = self.runtime_menu_viewport();
        let Some(position) = self.runtime_menu_position(position) else {
            return false;
        };
        self.handle_runtime_overlay_pointer_drag(position, viewport)
    }

    pub(super) fn clear_menu_pointer_drag(&mut self) {
        self.clear_runtime_overlay_pointer_drag();
    }

    pub(super) fn apply_dialog_start_request(&mut self, request: DialogStartRequest) {
        if let Err(error) = self.dialog_system.start_dialog(
            &self.game_system.game_state,
            &request.dialog_id,
            request.context,
        ) {
            tracing::warn!(
                "Failed to start dialog '{}' from rule request: {:?}",
                request.dialog_id,
                error
            );
        } else {
            self.runtime_overlay = None;
            self.game_system.clear_runtime_inputs();
        }
    }

    fn handle_dialog_input(&mut self, input: MenuInput) -> bool {
        if !self.dialog_system.is_open() {
            return false;
        }

        let result = self
            .dialog_system
            .handle_input(input, &self.game_system.game_state);
        self.apply_dialog_advance_result(result);
        true
    }

    fn apply_dialog_advance_result(
        &mut self,
        result: toki_core::dialog_runtime::DialogAdvanceResult,
    ) {
        if let toki_core::dialog_runtime::DialogAdvanceResult::Closed(completion) = result {
            if let Some(outcome_id) = completion.outcome_id.as_deref() {
                RuleSystem::record_dialog_completion(
                    &mut self.game_system.game_state,
                    completion.dialog_id.clone(),
                    outcome_id,
                );
            }
        }
    }

    pub(super) fn render_runtime_menu_overlay(&mut self) {
        if self.dialog_system.is_open() {
            let Some(dialog_view) = self.dialog_system.current_view() else {
                return;
            };
            let viewport = self.runtime_menu_viewport();
            let appearance = self
                .scaled_runtime_menu_appearance(narrative_dialog_appearance(&self.launch_options));
            let dialog_layout = build_dialog_layout(&dialog_view, &appearance, viewport);
            let dialog_composition = compose_dialog_ui(&dialog_layout, &appearance);
            self.rendering
                .render_viewport_ui_composition(&dialog_composition);
            return;
        }

        if !self.menu_system.is_open() {
            return;
        }

        let inventory = self.game_system.game_state.player_inventory_entries();
        let Some(view) = self.menu_system.current_view(&inventory) else {
            return;
        };

        let viewport = self.runtime_menu_viewport();
        let appearance =
            self.scaled_runtime_menu_appearance(&self.menu_system.settings().appearance);

        if self.render_runtime_settings_overlay(&appearance, viewport) {
            return;
        }

        let dialog_view = self.menu_system.current_dialog_view();
        let should_hide_main_menu = dialog_view.as_ref().is_some_and(|d| d.hide_main_menu);

        if !should_hide_main_menu {
            let layout = build_menu_layout(&view, &appearance, viewport);
            let composition = compose_menu_ui(&layout, &appearance);
            self.rendering.render_viewport_ui_composition(&composition);
        }

        if let Some(dialog_view) = dialog_view {
            let dialog_layout = build_dialog_layout(&dialog_view, &appearance, viewport);
            let dialog_composition = compose_dialog_ui(&dialog_layout, &appearance);
            self.rendering
                .render_viewport_ui_composition(&dialog_composition);
        }
    }

    fn apply_menu_command(&mut self, command: UiCommand) {
        match command {
            UiCommand::SaveGame { slot } => {
                tracing::info!("Runtime menu requested save to slot {}", slot);
                if let Err(error) = self.save_to_slot(slot) {
                    tracing::error!("Failed to save slot {} from runtime menu: {}", slot, error);
                } else {
                    self.close_runtime_menu();
                }
            }
            UiCommand::LoadGame { slot } => {
                tracing::info!("Runtime menu requested load from slot {}", slot);
                if let Err(error) = self.load_from_slot(slot) {
                    tracing::error!("Failed to load slot {} from runtime menu: {}", slot, error);
                } else {
                    self.close_runtime_menu();
                }
            }
            other => apply_menu_command(
                &mut self.exit_requested,
                &mut self.pending_ui_events,
                &mut self.runtime_overlay,
                other,
            ),
        }
    }
}

fn apply_menu_command(
    exit_requested: &mut bool,
    pending_ui_events: &mut Vec<String>,
    runtime_overlay: &mut Option<super::RuntimeMenuOverlay>,
    command: UiCommand,
) {
    match command {
        UiCommand::ExitRuntime => {
            *exit_requested = true;
        }
        UiCommand::OpenAudioSettings => {
            *runtime_overlay = Some(super::RuntimeMenuOverlay::audio());
        }
        UiCommand::OpenDisplaySettings => {
            *runtime_overlay = Some(super::RuntimeMenuOverlay::display());
        }
        UiCommand::OpenGraphicsSettings => {
            *runtime_overlay = Some(super::RuntimeMenuOverlay::graphics());
        }
        UiCommand::SaveGame { .. } | UiCommand::LoadGame { .. } => {}
        UiCommand::EmitEvent { event_id } => pending_ui_events.push(event_id),
    }
}

fn narrative_dialog_appearance(
    launch_options: &super::RuntimeLaunchOptions,
) -> &toki_core::menu::MenuAppearance {
    &launch_options.dialog_appearance
}

fn should_gate_gameplay(
    dialog_open: bool,
    dialog_gate_gameplay: bool,
    menu_open: bool,
    menu_gate_gameplay: bool,
) -> bool {
    (dialog_open && dialog_gate_gameplay) || (menu_open && menu_gate_gameplay)
}

fn dialog_entry_at_position(layout: &MenuDialogLayout, position: glam::Vec2) -> Option<usize> {
    layout.entries.iter().position(|entry| {
        position.x >= entry.rect.x
            && position.x <= entry.rect.x + entry.rect.width
            && position.y >= entry.rect.y
            && position.y <= entry.rect.y + entry.rect.height
    })
}

fn menu_entry_at_position(layout: &MenuLayout, position: glam::Vec2) -> Option<usize> {
    layout.entries.iter().position(|entry| {
        position.x >= entry.rect.x
            && position.x <= entry.rect.x + entry.rect.width
            && position.y >= entry.rect.y
            && position.y <= entry.rect.y + entry.rect.height
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use toki_core::menu::{
        menu_fill_color_rgba, menu_hex_color_rgba, MenuAppearance, MenuView, MenuViewEntry,
    };
    use toki_core::ui::UiCommand;

    #[test]
    fn menu_hex_color_rgba_parses_valid_hex_triplet() {
        assert_eq!(
            menu_hex_color_rgba("#7CFF7C"),
            Some([124.0 / 255.0, 1.0, 124.0 / 255.0, 1.0])
        );
    }

    #[test]
    fn menu_hex_color_rgba_rejects_invalid_hex() {
        assert!(menu_hex_color_rgba("#12").is_none());
        assert!(menu_hex_color_rgba("#GGFF7C").is_none());
    }

    #[test]
    fn exit_runtime_menu_command_sets_exit_requested_flag() {
        let mut exit_requested = false;
        let mut pending_ui_events = Vec::new();
        let mut runtime_overlay = None;

        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            &mut runtime_overlay,
            UiCommand::ExitRuntime,
        );

        assert!(exit_requested);
        assert!(pending_ui_events.is_empty());
    }

    #[test]
    fn emit_event_menu_command_is_queued_for_runtime_consumers() {
        let mut exit_requested = false;
        let mut pending_ui_events = Vec::new();
        let mut runtime_overlay = None;

        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            &mut runtime_overlay,
            UiCommand::EmitEvent {
                event_id: "start_game".to_string(),
            },
        );

        assert!(!exit_requested);
        assert_eq!(pending_ui_events, vec!["start_game".to_string()]);
    }

    #[test]
    fn open_graphics_settings_menu_command_activates_overlay() {
        let mut exit_requested = false;
        let mut pending_ui_events = Vec::new();
        let mut runtime_overlay = None;

        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            &mut runtime_overlay,
            UiCommand::OpenGraphicsSettings,
        );

        assert_eq!(
            runtime_overlay,
            Some(super::super::RuntimeMenuOverlay::graphics())
        );
    }

    #[test]
    fn open_display_settings_menu_command_activates_overlay() {
        let mut exit_requested = false;
        let mut pending_ui_events = Vec::new();
        let mut runtime_overlay = None;

        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            &mut runtime_overlay,
            UiCommand::OpenDisplaySettings,
        );

        assert_eq!(
            runtime_overlay,
            Some(super::super::RuntimeMenuOverlay::display())
        );
    }

    #[test]
    fn save_and_load_menu_commands_are_ignored_by_the_pure_overlay_helper() {
        let mut exit_requested = false;
        let mut pending_ui_events = Vec::new();
        let mut runtime_overlay = None;

        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            &mut runtime_overlay,
            UiCommand::SaveGame { slot: 1 },
        );
        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            &mut runtime_overlay,
            UiCommand::LoadGame { slot: 2 },
        );

        assert!(!exit_requested);
        assert!(pending_ui_events.is_empty());
        assert_eq!(runtime_overlay, None);
    }

    #[test]
    fn runtime_menu_layout_uses_shared_geometry() {
        let layout = build_menu_layout(
            &MenuView {
                screen_id: "pause".to_string(),
                title: "Paused".to_string(),
                title_border_style_override: None,
                entries: vec![
                    MenuViewEntry {
                        text: "Resume".to_string(),
                        selected: true,
                        selectable: true,
                        border_style_override: None,
                    },
                    MenuViewEntry {
                        text: "Inventory".to_string(),
                        selected: false,
                        selectable: true,
                        border_style_override: None,
                    },
                ],
            },
            &MenuAppearance::default(),
            glam::Vec2::new(320.0, 180.0),
        );

        assert_eq!(layout.panel.width, 281.6);
        assert_eq!(layout.entries.len(), 2);
        assert!(layout.entries[0].rect.width > 200.0);
    }

    #[test]
    fn menu_fill_color_rgba_supports_transparent_backgrounds() {
        assert_eq!(
            menu_fill_color_rgba("#112233", true, 100),
            Some([17.0 / 255.0, 34.0 / 255.0, 51.0 / 255.0, 0.0])
        );
        assert_eq!(
            menu_fill_color_rgba("#112233", false, 100),
            Some([17.0 / 255.0, 34.0 / 255.0, 51.0 / 255.0, 1.0])
        );
        assert_eq!(
            menu_fill_color_rgba("#112233", false, 50),
            Some([17.0 / 255.0, 34.0 / 255.0, 51.0 / 255.0, 0.5])
        );
    }

    #[test]
    fn narrative_dialogs_use_dedicated_dialog_appearance() {
        let mut launch_options = super::super::RuntimeLaunchOptions::default();
        launch_options.menu.appearance.border_color_hex = "#AABBCC".to_string();
        launch_options.dialog_appearance.border_color_hex = "#112233".to_string();

        assert_eq!(
            narrative_dialog_appearance(&launch_options).border_color_hex,
            "#112233"
        );
    }

    #[test]
    fn dialog_entry_hit_testing_uses_rendered_entry_rects() {
        let layout = MenuDialogLayout {
            panel: toki_core::ui::UiRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            title: toki_core::menu::MenuLayoutBlock {
                rect: toki_core::ui::UiRect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 10.0,
                },
                text: String::new(),
                border_style: toki_core::menu::MenuBorderStyle::Square,
            },
            body: toki_core::menu::MenuLayoutBlock {
                rect: toki_core::ui::UiRect {
                    x: 0.0,
                    y: 10.0,
                    width: 100.0,
                    height: 10.0,
                },
                text: String::new(),
                border_style: toki_core::menu::MenuBorderStyle::Square,
            },
            entries: vec![toki_core::menu::MenuEntryLayout {
                rect: toki_core::ui::UiRect {
                    x: 10.0,
                    y: 30.0,
                    width: 80.0,
                    height: 16.0,
                },
                text: "Ok".to_string(),
                selected: true,
                selectable: true,
                border_style: toki_core::menu::MenuBorderStyle::Square,
            }],
        };

        assert_eq!(
            dialog_entry_at_position(&layout, glam::Vec2::new(20.0, 35.0)),
            Some(0)
        );
        assert_eq!(
            dialog_entry_at_position(&layout, glam::Vec2::new(5.0, 5.0)),
            None
        );
    }

    #[test]
    fn menu_entry_hit_testing_uses_rendered_entry_rects() {
        let layout = MenuLayout {
            panel: toki_core::ui::UiRect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            title: toki_core::menu::MenuLayoutBlock {
                rect: toki_core::ui::UiRect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 10.0,
                },
                text: String::new(),
                border_style: toki_core::menu::MenuBorderStyle::Square,
            },
            entries: vec![toki_core::menu::MenuEntryLayout {
                rect: toki_core::ui::UiRect {
                    x: 10.0,
                    y: 30.0,
                    width: 80.0,
                    height: 16.0,
                },
                text: "Resume".to_string(),
                selected: true,
                selectable: true,
                border_style: toki_core::menu::MenuBorderStyle::Square,
            }],
            hint: toki_core::menu::MenuLayoutBlock {
                rect: toki_core::ui::UiRect {
                    x: 0.0,
                    y: 90.0,
                    width: 100.0,
                    height: 10.0,
                },
                text: String::new(),
                border_style: toki_core::menu::MenuBorderStyle::None,
            },
        };

        assert_eq!(
            menu_entry_at_position(&layout, glam::Vec2::new(20.0, 35.0)),
            Some(0)
        );
        assert_eq!(
            menu_entry_at_position(&layout, glam::Vec2::new(5.0, 5.0)),
            None
        );
    }

    #[test]
    fn gameplay_gate_requires_active_system_and_enabled_flag() {
        assert!(!should_gate_gameplay(true, false, false, false));
        assert!(should_gate_gameplay(true, true, false, false));
        assert!(!should_gate_gameplay(false, false, true, false));
        assert!(should_gate_gameplay(false, false, true, true));
    }
}
