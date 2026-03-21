// Entity Editor state for the dedicated entity editor tab
// Provides visual entity definition editing with category/tag filtering
//
// Phase 4.5A: Entity Editor Tab And Definition Browser
// Phase 4.5B: Optional Component Toggles
// Phase 4.5C: Property Editing

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use toki_core::entity::{
    AiBehavior, AiConfig, AnimationsDef, AttributesDef, AudioDef, CollisionDef,
    EntityDefinition, MovementProfile, MovementSoundTrigger, PickupDef,
    PrimaryProjectileDef, RenderingDef,
};

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

// ============================================================================
// Phase 4.5B: Component Toggles
// ============================================================================

/// Tracks which optional components are enabled for the current entity.
/// Components with toggles:
/// - Health: Optional<u32> health value
/// - Inventory: has_inventory flag
/// - Projectile: Optional PrimaryProjectileDef
/// - Pickup: Optional PickupDef
/// - AI: AiConfig (None behavior = disabled)
/// - Collision: collision.enabled flag
/// - Audio: treated as enabled when any audio settings configured
#[derive(Debug, Clone, Default)]
pub struct ComponentToggles {
    pub health_enabled: bool,
    pub inventory_enabled: bool,
    pub projectile_enabled: bool,
    pub pickup_enabled: bool,
    pub ai_enabled: bool,
    pub collision_enabled: bool,
    pub audio_enabled: bool,
}

impl ComponentToggles {
    /// Create toggles from an EntityDefinition, detecting which components are active
    pub fn from_definition(def: &EntityDefinition) -> Self {
        Self {
            health_enabled: def.attributes.health.is_some(),
            inventory_enabled: def.attributes.has_inventory,
            projectile_enabled: def.attributes.primary_projectile.is_some(),
            pickup_enabled: def.attributes.pickup.is_some(),
            ai_enabled: def.attributes.ai_config.behavior != AiBehavior::None,
            collision_enabled: def.collision.enabled,
            audio_enabled: Self::has_audio_config(&def.audio),
        }
    }

    /// Check if audio has any meaningful configuration
    fn has_audio_config(audio: &AudioDef) -> bool {
        !audio.movement_sound.is_empty() || audio.collision_sound.is_some()
    }

    /// Count how many components are enabled
    pub fn enabled_count(&self) -> usize {
        [
            self.health_enabled,
            self.inventory_enabled,
            self.projectile_enabled,
            self.pickup_enabled,
            self.ai_enabled,
            self.collision_enabled,
            self.audio_enabled,
        ]
        .iter()
        .filter(|&&b| b)
        .count()
    }
}

// ============================================================================
// Phase 4.5C: Entity Edit State
// ============================================================================

/// State for editing an entity definition.
/// Holds the full definition plus UI state for the editing session.
#[derive(Debug, Clone)]
pub struct EntityEditState {
    /// The entity definition being edited
    pub definition: EntityDefinition,
    /// Path to the definition file
    pub file_path: PathBuf,
    /// Which optional components are enabled
    pub toggles: ComponentToggles,
    /// Tags as editable comma-separated string
    pub tags_input: String,
    /// Whether changes have been made
    pub dirty: bool,
    /// Validation errors by field
    pub validation_errors: HashMap<String, String>,
}

impl EntityEditState {
    /// Create edit state from a loaded entity definition
    pub fn from_definition(def: EntityDefinition, file_path: PathBuf) -> Self {
        let toggles = ComponentToggles::from_definition(&def);
        let tags_input = def.tags.join(", ");
        Self {
            definition: def,
            file_path,
            toggles,
            tags_input,
            dirty: false,
            validation_errors: HashMap::new(),
        }
    }

