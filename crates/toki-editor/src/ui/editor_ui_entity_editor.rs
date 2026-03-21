// Entity Editor state for the dedicated entity editor tab
// Provides visual entity definition editing with category/tag filtering
//
// Phase 4.5A: Entity Editor Tab And Definition Browser

#![allow(dead_code)]

use std::collections::HashSet;
use std::path::PathBuf;

/// Predefined entity categories for v1
/// Built to allow easy extension to user-defined categories later
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityCategory {
    Npc,
    Enemy,
    Item,
    Prop,
    Player,
    Trigger,
    Projectile,
    Decoration,
}

impl EntityCategory {
    /// All available categories
    pub const ALL: &'static [EntityCategory] = &[
        EntityCategory::Npc,
        EntityCategory::Enemy,
        EntityCategory::Item,
        EntityCategory::Prop,
        EntityCategory::Player,
        EntityCategory::Trigger,
        EntityCategory::Projectile,
        EntityCategory::Decoration,
    ];

    /// Display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            EntityCategory::Npc => "NPC",
            EntityCategory::Enemy => "Enemy",
            EntityCategory::Item => "Item",
            EntityCategory::Prop => "Prop",
            EntityCategory::Player => "Player",
            EntityCategory::Trigger => "Trigger",
            EntityCategory::Projectile => "Projectile",
            EntityCategory::Decoration => "Decoration",
        }
    }

    /// Convert from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        let lower = s.trim().to_ascii_lowercase();
        match lower.as_str() {
            "npc" => Some(EntityCategory::Npc),
            "enemy" => Some(EntityCategory::Enemy),
            "item" | "items" => Some(EntityCategory::Item),
            "prop" | "props" => Some(EntityCategory::Prop),
            "player" => Some(EntityCategory::Player),
            "trigger" | "triggers" => Some(EntityCategory::Trigger),
            "projectile" | "projectiles" => Some(EntityCategory::Projectile),
            "decoration" | "decorations" => Some(EntityCategory::Decoration),
            _ => None,
        }
    }

    /// Convert to string for serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityCategory::Npc => "npc",
            EntityCategory::Enemy => "enemy",
            EntityCategory::Item => "item",
            EntityCategory::Prop => "prop",
            EntityCategory::Player => "player",
            EntityCategory::Trigger => "trigger",
            EntityCategory::Projectile => "projectile",
            EntityCategory::Decoration => "decoration",
        }
    }
}

/// Summary info for an entity definition (for browser listing)
#[derive(Debug, Clone)]
pub struct EntitySummary {
    /// Entity name (identifier)
    pub name: String,
    /// Human-readable display name
    pub display_name: String,
    /// Category string
    pub category: String,
    /// Tags for filtering
    pub tags: Vec<String>,
    /// Path to the definition file
    pub file_path: PathBuf,
}

impl EntitySummary {
    /// Check if this entity matches a search query (name or display_name)
    pub fn matches_search(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let query_lower = query.to_ascii_lowercase();
        self.name.to_ascii_lowercase().contains(&query_lower)
            || self.display_name.to_ascii_lowercase().contains(&query_lower)
    }

    /// Check if this entity matches a category filter
    pub fn matches_category(&self, category: &str) -> bool {
        if category.is_empty() {
            return true;
        }
        self.category.eq_ignore_ascii_case(category)
    }

    /// Check if this entity has any of the specified tags
    pub fn matches_tags(&self, tags: &HashSet<String>) -> bool {
        if tags.is_empty() {
            return true;
        }
        self.tags.iter().any(|t| tags.contains(t))
    }
}

/// Filter state for the entity browser
#[derive(Debug, Clone, Default)]
pub struct EntityBrowserFilter {
    /// Text search query (matches name or display_name)
    pub search_query: String,
    /// Category filter (empty = show all)
    pub category_filter: String,
    /// Tag filters (empty = show all)
    pub tag_filters: HashSet<String>,
}

