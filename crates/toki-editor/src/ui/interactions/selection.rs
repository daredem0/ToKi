use crate::scene::SceneViewport;
use crate::ui::EditorUI;

/// Handles entity selection and drag operations
pub struct SelectionInteraction;

impl SelectionInteraction {
    /// Handle entity selection click - starts drag operation
    pub fn handle_click(
        ui_state: &mut EditorUI,
        viewport: &mut SceneViewport,
        click_pos: egui::Pos2,
        rect: egui::Rect,
    ) {
        tracing::info!("Regular click detected at screen pos: {:?}", click_pos);
        let world_pos = viewport.screen_to_world_pos(click_pos, rect);

        if let Some(entity_id) = viewport.get_entity_at_world_pos(world_pos) {
            Self::start_entity_drag_operation(ui_state, viewport, entity_id);
        } else {
            tracing::info!(
                "No entity clicked at world position ({:.1}, {:.1})",
                world_pos.x,
                world_pos.y
            );
        }
    }

    /// Start drag operation for selected entity
    fn start_entity_drag_operation(
        ui_state: &mut EditorUI,
        viewport: &SceneViewport,
        entity_id: toki_core::entity::EntityId,
    ) {
        tracing::info!("Entity {} clicked - starting drag operation", entity_id);

        let Some(entity) = viewport
            .scene_manager()
            .game_state()
            .entity_manager()
            .get_entity(entity_id)
        else {
            tracing::warn!("Could not find entity {} for drag operation", entity_id);
            return;
        };

        let entity_def_name = Self::map_entity_type_to_definition(&entity.entity_type);
        tracing::info!(
            "Removing entity {} and entering placement mode with type: '{}'",
            entity_id,
            entity_def_name
        );

        Self::remove_entity_from_scene(ui_state, entity_id);
        ui_state.enter_placement_mode(entity_def_name.to_string());
    }

    /// Map entity type to definition name
    fn map_entity_type_to_definition(entity_type: &toki_core::entity::EntityType) -> &'static str {
        match entity_type {
            toki_core::entity::EntityType::Player => "player",
            toki_core::entity::EntityType::Npc => "slime",
            _ => "slime",
        }
    }

    /// Remove entity from the active scene
    fn remove_entity_from_scene(ui_state: &mut EditorUI, entity_id: toki_core::entity::EntityId) {
        let Some(active_scene_name) = &ui_state.active_scene else {
            tracing::warn!("No active scene to remove entity from");
            return;
        };

        let Some(scene) = ui_state
            .scenes
            .iter_mut()
            .find(|s| s.name == *active_scene_name)
        else {
            tracing::warn!("Active scene '{}' not found", active_scene_name);
            return;
        };

        scene.entities.retain(|e| e.id != entity_id);
        ui_state.scene_content_changed = true;
        tracing::info!(
            "Removed entity {} from scene '{}'",
            entity_id,
            active_scene_name
        );
    }
}
