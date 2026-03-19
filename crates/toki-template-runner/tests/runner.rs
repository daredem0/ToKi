use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use toki_template_runner::{ProjectTemplateProvider, ProjectTemplateSettings};
use toki_templates::{TemplateProvider, TemplateProviderErrorCode, TemplateValue};

fn toki_templates_crate_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("toki-templates")
        .canonicalize()
        .expect("toki-templates crate path should resolve")
}

fn write_fixture_template_crate(project_root: &Path, protocol_version: u32) {
    let crate_dir = project_root.join("templates");
    fs::create_dir_all(crate_dir.join("src")).expect("template src dir should exist");
    fs::create_dir_all(crate_dir.join(".cargo")).expect("template cargo config dir should exist");
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
    fs::write(crate_dir.join("Cargo.toml"), manifest).expect("fixture Cargo.toml should write");
    fs::write(crate_dir.join(".cargo/config.toml"), "[net]\noffline = true\n")
        .expect("fixture cargo config should write");
    let main_rs = format!(
        r#"use std::collections::BTreeMap;
use std::io::Read;

use toki_templates::{{
    AttackMode, TemplateDescriptor, TemplateEnumOption, TemplateInstantiateRequest,
    TemplateParameter, TemplateParameterKind, TemplateProviderError, TemplateProviderErrorCode,
    TemplateProviderRequest, TemplateProviderResponse, TemplateSemanticItem, TemplateSemanticPlan,
    TemplateValue,
}};

fn descriptor() -> TemplateDescriptor {{
    TemplateDescriptor {{
        id: "project/player_attack_clone".to_string(),
        display_name: "Project Attack".to_string(),
        category: "combat".to_string(),
        description: "Project-local attack template".to_string(),
        parameters: vec![
            TemplateParameter {{
                id: "actor_entity_definition_id".to_string(),
                label: "Actor".to_string(),
                description: None,
                kind: TemplateParameterKind::EntityDefinitionReference,
                default: None,
                required: true,
            }},
            TemplateParameter {{
                id: "attack_mode".to_string(),
                label: "Attack Mode".to_string(),
                description: None,
                kind: TemplateParameterKind::Enum {{
                    options: vec![TemplateEnumOption {{
                        id: "melee".to_string(),
                        label: "Melee".to_string(),
                        description: None,
                    }}],
                }},
                default: Some(TemplateValue::Enum("melee".to_string())),
                required: true,
            }},
            TemplateParameter {{
                id: "damage".to_string(),
                label: "Damage".to_string(),
                description: None,
                kind: TemplateParameterKind::Integer {{ min: Some(1), max: Some(99), step: Some(1) }},
                default: Some(TemplateValue::Integer(8)),
                required: true,
            }},
            TemplateParameter {{
                id: "cooldown_ticks".to_string(),
                label: "Cooldown".to_string(),
                description: None,
                kind: TemplateParameterKind::Integer {{ min: Some(1), max: Some(999), step: Some(1) }},
                default: Some(TemplateValue::Integer(20)),
                required: true,
            }},
        ],
    }}
}}

fn instantiate(request: TemplateInstantiateRequest) -> TemplateProviderResponse {{
    let descriptor = descriptor();
    let mut values = BTreeMap::new();
    values.extend(request.parameters);
    if let Err(error) = descriptor.validate_parameters(&values) {{
        return TemplateProviderResponse::Error {{
            protocol_version: {protocol_version},
            error: TemplateProviderError::new(TemplateProviderErrorCode::InvalidParameters, error.to_string()),
        }};
    }}
    let actor = match values.get("actor_entity_definition_id") {{
        Some(TemplateValue::EntityDefinitionReference(value)) => value.clone(),
        _ => String::new(),
    }};
    let damage = match values.get("damage") {{
        Some(TemplateValue::Integer(value)) => *value as u32,
        _ => 8,
    }};
    let cooldown_ticks = match values.get("cooldown_ticks") {{
        Some(TemplateValue::Integer(value)) => *value as u32,
        _ => 20,
    }};
    TemplateProviderResponse::Instantiate {{
        protocol_version: {protocol_version},
        descriptor,
        plan: TemplateSemanticPlan {{
            semantic_version: 1,
            items: vec![TemplateSemanticItem::CreateAttackBehavior {{
                id: "project_player_attack".to_string(),
                actor_entity_definition_id: Some(actor),
                trigger_input_action: "attack_primary".to_string(),
                mode: AttackMode::Melee,
                damage,
                cooldown_ticks,
                animation_state: None,
                projectile_entity_definition_id: None,
                sound_id: None,
            }}],
        }},
    }}
}}

fn main() {{
    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).unwrap();
    let request: TemplateProviderRequest = serde_json::from_str(&stdin).unwrap();
    let response = match request {{
        TemplateProviderRequest::List {{ .. }} => TemplateProviderResponse::List {{
            protocol_version: {protocol_version},
            templates: vec![descriptor()],
        }},
        TemplateProviderRequest::Describe {{ .. }} => TemplateProviderResponse::Describe {{
            protocol_version: {protocol_version},
            descriptor: descriptor(),
        }},
        TemplateProviderRequest::Instantiate {{ protocol_version, template_id, parameters }} => instantiate(TemplateInstantiateRequest {{ protocol_version, template_id, parameters }}),
    }};
    print!("{{}}", serde_json::to_string(&response).unwrap());
}}
"#,
    );
    fs::write(crate_dir.join("src/main.rs"), main_rs).expect("fixture main.rs should write");
}

