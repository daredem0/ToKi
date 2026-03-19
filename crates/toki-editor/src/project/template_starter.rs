use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use toki_template_lowering::ProjectFileChange;

const MODULE_MARKER: &str = "// __TOKI_TEMPLATE_MODULES__";
const LIST_MARKER: &str = "// __TOKI_TEMPLATE_LIST__";
const DESCRIBE_MARKER: &str = "// __TOKI_TEMPLATE_DESCRIBE__";
const INSTANTIATE_MARKER: &str = "// __TOKI_TEMPLATE_INSTANTIATE__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateStarterPlan {
    pub template_id: String,
    pub display_name: String,
    pub changes: Vec<ProjectFileChange>,
}

pub fn build_template_starter_plan(project_root: &Path, requested_name: &str) -> Result<TemplateStarterPlan> {
    let display_name = requested_name.trim().to_string();
    if display_name.is_empty() {
        return Err(anyhow!("template name must not be empty"));
    }
    let slug = template_slug(&display_name);
    if slug.is_empty() {
        return Err(anyhow!("template name must contain at least one letter or digit"));
    }

    let crate_dir = project_root.join("templates");
    let cargo_relative = PathBuf::from("templates/Cargo.toml");
    let main_relative = PathBuf::from("templates/src/main.rs");
    let mod_relative = PathBuf::from("templates/src/templates/mod.rs");
    let template_relative = PathBuf::from("templates/src/templates").join(format!("{slug}.rs"));
    let template_absolute = project_root.join(&template_relative);
    if template_absolute.exists() {
        return Err(anyhow!(
            "template starter '{}' already exists at '{}'",
            slug,
            template_absolute.display()
        ));
    }

    let mut changes = Vec::new();
    let dependency_path = relative_path_string(
        &crate_dir,
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("toki-templates")
            .canonicalize()?,
    )?;

    if !project_root.join(&cargo_relative).exists() {
        changes.push(ProjectFileChange {
            relative_path: cargo_relative,
            before_contents: None,
            after_contents: Some(cargo_toml_contents(&dependency_path)),
        });
        changes.push(ProjectFileChange {
            relative_path: main_relative,
            before_contents: None,
            after_contents: Some(main_rs_contents()),
        });
        changes.push(ProjectFileChange {
            relative_path: mod_relative.clone(),
            before_contents: None,
            after_contents: Some(mod_rs_contents(std::slice::from_ref(&slug))),
        });
    } else {
        let mod_absolute = project_root.join(&mod_relative);
        let before_mod = std::fs::read_to_string(&mod_absolute).map_err(|error| {
            anyhow!(
                "failed to read existing template registry '{}': {}",
                mod_absolute.display(),
                error
            )
        })?;
        ensure_starter_managed_registry(&before_mod)?;
        let mut slugs = collect_template_module_slugs(&project_root.join("templates/src/templates"))?;
        if !slugs.iter().any(|existing| existing == &slug) {
            slugs.push(slug.clone());
        }
        let after_mod = mod_rs_contents(&slugs);
        changes.push(ProjectFileChange {
            relative_path: mod_relative,
            before_contents: Some(before_mod),
            after_contents: Some(after_mod),
        });
    }

    changes.push(ProjectFileChange {
        relative_path: template_relative,
        before_contents: None,
        after_contents: Some(template_rs_contents(&slug, &display_name)),
    });

    Ok(TemplateStarterPlan {
        template_id: format!("project/{slug}"),
        display_name,
        changes,
    })
}