impl EntityBrowserFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any filter is active
    pub fn is_active(&self) -> bool {
        !self.search_query.is_empty()
            || !self.category_filter.is_empty()
            || !self.tag_filters.is_empty()
    }

    /// Clear all filters
    pub fn clear(&mut self) {
        self.search_query.clear();
        self.category_filter.clear();
        self.tag_filters.clear();
    }

    /// Check if an entity passes all filters
    pub fn matches(&self, entity: &EntitySummary) -> bool {
        entity.matches_search(&self.search_query)
            && entity.matches_category(&self.category_filter)
            && entity.matches_tags(&self.tag_filters)
    }
}

/// State for the new entity dialog
#[derive(Debug, Clone, Default)]
pub struct NewEntityDialogState {
    /// Whether the dialog is open
    pub is_open: bool,
    /// Name input (identifier)
    pub name_input: String,
    /// Display name input
    pub display_name_input: String,
    /// Description input
    pub description_input: String,
    /// Selected category
    pub category: String,
    /// Validation error message
    pub error_message: Option<String>,
}

impl NewEntityDialogState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog for creating a new entity
    pub fn open_for_new(&mut self) {
        self.is_open = true;
        self.name_input.clear();
        self.display_name_input.clear();
        self.description_input.clear();
        self.category = EntityCategory::Npc.as_str().to_string();
        self.error_message = None;
    }

    /// Open the dialog for duplicating an existing entity
    pub fn open_for_duplicate(&mut self, source: &EntitySummary) {
        self.is_open = true;
        self.name_input = format!("{}_copy", source.name);
        self.display_name_input = format!("{} (Copy)", source.display_name);
        self.description_input.clear();
        self.category = source.category.clone();
        self.error_message = None;
    }

    /// Close the dialog and clear state
    pub fn close(&mut self) {
        self.is_open = false;
        self.name_input.clear();
        self.display_name_input.clear();
        self.description_input.clear();
        self.error_message = None;
    }

    /// Validate the current input
    pub fn validate(&mut self, existing_names: &[String]) -> bool {
        // Name must be non-empty
        if self.name_input.trim().is_empty() {
            self.error_message = Some("Name is required".to_string());
            return false;
        }

        // Name must be a valid identifier (alphanumeric + underscore)
        let name = self.name_input.trim();
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.error_message =
                Some("Name must contain only letters, numbers, and underscores".to_string());
            return false;
        }

        // Name must not start with a number
        if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            self.error_message = Some("Name must not start with a number".to_string());
            return false;
        }

        // Name must be unique
        if existing_names
            .iter()
            .any(|n| n.eq_ignore_ascii_case(name))
        {
            self.error_message = Some("An entity with this name already exists".to_string());
            return false;
        }

        self.error_message = None;
        true
    }
}

/// State for the delete confirmation dialog
#[derive(Debug, Clone, Default)]
pub struct DeleteConfirmationState {
    /// Whether the dialog is open
    pub is_open: bool,
    /// Name of entity to delete
    pub entity_name: String,
}

impl DeleteConfirmationState {
    pub fn open(&mut self, entity_name: &str) {
        self.is_open = true;
        self.entity_name = entity_name.to_string();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.entity_name.clear();
    }
}

/// Main state for the Entity Editor tab
#[derive(Debug, Clone, Default)]
pub struct EntityEditorState {
    /// All discovered entity definitions
    pub entities: Vec<EntitySummary>,

    /// Currently selected entity name
    pub selected_entity: Option<String>,

    /// Browser filter state
    pub filter: EntityBrowserFilter,

    /// New entity dialog state
    pub new_entity_dialog: NewEntityDialogState,

    /// Delete confirmation dialog state
    pub delete_confirmation: DeleteConfirmationState,

    /// Path to entities directory
    pub entities_dir: Option<PathBuf>,

    /// Whether the entity list needs refresh
    pub needs_refresh: bool,

    /// Whether the currently selected entity has unsaved changes
    pub dirty: bool,

    // Layout state
    /// Width of the entity browser panel (left)
    pub browser_panel_width: f32,
}

impl EntityEditorState {
    pub fn new() -> Self {
        Self {
            browser_panel_width: 220.0,
            ..Default::default()
        }
    }

