//! Domain-specific inspector implementations.
//!
//! Each inspector handles a specific type of selection in the editor.

mod entity;
mod entity_definition;
mod map;
mod menu;
mod rule_graph_node;
mod scene;
mod scene_anchor;
mod scene_commands;
mod scene_helpers;
mod scene_player_entry;

use super::super::inspector_trait::{Inspector, NoSelectionInspector};
use crate::ui::editor_ui::Selection;

pub use entity::EntityInspector;
pub use entity_definition::EntityDefinitionInspector;
pub use map::{MapInspector, StandaloneMapInspector};
pub use menu::MenuSelectionInspector;
pub use rule_graph_node::RuleGraphNodeInspector;
pub use scene::SceneInspector;
pub use scene_anchor::SceneAnchorInspector;
pub use scene_commands::build_delete_scene_command;
pub use scene_player_entry::ScenePlayerEntryInspector;

/// Creates the appropriate inspector for the given selection.
/// This factory replaces the large match statement in render_selection_inspector_contents.
pub fn create_inspector_for_selection(selection: Option<&Selection>) -> Box<dyn Inspector> {
    match selection {
        Some(Selection::Scene(scene_name)) => Box::new(SceneInspector::new(scene_name.clone())),
        Some(Selection::ScenePlayerEntry(scene_name)) => {
            Box::new(ScenePlayerEntryInspector::new(scene_name.clone()))
        }
        Some(Selection::SceneAnchor {
            scene_name,
            anchor_id,
        }) => Box::new(SceneAnchorInspector::new(
            scene_name.clone(),
            anchor_id.clone(),
        )),
        Some(Selection::RuleGraphNode {
            scene_name,
            node_key,
        }) => Box::new(RuleGraphNodeInspector::new(
            scene_name.clone(),
            node_key.clone(),
        )),
        Some(Selection::Map(scene_name, map_name)) => {
            Box::new(MapInspector::new(scene_name.clone(), map_name.clone()))
        }
        Some(Selection::Entity(entity_id)) => Box::new(EntityInspector::new(*entity_id)),
        Some(Selection::StandaloneMap(map_name)) => {
            Box::new(StandaloneMapInspector::new(map_name.clone()))
        }
        Some(Selection::EntityDefinition(entity_name)) => {
            Box::new(EntityDefinitionInspector::new(entity_name.clone()))
        }
        Some(Selection::MenuScreen(_))
        | Some(Selection::MenuDialog(_))
        | Some(Selection::MenuEntry { .. }) => Box::new(MenuSelectionInspector),
        None => Box::new(NoSelectionInspector),
    }
}

#[cfg(test)]
mod tests;
