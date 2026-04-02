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

use super::{collect_entity_definitions_for_toolbox, toolbox_tab_for_category};
use crate::project::assets::EntityAsset;
use crate::project::ProjectAssets;
use crate::ui::editor_ui::ToolboxTab;
use crate::ui::entity_editor::{EntityEditState, EntitySummary};
use crate::ui::EditorUI;
use std::path::PathBuf;
use toki_core::entity::{
    AnimationsDef, AudioDef, CollisionDef, ComponentsDef, EntityDefinition, RenderingDef,
};

#[test]
fn toolbox_tab_for_category_maps_creature_variants() {
    assert_eq!(
        toolbox_tab_for_category("creature"),
        Some(ToolboxTab::Creatures)
    );
    assert_eq!(
        toolbox_tab_for_category("creatures"),
        Some(ToolboxTab::Creatures)
    );
    assert_eq!(
        toolbox_tab_for_category("enemy"),
        Some(ToolboxTab::Creatures)
    );
}

#[test]
fn toolbox_tab_for_category_maps_human_variants() {
    assert_eq!(toolbox_tab_for_category("human"), Some(ToolboxTab::Humans));
    assert_eq!(toolbox_tab_for_category("humans"), Some(ToolboxTab::Humans));
    assert_eq!(toolbox_tab_for_category("player"), Some(ToolboxTab::Humans));
    assert_eq!(
        toolbox_tab_for_category("players"),
        Some(ToolboxTab::Humans)
    );
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

fn minimal_definition(name: &str, category: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.into(),
        display_name: name.to_string(),
        description: String::new(),
        category: category.to_string(),
        tags: Vec::new(),
        rendering: RenderingDef {
            size: [16, 16],
            render_layer: 0,
            visible: true,
            has_shadow: true,
            palette_override: None,
            static_object: None,
            grounding: Default::default(),
        },
        solid: false,
        active: true,
        components: ComponentsDef::default(),
        collision: CollisionDef {
            enabled: false,
            offset: [0, 0],
            size: [16, 16],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: toki_core::entity::MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: String::new(),
            clips: Vec::new(),
            default_state: String::new(),
        },
    }
}

#[test]
fn collect_entity_definitions_for_toolbox_groups_from_entity_editor_state() {
    let mut ui_state = EditorUI::new();
    let entity_editor = ui_state.entity_editor_context_mut();
    entity_editor.entity_editor.entities = vec![
        EntitySummary {
            name: "goblin".to_string(),
            display_name: "Goblin".to_string(),
            category: "creature".to_string(),
            tags: Vec::new(),
            file_path: PathBuf::from("goblin.json"),
        },
        EntitySummary {
            name: "knight".to_string(),
            display_name: "Knight".to_string(),
            category: "human".to_string(),
            tags: Vec::new(),
            file_path: PathBuf::from("knight.json"),
        },
        EntitySummary {
            name: "coin".to_string(),
            display_name: "Coin".to_string(),
            category: "item".to_string(),
            tags: Vec::new(),
            file_path: PathBuf::from("coin.json"),
        },
    ];

    let result = collect_entity_definitions_for_toolbox(&ui_state, None);
    assert_eq!(
        result.get(&ToolboxTab::Creatures).unwrap()[0].name,
        "goblin"
    );
    assert_eq!(
        result.get(&ToolboxTab::Creatures).unwrap()[0].display_name,
        "Goblin"
    );
    assert_eq!(result.get(&ToolboxTab::Humans).unwrap()[0].name, "knight");
    assert_eq!(result.get(&ToolboxTab::Items).unwrap()[0].name, "coin");
    assert!(!result.contains_key(&ToolboxTab::Decorations));
}

#[test]
fn collect_entity_definitions_for_toolbox_sorts_names_deterministically() {
    let mut ui_state = EditorUI::new();
    ui_state.entity_editor_context_mut().entity_editor.entities = vec![
        EntitySummary {
            name: "zombie".to_string(),
            display_name: "Zombie".to_string(),
            category: "creature".to_string(),
            tags: Vec::new(),
            file_path: PathBuf::from("zombie.json"),
        },
        EntitySummary {
            name: "bat".to_string(),
            display_name: "Bat".to_string(),
            category: "creature".to_string(),
            tags: Vec::new(),
            file_path: PathBuf::from("bat.json"),
        },
    ];

    let result = collect_entity_definitions_for_toolbox(&ui_state, None);
    let creatures = result.get(&ToolboxTab::Creatures).unwrap();
    assert_eq!(creatures[0].name, "bat");
    assert_eq!(creatures[1].name, "zombie");
}

#[test]
fn collect_entity_definitions_for_toolbox_prefers_unsaved_edit_state_category() {
    let mut ui_state = EditorUI::new();
    {
        let entity_editor = ui_state.entity_editor_context_mut();
        entity_editor.entity_editor.entities = vec![EntitySummary {
            name: "coin".to_string(),
            display_name: "Coin".to_string(),
            category: "item".to_string(),
            tags: Vec::new(),
            file_path: PathBuf::from("/tmp/coin.json"),
        }];
        let mut definition = minimal_definition("coin", "human");
        definition.display_name = "Coin Human".to_string();
        entity_editor.entity_editor.edit_state = Some(EntityEditState::from_definition(
            definition,
            PathBuf::from("/tmp/coin.json"),
        ));
    }

    let result = collect_entity_definitions_for_toolbox(&ui_state, None);
    assert!(!result.contains_key(&ToolboxTab::Items));
    assert_eq!(result.get(&ToolboxTab::Humans).unwrap()[0].name, "coin");
    assert_eq!(
        result.get(&ToolboxTab::Humans).unwrap()[0].display_name,
        "Coin Human"
    );
}

#[test]
fn collect_entity_definitions_for_toolbox_falls_back_to_project_assets() {
    let dir = tempfile::tempdir().unwrap();
    let mut project_assets = ProjectAssets::new(dir.path().to_path_buf());
    project_assets.entities.insert(
        "goblin".to_string(),
        EntityAsset {
            path: dir.path().join("entities/goblin.json"),
            definition: Some(minimal_definition("goblin", "creature")),
        },
    );
    project_assets.entities.insert(
        "coin".to_string(),
        EntityAsset {
            path: dir.path().join("entities/coin.json"),
            definition: Some(minimal_definition("coin", "item")),
        },
    );

    let ui_state = EditorUI::new();
    let result = collect_entity_definitions_for_toolbox(&ui_state, Some(&mut project_assets));

    assert_eq!(
        result.get(&ToolboxTab::Creatures).unwrap()[0].name,
        "goblin"
    );
    assert_eq!(result.get(&ToolboxTab::Items).unwrap()[0].name, "coin");
}
