//! Entity editor types - categories, summaries, and filters.

use std::collections::HashSet;
use std::path::PathBuf;

/// Predefined entity categories for v1
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
            || self
                .display_name
                .to_ascii_lowercase()
                .contains(&query_lower)
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
