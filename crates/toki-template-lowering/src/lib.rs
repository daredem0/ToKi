mod error;
mod lowering;
mod project_fs;

pub use error::{TemplateLoweringError, TemplateLoweringErrorCode};
pub use lowering::{
    EntityDefinitionResolver, LoweredTemplateOperation, LoweredTemplatePlan, TemplateLowerer,
};
pub use project_fs::{
    apply_lowered_plan_to_project, apply_project_file_changes, build_project_file_changes,
    lower_and_apply_plan_to_project, lower_plan_for_project, revert_project_file_changes,
    ProjectFileChange, ProjectFilesystemResolver,
};
