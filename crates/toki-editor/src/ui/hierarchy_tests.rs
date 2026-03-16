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