    /// Mark the entity as modified
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Sync tags from the input string back to the definition
    pub fn sync_tags(&mut self) {
        self.definition.tags = self
            .tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // --- Component Toggle Methods ---

    /// Toggle health component on/off
    pub fn toggle_health(&mut self) {
        self.toggles.health_enabled = !self.toggles.health_enabled;
        if self.toggles.health_enabled {
            // Initialize with default health
            self.definition.attributes.health = Some(100);
        } else {
            self.definition.attributes.health = None;
        }
        self.mark_dirty();
    }

    /// Toggle inventory component on/off
    pub fn toggle_inventory(&mut self) {
        self.toggles.inventory_enabled = !self.toggles.inventory_enabled;
        self.definition.attributes.has_inventory = self.toggles.inventory_enabled;
        self.mark_dirty();
    }

    /// Toggle projectile component on/off
    pub fn toggle_projectile(&mut self) {
        self.toggles.projectile_enabled = !self.toggles.projectile_enabled;
        if self.toggles.projectile_enabled {
            self.definition.attributes.primary_projectile = Some(default_projectile_def());
        } else {
            self.definition.attributes.primary_projectile = None;
        }
        self.mark_dirty();
    }

    /// Toggle pickup component on/off
    pub fn toggle_pickup(&mut self) {
        self.toggles.pickup_enabled = !self.toggles.pickup_enabled;
        if self.toggles.pickup_enabled {
            self.definition.attributes.pickup = Some(default_pickup_def());
        } else {
            self.definition.attributes.pickup = None;
        }
        self.mark_dirty();
    }

    /// Toggle AI component on/off
    pub fn toggle_ai(&mut self) {
        self.toggles.ai_enabled = !self.toggles.ai_enabled;
        if self.toggles.ai_enabled {
            self.definition.attributes.ai_config = AiConfig {
                behavior: AiBehavior::Wander,
                detection_radius: 128,
            };
        } else {
            self.definition.attributes.ai_config = AiConfig::default();
        }
        self.mark_dirty();
    }

    /// Toggle collision component on/off
    pub fn toggle_collision(&mut self) {
        self.toggles.collision_enabled = !self.toggles.collision_enabled;
        self.definition.collision.enabled = self.toggles.collision_enabled;
        self.mark_dirty();
    }

    /// Toggle audio component on/off
    pub fn toggle_audio(&mut self) {
        self.toggles.audio_enabled = !self.toggles.audio_enabled;
        if !self.toggles.audio_enabled {
            // Clear audio settings
            self.definition.audio.movement_sound.clear();
            self.definition.audio.collision_sound = None;
        }
        self.mark_dirty();
    }

    // --- Validation Methods ---

    /// Validate the current entity definition
    pub fn validate(&mut self) -> bool {
        self.validation_errors.clear();

        // Name validation
        let name = self.definition.name.trim();
        if name.is_empty() {
            self.validation_errors
                .insert("name".to_string(), "Name is required".to_string());
        } else if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.validation_errors.insert(
                "name".to_string(),
                "Name must contain only letters, numbers, and underscores".to_string(),
            );
        } else if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            self.validation_errors.insert(
                "name".to_string(),
                "Name must not start with a number".to_string(),
            );
        }

        // Display name validation
        if self.definition.display_name.trim().is_empty() {
            self.validation_errors.insert(
                "display_name".to_string(),
                "Display name is required".to_string(),
            );
        }

        // Size validation
        if self.definition.rendering.size[0] == 0 || self.definition.rendering.size[1] == 0 {
            self.validation_errors.insert(
                "size".to_string(),
                "Size must be greater than zero".to_string(),
            );
        }

        // Health validation (if enabled)
        if self.toggles.health_enabled {
            if let Some(health) = self.definition.attributes.health {
                if health == 0 {
                    self.validation_errors
                        .insert("health".to_string(), "Health must be greater than zero".to_string());
                }
            }
        }

        // Collision size validation (if enabled)
        if self.toggles.collision_enabled
            && (self.definition.collision.size[0] == 0 || self.definition.collision.size[1] == 0)
        {
            self.validation_errors.insert(
                "collision_size".to_string(),
                "Collision size must be greater than zero".to_string(),
            );
        }

        self.validation_errors.is_empty()
    }

    /// Check if a specific field has a validation error
    pub fn has_error(&self, field: &str) -> bool {
        self.validation_errors.contains_key(field)
    }

    /// Get the validation error for a field
    pub fn get_error(&self, field: &str) -> Option<&String> {
        self.validation_errors.get(field)
    }
}

// ============================================================================
// Default Component Values
// ============================================================================

/// Create default projectile definition
fn default_projectile_def() -> PrimaryProjectileDef {
    PrimaryProjectileDef {
        sheet: String::new(),
        object_name: String::new(),
        size: [8, 8],
        speed: 200,
        damage: 10,
        lifetime_ticks: 60,
        spawn_offset: [0, 0],
    }
}

/// Create default pickup definition
fn default_pickup_def() -> PickupDef {
    PickupDef {
        item_id: String::new(),
        count: 1,
    }
}

