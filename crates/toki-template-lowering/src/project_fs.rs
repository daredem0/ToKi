use std::fs;
use std::path::Path;

use toki_core::entity::EntityDefinition;
use toki_templates::TemplateSemanticPlan;

use crate::{
    EntityDefinitionResolver, LoweredTemplateOperation, LoweredTemplatePlan, TemplateLowerer,
    TemplateLoweringError, TemplateLoweringErrorCode,
};

pub struct ProjectFilesystemResolver<'a> {
    project_root: &'a Path,
}

impl<'a> ProjectFilesystemResolver<'a> {
    pub fn new(project_root: &'a Path) -> Self {
        Self { project_root }
    }

    fn entity_definition_path(&self, entity_definition_id: &str) -> std::path::PathBuf {
        self.project_root
            .join("entities")
            .join(format!("{entity_definition_id}.json"))
    }
}

impl EntityDefinitionResolver for ProjectFilesystemResolver<'_> {
    fn load_entity_definition(
        &self,
        entity_definition_id: &str,
    ) -> Result<Option<EntityDefinition>, TemplateLoweringError> {
        let path = self.entity_definition_path(entity_definition_id);
        if !path.exists() {
            return Ok(None);
        }

        let json_data = fs::read_to_string(&path).map_err(|error| {
            TemplateLoweringError::new(
                TemplateLoweringErrorCode::InvalidLoweringTarget,
                format!(
                    "failed to read entity definition '{}' at '{}': {}",
                    entity_definition_id,
                    path.display(),
                    error
                ),
            )
        })?;
        let definition = serde_json::from_str(&json_data).map_err(|error| {
            TemplateLoweringError::new(
                TemplateLoweringErrorCode::InvalidLoweringTarget,
                format!(
                    "failed to parse entity definition '{}' at '{}': {}",
                    entity_definition_id,
                    path.display(),
                    error
                ),
            )
        })?;
        Ok(Some(definition))
    }
}

pub fn lower_plan_for_project(
    project_root: &Path,
    plan: &TemplateSemanticPlan,
) -> Result<LoweredTemplatePlan, TemplateLoweringError> {
    let resolver = ProjectFilesystemResolver::new(project_root);
    TemplateLowerer::new().lower_plan(plan, &resolver)
}

pub fn apply_lowered_plan_to_project(
    project_root: &Path,
    lowered: &LoweredTemplatePlan,
) -> Result<(), TemplateLoweringError> {
    let entities_dir = project_root.join("entities");
    fs::create_dir_all(&entities_dir).map_err(|error| {
        TemplateLoweringError::new(
            TemplateLoweringErrorCode::ApplyFailed,
            format!(
                "failed to create entities directory '{}': {}",
                entities_dir.display(),
                error
            ),
        )
    })?;

    for operation in &lowered.operations {
        match operation {
            LoweredTemplateOperation::UpsertEntityDefinition {
                entity_definition_id,
                definition,
            } => {
                let path = entities_dir.join(format!("{entity_definition_id}.json"));
                let json_data = serde_json::to_string_pretty(definition).map_err(|error| {
                    TemplateLoweringError::new(
                        TemplateLoweringErrorCode::ApplyFailed,
                        format!(
                            "failed to serialize entity definition '{}' for '{}': {}",
                            definition.name,
                            path.display(),
                            error
                        ),
                    )
                })?;
                fs::write(&path, json_data).map_err(|error| {
                    TemplateLoweringError::new(
                        TemplateLoweringErrorCode::ApplyFailed,
                        format!(
                            "failed to write entity definition '{}' to '{}': {}",
                            definition.name,
                            path.display(),
                            error
                        ),
                    )
                })?;
            }
        }
    }

    Ok(())
}

pub fn lower_and_apply_plan_to_project(
    project_root: &Path,
    plan: &TemplateSemanticPlan,
) -> Result<(), TemplateLoweringError> {
    let lowered = lower_plan_for_project(project_root, plan)?;
    apply_lowered_plan_to_project(project_root, &lowered)
}
