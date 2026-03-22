//! Tests for entity editor state types.

use super::*;
use std::collections::HashSet;
use std::path::PathBuf;

// === EntityCategory Tests ===

#[test]
fn category_from_str_handles_common_cases() {
    assert_eq!(EntityCategory::from_str("npc"), Some(EntityCategory::Npc));
    assert_eq!(EntityCategory::from_str("NPC"), Some(EntityCategory::Npc));
    assert_eq!(EntityCategory::from_str("enemy"), Some(EntityCategory::Enemy));
    assert_eq!(EntityCategory::from_str("items"), Some(EntityCategory::Item));
    assert_eq!(EntityCategory::from_str("unknown"), None);
}

#[test]
fn category_as_str_returns_lowercase() {
    assert_eq!(EntityCategory::Npc.as_str(), "npc");
    assert_eq!(EntityCategory::Enemy.as_str(), "enemy");
}

#[test]
fn category_all_contains_all_variants() {
    assert_eq!(EntityCategory::ALL.len(), 8);
}

// === EntitySummary Tests ===

fn make_test_summary(name: &str, display_name: &str, category: &str, tags: Vec<&str>) -> EntitySummary {
    EntitySummary {
        name: name.to_string(),
        display_name: display_name.to_string(),
        category: category.to_string(),
        tags: tags.into_iter().map(String::from).collect(),
        file_path: PathBuf::from(format!("/test/{}.json", name)),
    }
}

#[test]
fn summary_matches_search_empty_query() {
    let summary = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    assert!(summary.matches_search(""));
}

#[test]
fn summary_matches_search_name() {
    let summary = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    assert!(summary.matches_search("gob"));
    assert!(summary.matches_search("GOB"));
}

#[test]
fn summary_matches_search_display_name() {
    let summary = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    assert!(summary.matches_search("warrior"));
    assert!(summary.matches_search("WARRIOR"));
}

#[test]
fn summary_matches_search_no_match() {
    let summary = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    assert!(!summary.matches_search("dragon"));
}

#[test]
fn summary_matches_category_empty_filter() {
    let summary = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    assert!(summary.matches_category(""));
}

#[test]
fn summary_matches_category_exact() {
    let summary = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    assert!(summary.matches_category("enemy"));
    assert!(summary.matches_category("Enemy"));
}

#[test]
fn summary_matches_category_no_match() {
    let summary = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    assert!(!summary.matches_category("npc"));
}

#[test]
fn summary_matches_tags_empty_filter() {
    let summary = make_test_summary("goblin", "Goblin", "enemy", vec!["hostile", "forest"]);
    let empty: HashSet<String> = HashSet::new();
    assert!(summary.matches_tags(&empty));
}

#[test]
fn summary_matches_tags_any_match() {
    let summary = make_test_summary("goblin", "Goblin", "enemy", vec!["hostile", "forest"]);
    let tags: HashSet<String> = ["hostile"].iter().map(|s| s.to_string()).collect();
    assert!(summary.matches_tags(&tags));
}

#[test]
fn summary_matches_tags_no_match() {
    let summary = make_test_summary("goblin", "Goblin", "enemy", vec!["hostile", "forest"]);
    let tags: HashSet<String> = ["peaceful"].iter().map(|s| s.to_string()).collect();
    assert!(!summary.matches_tags(&tags));
}

// === EntityBrowserFilter Tests ===

#[test]
fn filter_is_active_when_search_set() {
    let mut filter = EntityBrowserFilter::new();
    assert!(!filter.is_active());
    filter.search_query = "test".to_string();
    assert!(filter.is_active());
}

#[test]
fn filter_is_active_when_category_set() {
    let mut filter = EntityBrowserFilter::new();
    filter.category_filter = "enemy".to_string();
    assert!(filter.is_active());
}

#[test]
fn filter_is_active_when_tags_set() {
    let mut filter = EntityBrowserFilter::new();
    filter.tag_filters.insert("hostile".to_string());
    assert!(filter.is_active());
}

#[test]
fn filter_clear_resets_all() {
    let mut filter = EntityBrowserFilter::new();
    filter.search_query = "test".to_string();
    filter.category_filter = "enemy".to_string();
    filter.tag_filters.insert("hostile".to_string());
    filter.clear();
    assert!(!filter.is_active());
}

