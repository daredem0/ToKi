use super::SelectionInteraction;
use crate::ui::EditorUI;
use glam::{IVec2, UVec2};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use toki_core::entity::{
    AnimationClipDef, AnimationsDef, AttributesDef, AudioDef, CollisionDef, EntityAttributes,
    EntityDefinition, EntityKind, EntityManager, RenderingDef,
};

fn unique_temp_project_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "toki-selection-tests-{}-{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(dir.join("entities")).expect("failed to create temp entities directory");
    dir
}

fn sample_entity_definition(name: &str, category: &str, size: [u32; 2]) -> EntityDefinition {
    EntityDefinition {
        name: name.to_string(),
        display_name: format!("Display {name}"),
        description: format!("Definition for {name}"),
        rendering: RenderingDef {
            size,
            render_layer: 0,
            visible: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: Some(50),
            stats: std::collections::HashMap::new(),
            speed: 1,
            solid: true,
            active: true,
            can_move: false,
            ai_behavior: toki_core::entity::AiBehavior::Wander,
            movement_profile: toki_core::entity::MovementProfile::None,
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true,
            offset: [0, 0],
            size,
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 16.0,
            hearing_radius: 192,
            movement_sound_trigger: toki_core::entity::MovementSoundTrigger::Distance,
            movement_sound: "sfx_step".to_string(),
            collision_sound: Some("sfx_hit2".to_string()),
        },
        animations: AnimationsDef {
            atlas_name: "creatures".to_string(),
            clips: vec![AnimationClipDef {
                state: "idle".to_string(),
                frame_tiles: vec!["slime/idle_0".to_string()],
                frame_duration_ms: 150.0,
                loop_mode: "loop".to_string(),
            }],
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: vec!["selection".to_string()],
    }
}

fn write_entity_definition_file(project_dir: &Path, definition: &EntityDefinition) {
    let file_path = project_dir
        .join("entities")
        .join(format!("{}.json", definition.name));
    let json =
        serde_json::to_string_pretty(definition).expect("failed to serialize entity definition");
    fs::write(&file_path, json).expect("failed to write entity definition file");
}

fn update_scene_entity_position(
    ui_state: &mut EditorUI,
    scene_name: &str,
    entity_id: u32,
    position: IVec2,
) -> bool {
    let Some(scene) = ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == scene_name)
    else {
        return false;
    };
    let Some(entity) = scene
        .entities
        .iter_mut()
        .find(|entity| entity.id == entity_id)
    else {
        return false;
    };
    entity.position = position;
    true
}

fn update_scene_entities_position(
    ui_state: &mut EditorUI,
    scene_name: &str,
    dragged_entities: &[toki_core::entity::Entity],
    drag_delta: IVec2,
) -> usize {
    let Some(scene) = ui_state
        .scenes
        .iter_mut()
        .find(|scene| scene.name == scene_name)
    else {
        return 0;
    };
    let mut moved = 0;
    for dragged in dragged_entities {
        let Some(entity) = scene
            .entities
            .iter_mut()
            .find(|entity| entity.id == dragged.id)
        else {
            continue;
        };
        entity.position = dragged.position + drag_delta;
        moved += 1;
    }
    moved
}

#[test]
fn resolve_entity_definition_name_prefers_entity_metadata() {
    let mut manager = EntityManager::new();
    let entity_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(10, 20),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let mut entity = manager
        .get_entity(entity_id)
        .expect("missing spawned entity")
        .clone();
    entity.definition_name = Some("custom_npc".to_string());

    let resolved = SelectionInteraction::resolve_entity_definition_name(&entity, None);
    assert_eq!(resolved, Some("custom_npc".to_string()));
}

#[test]
fn resolve_entity_definition_name_uses_project_lookup_for_legacy_entities() {
    let project_dir = unique_temp_project_dir();

    let small = sample_entity_definition("small_creature", "creature", [16, 16]);
    let large = sample_entity_definition("large_creature", "creature", [32, 32]);
    write_entity_definition_file(&project_dir, &small);
    write_entity_definition_file(&project_dir, &large);

    let mut legacy_entity = large
        .create_entity(IVec2::new(0, 0), 1)
        .expect("failed to create runtime entity");
    legacy_entity.definition_name = None;

    let resolved = SelectionInteraction::resolve_entity_definition_name(
        &legacy_entity,
        Some(project_dir.as_path()),
    );
    assert_eq!(resolved, Some("large_creature".to_string()));
}

