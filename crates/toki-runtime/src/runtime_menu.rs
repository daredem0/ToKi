use toki_core::menu::{
    build_dialog_layout, build_menu_layout, compose_dialog_ui, compose_menu_ui, MenuInput,
};
use toki_core::ui::UiCommand;

use super::App;

impl App {
    pub(super) fn is_menu_open(&self) -> bool {
        self.menu_system.is_open()
    }

    pub(super) fn should_gate_gameplay_for_menu(&self) -> bool {
        self.menu_system.is_open() && self.menu_system.settings().gate_gameplay_when_open
    }

    pub(super) fn open_pause_menu(&mut self) {
        self.menu_system.open_pause_root();
        self.game_system.clear_runtime_inputs();
    }

    pub(super) fn handle_menu_input(&mut self, input: MenuInput) {
        if let Some(command) = self.menu_system.handle_input(input) {
            self.apply_menu_command(command);
        }
    }

    pub(super) fn render_runtime_menu_overlay(&mut self) {
        if !self.menu_system.is_open() {
            return;
        }

        let inventory = self.game_system.player_inventory_entries();
        let Some(view) = self.menu_system.current_view(&inventory) else {
            return;
        };

        let viewport = self
            .platform
            .inner_size()
            .map(|size| glam::Vec2::new(size.width as f32, size.height as f32))
            .unwrap_or_else(|| {
                let size = self.camera_system.viewport_size();
                glam::Vec2::new(size.x as f32, size.y as f32)
            });
        let appearance = self.menu_system.settings().appearance.clone();
        let layout = build_menu_layout(&view, &appearance, viewport);
        let composition = compose_menu_ui(&layout, &appearance);
        self.rendering.render_ui_composition(&composition);

        if let Some(dialog_view) = self.menu_system.current_dialog_view() {
            let dialog_layout = build_dialog_layout(&dialog_view, &appearance, viewport);
            let dialog_composition = compose_dialog_ui(&dialog_layout, &appearance);
            self.rendering.render_ui_composition(&dialog_composition);
        }
    }

    fn apply_menu_command(&mut self, command: UiCommand) {
        apply_menu_command(
            &mut self.exit_requested,
            &mut self.pending_ui_events,
            command,
        );
    }
}

fn apply_menu_command(
    exit_requested: &mut bool,
    pending_ui_events: &mut Vec<String>,
    command: UiCommand,
) {
    match command {
        UiCommand::ExitRuntime => {
            *exit_requested = true;
        }
        UiCommand::EmitEvent { event_id } => pending_ui_events.push(event_id),
    }
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

        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            UiCommand::ExitRuntime,
        );

        assert!(exit_requested);
        assert!(pending_ui_events.is_empty());
    }

    #[test]
    fn emit_event_menu_command_is_queued_for_runtime_consumers() {
        let mut exit_requested = false;
        let mut pending_ui_events = Vec::new();

        apply_menu_command(
            &mut exit_requested,
            &mut pending_ui_events,
            UiCommand::EmitEvent {
                event_id: "start_game".to_string(),
            },
        );

        assert!(!exit_requested);
        assert_eq!(pending_ui_events, vec!["start_game".to_string()]);
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
}