#[test]
fn filter_matches_all_criteria() {
    let filter = EntityBrowserFilter {
        search_query: "gob".to_string(),
        category_filter: "enemy".to_string(),
        tag_filters: ["hostile"].iter().map(|s| s.to_string()).collect(),
    };

    let matching = make_test_summary("goblin", "Goblin", "enemy", vec!["hostile"]);
    assert!(filter.matches(&matching));

    let wrong_name = make_test_summary("orc", "Orc", "enemy", vec!["hostile"]);
    assert!(!filter.matches(&wrong_name));

    let wrong_category = make_test_summary("goblin_npc", "Goblin NPC", "npc", vec!["hostile"]);
    assert!(!filter.matches(&wrong_category));

    let wrong_tags = make_test_summary("goblin_peaceful", "Goblin", "enemy", vec!["peaceful"]);
    assert!(!filter.matches(&wrong_tags));
}

// === NewEntityDialogState Tests ===

#[test]
fn new_dialog_open_for_new_clears_fields() {
    let mut dialog = NewEntityDialogState::new();
    dialog.name_input = "old".to_string();
    dialog.open_for_new();

    assert!(dialog.is_open);
    assert!(dialog.name_input.is_empty());
    assert!(dialog.display_name_input.is_empty());
    assert_eq!(dialog.category, "npc");
}

#[test]
fn new_dialog_open_for_duplicate_prefills_fields() {
    let source = make_test_summary("goblin", "Goblin Warrior", "enemy", vec![]);
    let mut dialog = NewEntityDialogState::new();
    dialog.open_for_duplicate(&source);

    assert!(dialog.is_open);
    assert_eq!(dialog.name_input, "goblin_copy");
    assert_eq!(dialog.display_name_input, "Goblin Warrior (Copy)");
    assert_eq!(dialog.category, "enemy");
}

#[test]
fn new_dialog_validate_rejects_empty_name() {
    let mut dialog = NewEntityDialogState::new();
    dialog.name_input = "  ".to_string();

    assert!(!dialog.validate(&[]));
    assert!(dialog.error_message.is_some());
}

#[test]
fn new_dialog_validate_rejects_invalid_chars() {
    let mut dialog = NewEntityDialogState::new();
    dialog.name_input = "my-entity".to_string();

    assert!(!dialog.validate(&[]));
    assert!(dialog.error_message.as_ref().unwrap().contains("letters"));
}

#[test]
fn new_dialog_validate_rejects_leading_digit() {
    let mut dialog = NewEntityDialogState::new();
    dialog.name_input = "123entity".to_string();

    assert!(!dialog.validate(&[]));
    assert!(dialog.error_message.as_ref().unwrap().contains("number"));
}

#[test]
fn new_dialog_validate_rejects_duplicate_name() {
    let mut dialog = NewEntityDialogState::new();
    dialog.name_input = "goblin".to_string();

    let existing = vec!["goblin".to_string(), "orc".to_string()];
    assert!(!dialog.validate(&existing));
    assert!(dialog.error_message.as_ref().unwrap().contains("already exists"));
}

#[test]
fn new_dialog_validate_accepts_valid_name() {
    let mut dialog = NewEntityDialogState::new();
    dialog.name_input = "my_new_entity".to_string();

    let existing = vec!["goblin".to_string()];
    assert!(dialog.validate(&existing));
    assert!(dialog.error_message.is_none());
}

// === EntityEditorState Tests ===

#[test]
fn editor_state_new_defaults() {
    let state = EntityEditorState::new();
    assert!(!state.has_entity());
    assert!(state.entities.is_empty());
    assert!(!state.is_dirty());
    assert!(state.browser_panel_width > 0.0);
}

#[test]
fn editor_state_select_entity() {
    let mut state = EntityEditorState::new();
    state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec![]));

    state.select_entity("goblin");
    assert!(state.has_entity());
    assert_eq!(state.selected_entity, Some("goblin".to_string()));
}

#[test]
fn editor_state_select_nonexistent_entity_does_nothing() {
    let mut state = EntityEditorState::new();
    state.select_entity("nonexistent");
    assert!(!state.has_entity());
}