pub fn build_remove_template_starter_plan(
    project_root: &Path,
    template_id: &str,
    display_name: &str,
) -> Result<TemplateStarterPlan> {
    let slug = template_id
        .strip_prefix("project/")
        .ok_or_else(|| anyhow!("only project/... template ids can be deleted"))?
        .trim()
        .to_string();
    if slug.is_empty() {
        return Err(anyhow!("template id must include a project template slug"));
    }

    let mod_relative = PathBuf::from("templates/src/templates/mod.rs");
    let template_relative = PathBuf::from("templates/src/templates").join(format!("{slug}.rs"));
    let mod_absolute = project_root.join(&mod_relative);
    let template_absolute = project_root.join(&template_relative);

    let before_mod = std::fs::read_to_string(&mod_absolute).map_err(|error| {
        anyhow!(
            "failed to read existing template registry '{}': {}",
            mod_absolute.display(),
            error
        )
    })?;
    ensure_starter_managed_registry(&before_mod)?;
    let mut remaining_slugs = collect_template_module_slugs(&project_root.join("templates/src/templates"))?;
    let removed_from_files = if let Some(index) = remaining_slugs.iter().position(|existing| existing == &slug) {
        remaining_slugs.remove(index);
        true
    } else {
        false
    };
    let removed_from_registry = before_mod.contains(&format!("\"project/{slug}\""))
        || before_mod.contains(&format!("pub mod {slug};"));
    if !removed_from_files && !removed_from_registry {
        return Err(anyhow!(
            "project template '{}' is not present in the starter-managed template crate",
            template_id
        ));
    }

    let mut changes = Vec::new();
    let after_mod = mod_rs_contents(&remaining_slugs);
    if before_mod != after_mod {
        changes.push(ProjectFileChange {
            relative_path: mod_relative,
            before_contents: Some(before_mod),
            after_contents: Some(after_mod),
        });
    }
    if template_absolute.exists() {
        let before_contents = std::fs::read_to_string(&template_absolute).map_err(|error| {
            anyhow!(
                "failed to read template source '{}': {}",
                template_absolute.display(),
                error
            )
        })?;
        changes.push(ProjectFileChange {
            relative_path: template_relative,
            before_contents: Some(before_contents),
            after_contents: None,
        });
    }

    Ok(TemplateStarterPlan {
        template_id: template_id.to_string(),
        display_name: display_name.to_string(),
        changes,
    })
}

pub fn template_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator && !slug.is_empty() {
            slug.push('_');
            previous_was_separator = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    slug
}

fn cargo_toml_contents(dependency_path: &str) -> String {
    format!(
        r#"[package]
name = "project-templates"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
serde_json = "1.0"
toki-templates = {{ path = "{dependency_path}" }}
"#
    )
}

fn main_rs_contents() -> String {
    r#"use std::io::Read;

mod templates;

use toki_templates::{
    TemplateProviderRequest, TemplateProviderResponse, TEMPLATE_PROTOCOL_VERSION,
};

fn main() {
    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).unwrap();
    let request: TemplateProviderRequest = serde_json::from_str(&stdin).unwrap();

    let response = match request {
        TemplateProviderRequest::List { .. } => TemplateProviderResponse::List {
            protocol_version: TEMPLATE_PROTOCOL_VERSION,
            templates: templates::list_templates(),
        },
        TemplateProviderRequest::Describe { template_id, .. } => match templates::describe_template(&template_id) {
            Ok(descriptor) => TemplateProviderResponse::Describe {
                protocol_version: TEMPLATE_PROTOCOL_VERSION,
                descriptor,
            },
            Err(error) => TemplateProviderResponse::Error {
                protocol_version: TEMPLATE_PROTOCOL_VERSION,
                error,
            },
        },
        TemplateProviderRequest::Instantiate { template_id, parameters, .. } => {
            match templates::instantiate_template(&template_id, parameters) {
                Ok(instantiation) => TemplateProviderResponse::Instantiate {
                    protocol_version: TEMPLATE_PROTOCOL_VERSION,
                    descriptor: instantiation.descriptor,
                    plan: instantiation.plan,
                },
                Err(error) => TemplateProviderResponse::Error {
                    protocol_version: TEMPLATE_PROTOCOL_VERSION,
                    error,
                },
            }
        }
    };

    print!("{}", serde_json::to_string(&response).unwrap());
}
"#
    .to_string()
}

