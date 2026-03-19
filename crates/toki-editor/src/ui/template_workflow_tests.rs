use super::*;
use crate::project::Project;
use tempfile::tempdir;

fn write_player_entity(project_root: &std::path::Path) {
    std::fs::create_dir_all(project_root.join("entities")).expect("entities dir should exist");
    std::fs::write(
        project_root.join("entities/player.json"),
        serde_json::to_string_pretty(&toki_core::entity::EntityDefinition {
            name: "player".to_string(),
            display_name: "Player".to_string(),
            description: String::new(),
            rendering: toki_core::entity::RenderingDef {
                size: [16, 16],
                render_layer: 1,
                visible: true,
                static_object: None,
            },
            attributes: toki_core::entity::AttributesDef {
                health: Some(10),
                stats: std::collections::HashMap::new(),
                speed: 2.0,
                solid: true,
                active: true,
                can_move: true,
                ai_behavior: toki_core::entity::AiBehavior::None,
                movement_profile: toki_core::entity::MovementProfile::PlayerWasd,
                primary_projectile: None,
                primary_action: None,
                pickup: None,
                has_inventory: true,
            },
            collision: toki_core::entity::CollisionDef {
                enabled: true,
                offset: [0, 0],
                size: [16, 16],
                trigger: false,
            },
            audio: toki_core::entity::AudioDef {
                footstep_trigger_distance: 16.0,
                hearing_radius: 64,
                movement_sound_trigger: toki_core::entity::MovementSoundTrigger::Distance,
                movement_sound: "".to_string(),
                collision_sound: None,
            },
            animations: toki_core::entity::AnimationsDef {
                atlas_name: "players.json".to_string(),
                clips: vec![toki_core::entity::AnimationClipDef {
                    state: "attack_right".to_string(),
                    frame_tiles: vec!["player/attack_right".to_string()],
                    frame_duration_ms: 100.0,
                    loop_mode: "once".to_string(),
                }],
                default_state: "idle_down".to_string(),
            },
            category: "human".to_string(),
            tags: vec![],
        })
        .expect("entity should serialize"),
    )
    .expect("entity file should write");
}

#[test]
fn sync_template_editor_state_selects_first_filtered_template_and_seeds_defaults() {
    let descriptors = built_in_template_descriptors();
    let mut state = TemplateEditorState {
        category_filter: Some("combat".to_string()),
        ..TemplateEditorState::default()
    };

    sync_template_editor_state(&mut state, &descriptors);

    assert_eq!(
        state.selected_template_id.as_deref(),
        Some("toki/player_attack")
    );
    let params = state
        .parameters_by_template
        .get("toki/player_attack")
        .expect("player attack params should exist");
    assert_eq!(params.get("damage"), Some(&TemplateValue::Integer(8)));
    assert_eq!(
        params.get("attack_mode"),
        Some(&TemplateValue::Enum("melee".to_string()))
    );
}

#[test]
fn preview_selected_template_returns_lowered_entity_definition_change() {
    let temp = tempdir().expect("temp dir should exist");
    write_player_entity(temp.path());
    let project = Project::new("TestProject".to_string(), temp.path().to_path_buf());
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let descriptors = built_in_template_descriptors();
    let mut state = TemplateEditorState {
        selected_template_id: Some("toki/player_attack".to_string()),
        ..TemplateEditorState::default()
    };
    sync_template_editor_state(&mut state, &descriptors);
    let params = state
        .parameters_by_template
        .get_mut("toki/player_attack")
        .expect("player attack params should exist");
    params.insert(
        "actor_entity_definition_id".to_string(),
        TemplateValue::EntityDefinitionReference("player".to_string()),
    );
    params.insert(
        "animation_state".to_string(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::AnimationStateReference(
            "attack_right".to_string(),
        )))),
    );

    let preview = preview_selected_template(&state, &project).expect("preview should succeed");
    assert_eq!(preview.file_changes.len(), 2);
    assert!(preview
        .lowered_summary_lines
        .iter()
        .any(|line| line.contains("entities/player.json")));
    assert!(preview
        .lowered_summary_lines
        .iter()
        .any(|line| line.contains("project.toml")));
    assert_eq!(
        preview.selection_after_apply,
        Some(crate::ui::editor_ui::Selection::EntityDefinition(
            "player".to_string()
        ))
    );
}

