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

fn toki_templates_crate_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("toki-templates")
        .canonicalize()
        .expect("toki-templates crate path should resolve")
}

fn write_project_template_crate(project_root: &std::path::Path) {
    let crate_dir = project_root.join("templates");
    std::fs::create_dir_all(crate_dir.join("src")).expect("template src dir should exist");
    std::fs::create_dir_all(crate_dir.join(".cargo")).expect("template cargo config dir should exist");
    let manifest = format!(
        r#"[package]
name = "project-templates"
version = "0.1.0"
edition = "2021"

[dependencies]
serde_json = "1.0"
toki-templates = {{ path = "{}" }}
"#,
        toki_templates_crate_path().display()
    );
    std::fs::write(crate_dir.join("Cargo.toml"), manifest)
        .expect("template Cargo.toml should write");
    std::fs::write(crate_dir.join(".cargo/config.toml"), "[net]\noffline = true\n")
        .expect("template cargo config should write");
    let main_rs = r#"use std::collections::BTreeMap;
use std::io::Read;

use toki_templates::{
    AttackMode, TemplateDescriptor, TemplateEnumOption, TemplateInstantiateRequest,
    TemplateParameter, TemplateParameterKind, TemplateProviderError, TemplateProviderErrorCode,
    TemplateProviderRequest, TemplateProviderResponse, TemplateSemanticItem, TemplateSemanticPlan,
    TemplateValue,
};

fn descriptor() -> TemplateDescriptor {
    TemplateDescriptor {
        id: "project/player_attack_clone".to_string(),
        display_name: "Project Attack".to_string(),
        category: "combat".to_string(),
        description: "Project-local attack template".to_string(),
        parameters: vec![
            TemplateParameter {
                id: "actor_entity_definition_id".to_string(),
                label: "Actor".to_string(),
                description: None,
                kind: TemplateParameterKind::EntityDefinitionReference,
                default: None,
                required: true,
            },
            TemplateParameter {
                id: "attack_mode".to_string(),
                label: "Attack Mode".to_string(),
                description: None,
                kind: TemplateParameterKind::Enum {
                    options: vec![TemplateEnumOption {
                        id: "melee".to_string(),
                        label: "Melee".to_string(),
                        description: None,
                    }],
                },
                default: Some(TemplateValue::Enum("melee".to_string())),
                required: true,
            },
            TemplateParameter {
                id: "damage".to_string(),
                label: "Damage".to_string(),
                description: None,
                kind: TemplateParameterKind::Integer { min: Some(1), max: Some(99), step: Some(1) },
                default: Some(TemplateValue::Integer(8)),
                required: true,
            },
            TemplateParameter {
                id: "cooldown_ticks".to_string(),
                label: "Cooldown".to_string(),
                description: None,
                kind: TemplateParameterKind::Integer { min: Some(1), max: Some(999), step: Some(1) },
                default: Some(TemplateValue::Integer(20)),
                required: true,
            },
        ],
    }
}

fn instantiate(request: TemplateInstantiateRequest) -> TemplateProviderResponse {
    let descriptor = descriptor();
    let mut values = BTreeMap::new();
    values.extend(request.parameters);
    if let Err(error) = descriptor.validate_parameters(&values) {
        return TemplateProviderResponse::Error {
            protocol_version: 1,
            error: TemplateProviderError::new(TemplateProviderErrorCode::InvalidParameters, error.to_string()),
        };
    }
    let actor = match values.get("actor_entity_definition_id") {
        Some(TemplateValue::EntityDefinitionReference(value)) => value.clone(),
        _ => String::new(),
    };
    let damage = match values.get("damage") {
        Some(TemplateValue::Integer(value)) => *value as u32,
        _ => 8,
    };
    let cooldown_ticks = match values.get("cooldown_ticks") {
        Some(TemplateValue::Integer(value)) => *value as u32,
        _ => 20,
    };
    TemplateProviderResponse::Instantiate {
        protocol_version: 1,
        descriptor,
        plan: TemplateSemanticPlan {
            semantic_version: 1,
            items: vec![TemplateSemanticItem::CreateAttackBehavior {
                id: "project_player_attack".to_string(),
                actor_entity_definition_id: Some(actor),
                trigger_input_action: "attack_primary".to_string(),
                mode: AttackMode::Melee,
                damage,
                cooldown_ticks,
                animation_state: None,
                projectile_entity_definition_id: None,
                sound_id: None,
            }],
        },
    }
}

fn main() {
    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).unwrap();
    let request: TemplateProviderRequest = serde_json::from_str(&stdin).unwrap();
    let response = match request {
        TemplateProviderRequest::List { .. } => TemplateProviderResponse::List {
            protocol_version: 1,
            templates: vec![descriptor()],
        },
        TemplateProviderRequest::Describe { .. } => TemplateProviderResponse::Describe {
            protocol_version: 1,
            descriptor: descriptor(),
        },
        TemplateProviderRequest::Instantiate { protocol_version, template_id, parameters } => {
            instantiate(TemplateInstantiateRequest { protocol_version, template_id, parameters })
        }
    };
    print!("{}", serde_json::to_string(&response).unwrap());
}
"#;
    std::fs::write(crate_dir.join("src/main.rs"), main_rs).expect("template main.rs should write");
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

