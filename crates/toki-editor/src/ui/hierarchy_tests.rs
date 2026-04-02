use super::HierarchySystem;

#[test]
fn category_label_humanizes_legacy_and_snake_case_values() {
    assert_eq!(HierarchySystem::category_label("npc"), "Npc");
    assert_eq!(
        HierarchySystem::category_label("player_character"),
        "Player Character"
    );
    assert_eq!(HierarchySystem::category_label("creature"), "Creature");
}

#[test]
fn category_section_label_pluralizes_editor_palette_categories() {
    assert_eq!(
        HierarchySystem::category_section_label("creature"),
        "Creatures"
    );
    assert_eq!(HierarchySystem::category_section_label("human"), "Humans");
    assert_eq!(HierarchySystem::category_section_label("item"), "Items");
    assert_eq!(
        HierarchySystem::category_section_label("player_character"),
        "Player Character"
    );
}

#[test]
fn category_label_falls_back_for_empty_input() {
    assert_eq!(HierarchySystem::category_label(""), "Uncategorized");
    assert_eq!(HierarchySystem::category_label("   "), "Uncategorized");
}

// --- ToolboxTab category mapping ---

use super::toolbox_tab_for_category;
use crate::ui::editor_ui::ToolboxTab;

#[test]
fn toolbox_tab_for_category_maps_creature_variants() {
    assert_eq!(toolbox_tab_for_category("creature"), Some(ToolboxTab::Creatures));
    assert_eq!(toolbox_tab_for_category("creatures"), Some(ToolboxTab::Creatures));
    assert_eq!(toolbox_tab_for_category("enemy"), Some(ToolboxTab::Creatures));
}

#[test]
fn toolbox_tab_for_category_maps_human_variants() {
    assert_eq!(toolbox_tab_for_category("human"), Some(ToolboxTab::Humans));
    assert_eq!(toolbox_tab_for_category("humans"), Some(ToolboxTab::Humans));
    assert_eq!(toolbox_tab_for_category("player"), Some(ToolboxTab::Humans));
    assert_eq!(toolbox_tab_for_category("players"), Some(ToolboxTab::Humans));
}

#[test]
fn toolbox_tab_for_category_maps_item_variants() {
    assert_eq!(toolbox_tab_for_category("item"), Some(ToolboxTab::Items));
    assert_eq!(toolbox_tab_for_category("items"), Some(ToolboxTab::Items));
}

#[test]
fn toolbox_tab_for_category_returns_none_for_non_placeable() {
    assert_eq!(toolbox_tab_for_category("decoration"), None);
    assert_eq!(toolbox_tab_for_category("trigger"), None);
    assert_eq!(toolbox_tab_for_category("projectile"), None);
    assert_eq!(toolbox_tab_for_category("unknown"), None);
    assert_eq!(toolbox_tab_for_category(""), None);
}

#[test]
fn scan_entity_definitions_for_toolbox_groups_by_tab() {
    use super::scan_entity_definitions_for_toolbox;

    let dir = tempfile::tempdir().unwrap();
    let entities = dir.path().join("entities");
    std::fs::create_dir_all(&entities).unwrap();

    let minimal = |cat: &str| {
        format!(
            r#"{{"name":"x","category":"{cat}","display_name":"X","description":"","rendering":{{"size":[16,16]}},"solid":true,"active":true,"components":{{}},"collision":{{"enabled":false}},"audio":{{}},"animations":{{}}}}"#
        )
    };

    std::fs::write(entities.join("goblin.json"), minimal("creature")).unwrap();
    std::fs::write(entities.join("knight.json"), minimal("human")).unwrap();
    std::fs::write(entities.join("coin.json"), minimal("item")).unwrap();
    std::fs::write(entities.join("tree.json"), minimal("decoration")).unwrap();

    let result = scan_entity_definitions_for_toolbox(dir.path());

    assert_eq!(result.get(&ToolboxTab::Creatures).unwrap(), &["goblin"]);
    assert_eq!(result.get(&ToolboxTab::Humans).unwrap(), &["knight"]);
    assert_eq!(result.get(&ToolboxTab::Items).unwrap(), &["coin"]);
    assert!(result.get(&ToolboxTab::Decorations).is_none());
}
