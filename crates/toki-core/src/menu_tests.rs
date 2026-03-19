use super::{
    apply_menu_opacity, build_dialog_layout, build_menu_layout, compose_dialog_ui, compose_menu_ui,
    menu_border_color, menu_fill_color_rgba, menu_hex_color_rgba, menu_visual_metrics,
    InventoryEntry, MenuAppearance, MenuBorderStyle, MenuController, MenuDialogDefinition,
    MenuInput, MenuItemDefinition, MenuListSource, MenuScreenDefinition, MenuSettings, MenuView,
    MenuViewEntry, UiAction, UiCommand,
};

#[test]
fn menu_controller_opens_pause_root_and_renders_default_selection() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();

    let view = controller
        .current_view(&[])
        .expect("pause menu should open");
    assert_eq!(view.screen_id, "pause_menu");
    assert_eq!(view.title, "Paused");
    assert_eq!(view.entries[0].text, "Resume");
    assert!(view.entries[0].selected);
    assert!(view.entries[0].selectable);
}

#[test]
fn menu_controller_wraps_selection_across_selectable_entries() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();
    controller.handle_input(MenuInput::Up);

    let view = controller
        .current_view(&[])
        .expect("pause menu should remain open");
    assert_eq!(view.entries[1].text, "Inventory");
    assert!(view.entries[1].selected);
}

#[test]
fn menu_controller_opens_submenu_and_back_returns_to_previous_screen() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();
    controller.handle_input(MenuInput::Down);
    assert_eq!(controller.handle_input(MenuInput::Confirm), None);

    let inventory_view = controller
        .current_view(&[])
        .expect("inventory menu should be open");
    assert_eq!(inventory_view.screen_id, "inventory_menu");

    assert_eq!(controller.handle_input(MenuInput::Back), None);
    let pause_view = controller
        .current_view(&[])
        .expect("pause menu should be restored");
    assert_eq!(pause_view.screen_id, "pause_menu");
}

#[test]
fn menu_controller_back_on_root_closes_menu() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();
    assert_eq!(controller.handle_input(MenuInput::Back), None);
    assert!(!controller.is_open());
}

#[test]
fn dynamic_inventory_list_is_rendered_as_non_selectable_entries() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();
    controller.handle_input(MenuInput::Down);
    assert_eq!(controller.handle_input(MenuInput::Confirm), None);

    let view = controller
        .current_view(&[
            InventoryEntry {
                item_id: "coin".to_string(),
                count: 3,
            },
            InventoryEntry {
                item_id: "gem".to_string(),
                count: 1,
            },
        ])
        .expect("inventory view should render");

    assert_eq!(view.entries[0].text, "Items");
    assert!(!view.entries[0].selectable);
    assert_eq!(view.entries[1].text, "coin x3");
    assert!(!view.entries[1].selectable);
    assert_eq!(view.entries[2].text, "gem x1");
    assert!(!view.entries[2].selectable);
    assert_eq!(view.entries[3].text, "Back");
    assert!(view.entries[3].selectable);
    assert!(view.entries[3].selected);
}

#[test]
fn navigation_skips_non_selectable_items() {
    let settings = MenuSettings {
        pause_root_screen_id: "custom".to_string(),
        gate_gameplay_when_open: true,
        appearance: Default::default(),
        screens: vec![MenuScreenDefinition {
            id: "custom".to_string(),
            title: "Custom".to_string(),
            title_border_style_override: None,
            items: vec![
                MenuItemDefinition::Label {
                    text: "Heading".to_string(),
                    border_style_override: None,
                },
                MenuItemDefinition::DynamicList {
                    heading: Some("Items".to_string()),
                    source: MenuListSource::PlayerInventory,
                    empty_text: "Empty".to_string(),
                    border_style_override: None,
                },
                MenuItemDefinition::Button {
                    text: "Resume".to_string(),
                    border_style_override: None,
                    action: UiAction::CloseUi,
                },
                MenuItemDefinition::Button {
                    text: "Next".to_string(),
                    border_style_override: None,
                    action: UiAction::OpenSurface {
                        surface_id: "custom".to_string(),
                    },
                },
            ],
        }],
        dialogs: Vec::new(),
    };

    let mut controller = MenuController::new(settings);
    controller.open_pause_root();
    let initial = controller.current_view(&[]).expect("view");
    assert!(initial.entries[3].selected);

    controller.handle_input(MenuInput::Down);
    let moved = controller.current_view(&[]).expect("view");
    assert!(moved.entries[4].selected);
}