#[test]
fn available_template_catalog_merges_project_templates_with_built_ins() {
    let temp = tempdir().expect("temp dir should exist");
    write_project_template_crate(temp.path());
    let project = Project::new("TestProject".to_string(), temp.path().to_path_buf());

    let catalog = available_template_catalog(Some(&project));
    assert!(catalog.diagnostics.is_empty());
    assert!(catalog
        .descriptors
        .iter()
        .any(|descriptor| descriptor.id == "toki/player_attack"));
    assert!(catalog
        .descriptors
        .iter()
        .any(|descriptor| descriptor.id == "project/player_attack_clone"));
}

#[test]
fn preview_and_apply_project_template_updates_entity_definition_via_runner() {
    let temp = tempdir().expect("temp dir should exist");
    write_player_entity(temp.path());
    write_project_template_crate(temp.path());
    let mut project = Project::new("TestProject".to_string(), temp.path().to_path_buf());
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let catalog = available_template_catalog(Some(&project));
    let mut state = TemplateEditorState {
        selected_template_id: Some("project/player_attack_clone".to_string()),
        ..TemplateEditorState::default()
    };
    sync_template_editor_state(&mut state, &catalog.descriptors);
    let params = state
        .parameters_by_template
        .get_mut("project/player_attack_clone")
        .expect("project template params should exist");
    params.insert(
        "actor_entity_definition_id".to_string(),
        TemplateValue::EntityDefinitionReference("player".to_string()),
    );
    params.insert("attack_mode".to_string(), TemplateValue::Enum("melee".to_string()));
    params.insert("damage".to_string(), TemplateValue::Integer(11));
    params.insert("cooldown_ticks".to_string(), TemplateValue::Integer(14));

    let preview = preview_selected_template(&state, &project).expect("preview should succeed");
    assert!(preview
        .lowered_summary_lines
        .iter()
        .any(|line| line.contains("entities/player.json")));

    let command = build_apply_template_command(&state, &project, None)
        .expect("apply command should build");
    let mut ui_state = crate::ui::EditorUI::new();
    assert!(ui_state.execute_command_with_project(&mut project, command));

    let updated = std::fs::read_to_string(temp.path().join("entities/player.json"))
        .expect("entity definition should read");
    assert!(updated.contains("\"damage\": 11"));
    assert_eq!(project.metadata.editor.template_applications.len(), 1);
    assert_eq!(
        project.metadata.editor.template_applications[0].template_id,
        "project/player_attack_clone"
    );
}

#[test]
fn build_delete_project_template_command_removes_starter_source_and_registry() {
    let temp = tempdir().expect("temp dir should exist");
    let starter = crate::project::build_template_starter_plan(temp.path(), "ProjectTest")
        .expect("starter plan should build");
    toki_template_lowering::apply_project_file_changes(temp.path(), &starter.changes)
        .expect("starter plan should apply");

    let mut project = Project::new("TestProject".to_string(), temp.path().to_path_buf());
    std::fs::write(
        project.project_file_path(),
        toml::to_string_pretty(&project.metadata).expect("project metadata should serialize"),
    )
    .expect("project metadata should write");

    let command = build_delete_project_template_command(
        &project,
        "project/projecttest",
        "ProjectTest",
        None,
    )
    .expect("delete command should build");

    let mut ui_state = crate::ui::EditorUI::new();
    assert!(ui_state.execute_command_with_project(&mut project, command));

    assert!(!temp
        .path()
        .join("templates/src/templates/projecttest.rs")
        .exists());
    let mod_rs = std::fs::read_to_string(temp.path().join("templates/src/templates/mod.rs"))
        .expect("mod.rs should read");
    assert!(!mod_rs.contains("pub mod projecttest;"));
    assert!(!mod_rs.contains("\"project/projecttest\""));
}
