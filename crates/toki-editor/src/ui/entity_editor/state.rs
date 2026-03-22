//! Main state for the Entity Editor tab.

use std::collections::HashSet;
use std::path::PathBuf;

use toki_core::entity::EntityDefinition;

use super::dialogs::{DeleteConfirmationState, NewEntityDialogState};
use super::edit_state::EntityEditState;
use super::types::{EntityBrowserFilter, EntitySummary};

/// Main state for the Entity Editor tab
#[derive(Debug, Clone, Default)]
pub struct EntityEditorState {
    /// All discovered entity definitions
    pub entities: Vec<EntitySummary>,
    /// Currently selected entity name
    pub selected_entity: Option<String>,
    /// Currently loaded entity for editing
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
    /// Width of the entity browser panel (left)
    pub browser_panel_width: f32,
    /// Available SFX sound names (discovered from assets/audio/sfx)
    pub available_sfx: Vec<String>,
    /// Available sprite atlas names (discovered from assets/sprites/*.json)
    pub available_atlases: Vec<String>,
}

impl EntityEditorState {
    #[cfg_attr(not(test), allow(dead_code))]
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

    #[cfg_attr(not(test), allow(dead_code))]
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
            if self.selected_entity.as_ref().is_some_and(|s| s == name) {
                self.selected_entity = None;
                self.edit_state = None;
            }
            true
        } else {
            false
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn clear_selection(&mut self) {
        self.selected_entity = None;
        self.edit_state = None;
    }

    #[cfg_attr(not(test), allow(dead_code))]
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
