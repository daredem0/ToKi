use toki_core::menu::MenuInput;
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
        self.menu_system.handle_input(input);
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

        let title_style = TextStyle {
            font_family: "Sans".to_string(),
            size_px: 18.0,
            weight: TextWeight::Bold,
            color: [1.0, 1.0, 1.0, 1.0],
            ..TextStyle::default()
        };
        let entry_style = TextStyle {
            font_family: "Sans".to_string(),
            size_px: 14.0,
            weight: TextWeight::Normal,
            color: [1.0, 1.0, 1.0, 1.0],
            ..TextStyle::default()
        };
        let selected_style = TextStyle {
            color: [0.78, 1.0, 0.78, 1.0],
            weight: TextWeight::Bold,
            ..entry_style.clone()
        };
        let title_box = TextBoxStyle {
            padding: glam::Vec2::new(14.0, 10.0),
            background_color: [0.03, 0.08, 0.03, 0.9],
            border_color: Some([0.6, 0.9, 0.6, 0.95]),
        };
        let entry_box = TextBoxStyle {
            padding: glam::Vec2::new(10.0, 6.0),
            background_color: [0.0, 0.0, 0.0, 0.72],
            border_color: Some([0.5, 0.5, 0.5, 0.55]),
        };
        let selected_box = TextBoxStyle {
            background_color: [0.1, 0.22, 0.1, 0.88],
            border_color: Some([0.7, 1.0, 0.7, 0.95]),
            ..entry_box.clone()
        };

        self.rendering.add_text_item(
            TextItem::new_screen(view.title, glam::Vec2::new(center_x, MENU_TITLE_Y), title_style)
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
                    border_color: None,
                    ..entry_box.clone()
                }
            };
            self.rendering.add_text_item(
                TextItem::new_screen(
                    format!("{prefix}{}", entry.text),
                    glam::Vec2::new(center_x, MENU_ENTRIES_START_Y + index as f32 * MENU_ENTRY_SPACING_Y),
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
                    size_px: 12.0,
                    color: [0.85, 0.85, 0.85, 1.0],
                    ..TextStyle::default()
                },
            )
            .with_anchor(TextAnchor::BottomCenter)
            .with_layer(10)
            .with_box_style(TextBoxStyle {
                padding: glam::Vec2::new(8.0, 4.0),
                background_color: [0.0, 0.0, 0.0, 0.65],
                border_color: None,
            }),
        );
    }
}