#[test]
fn editor_state_clear_selection() {
    let mut state = EntityEditorState::new();
    state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec![]));
    let def = create_default_definition("goblin", "Goblin", "enemy");
    state.load_for_editing(def, PathBuf::from("/test/goblin.json"));

    state.clear_selection();
    assert!(!state.has_entity());
    assert!(state.edit_state.is_none());
}

#[test]
fn editor_state_filtered_entities() {
    let mut state = EntityEditorState::new();
    state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec!["hostile"]));
    state.entities.push(make_test_summary("villager", "Villager", "npc", vec!["friendly"]));

    assert_eq!(state.filtered_entities().len(), 2);

    state.filter.category_filter = "enemy".to_string();
    let filtered = state.filtered_entities();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "goblin");
}

#[test]
fn editor_state_all_tags() {
    let mut state = EntityEditorState::new();
    state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec!["hostile", "forest"]));
    state.entities.push(make_test_summary("orc", "Orc", "enemy", vec!["hostile", "mountain"]));

    let tags = state.all_tags();
    assert_eq!(tags.len(), 3);
    assert!(tags.contains("hostile"));
    assert!(tags.contains("forest"));
    assert!(tags.contains("mountain"));
}

#[test]
fn editor_state_add_entity() {
    let mut state = EntityEditorState::new();
    let summary = make_test_summary("goblin", "Goblin", "enemy", vec![]);

    state.add_entity(summary);
    assert_eq!(state.entities.len(), 1);
    assert_eq!(state.selected_entity, Some("goblin".to_string()));
}

#[test]
fn editor_state_remove_entity() {
    let mut state = EntityEditorState::new();
    state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec![]));
    state.entities.push(make_test_summary("orc", "Orc", "enemy", vec![]));
    state.selected_entity = Some("goblin".to_string());

    assert!(state.remove_entity("goblin"));
    assert_eq!(state.entities.len(), 1);
    assert!(state.selected_entity.is_none());

    assert!(!state.remove_entity("nonexistent"));
}

#[test]
fn editor_state_remove_entity_preserves_other_selection() {
    let mut state = EntityEditorState::new();
    state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec![]));
    state.entities.push(make_test_summary("orc", "Orc", "enemy", vec![]));
    state.selected_entity = Some("orc".to_string());

    state.remove_entity("goblin");
    assert_eq!(state.selected_entity, Some("orc".to_string()));
}

#[test]
fn editor_state_clear() {
    let mut state = EntityEditorState::new();
    state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec![]));
    let def = create_default_definition("goblin", "Goblin", "enemy");
    state.load_for_editing(def, PathBuf::from("/test/goblin.json"));
    state.filter.search_query = "test".to_string();

    state.clear();
    assert!(state.entities.is_empty());
    assert!(state.selected_entity.is_none());
    assert!(state.edit_state.is_none());
    assert!(!state.filter.is_active());
}

#[test]
fn editor_state_load_for_editing() {
    let mut state = EntityEditorState::new();
    let def = create_default_definition("goblin", "Goblin", "enemy");

    state.load_for_editing(def, PathBuf::from("/test/goblin.json"));
    assert!(state.has_entity());
    assert_eq!(state.selected_entity, Some("goblin".to_string()));
    assert!(state.edit_state.is_some());

    let edit = state.edit_state.as_ref().unwrap();
    assert_eq!(edit.definition.name, "goblin");
    assert!(!edit.dirty);
}

#[test]
fn editor_state_is_dirty_when_edit_state_dirty() {
    let mut state = EntityEditorState::new();
    assert!(!state.is_dirty());

    let def = create_default_definition("goblin", "Goblin", "enemy");
    state.load_for_editing(def, PathBuf::from("/test/goblin.json"));
    assert!(!state.is_dirty());

    state.edit_state.as_mut().unwrap().mark_dirty();
    assert!(state.is_dirty());
}

// === ComponentToggles Tests ===

fn make_test_definition() -> toki_core::entity::EntityDefinition {
    create_default_definition("test_entity", "Test Entity", "npc")
}

#[test]
fn toggles_from_definition_defaults() {
    let def = make_test_definition();
    let toggles = ComponentToggles::from_definition(&def);

    assert!(!toggles.health_enabled);
    assert!(!toggles.inventory_enabled);
    assert!(!toggles.projectile_enabled);
    assert!(!toggles.pickup_enabled);
    assert!(!toggles.ai_enabled);
    assert!(toggles.collision_enabled);
    assert!(!toggles.audio_enabled);
}