#[test]
fn detect_project_template_provider_uses_default_templates_convention() {
    let temp = tempdir().expect("temp dir should exist");
    write_fixture_template_crate(temp.path(), 1);

    let provider = ProjectTemplateProvider::detect(temp.path(), &ProjectTemplateSettings::default())
        .expect("detection should succeed");
    assert!(provider.is_some());
}

#[test]
fn project_template_provider_lists_and_instantiates_templates_via_protocol() {
    let temp = tempdir().expect("temp dir should exist");
    write_fixture_template_crate(temp.path(), 1);

    let provider = ProjectTemplateProvider::detect(temp.path(), &ProjectTemplateSettings::default())
        .expect("detection should succeed")
        .expect("provider should exist");
    let descriptors = provider
        .list_templates()
        .expect("listing should succeed");
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].id, "project/player_attack_clone");

    let mut parameters = BTreeMap::new();
    parameters.insert(
        "actor_entity_definition_id".to_string(),
        TemplateValue::EntityDefinitionReference("player".to_string()),
    );
    parameters.insert("attack_mode".to_string(), TemplateValue::Enum("melee".to_string()));
    parameters.insert("damage".to_string(), TemplateValue::Integer(7));
    parameters.insert("cooldown_ticks".to_string(), TemplateValue::Integer(16));
    let instantiation = provider
        .instantiate_template("project/player_attack_clone", parameters)
        .expect("instantiation should succeed");
    assert_eq!(instantiation.descriptor.id, "project/player_attack_clone");
    assert_eq!(instantiation.plan.items.len(), 1);
}

#[test]
fn project_template_provider_updates_cache_fingerprint_when_sources_change() {
    let temp = tempdir().expect("temp dir should exist");
    write_fixture_template_crate(temp.path(), 1);

    let provider = ProjectTemplateProvider::detect(temp.path(), &ProjectTemplateSettings::default())
        .expect("detection should succeed")
        .expect("provider should exist");
    provider.list_templates().expect("initial listing should succeed");
    let cache_path = temp.path().join(".toki/project_template_runner_cache.json");
    let first_cache = fs::read_to_string(&cache_path).expect("cache should exist after build");

    fs::write(
        temp.path().join("templates/src/main.rs"),
        fs::read_to_string(temp.path().join("templates/src/main.rs"))
            .expect("main.rs should read")
            + "\n// fingerprint change\n",
    )
    .expect("main.rs should update");

    provider.list_templates().expect("listing after change should succeed");
    let second_cache = fs::read_to_string(&cache_path).expect("cache should still exist");
    assert_ne!(first_cache, second_cache);
}

#[test]
fn project_template_provider_rejects_protocol_version_mismatch() {
    let temp = tempdir().expect("temp dir should exist");
    write_fixture_template_crate(temp.path(), 999);

    let provider = ProjectTemplateProvider::detect(temp.path(), &ProjectTemplateSettings::default())
        .expect("detection should succeed")
        .expect("provider should exist");
    let error = provider
        .list_templates()
        .expect_err("protocol mismatch should fail");
    assert_eq!(error.code, TemplateProviderErrorCode::UnsupportedProtocolVersion);
}
