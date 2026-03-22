//! Entity editor module - organized into focused submodules.
//!
//! This module provides types and state for the entity definition editor tab,
//! including the entity browser, filter system, and component editing.

mod defaults;
mod dialogs;
mod edit_state;
mod state;
mod toggles;
mod types;

// Re-export all public types
pub use defaults::create_default_definition;
pub use dialogs::{DeleteConfirmationState, NewEntityDialogState};
pub use edit_state::EntityEditState;
pub use state::EntityEditorState;
pub use toggles::ComponentToggles;
pub use types::{EntityBrowserFilter, EntityCategory, EntitySummary};

#[cfg(test)]
#[path = "../editor_ui_entity_editor_tests.rs"]
mod tests;