#[test]
fn build_apply_template_command_creates_history_safe_project_file_change_command() {
    let temp = tempdir().expect("temp dir should exist");
    write_player_entity(temp.path());
    let mut project = Project::new("TestProject".to_string(), temp.path().to_path_buf());
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let descriptors = built_in_template_descriptors();
    let mut state = TemplateEditorState {
        selected_template_id: Some("toki/player_attack".to_string()),
        ..TemplateEditorState::default()
    };
    sync_template_editor_state(&mut state, &descriptors);
    let params = state
        .parameters_by_template
        .get_mut("toki/player_attack")
        .expect("player attack params should exist");
    params.insert(
        "actor_entity_definition_id".to_string(),
        TemplateValue::EntityDefinitionReference("player".to_string()),
    );
    params.insert(
        "animation_state".to_string(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::AnimationStateReference(
            "attack_right".to_string(),
        )))),
    );

    let command = build_apply_template_command(
        &state,
        &project,
        Some(crate::ui::editor_ui::Selection::Scene(
            "Main Scene".to_string(),
        )),
    )
    .expect("apply command should build");

    let mut ui_state = crate::ui::EditorUI::new();
    assert!(ui_state.execute_command_with_project(&mut project, command));
    let updated = std::fs::read_to_string(temp.path().join("entities/player.json"))
        .expect("entity file should read after apply");
    assert!(updated.contains("primary_action"));
    assert_eq!(project.metadata.editor.template_applications.len(), 1);
    assert!(matches!(
        ui_state.selection,
        Some(crate::ui::editor_ui::Selection::EntityDefinition(ref id)) if id == "player"
    ));
}

#[test]
fn build_remove_template_application_command_reverts_entity_changes_and_removes_record() {
    let temp = tempdir().expect("temp dir should exist");
    write_player_entity(temp.path());
    let mut project = Project::new("TestProject".to_string(), temp.path().to_path_buf());
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let descriptors = built_in_template_descriptors();
    let mut state = TemplateEditorState {
        selected_template_id: Some("toki/player_attack".to_string()),
        ..TemplateEditorState::default()
    };
    sync_template_editor_state(&mut state, &descriptors);
    let params = state
        .parameters_by_template
        .get_mut("toki/player_attack")
        .expect("player attack params should exist");
    params.insert(
        "actor_entity_definition_id".to_string(),
        TemplateValue::EntityDefinitionReference("player".to_string()),
    );
    params.insert(
        "animation_state".to_string(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::AnimationStateReference(
            "attack_right".to_string(),
        )))),
    );

    let apply_command = build_apply_template_command(&state, &project, None)
        .expect("apply command should build");
    let mut ui_state = crate::ui::EditorUI::new();
    assert!(ui_state.execute_command_with_project(&mut project, apply_command));
    assert_eq!(project.metadata.editor.template_applications.len(), 1);

    let application_id = project.metadata.editor.template_applications[0]
        .application_id
        .clone();
    let remove_command =
        build_remove_template_application_command(&project, &application_id, None)
            .expect("remove command should build");
    assert!(ui_state.execute_command_with_project(&mut project, remove_command));

    let reverted = std::fs::read_to_string(temp.path().join("entities/player.json"))
        .expect("entity file should read after remove");
    assert!(!reverted.contains("primary_action"));
    assert!(project.metadata.editor.template_applications.is_empty());
}

#[test]
fn animation_state_choices_follow_selected_actor_definition() {
    let parameter = toki_templates::TemplateParameter {
        id: "animation_state".to_string(),
        label: "Animation State".to_string(),
        description: None,
        kind: toki_templates::TemplateParameterKind::Optional {
            inner: Box::new(toki_templates::TemplateParameterKind::AnimationStateReference {
                entity_parameter_id: "actor_entity_definition_id".to_string(),
            }),
        },
        default: Some(TemplateValue::Optional(None)),
        required: false,
    };
    let mut values = std::collections::BTreeMap::new();
    values.insert(
        "actor_entity_definition_id".to_string(),
        TemplateValue::EntityDefinitionReference("player".to_string()),
    );
    let choices = TemplateAssetChoices {
        entity_definition_ids: vec!["player".to_string()],
        entity_animation_states: std::collections::BTreeMap::from([(
            "player".to_string(),
            vec!["attack_right".to_string(), "attack_up".to_string()],
        )]),
        ..TemplateAssetChoices::default()
    };

    let animation_choices =
        animation_state_choices_for_parameter(&parameter, &values, Some(&choices))
            .expect("animation choices should be available");
    assert_eq!(
        animation_choices,
        vec!["attack_right".to_string(), "attack_up".to_string()]
    );
}