fn mod_rs_contents(slugs: &[String]) -> String {
    let mut sorted_slugs = slugs.to_vec();
    sorted_slugs.sort();
    let module_lines = sorted_slugs
        .iter()
        .map(|slug| format!("pub mod {slug};"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let list_lines = sorted_slugs
        .iter()
        .map(|slug| format!("        {slug}::descriptor(),"))
        .collect::<Vec<_>>()
        .join("\n");
    let describe_lines = sorted_slugs
        .iter()
        .map(|slug| format!("        \"project/{slug}\" => Ok({slug}::descriptor()),"))
        .collect::<Vec<_>>()
        .join("\n");
    let instantiate_lines = sorted_slugs
        .iter()
        .map(|slug| {
            format!(
                "        \"project/{slug}\" => {slug}::instantiate(parameters)\n            .map(|plan| TemplateInstantiation {{\n                descriptor: {slug}::descriptor(),\n                plan,\n            }}),"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        r#"use std::collections::BTreeMap;

use toki_templates::{{
    TemplateDescriptor, TemplateInstantiation, TemplateProviderError, TemplateProviderErrorCode,
    TemplateValue,
}};

{MODULE_MARKER}
{module_lines}

pub fn list_templates() -> Vec<TemplateDescriptor> {{
    vec![
        {LIST_MARKER}
{list_lines}
    ]
}}

pub fn describe_template(template_id: &str) -> Result<TemplateDescriptor, TemplateProviderError> {{
    match template_id {{
        {DESCRIBE_MARKER}
{describe_lines}
        _ => Err(TemplateProviderError::new(
            TemplateProviderErrorCode::TemplateNotFound,
            format!("unknown project template '{{template_id}}'"),
        )),
    }}
}}

pub fn instantiate_template(
    template_id: &str,
    parameters: BTreeMap<String, TemplateValue>,
) -> Result<TemplateInstantiation, TemplateProviderError> {{
    match template_id {{
        {INSTANTIATE_MARKER}
{instantiate_lines}
        _ => Err(TemplateProviderError::new(
            TemplateProviderErrorCode::TemplateNotFound,
            format!("unknown project template '{{template_id}}'"),
        )),
    }}
}}
"#
    )
}

fn template_rs_contents(slug: &str, display_name: &str) -> String {
    format!(
        r#"use std::collections::BTreeMap;

use toki_templates::{{
    TemplateDescriptor, TemplateParameter, TemplateParameterKind, TemplateProviderError,
    TemplateProviderErrorCode, TemplateSemanticPlan, TemplateValue,
}};

pub fn descriptor() -> TemplateDescriptor {{
    TemplateDescriptor {{
        id: "project/{slug}".to_string(),
        display_name: "{display_name}".to_string(),
        category: "custom".to_string(),
        description: "Template starter generated by ToKi. Replace this with a real description once you know what the template should create.".to_string(),
        parameters: vec![
            TemplateParameter {{
                id: "example_name".to_string(),
                label: "Example Name".to_string(),
                description: Some("Rename or remove this parameter once the template has a real purpose.".to_string()),
                kind: TemplateParameterKind::String {{
                    multiline: false,
                    min_length: Some(1),
                    max_length: Some(64),
                }},
                default: Some(TemplateValue::String("replace_me".to_string())),
                required: true,
            }},
        ],
    }}
}}

pub fn instantiate(
    parameters: BTreeMap<String, TemplateValue>,
) -> Result<TemplateSemanticPlan, TemplateProviderError> {{
    let descriptor = descriptor();
    let resolved_parameters = descriptor.resolve_parameters(&parameters).map_err(|error| {{
        TemplateProviderError::new(
            TemplateProviderErrorCode::InvalidParameters,
            error.to_string(),
        )
    }})?;

    let _example_name = match resolved_parameters.get("example_name") {{
        Some(TemplateValue::String(value)) => value,
        _ => {{
            return Err(TemplateProviderError::new(
                TemplateProviderErrorCode::InvalidParameters,
                "example_name must be a string",
            ))
        }}
    }};

    // TODO: Replace this stub with real semantic output.
    //
    // A TemplateSemanticPlan is a list of semantic items from toki_templates
    // that say what should exist after the template is applied.
    //
    // Think in terms of intent, not editor wiring. Examples:
    //
    // - "this entity definition should gain a melee attack"
    //   -> TemplateSemanticItem::CreateAttackBehavior {{ ... }}
    //
    // - "this pickup should grant an inventory item"
    //   -> TemplateSemanticItem::CreatePickupBehavior {{ ... }}
    //
    // - "this menu flow should have an exit confirmation dialog"
    //   -> TemplateSemanticItem::CreateConfirmationDialog {{ ... }}
    //
    // The template code should decide *what* behavior/content is needed and
    // provide the important parameters for it. ToKi then lowers that semantic
    // intent into authored data such as entity config, menus, dialogs, rules,
    // file changes, layout, and undoable editor mutations.
    //
    // Minimal shape:
    //
    // Ok(TemplateSemanticPlan {{
    //     semantic_version: 1,
    //     items: vec![
    //         // TemplateSemanticItem::CreateAttackBehavior {{ ... }},
    //     ],
    // }})
    //
    // Do not do these things here:
    // - do not create graph nodes or edges directly
    // - do not write files directly
    // - do not depend on editor-only wiring or layout details
    //
    // Good examples to look at:
    // - crates/toki-template-builtins/src/templates/player_attack.rs
    // - crates/toki-template-builtins/src/templates/pickup_collect.rs
    // - crates/toki-template-builtins/src/templates/exit_confirmation_dialog.rs
    Err(TemplateProviderError::new(
        TemplateProviderErrorCode::SemanticValidation,
        "TODO: implement this template starter by returning a semantic template plan",
    ))
}}
"#
    )
}

fn ensure_starter_managed_registry(contents: &str) -> Result<()> {
    for marker in [MODULE_MARKER, LIST_MARKER, DESCRIBE_MARKER, INSTANTIATE_MARKER] {
        if !contents.contains(marker) {
            return Err(anyhow!(
                "starter-managed template registry marker '{}' is missing",
                marker
            ));
        }
    }
    Ok(())
}

fn collect_template_module_slugs(template_dir: &Path) -> Result<Vec<String>> {
    let mut slugs = std::fs::read_dir(template_dir)
        .map_err(|error| anyhow!("failed to read template source directory '{}': {}", template_dir.display(), error))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let extension = path.extension()?.to_str()?;
            if extension != "rs" {
                return None;
            }
            let stem = path.file_stem()?.to_str()?;
            if stem == "mod" {
                return None;
            }
            Some(stem.to_string())
        })
        .collect::<Vec<_>>();
    slugs.sort();
    Ok(slugs)
}

fn relative_path_string(from_dir: &Path, to_path: &Path) -> Result<String> {
    let from = from_dir.canonicalize().or_else(|_| Ok::<_, std::io::Error>(from_dir.to_path_buf()))?;
    let to = to_path.canonicalize()?;

    let from_components = from.components().collect::<Vec<_>>();
    let to_components = to.components().collect::<Vec<_>>();
    let shared_len = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut relative = PathBuf::new();
    for _ in shared_len..from_components.len() {
        relative.push("..");
    }
    for component in &to_components[shared_len..] {
        relative.push(component.as_os_str());
    }

    if relative.as_os_str().is_empty() {
        return Ok(".".to_string());
    }

    Ok(relative.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn build_template_starter_plan_creates_scaffold_when_template_crate_is_missing() {
        let temp = tempdir().expect("temp dir should exist");
        let plan = build_template_starter_plan(temp.path(), "My Attack").expect("plan should build");
        assert_eq!(plan.template_id, "project/my_attack");
        assert!(plan.changes.iter().any(|change| change.relative_path == Path::new("templates/Cargo.toml")));
        assert!(plan.changes.iter().any(|change| change.relative_path == Path::new("templates/src/main.rs")));
        assert!(plan.changes.iter().any(|change| change.relative_path == Path::new("templates/src/templates/mod.rs")));
        assert!(plan.changes.iter().any(|change| change.relative_path == Path::new("templates/src/templates/my_attack.rs")));
        let template_change = plan
            .changes
            .iter()
            .find(|change| change.relative_path == Path::new("templates/src/templates/my_attack.rs"))
            .expect("starter template file should exist");
        let template_contents = template_change
            .after_contents
            .as_deref()
            .expect("starter template file should have contents");
        assert!(template_contents.contains("A TemplateSemanticPlan is a list of semantic items"));
        assert!(template_contents.contains("TemplateSemanticItem::CreateAttackBehavior"));
        assert!(template_contents.contains("TemplateSemanticItem::CreatePickupBehavior"));
        assert!(template_contents.contains("TemplateSemanticItem::CreateConfirmationDialog"));
        assert!(template_contents.contains("Minimal shape:"));
        assert!(template_contents.contains("do not create graph nodes or edges directly"));
        assert!(template_contents.contains("ToKi then lowers that semantic"));
    }

    #[test]
    fn build_template_starter_plan_appends_template_to_existing_starter_registry() {
        let temp = tempdir().expect("temp dir should exist");
        let first = build_template_starter_plan(temp.path(), "First Template").expect("first plan should build");
        toki_template_lowering::apply_project_file_changes(temp.path(), &first.changes)
            .expect("first starter should apply");

        let second = build_template_starter_plan(temp.path(), "Second Template").expect("second plan should build");
        let mod_change = second
            .changes
            .iter()
            .find(|change| change.relative_path == Path::new("templates/src/templates/mod.rs"))
            .expect("mod.rs update should exist");
        let after = mod_change.after_contents.as_ref().expect("updated mod.rs should exist");
        assert!(after.contains("pub mod first_template;"));
        assert!(after.contains("pub mod second_template;"));
        assert!(after.contains("\"project/first_template\" => Ok(first_template::descriptor())"));
        assert!(after.contains("\"project/second_template\" => Ok(second_template::descriptor())"));
        assert!(
            after.contains(
                "\"project/second_template\" => second_template::instantiate(parameters)\n            .map(|plan| TemplateInstantiation {\n                descriptor: second_template::descriptor(),\n                plan,\n            }),"
            )
        );
        assert!(!after.contains("\\n"));
    }

    #[test]
    fn build_remove_template_starter_plan_deletes_source_and_registry_entries() {
        let temp = tempdir().expect("temp dir should exist");
        let first = build_template_starter_plan(temp.path(), "First Template").expect("first plan should build");
        toki_template_lowering::apply_project_file_changes(temp.path(), &first.changes)
            .expect("first starter should apply");
        let second = build_template_starter_plan(temp.path(), "Second Template").expect("second plan should build");
        toki_template_lowering::apply_project_file_changes(temp.path(), &second.changes)
            .expect("second starter should apply");

        let removal = build_remove_template_starter_plan(
            temp.path(),
            "project/first_template",
            "First Template",
        )
        .expect("removal plan should build");

        let mod_change = removal
            .changes
            .iter()
            .find(|change| change.relative_path == Path::new("templates/src/templates/mod.rs"))
            .expect("mod.rs update should exist");
        let after = mod_change.after_contents.as_ref().expect("updated mod.rs should exist");
        assert!(!after.contains("pub mod first_template;"));
        assert!(after.contains("pub mod second_template;"));
        assert!(!after.contains("\"project/first_template\""));
        assert!(after.contains("\"project/second_template\""));
        assert!(removal
            .changes
            .iter()
            .any(|change| change.relative_path == Path::new("templates/src/templates/first_template.rs")
                && change.after_contents.is_none()));
    }

    #[test]
    fn template_slug_normalizes_human_friendly_names() {
        assert_eq!(template_slug("Heavy Attack"), "heavy_attack");
        assert_eq!(template_slug("  Boss-Attack!!  "), "boss_attack");
    }
}
