use std::fs;
use std::path::{Path, PathBuf};

use toki_core::entity::EntityDefinition;
use toki_templates::TemplateSemanticPlan;

use crate::{
    EntityDefinitionResolver, LoweredTemplateOperation, LoweredTemplatePlan, TemplateLowerer,
    TemplateLoweringError, TemplateLoweringErrorCode,
};

pub struct ProjectFilesystemResolver<'a> {
    project_root: &'a Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFileChange {
    pub relative_path: PathBuf,
    pub before_contents: Option<String>,
    pub after_contents: Option<String>,
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
    let changes = build_project_file_changes(project_root, lowered)?;
    apply_project_file_changes(project_root, &changes)
}

pub fn lower_and_apply_plan_to_project(
    project_root: &Path,
    plan: &TemplateSemanticPlan,
) -> Result<(), TemplateLoweringError> {
    let lowered = lower_plan_for_project(project_root, plan)?;
    apply_lowered_plan_to_project(project_root, &lowered)
}

pub fn build_project_file_changes(
    project_root: &Path,
    lowered: &LoweredTemplatePlan,
) -> Result<Vec<ProjectFileChange>, TemplateLoweringError> {
    let mut changes = Vec::new();

    for operation in &lowered.operations {
        match operation {
            LoweredTemplateOperation::UpsertEntityDefinition {
                entity_definition_id,
                definition,
            } => {
                let relative_path =
                    PathBuf::from("entities").join(format!("{entity_definition_id}.json"));
                let absolute_path = project_root.join(&relative_path);
                let before_contents = if absolute_path.exists() {
                    Some(fs::read_to_string(&absolute_path).map_err(|error| {
                        TemplateLoweringError::new(
                            TemplateLoweringErrorCode::ApplyFailed,
                            format!(
                                "failed to read existing entity definition '{}': {}",
                                absolute_path.display(),
                                error
                            ),
                        )
                    })?)
                } else {
                    None
                };
                let after_contents =
                    Some(serde_json::to_string_pretty(definition).map_err(|error| {
                        TemplateLoweringError::new(
                            TemplateLoweringErrorCode::ApplyFailed,
                            format!(
                                "failed to serialize lowered entity definition '{}' for '{}': {}",
                                definition.name,
                                absolute_path.display(),
                                error
                            ),
                        )
                    })?);

                changes.push(ProjectFileChange {
                    relative_path,
                    before_contents,
                    after_contents,
                });
            }
        }
    }

    Ok(changes)
}

pub fn apply_project_file_changes(
    project_root: &Path,
    changes: &[ProjectFileChange],
) -> Result<(), TemplateLoweringError> {
    apply_project_file_changes_direction(project_root, changes, ProjectFileChangeDirection::Forward)
}

pub fn revert_project_file_changes(
    project_root: &Path,
    changes: &[ProjectFileChange],
) -> Result<(), TemplateLoweringError> {
    apply_project_file_changes_direction(project_root, changes, ProjectFileChangeDirection::Reverse)
}

#[derive(Debug, Clone, Copy)]
enum ProjectFileChangeDirection {
    Forward,
    Reverse,
}

fn apply_project_file_changes_direction(
    project_root: &Path,
    changes: &[ProjectFileChange],
    direction: ProjectFileChangeDirection,
) -> Result<(), TemplateLoweringError> {
    let mut applied: Vec<&ProjectFileChange> = Vec::new();

    for change in changes {
        if let Err(error) = write_project_file_change(project_root, change, direction) {
            for already_applied in applied.iter().rev() {
                let _ = write_project_file_change(
                    project_root,
                    already_applied,
                    reverse_direction(direction),
                );
            }
            return Err(error);
        }
        applied.push(change);
    }

    Ok(())
}

fn write_project_file_change(
    project_root: &Path,
    change: &ProjectFileChange,
    direction: ProjectFileChangeDirection,
) -> Result<(), TemplateLoweringError> {
    let absolute_path = project_root.join(&change.relative_path);
    let target_contents = match direction {
        ProjectFileChangeDirection::Forward => &change.after_contents,
        ProjectFileChangeDirection::Reverse => &change.before_contents,
    };

    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            TemplateLoweringError::new(
                TemplateLoweringErrorCode::ApplyFailed,
                format!(
                    "failed to create parent directory '{}' for '{}': {}",
                    parent.display(),
                    absolute_path.display(),
                    error
                ),
            )
        })?;
    }

    match target_contents {
        Some(contents) => fs::write(&absolute_path, contents).map_err(|error| {
            TemplateLoweringError::new(
                TemplateLoweringErrorCode::ApplyFailed,
                format!(
                    "failed to write '{}' during template apply: {}",
                    absolute_path.display(),
                    error
                ),
            )
        }),
        None => {
            if absolute_path.exists() {
                fs::remove_file(&absolute_path).map_err(|error| {
                    TemplateLoweringError::new(
                        TemplateLoweringErrorCode::ApplyFailed,
                        format!(
                            "failed to remove '{}' during template apply: {}",
                            absolute_path.display(),
                            error
                        ),
                    )
                })?;
            }
            Ok(())
        }
    }
}

fn reverse_direction(direction: ProjectFileChangeDirection) -> ProjectFileChangeDirection {
    match direction {
        ProjectFileChangeDirection::Forward => ProjectFileChangeDirection::Reverse,
        ProjectFileChangeDirection::Reverse => ProjectFileChangeDirection::Forward,
    }
}
