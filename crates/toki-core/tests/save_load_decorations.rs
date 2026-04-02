use glam::{IVec2, UVec2};
use tempfile::tempdir;
use toki_core::entity::{build_decoration_entity, DecorationSpec};
use toki_core::game::{RenderQueryService, SceneSystem};
use toki_core::{load_save_data_from_slot, save_game_to_slot, GameState, Scene};

#[test]
fn save_and_restore_keeps_inactive_decorations_renderable() {
    let mut state = GameState::new_empty();
    let mut scene = Scene::new("main".to_string());
    let decoration = build_decoration_entity(
        5,
        DecorationSpec::new(IVec2::new(32, 48), UVec2::new(16, 16), "decor", "tree"),
    );
    assert!(
        !decoration.active,
        "decorations are expected to be inactive"
    );
    scene.add_entity(decoration);
    SceneSystem::add_scene(&mut state, scene);
    SceneSystem::load(&mut state, "main").expect("scene should load");

    let temp_dir = tempdir().expect("temp dir should exist");
    save_game_to_slot(&mut state, temp_dir.path(), 1).expect("save should succeed");
    let save = load_save_data_from_slot(temp_dir.path(), 1).expect("save should load");

    let mut restored = GameState::new_empty();
    SceneSystem::restore_from_save_data(&mut restored, &save).expect("save should restore");

    let active_scene = SceneSystem::active_scene(&restored).expect("active scene should exist");
    let restored_decoration = active_scene
        .get_entity(5)
        .expect("inactive decoration should still exist after restore");
    assert!(!restored_decoration.active);
    assert_eq!(
        restored_decoration
            .rendering
            .static_object_render
            .as_ref()
            .map(|render| render.object_name.as_str()),
        Some("tree")
    );

    let renderable_ids = RenderQueryService::new(
        restored.world().entity_manager(),
        restored.world().player_id(),
        false,
    )
    .renderable_entities()
    .into_iter()
    .map(|(entity_id, _, _)| entity_id)
    .collect::<Vec<_>>();
    assert!(
        renderable_ids.contains(&5),
        "inactive decorations should still be renderable after save/load"
    );
}
