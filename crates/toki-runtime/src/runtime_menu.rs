use toki_core::menu::{
    apply_menu_opacity, build_menu_layout, menu_border_color, menu_fill_color_rgba,
    menu_hex_color_rgba, MenuCommand, MenuInput,
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
        let border_color =
            menu_hex_color_rgba(&self.menu_system.settings().appearance.border_color_hex)
                .unwrap_or([0.49, 1.0, 0.49, 1.0]);
        let appearance = self.menu_system.settings().appearance.clone();
        let opacity_alpha = (appearance.opacity_percent.clamp(0, 100) as f32) / 100.0;
        let text_color = apply_menu_opacity(
            menu_hex_color_rgba(&appearance.text_color_hex).unwrap_or([1.0, 1.0, 1.0, 1.0]),
            appearance.opacity_percent,
        );
        let layout = build_menu_layout(&view, &appearance, viewport);
        let panel_rect = layout.panel;
        self.render_menu_layout_rect(
            &panel_rect,
            menu_fill_color_rgba(
                &appearance.menu_background_color_hex,
                appearance.menu_background_transparent,
                appearance.opacity_percent,
            ),
            menu_border_color(appearance.border_style, border_color, opacity_alpha),
        );

        let title_style = TextStyle {
            font_family: appearance.font_family.clone(),
            size_px: appearance.font_size_px as f32 + 4.0,
            weight: TextWeight::Bold,
            color: text_color,
            ..TextStyle::default()
        };
        let entry_style = TextStyle {
            font_family: appearance.font_family.clone(),
            size_px: appearance.font_size_px as f32,
            weight: TextWeight::Normal,
            color: text_color,
            ..TextStyle::default()
        };
        let selected_style = TextStyle {
            color: text_color,
            weight: TextWeight::Bold,
            ..entry_style.clone()
        };
        self.render_menu_layout_rect(
            &layout.title.rect,
            menu_fill_color_rgba(
                &appearance.title_background_color_hex,
                appearance.title_background_transparent,
                appearance.opacity_percent,
            ),
            menu_border_color(layout.title.border_style, border_color, opacity_alpha),
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
                menu_fill_color_rgba(
                    &appearance.entry_background_color_hex,
                    appearance.entry_background_transparent,
                    appearance.opacity_percent,
                ),
                menu_border_color(entry.border_style, border_color, opacity_alpha),
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

        self.render_menu_layout_rect(&layout.hint.rect, None, None);
        self.rendering.add_text_item(
            TextItem::new_screen(
                layout.hint.text,
                glam::Vec2::new(layout.hint.rect.center_x(), layout.hint.rect.y + 4.0),
                TextStyle {
                    font_family: appearance.font_family.clone(),
                    size_px: (appearance.font_size_px as f32 - 2.0).max(10.0),
                    color: text_color,
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
        fill: Option<[f32; 4]>,
        border: Option<[f32; 4]>,
    ) {
        if let Some(fill) = fill {
            self.rendering
                .add_filled_ui_rect(rect.x, rect.y, rect.width, rect.height, fill);
        }
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