#[test]
fn toggles_from_definition_detects_health() {
    let mut def = make_test_definition();
    def.attributes.health = Some(50);

    let toggles = ComponentToggles::from_definition(&def);
    assert!(toggles.health_enabled);
}

#[test]
fn toggles_from_definition_detects_inventory() {
    let mut def = make_test_definition();
    def.attributes.has_inventory = true;

    let toggles = ComponentToggles::from_definition(&def);
    assert!(toggles.inventory_enabled);
}

#[test]
fn toggles_from_definition_detects_ai() {
    use toki_core::entity::{AiBehavior, AiConfig};
    let mut def = make_test_definition();
    def.attributes.ai_config = AiConfig {
        behavior: AiBehavior::Chase,
        detection_radius: 128,
    };

    let toggles = ComponentToggles::from_definition(&def);
    assert!(toggles.ai_enabled);
}

#[test]
fn toggles_from_definition_detects_audio() {
    let mut def = make_test_definition();
    def.audio.movement_sound = "walk.ogg".to_string();

    let toggles = ComponentToggles::from_definition(&def);
    assert!(toggles.audio_enabled);
}

#[test]
fn toggles_enabled_count() {
    let mut toggles = ComponentToggles::default();
    assert_eq!(toggles.enabled_count(), 0);

    toggles.health_enabled = true;
    toggles.collision_enabled = true;
    assert_eq!(toggles.enabled_count(), 2);
}

// === EntityEditState Tests ===

fn make_edit_state() -> EntityEditState {
    let def = make_test_definition();
    EntityEditState::from_definition(def, PathBuf::from("/test/entity.json"))
}

#[test]
fn edit_state_from_definition() {
    let state = make_edit_state();
    assert_eq!(state.definition.name, "test_entity");
    assert!(!state.dirty);
    assert!(state.validation_errors.is_empty());
}

#[test]
fn edit_state_mark_dirty() {
    let mut state = make_edit_state();
    assert!(!state.dirty);
    state.mark_dirty();
    assert!(state.dirty);
}

#[test]
fn edit_state_sync_tags() {
    let mut state = make_edit_state();
    state.tags_input = "hostile, forest, strong".to_string();
    state.sync_tags();

    assert_eq!(state.definition.tags.len(), 3);
    assert!(state.definition.tags.contains(&"hostile".to_string()));
    assert!(state.definition.tags.contains(&"forest".to_string()));
    assert!(state.definition.tags.contains(&"strong".to_string()));
}

#[test]
fn edit_state_sync_tags_handles_empty() {
    let mut state = make_edit_state();
    state.tags_input = "  ,  ,  ".to_string();
    state.sync_tags();
    assert!(state.definition.tags.is_empty());
}

#[test]
fn edit_state_toggle_health() {
    let mut state = make_edit_state();
    assert!(!state.toggles.health_enabled);
    assert!(state.definition.attributes.health.is_none());

    state.toggle_health();
    assert!(state.toggles.health_enabled);
    assert_eq!(state.definition.attributes.health, Some(100));
    assert!(state.dirty);

    state.toggle_health();
    assert!(!state.toggles.health_enabled);
    assert!(state.definition.attributes.health.is_none());
}

#[test]
fn edit_state_toggle_inventory() {
    let mut state = make_edit_state();
    assert!(!state.toggles.inventory_enabled);
    assert!(!state.definition.attributes.has_inventory);

    state.toggle_inventory();
    assert!(state.toggles.inventory_enabled);
    assert!(state.definition.attributes.has_inventory);
    assert!(state.dirty);
}

#[test]
fn edit_state_toggle_ai() {
    use toki_core::entity::AiBehavior;
    let mut state = make_edit_state();
    assert!(!state.toggles.ai_enabled);
    assert_eq!(state.definition.attributes.ai_config.behavior, AiBehavior::None);

    state.toggle_ai();
    assert!(state.toggles.ai_enabled);
    assert_eq!(state.definition.attributes.ai_config.behavior, AiBehavior::Wander);
    assert_eq!(state.definition.attributes.ai_config.detection_radius, 128);

    state.toggle_ai();
    assert!(!state.toggles.ai_enabled);
    assert_eq!(state.definition.attributes.ai_config.behavior, AiBehavior::None);
}

