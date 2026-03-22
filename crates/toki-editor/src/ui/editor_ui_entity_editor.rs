//! Entity editor re-exports for backward compatibility.
//!
//! The actual implementation is in the `entity_editor` module.

#[allow(unused_imports)]
pub use crate::ui::entity_editor::{
    create_default_definition, ComponentToggles, DeleteConfirmationState, EntityBrowserFilter,
    EntityCategory, EntityEditState, EntityEditorState, EntitySummary, NewEntityDialogState,
};
