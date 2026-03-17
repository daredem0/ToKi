use toki_core::menu::{MenuBorderStyle, MenuCommand, MenuInput};
use toki_core::text::{TextAnchor, TextBoxStyle, TextItem, TextStyle, TextWeight};

use super::App;

const MENU_TITLE_Y: f32 = 22.0;
const MENU_ENTRIES_START_Y: f32 = 52.0;
const MENU_ENTRY_SPACING_Y: f32 = 20.0;
const MENU_HINT_Y_PADDING: f32 = 18.0;

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
        let center_x = viewport.x * 0.5;

        let accent = menu_hex_color_rgba(&self.menu_system.settings().appearance.color_hex)
            .unwrap_or([0.49, 1.0, 0.49, 1.0]);
        let appearance = &self.menu_system.settings().appearance;
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
        let title_box = TextBoxStyle {
            padding: glam::Vec2::new(14.0, 10.0),
            background_color: tinted_menu_background(accent, 0.16, 0.9),
            border_color: menu_border_color(appearance.border_style, accent, 0.95),
        };
        let entry_box = TextBoxStyle {
            padding: glam::Vec2::new(10.0, 6.0),
            background_color: tinted_menu_background(accent, 0.08, 0.72),
            border_color: menu_border_color(appearance.border_style, accent, 0.55),
        };
        let selected_box = TextBoxStyle {
            background_color: tinted_menu_background(accent, 0.22, 0.88),
            border_color: menu_border_color(appearance.border_style, accent, 0.95),
            ..entry_box.clone()
        };

        self.rendering.add_text_item(
            TextItem::new_screen(
                view.title,
                glam::Vec2::new(center_x, MENU_TITLE_Y),
                title_style,
            )
            .with_anchor(TextAnchor::TopCenter)
            .with_layer(10)
            .with_box_style(title_box),
        );

        for (index, entry) in view.entries.iter().enumerate() {
            let prefix = if entry.selected { "> " } else { "  " };
            let style = if entry.selected {
                selected_style.clone()
            } else {
                entry_style.clone()
            };
            let box_style = if entry.selectable {
                if entry.selected {
                    selected_box.clone()
                } else {
                    entry_box.clone()
                }
            } else {
                TextBoxStyle {
                    background_color: [0.0, 0.0, 0.0, 0.45],
                    border_color: menu_border_color(appearance.border_style, accent, 0.0),
                    ..entry_box.clone()
                }
            };
            self.rendering.add_text_item(
                TextItem::new_screen(
                    format!("{prefix}{}", entry.text),
                    glam::Vec2::new(
                        center_x,
                        MENU_ENTRIES_START_Y + index as f32 * MENU_ENTRY_SPACING_Y,
                    ),
                    style,
                )
                .with_anchor(TextAnchor::TopCenter)
                .with_layer(10)
                .with_box_style(box_style),
            );
        }

        self.rendering.add_text_item(
            TextItem::new_screen(
                "Esc: Back   Enter/Space: Select",
                glam::Vec2::new(center_x, viewport.y - MENU_HINT_Y_PADDING),
                TextStyle {
                    font_family: appearance.font_family.clone(),
                    size_px: (appearance.font_size_px as f32 - 2.0).max(10.0),
                    color: [0.85, 0.85, 0.85, 1.0],
                    ..TextStyle::default()
                },
            )
            .with_anchor(TextAnchor::BottomCenter)
            .with_layer(10)
            .with_box_style(TextBoxStyle {
                padding: glam::Vec2::new(8.0, 4.0),
                background_color: [0.0, 0.0, 0.0, 0.65],
                border_color: menu_border_color(appearance.border_style, accent, 0.0),
            }),
        );
    }

    fn apply_menu_command(&mut self, command: MenuCommand) {
        apply_menu_command(&mut self.exit_requested, command);
    }
}

fn apply_menu_command(exit_requested: &mut bool, command: MenuCommand) {
    match command {
        MenuCommand::ExitRuntime => {
            *exit_requested = true;
        }
    }
}

fn menu_hex_color_rgba(hex: &str) -> Option<[f32; 4]> {
    let trimmed = hex.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let green = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some([
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        1.0,
    ])
}

fn tinted_menu_background(accent: [f32; 4], shade: f32, alpha: f32) -> [f32; 4] {
    [
        accent[0] * shade,
        accent[1] * shade,
        accent[2] * shade,
        alpha,
    ]
}

fn menu_border_color(
    border_style: MenuBorderStyle,
    accent: [f32; 4],
    alpha: f32,
) -> Option<[f32; 4]> {
    match border_style {
        MenuBorderStyle::Square if alpha > 0.0 => Some([accent[0], accent[1], accent[2], alpha]),
        MenuBorderStyle::Square => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toki_core::menu::MenuCommand;

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
}
