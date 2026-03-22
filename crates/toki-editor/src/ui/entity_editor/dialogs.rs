//! Dialog state types for entity editor.

use super::types::{EntityCategory, EntitySummary};

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
        if self.name_input.trim().is_empty() {
            self.error_message = Some("Name is required".to_string());
            return false;
        }

        let name = self.name_input.trim();
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            self.error_message =
                Some("Name must contain only letters, numbers, and underscores".to_string());
            return false;
        }

        if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            self.error_message = Some("Name must not start with a number".to_string());
            return false;
        }

        if existing_names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
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