#[test]
fn menu_controller_returns_exit_runtime_command_for_exit_game_action() {
    let settings = MenuSettings {
        pause_root_screen_id: "pause_menu".to_string(),
        gate_gameplay_when_open: true,
        appearance: Default::default(),
        screens: vec![MenuScreenDefinition {
            id: "pause_menu".to_string(),
            title: "Paused".to_string(),
            title_border_style_override: None,
            items: vec![MenuItemDefinition::Button {
                text: "Exit".to_string(),
                border_style_override: None,
                action: UiAction::ExitRuntime,
            }],
        }],
        dialogs: Vec::new(),
    };

    let mut controller = MenuController::new(settings);
    controller.open_pause_root();

    assert_eq!(
        controller.handle_input(MenuInput::Confirm),
        Some(UiCommand::ExitRuntime)
    );
    assert!(
        controller.is_open(),
        "menu stays open until runtime handles exit"
    );
}

#[test]
fn menu_controller_returns_emit_event_command_for_generic_ui_event_action() {
    let settings = MenuSettings {
        pause_root_screen_id: "pause_menu".to_string(),
        gate_gameplay_when_open: true,
        appearance: Default::default(),
        screens: vec![MenuScreenDefinition {
            id: "pause_menu".to_string(),
            title: "Paused".to_string(),
            title_border_style_override: None,
            items: vec![MenuItemDefinition::Button {
                text: "Start".to_string(),
                border_style_override: None,
                action: UiAction::EmitEvent {
                    event_id: "start_game".to_string(),
                },
            }],
        }],
        dialogs: Vec::new(),
    };

    let mut controller = MenuController::new(settings);
    controller.open_pause_root();

    assert_eq!(
        controller.handle_input(MenuInput::Confirm),
        Some(UiCommand::EmitEvent {
            event_id: "start_game".to_string(),
        })
    );
    assert!(controller.is_open());
}

#[test]
fn menu_settings_default_includes_appearance_defaults() {
    let settings = MenuSettings::default();

    assert_eq!(settings.appearance.font_family, "Sans");
    assert_eq!(settings.appearance.font_size_px, 14);
    assert_eq!(settings.appearance.menu_width_percent, 88);
    assert_eq!(settings.appearance.menu_height_percent, 70);
    assert_eq!(settings.appearance.title_spacing_px, 8);
    assert_eq!(settings.appearance.button_spacing_px, 8);
    assert_eq!(settings.appearance.footer_spacing_px, 16);
    assert_eq!(settings.appearance.opacity_percent, 100);
    assert_eq!(settings.appearance.border_color_hex, "#7CFF7C");
    assert_eq!(settings.appearance.text_color_hex, "#FFFFFF");
    assert_eq!(settings.appearance.menu_background_color_hex, "#142914");
    assert_eq!(settings.appearance.title_background_color_hex, "#143614");
    assert_eq!(settings.appearance.entry_background_color_hex, "#0F1F0F");
    assert_eq!(
        settings.appearance.footer_text,
        "Esc: Back   Enter/Space: Select"
    );
    assert!(!settings.appearance.menu_background_transparent);
    assert!(!settings.appearance.title_background_transparent);
    assert!(!settings.appearance.entry_background_transparent);
    assert!(settings.dialogs.is_empty());
}