/// Create a default entity definition with sensible defaults
pub fn create_default_definition(name: &str, display_name: &str, category: &str) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string(),
        display_name: display_name.to_string(),
        description: String::new(),
        rendering: RenderingDef {
            size: [32, 32],
            render_layer: 0,
            visible: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: HashMap::new(),
            speed: 100.0,
            solid: true,
            active: true,
            can_move: true,
            interactable: false,
            interaction_reach: 32,
            ai_config: AiConfig::default(),
            movement_profile: MovementProfile::default(),
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true, // Collision enabled by default
            offset: [0, 0],
            size: [32, 32],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: String::new(),
            clips: Vec::new(),
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: Vec::new(),
    }
}

// ============================================================================
// Entity Editor State
// ============================================================================

/// Main state for the Entity Editor tab
#[derive(Debug, Clone, Default)]
pub struct EntityEditorState {
    /// All discovered entity definitions
    pub entities: Vec<EntitySummary>,

    /// Currently selected entity name
    pub selected_entity: Option<String>,

    /// Currently loaded entity for editing (Phase 4.5B/C)
    pub edit_state: Option<EntityEditState>,

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

    // Layout state
    /// Width of the entity browser panel (left)
    pub browser_panel_width: f32,

    /// Available SFX sound names (discovered from assets/audio/sfx)
    pub available_sfx: Vec<String>,

    /// Available sprite atlas names (discovered from assets/sprites/*.json)
    pub available_atlases: Vec<String>,
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

    /// Check if there are unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.edit_state.as_ref().map(|e| e.dirty).unwrap_or(false)
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

    /// Load an entity definition for editing
    pub fn load_for_editing(&mut self, def: EntityDefinition, file_path: PathBuf) {
        let name = def.name.clone();
        self.selected_entity = Some(name);
        self.edit_state = Some(EntityEditState::from_definition(def, file_path));
    }

    /// Clear the current edit state
    pub fn clear_edit_state(&mut self) {
        self.edit_state = None;
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_entity = None;
        self.edit_state = None;
    }

    /// Get mutable reference to the edit state
    pub fn edit_state_mut(&mut self) -> Option<&mut EntityEditState> {
        self.edit_state.as_mut()
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
            if self.selected_entity.as_ref().is_some_and(|s| s == name) {
                self.selected_entity = None;
                self.edit_state = None;
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
        self.edit_state = None;
        self.filter.clear();
        self.new_entity_dialog.close();
        self.delete_confirmation.close();
        self.entities_dir = None;
        self.needs_refresh = false;
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

    fn make_test_definition() -> EntityDefinition {
        create_default_definition("test_entity", "Test Entity", "npc")
    }

    #[test]
    fn toggles_from_definition_defaults() {
        let def = make_test_definition();
        let toggles = ComponentToggles::from_definition(&def);

        // Default definition has collision enabled, nothing else
        assert!(!toggles.health_enabled);
        assert!(!toggles.inventory_enabled);
        assert!(!toggles.projectile_enabled);
        assert!(!toggles.pickup_enabled);
        assert!(!toggles.ai_enabled);
        assert!(toggles.collision_enabled); // Collision enabled by default
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
    fn toggles_from_definition_detects_projectile() {
        let mut def = make_test_definition();
        def.attributes.primary_projectile = Some(default_projectile_def());

        let toggles = ComponentToggles::from_definition(&def);
        assert!(toggles.projectile_enabled);
    }

    #[test]
    fn toggles_from_definition_detects_pickup() {
        let mut def = make_test_definition();
        def.attributes.pickup = Some(default_pickup_def());

        let toggles = ComponentToggles::from_definition(&def);
        assert!(toggles.pickup_enabled);
    }

    #[test]
    fn toggles_from_definition_detects_ai() {
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
    fn edit_state_toggle_projectile() {
        let mut state = make_edit_state();
        assert!(!state.toggles.projectile_enabled);

        state.toggle_projectile();
        assert!(state.toggles.projectile_enabled);
        assert!(state.definition.attributes.primary_projectile.is_some());

        let proj = state.definition.attributes.primary_projectile.as_ref().unwrap();
        assert_eq!(proj.damage, 10);
        assert_eq!(proj.speed, 200);
    }

    #[test]
    fn edit_state_toggle_pickup() {
        let mut state = make_edit_state();
        assert!(!state.toggles.pickup_enabled);

        state.toggle_pickup();
        assert!(state.toggles.pickup_enabled);
        assert!(state.definition.attributes.pickup.is_some());

        let pickup = state.definition.attributes.pickup.as_ref().unwrap();
        assert_eq!(pickup.count, 1);
    }

    #[test]
    fn edit_state_toggle_ai() {
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
        assert!(state.toggles.collision_enabled); // Enabled by default

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
        state.definition.attributes.health = Some(0); // Shouldn't matter

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
}
