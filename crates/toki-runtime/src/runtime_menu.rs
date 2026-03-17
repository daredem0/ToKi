use toki_core::menu::{
    build_menu_layout, menu_border_color, menu_hex_color_rgba, tinted_menu_background, MenuCommand,
    MenuInput,
};
use toki_core::text::{TextAnchor, TextItem, TextStyle, TextWeight};

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
        let accent = menu_hex_color_rgba(&self.menu_system.settings().appearance.color_hex)
            .unwrap_or([0.49, 1.0, 0.49, 1.0]);
        let appearance = self.menu_system.settings().appearance.clone();
        let layout = build_menu_layout(&view, &appearance, viewport);
        let panel_rect = layout.panel;
        self.rendering.add_filled_ui_rect(
            panel_rect.x,
            panel_rect.y,
            panel_rect.width,
            panel_rect.height,
            tinted_menu_background(accent, 0.16, 0.88),
        );
        if let Some(border_color) = menu_border_color(appearance.border_style, accent, 0.95) {
            self.rendering.add_ui_rect(
                panel_rect.x,
                panel_rect.y,
                panel_rect.width,
                panel_rect.height,
                border_color,
            );
        }

        let title_style = TextStyle {
            font_family: appearance.font_family.clone(),
            size_px: appearance.font_size_px as f32 + 4.0,
            weight: TextWeight::Bold,
            color: [1.0, 1.0, 1.0, 1.0],
            ..TextStyle::default()
        };
        let entry_style = TextStyle {
            font_family: appearance.font_family.clone(),
            size_px: appearance.font_size_px as f32,
            weight: TextWeight::Normal,
            color: [1.0, 1.0, 1.0, 1.0],
            ..TextStyle::default()
        };
        let selected_style = TextStyle {
            color: accent,
            weight: TextWeight::Bold,
            ..entry_style.clone()
        };
        self.render_menu_layout_rect(
            &layout.title.rect,
            tinted_menu_background(accent, 0.16, 0.9),
            menu_border_color(appearance.border_style, accent, 0.95),
        );

        self.rendering.add_text_item(
            TextItem::new_screen(
                layout.title.text,
                glam::Vec2::new(layout.title.rect.center_x(), layout.title.rect.y + 10.0),
                title_style,
            )
            .with_anchor(TextAnchor::TopCenter)
            .with_layer(10),
        );

        for entry in &layout.entries {
            self.render_menu_layout_rect(
                &entry.rect,
                if entry.selected {
                    tinted_menu_background(accent, 0.22, 0.88)
                } else if entry.selectable {
                    tinted_menu_background(accent, 0.08, 0.72)
                } else {
                    [0.0, 0.0, 0.0, 0.45]
                },
                if entry.selectable {
                    menu_border_color(
                        entry.border_style,
                        accent,
                        if entry.selected { 0.95 } else { 0.55 },
                    )
                } else {
                    None
                },
            );

            let style = if entry.selected {
                selected_style.clone()
            } else {
                entry_style.clone()
            };
            self.rendering.add_text_item(
                TextItem::new_screen(
                    if entry.selected {
                        format!("> {}", entry.text)
                    } else {
                        format!("  {}", entry.text)
                    },
                    glam::Vec2::new(entry.rect.center_x(), entry.rect.y + 6.0),
                    style,
                )
                .with_anchor(TextAnchor::TopCenter)
                .with_layer(10),
            );
        }

        self.render_menu_layout_rect(&layout.hint.rect, [0.0, 0.0, 0.0, 0.65], None);
        self.rendering.add_text_item(
            TextItem::new_screen(
                layout.hint.text,
                glam::Vec2::new(layout.hint.rect.center_x(), layout.hint.rect.y + 4.0),
                TextStyle {
                    font_family: appearance.font_family.clone(),
                    size_px: (appearance.font_size_px as f32 - 2.0).max(10.0),
                    color: [0.85, 0.85, 0.85, 1.0],
                    ..TextStyle::default()
                },
            )
            .with_anchor(TextAnchor::BottomCenter)
            .with_layer(10),
        );
    }

    fn apply_menu_command(&mut self, command: MenuCommand) {
        apply_menu_command(&mut self.exit_requested, command);
    }

    fn render_menu_layout_rect(
        &mut self,
        rect: &toki_core::menu::MenuRect,
        fill: [f32; 4],
        border: Option<[f32; 4]>,
    ) {
        self.rendering
            .add_filled_ui_rect(rect.x, rect.y, rect.width, rect.height, fill);
        if let Some(border) = border {
            self.rendering
                .add_ui_rect(rect.x, rect.y, rect.width, rect.height, border);
        }
    }
}

fn apply_menu_command(exit_requested: &mut bool, command: MenuCommand) {
    match command {
        MenuCommand::ExitRuntime => {
            *exit_requested = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toki_core::menu::{MenuAppearance, MenuCommand, MenuView, MenuViewEntry};

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

        apply_menu_command(&mut exit_requested, MenuCommand::ExitRuntime);

        assert!(exit_requested);
    }

    #[test]
    fn runtime_menu_layout_uses_shared_geometry() {
        let layout = build_menu_layout(
            &MenuView {
                screen_id: "pause".to_string(),
                title: "Paused".to_string(),
                entries: vec![
                    MenuViewEntry {
                        text: "Resume".to_string(),
                        selected: true,
                        selectable: true,
                        border_style: toki_core::menu::MenuBorderStyle::Square,
                    },
                    MenuViewEntry {
                        text: "Inventory".to_string(),
                        selected: false,
                        selectable: true,
                        border_style: toki_core::menu::MenuBorderStyle::Square,
                    },
                ],
            },
            &MenuAppearance::default(),
            glam::Vec2::new(320.0, 180.0),
        );

        assert_eq!(layout.panel.width, 280.0);
        assert_eq!(layout.entries.len(), 2);
        assert!(layout.entries[0].rect.width > 200.0);
    }
}