#[test]
fn menu_hex_color_and_border_helpers_follow_shared_menu_visual_rules() {
    let accent = menu_hex_color_rgba("#7CFF7C").expect("valid accent color");
    assert_eq!(accent, [124.0 / 255.0, 1.0, 124.0 / 255.0, 1.0]);
    assert_eq!(
        menu_border_color(MenuBorderStyle::Square, accent, 0.95),
        Some([124.0 / 255.0, 1.0, 124.0 / 255.0, 0.95])
    );
    assert_eq!(menu_border_color(MenuBorderStyle::None, accent, 0.95), None);
    assert_eq!(
        menu_fill_color_rgba("#7CFF7C", false, 100),
        Some([124.0 / 255.0, 1.0, 124.0 / 255.0, 1.0])
    );
    assert_eq!(
        menu_fill_color_rgba("#7CFF7C", true, 100),
        Some([124.0 / 255.0, 1.0, 124.0 / 255.0, 0.0])
    );
    assert_eq!(
        apply_menu_opacity([1.0, 1.0, 1.0, 1.0], 40),
        [1.0, 1.0, 1.0, 0.4]
    );
}

#[test]
fn shared_menu_visual_metrics_match_runtime_overlay_defaults() {
    let metrics = menu_visual_metrics();

    assert_eq!(metrics.panel_width_px, 280.0);
    assert_eq!(metrics.panel_inner_margin_px, 16.0);
    assert_eq!(metrics.title_top_y_px, 22.0);
    assert_eq!(metrics.entries_start_y_px, 52.0);
    assert_eq!(metrics.entry_spacing_y_px, 20.0);
    assert_eq!(metrics.hint_bottom_padding_px, 18.0);
}

#[test]
fn build_menu_layout_uses_fixed_panel_width_and_shared_entry_geometry() {
    let appearance = MenuAppearance {
        menu_width_percent: 90,
        menu_height_percent: 80,
        title_spacing_px: 18,
        button_spacing_px: 12,
        footer_spacing_px: 22,
        opacity_percent: 75,
        footer_text: "Press confirm".to_string(),
        ..MenuAppearance::default()
    };
    let layout = build_menu_layout(
        &MenuView {
            screen_id: "pause".to_string(),
            title: "Paused".to_string(),
            title_border_style_override: Some(MenuBorderStyle::None),
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
                    border_style_override: Some(MenuBorderStyle::None),
                },
            ],
        },
        &appearance,
        glam::Vec2::new(320.0, 180.0),
    );

    assert_eq!(layout.panel.width, 288.0);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(layout.entries[0].rect.width, 256.0);
    assert_eq!(
        layout.entries[0].rect.y - (layout.title.rect.y + layout.title.rect.height),
        appearance.title_spacing_px as f32
    );
    assert_eq!(
        layout.entries[1].rect.y - layout.entries[0].rect.y,
        layout.entries[0].rect.height + appearance.button_spacing_px as f32
    );
    assert_eq!(layout.entries[0].border_style, appearance.border_style);
    assert_eq!(layout.entries[1].border_style, MenuBorderStyle::None);
    assert_eq!(layout.title.rect.width, layout.entries[0].rect.width);
    assert_eq!(layout.title.border_style, MenuBorderStyle::None);
    assert_eq!(layout.hint.text, "Press confirm");
    assert_eq!(
        layout.hint.rect.y - (layout.entries[1].rect.y + layout.entries[1].rect.height),
        appearance.footer_spacing_px as f32
    );
    assert!(layout.panel.height >= 144.0);
}

#[test]
fn compose_menu_ui_builds_generic_blocks_for_runtime_and_preview() {
    let appearance = MenuAppearance::default();
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
        &appearance,
        glam::Vec2::new(320.0, 180.0),
    );

    let composition = compose_menu_ui(&layout, &appearance);

    assert_eq!(composition.blocks.len(), 5);
    assert_eq!(composition.blocks[0].rect, layout.panel);
    assert_eq!(
        composition.blocks[1]
            .text
            .as_ref()
            .expect("title text block")
            .content,
        "Paused"
    );
    assert_eq!(
        composition.blocks[2]
            .text
            .as_ref()
            .expect("selected entry")
            .content,
        "> Resume"
    );
    assert_eq!(
        composition.blocks[4]
            .text
            .as_ref()
            .expect("footer text")
            .content,
        appearance.footer_text
    );
}