    /// Check if an entity is currently loaded
    pub fn has_entity(&self) -> bool {
        self.selected_entity.is_some()
    }

    /// Get the currently selected entity summary
    pub fn selected_entity_summary(&self) -> Option<&EntitySummary> {
        let name = self.selected_entity.as_ref()?;
        self.entities.iter().find(|e| &e.name == name)
    }

    /// Select an entity by name
    pub fn select_entity(&mut self, name: &str) {
        if self.entities.iter().any(|e| e.name == name) {
            self.selected_entity = Some(name.to_string());
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_entity = None;
        self.dirty = false;
    }

    /// Get filtered entities based on current filter state
    pub fn filtered_entities(&self) -> Vec<&EntitySummary> {
        self.entities
            .iter()
            .filter(|e| self.filter.matches(e))
            .collect()
    }

    /// Get all unique tags from loaded entities
    pub fn all_tags(&self) -> HashSet<String> {
        self.entities
            .iter()
            .flat_map(|e| e.tags.iter().cloned())
            .collect()
    }

    /// Get all unique categories from loaded entities
    pub fn all_categories(&self) -> HashSet<String> {
        self.entities
            .iter()
            .map(|e| e.category.clone())
            .filter(|c| !c.is_empty())
            .collect()
    }

    /// Get names of all existing entities (for validation)
    pub fn existing_names(&self) -> Vec<String> {
        self.entities.iter().map(|e| e.name.clone()).collect()
    }

    /// Add a new entity summary (after creation)
    pub fn add_entity(&mut self, summary: EntitySummary) {
        let name = summary.name.clone();
        self.entities.push(summary);
        self.selected_entity = Some(name);
    }

    /// Remove an entity by name
    pub fn remove_entity(&mut self, name: &str) -> bool {
        let initial_len = self.entities.len();
        self.entities.retain(|e| e.name != name);

        if self.entities.len() < initial_len {
            // Clear selection if we removed the selected entity
            if self.selected_entity.as_ref().map(|s| s == name).unwrap_or(false) {
                self.selected_entity = None;
            }
            true
        } else {
            false
        }
    }

    /// Clear all state (when project changes)
    pub fn clear(&mut self) {
        self.entities.clear();
        self.selected_entity = None;
        self.filter.clear();
        self.new_entity_dialog.close();
        self.delete_confirmation.close();
        self.entities_dir = None;
        self.needs_refresh = false;
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(summary.matches_search("GOB")); // case insensitive
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
        assert!(summary.matches_category("Enemy")); // case insensitive
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
        assert!(!state.dirty);
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
        state.select_entity("goblin");
        state.dirty = true;

        state.clear_selection();
        assert!(!state.has_entity());
        assert!(!state.dirty);
    }

    #[test]
    fn editor_state_filtered_entities() {
        let mut state = EntityEditorState::new();
        state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec!["hostile"]));
        state.entities.push(make_test_summary("villager", "Villager", "npc", vec!["friendly"]));

        // No filter - all entities
        assert_eq!(state.filtered_entities().len(), 2);

        // Category filter
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
        assert!(state.selected_entity.is_none()); // Cleared because we removed selected

        assert!(!state.remove_entity("nonexistent"));
    }

    #[test]
    fn editor_state_remove_entity_preserves_other_selection() {
        let mut state = EntityEditorState::new();
        state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec![]));
        state.entities.push(make_test_summary("orc", "Orc", "enemy", vec![]));
        state.selected_entity = Some("orc".to_string());

        state.remove_entity("goblin");
        assert_eq!(state.selected_entity, Some("orc".to_string())); // Preserved
    }

    #[test]
    fn editor_state_clear() {
        let mut state = EntityEditorState::new();
        state.entities.push(make_test_summary("goblin", "Goblin", "enemy", vec![]));
        state.selected_entity = Some("goblin".to_string());
        state.dirty = true;
        state.filter.search_query = "test".to_string();

        state.clear();
        assert!(state.entities.is_empty());
        assert!(state.selected_entity.is_none());
        assert!(!state.dirty);
        assert!(!state.filter.is_active());
    }
}
