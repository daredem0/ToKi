use super::{
    build_menu_layout, menu_border_color, menu_hex_color_rgba, menu_visual_metrics,
    tinted_menu_background, InventoryEntry, MenuAction, MenuAppearance, MenuBorderStyle,
    MenuCommand, MenuController, MenuInput, MenuItemDefinition, MenuListSource,
    MenuScreenDefinition, MenuSettings, MenuView, MenuViewEntry,
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
            items: vec![
                MenuItemDefinition::Label {
                    text: "Heading".to_string(),
                },
                MenuItemDefinition::DynamicList {
                    heading: Some("Items".to_string()),
                    source: MenuListSource::PlayerInventory,
                    empty_text: "Empty".to_string(),
                },
                MenuItemDefinition::Button {
                    text: "Resume".to_string(),
                    action: MenuAction::CloseMenu,
                },
                MenuItemDefinition::Button {
                    text: "Next".to_string(),
                    action: MenuAction::OpenScreen {
                        screen_id: "custom".to_string(),
                    },
                },
            ],
        }],
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
            items: vec![MenuItemDefinition::Button {
                text: "Exit".to_string(),
                action: MenuAction::ExitGame,
            }],
        }],
    };

    let mut controller = MenuController::new(settings);
    controller.open_pause_root();

    assert_eq!(
        controller.handle_input(MenuInput::Confirm),
        Some(MenuCommand::ExitRuntime)
    );
    assert!(
        controller.is_open(),
        "menu stays open until runtime handles exit"
    );
}

#[test]
fn menu_settings_default_includes_appearance_defaults() {
    let settings = MenuSettings::default();

    assert_eq!(settings.appearance.font_family, "Sans");
    assert_eq!(settings.appearance.font_size_px, 14);
    assert_eq!(settings.appearance.color_hex, "#7CFF7C");
}

#[test]
fn menu_hex_color_and_border_helpers_follow_shared_menu_visual_rules() {
    let accent = menu_hex_color_rgba("#7CFF7C").expect("valid accent color");
    assert_eq!(accent, [124.0 / 255.0, 1.0, 124.0 / 255.0, 1.0]);
    assert_eq!(
        menu_border_color(MenuBorderStyle::Square, accent, 0.95),
        Some([124.0 / 255.0, 1.0, 124.0 / 255.0, 0.95])
    );
    assert_eq!(
        tinted_menu_background(accent, 0.16, 0.9),
        [accent[0] * 0.16, accent[1] * 0.16, accent[2] * 0.16, 0.9]
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
    let layout = build_menu_layout(
        &MenuView {
            screen_id: "pause".to_string(),
            title: "Paused".to_string(),
            entries: vec![
                MenuViewEntry {
                    text: "Resume".to_string(),
                    selected: true,
                    selectable: true,
                },
                MenuViewEntry {
                    text: "Inventory".to_string(),
                    selected: false,
                    selectable: true,
                },
            ],
        },
        &MenuAppearance::default(),
        glam::Vec2::new(320.0, 180.0),
    );

    assert_eq!(layout.panel.width, 280.0);
    assert_eq!(layout.entries.len(), 2);
    assert_eq!(layout.entries[0].rect.width, 248.0);
    assert!(layout.entries[1].rect.y > layout.entries[0].rect.y);
    assert_eq!(layout.title.rect.width, layout.entries[0].rect.width);
}