#[test]
fn menu_controller_opens_dialog_and_back_dismisses_it() {
    let settings = MenuSettings {
        pause_root_screen_id: "pause_menu".to_string(),
        gate_gameplay_when_open: true,
        appearance: Default::default(),
        screens: vec![MenuScreenDefinition {
            id: "pause_menu".to_string(),
            title: "Paused".to_string(),
            title_border_style_override: None,
            items: vec![MenuItemDefinition::Button {
                text: "Exit".to_string(),
                border_style_override: None,
                action: UiAction::OpenSurface {
                    surface_id: "exit_confirm".to_string(),
                },
            }],
        }],
        dialogs: vec![MenuDialogDefinition {
            id: "exit_confirm".to_string(),
            title: "Exit Game?".to_string(),
            body: "Unsaved progress may be lost.".to_string(),
            confirm_text: "Exit".to_string(),
            cancel_text: "Cancel".to_string(),
            confirm_action: UiAction::ExitRuntime,
            cancel_action: UiAction::CloseSurface,
            hide_main_menu: false,
        }],
    };
    let mut controller = MenuController::new(settings);
    controller.open_pause_root();

    assert_eq!(controller.handle_input(MenuInput::Confirm), None);
    assert!(controller.is_dialog_open());
    let dialog = controller.current_dialog_view().expect("dialog view");
    assert!(dialog.confirm_selected);

    assert_eq!(controller.handle_input(MenuInput::Back), None);
    assert!(!controller.is_dialog_open());
}

#[test]
fn menu_controller_dialog_confirm_returns_runtime_command() {
    let settings = MenuSettings {
        pause_root_screen_id: "pause_menu".to_string(),
        gate_gameplay_when_open: true,
        appearance: Default::default(),
        screens: vec![MenuScreenDefinition {
            id: "pause_menu".to_string(),
            title: "Paused".to_string(),
            title_border_style_override: None,
            items: vec![MenuItemDefinition::Button {
                text: "Exit".to_string(),
                border_style_override: None,
                action: UiAction::OpenSurface {
                    surface_id: "exit_confirm".to_string(),
                },
            }],
        }],
        dialogs: vec![MenuDialogDefinition {
            id: "exit_confirm".to_string(),
            title: "Exit Game?".to_string(),
            body: "Unsaved progress may be lost.".to_string(),
            confirm_text: "Exit".to_string(),
            cancel_text: "Cancel".to_string(),
            confirm_action: UiAction::ExitRuntime,
            cancel_action: UiAction::CloseSurface,
            hide_main_menu: false,
        }],
    };
    let mut controller = MenuController::new(settings);
    controller.open_pause_root();
    controller.handle_input(MenuInput::Confirm);

    assert_eq!(
        controller.handle_input(MenuInput::Confirm),
        Some(UiCommand::ExitRuntime)
    );
}

#[test]
fn compose_dialog_ui_builds_generic_blocks_for_dialog_overlay() {
    let appearance = MenuAppearance::default();
    let layout = build_dialog_layout(
        &super::MenuDialogView {
            dialog_id: "exit_confirm".to_string(),
            title: "Exit Game?".to_string(),
            body: "Unsaved progress may be lost.".to_string(),
            confirm_text: "Exit".to_string(),
            cancel_text: "Cancel".to_string(),
            confirm_selected: true,
            hide_main_menu: false,
        },
        &appearance,
        glam::Vec2::new(320.0, 180.0),
    );

    let composition = compose_dialog_ui(&layout, &appearance);

    assert_eq!(composition.blocks.len(), 5);
    assert_eq!(
        composition.blocks[1]
            .text
            .as_ref()
            .expect("dialog title")
            .content,
        "Exit Game?"
    );
    assert_eq!(
        composition.blocks[3]
            .text
            .as_ref()
            .expect("confirm button")
            .content,
        "> Exit"
    );
}
