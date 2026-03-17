use super::{
    InventoryEntry, MenuAction, MenuController, MenuInput, MenuItemDefinition, MenuListSource,
    MenuScreenDefinition, MenuSettings,
};

#[test]
fn menu_controller_opens_pause_root_and_renders_default_selection() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();

    let view = controller.current_view(&[]).expect("pause menu should open");
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

    let view = controller.current_view(&[]).expect("pause menu should remain open");
    assert_eq!(view.entries[1].text, "Inventory");
    assert!(view.entries[1].selected);
}

#[test]
fn menu_controller_opens_submenu_and_back_returns_to_previous_screen() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();
    controller.handle_input(MenuInput::Down);
    controller.handle_input(MenuInput::Confirm);

    let inventory_view = controller
        .current_view(&[])
        .expect("inventory menu should be open");
    assert_eq!(inventory_view.screen_id, "inventory_menu");

    controller.handle_input(MenuInput::Back);
    let pause_view = controller
        .current_view(&[])
        .expect("pause menu should be restored");
    assert_eq!(pause_view.screen_id, "pause_menu");
}

#[test]
fn menu_controller_back_on_root_closes_menu() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();
    controller.handle_input(MenuInput::Back);
    assert!(!controller.is_open());
}

#[test]
fn dynamic_inventory_list_is_rendered_as_non_selectable_entries() {
    let mut controller = MenuController::new(MenuSettings::default());
    controller.open_pause_root();
    controller.handle_input(MenuInput::Down);
    controller.handle_input(MenuInput::Confirm);

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