#[test]
fn resolve_entity_definition_name_falls_back_to_entity_type_name() {
    let mut manager = EntityManager::new();
    let entity_id = manager.spawn_entity(
        EntityKind::Player,
        IVec2::new(0, 0),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let entity = manager
        .get_entity(entity_id)
        .expect("missing spawned entity")
        .clone();

    let resolved = SelectionInteraction::resolve_entity_definition_name(&entity, None);
    assert_eq!(resolved, Some("human".to_string()));
}

#[test]
fn update_scene_entity_position_moves_target_entity() {
    let mut ui_state = EditorUI::new();
    let mut manager = EntityManager::new();
    let entity_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(10, 20),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let entity = manager
        .get_entity(entity_id)
        .expect("missing spawned entity")
        .clone();

    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|s| s.name == "Main Scene")
        .expect("missing default scene");
    scene.entities.push(entity);

    let moved =
        update_scene_entity_position(&mut ui_state, "Main Scene", entity_id, IVec2::new(42, 84));
    assert!(moved);

    let scene = ui_state
        .scenes
        .iter()
        .find(|s| s.name == "Main Scene")
        .expect("missing default scene");
    let moved_entity = scene
        .entities
        .iter()
        .find(|e| e.id == entity_id)
        .expect("missing moved entity");
    assert_eq!(moved_entity.position, IVec2::new(42, 84));
}

#[test]
fn update_scene_entity_position_returns_false_when_entity_missing() {
    let mut ui_state = EditorUI::new();
    let moved = update_scene_entity_position(&mut ui_state, "Main Scene", 999, IVec2::new(42, 84));
    assert!(!moved);
}

#[test]
fn update_scene_entity_position_returns_false_when_scene_missing() {
    let mut ui_state = EditorUI::new();
    let moved = update_scene_entity_position(&mut ui_state, "Missing Scene", 1, IVec2::new(42, 84));
    assert!(!moved);
}

#[test]
fn drag_entities_for_start_uses_multi_selection_when_clicking_selected_entity() {
    let mut ui_state = EditorUI::new();
    let mut manager = EntityManager::new();
    let first_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(0, 0),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let second_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(32, 0),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let first = manager
        .get_entity(first_id)
        .expect("first entity should exist")
        .clone();
    let second = manager
        .get_entity(second_id)
        .expect("second entity should exist")
        .clone();

    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|s| s.name == "Main Scene")
        .expect("default scene should exist");
    scene.entities.push(first.clone());
    scene.entities.push(second.clone());

    ui_state.set_single_entity_selection(first_id);
    ui_state.toggle_entity_selection(second_id);

    let dragged =
        SelectionInteraction::drag_entities_for_start(&ui_state, "Main Scene", &first, first_id);
    let mut ids = dragged.iter().map(|entity| entity.id).collect::<Vec<_>>();
    ids.sort_unstable();
    assert_eq!(ids, vec![first_id, second_id]);
}

#[test]
fn update_scene_entities_position_moves_all_dragged_entities_by_delta() {
    let mut ui_state = EditorUI::new();
    let mut manager = EntityManager::new();
    let first_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(10, 20),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let second_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(30, 40),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let first = manager
        .get_entity(first_id)
        .expect("first entity should exist")
        .clone();
    let second = manager
        .get_entity(second_id)
        .expect("second entity should exist")
        .clone();

    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|s| s.name == "Main Scene")
        .expect("default scene should exist");
    scene.entities.push(first.clone());
    scene.entities.push(second.clone());

    let moved = update_scene_entities_position(
        &mut ui_state,
        "Main Scene",
        &[first.clone(), second.clone()],
        IVec2::new(5, -3),
    );
    assert_eq!(moved, 2);

    let scene = ui_state
        .scenes
        .iter()
        .find(|s| s.name == "Main Scene")
        .expect("default scene should exist");
    let moved_first = scene
        .entities
        .iter()
        .find(|entity| entity.id == first_id)
        .expect("first moved entity should exist");
    let moved_second = scene
        .entities
        .iter()
        .find(|entity| entity.id == second_id)
        .expect("second moved entity should exist");

    assert_eq!(moved_first.position, IVec2::new(15, 17));
    assert_eq!(moved_second.position, IVec2::new(35, 37));
}