#[test]
fn edit_state_toggle_collision() {
    let mut state = make_edit_state();
    assert!(state.toggles.collision_enabled);

    state.toggle_collision();
    assert!(!state.toggles.collision_enabled);
    assert!(!state.definition.collision.enabled);

    state.toggle_collision();
    assert!(state.toggles.collision_enabled);
    assert!(state.definition.collision.enabled);
}

#[test]
fn edit_state_toggle_audio() {
    let mut state = make_edit_state();
    state.definition.audio.movement_sound = "walk.ogg".to_string();
    state.toggles.audio_enabled = true;

    state.toggle_audio();
    assert!(!state.toggles.audio_enabled);
    assert!(state.definition.audio.movement_sound.is_empty());
    assert!(state.definition.audio.collision_sound.is_none());
}

// === Validation Tests ===

#[test]
fn validation_passes_for_valid_entity() {
    let mut state = make_edit_state();
    assert!(state.validate());
    assert!(state.validation_errors.is_empty());
}

#[test]
fn validation_fails_for_empty_name() {
    let mut state = make_edit_state();
    state.definition.name = "  ".to_string();

    assert!(!state.validate());
    assert!(state.has_error("name"));
}

#[test]
fn validation_fails_for_invalid_name_chars() {
    let mut state = make_edit_state();
    state.definition.name = "my-entity".to_string();

    assert!(!state.validate());
    assert!(state.has_error("name"));
}

#[test]
fn validation_fails_for_name_starting_with_digit() {
    let mut state = make_edit_state();
    state.definition.name = "123entity".to_string();

    assert!(!state.validate());
    assert!(state.has_error("name"));
}

#[test]
fn validation_fails_for_empty_display_name() {
    let mut state = make_edit_state();
    state.definition.display_name = "".to_string();

    assert!(!state.validate());
    assert!(state.has_error("display_name"));
}

#[test]
fn validation_fails_for_zero_size() {
    let mut state = make_edit_state();
    state.definition.rendering.size = [0, 32];

    assert!(!state.validate());
    assert!(state.has_error("size"));
}

#[test]
fn validation_fails_for_zero_health_when_enabled() {
    let mut state = make_edit_state();
    state.toggles.health_enabled = true;
    state.definition.attributes.health = Some(0);

    assert!(!state.validate());
    assert!(state.has_error("health"));
}

#[test]
fn validation_skips_health_when_disabled() {
    let mut state = make_edit_state();
    state.toggles.health_enabled = false;
    state.definition.attributes.health = Some(0);

    assert!(state.validate());
}

#[test]
fn validation_fails_for_zero_collision_size_when_enabled() {
    let mut state = make_edit_state();
    state.toggles.collision_enabled = true;
    state.definition.collision.size = [0, 32];

    assert!(!state.validate());
    assert!(state.has_error("collision_size"));
}

#[test]
fn get_error_returns_message() {
    let mut state = make_edit_state();
    state.definition.name = "".to_string();
    state.validate();

    let error = state.get_error("name");
    assert!(error.is_some());
    assert!(error.unwrap().contains("required"));
}

// === create_default_definition Tests ===

#[test]
fn default_definition_has_collision_enabled() {
    let def = create_default_definition("test", "Test", "npc");
    assert!(def.collision.enabled);
}

#[test]
fn default_definition_has_no_health() {
    let def = create_default_definition("test", "Test", "npc");
    assert!(def.attributes.health.is_none());
}

#[test]
fn default_definition_has_sensible_defaults() {
    let def = create_default_definition("test", "Test Entity", "enemy");

    assert_eq!(def.name, "test");
    assert_eq!(def.display_name, "Test Entity");
    assert_eq!(def.category, "enemy");
    assert_eq!(def.rendering.size, [32, 32]);
    assert!(def.rendering.visible);
    assert_eq!(def.attributes.speed, 100.0);
    assert!(def.attributes.solid);
    assert!(def.attributes.active);
    assert!(def.attributes.can_move);
    assert!(!def.attributes.interactable);
    assert!(def.collision.enabled);
    assert_eq!(def.collision.size, [32, 32]);
    assert!(!def.collision.trigger);
}