#[test]
fn drop_world_position_to_entity_position_keeps_snapped_top_left() {
    // Regression: drag-drop used an extra center->top-left conversion, causing one-tile offset.
    let drop_world = glam::Vec2::new(32.0, 48.0);
    let dropped = SelectionInteraction::drop_world_position_to_entity_position(drop_world);
    assert_eq!(dropped, IVec2::new(32, 48));
}

#[test]
fn apply_click_selection_plain_click_replaces_with_single_entity() {
    let mut ui_state = EditorUI::new();
    ui_state.set_single_entity_selection(1);
    ui_state.toggle_entity_selection(2);
    assert_eq!(ui_state.selected_entity_ids(), &[1, 2]);

    SelectionInteraction::apply_click_selection(&mut ui_state, Some(7), false);

    assert_eq!(ui_state.selected_entity_id(), Some(7));
    assert_eq!(ui_state.selected_entity_ids(), &[7]);
}

#[test]
fn apply_click_selection_ctrl_click_toggles_entity_membership() {
    let mut ui_state = EditorUI::new();
    ui_state.set_single_entity_selection(3);

    SelectionInteraction::apply_click_selection(&mut ui_state, Some(5), true);
    assert_eq!(ui_state.selected_entity_ids(), &[3, 5]);

    SelectionInteraction::apply_click_selection(&mut ui_state, Some(3), true);
    assert_eq!(ui_state.selected_entity_ids(), &[5]);
    assert_eq!(ui_state.selected_entity_id(), Some(5));
}

#[test]
fn apply_click_selection_plain_click_on_empty_clears_selection() {
    let mut ui_state = EditorUI::new();
    ui_state.set_single_entity_selection(3);
    SelectionInteraction::apply_click_selection(&mut ui_state, None, false);
    assert!(ui_state.selection.is_none());
    assert!(ui_state.selected_entity_ids().is_empty());
}

#[test]
fn apply_click_selection_ctrl_click_on_empty_keeps_selection() {
    let mut ui_state = EditorUI::new();
    ui_state.set_single_entity_selection(9);
    ui_state.toggle_entity_selection(10);

    SelectionInteraction::apply_click_selection(&mut ui_state, None, true);

    assert_eq!(ui_state.selected_entity_ids(), &[9, 10]);
}

#[test]
fn collect_scene_entities_in_world_rect_returns_intersecting_scene_entities() {
    let mut ui_state = EditorUI::new();
    let mut manager = EntityManager::new();
    let first_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(0, 0),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );
    let second_id = manager.spawn_entity(
        EntityKind::Npc,
        IVec2::new(48, 48),
        UVec2::new(16, 16),
        EntityAttributes::default(),
    );

    let scene = ui_state
        .scenes
        .iter_mut()
        .find(|s| s.name == "Main Scene")
        .expect("missing default scene");
    scene.entities.push(
        manager
            .get_entity(first_id)
            .expect("first entity should exist")
            .clone(),
    );
    scene.entities.push(
        manager
            .get_entity(second_id)
            .expect("second entity should exist")
            .clone(),
    );

    let selected = SelectionInteraction::collect_scene_entities_in_world_rect(
        &ui_state,
        glam::Vec2::new(-4.0, -4.0),
        glam::Vec2::new(20.0, 20.0),
    );
    assert_eq!(selected, vec![first_id]);
}

#[test]
fn apply_marquee_selection_ctrl_adds_without_duplicate_entries() {
    let mut ui_state = EditorUI::new();
    ui_state.set_single_entity_selection(5);

    SelectionInteraction::apply_marquee_selection(&mut ui_state, vec![5, 7], true);
    assert_eq!(ui_state.selected_entity_ids(), &[5, 7]);
}

#[test]
fn apply_marquee_selection_without_ctrl_replaces_and_clears_on_empty() {
    let mut ui_state = EditorUI::new();
    ui_state.set_single_entity_selection(2);
    ui_state.toggle_entity_selection(3);

    SelectionInteraction::apply_marquee_selection(&mut ui_state, vec![10, 11], false);
    assert_eq!(ui_state.selected_entity_ids(), &[10, 11]);

    SelectionInteraction::apply_marquee_selection(&mut ui_state, Vec::new(), false);
    assert!(ui_state.selected_entity_ids().is_empty());
    assert!(ui_state.selection.is_none());
}
