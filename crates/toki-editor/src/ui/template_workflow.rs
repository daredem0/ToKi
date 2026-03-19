use std::collections::{BTreeMap, BTreeSet};

use crate::project::{Project, ProjectAssets, TemplateApplicationRecord};
use crate::ui::editor_ui::Selection;
use crate::ui::undo_redo::EditorCommand;
use toki_template_builtins::BuiltInTemplateRegistry;
use toki_template_lowering::{
    build_project_file_changes, lower_plan_for_project, ProjectFileChange,
};
use toki_template_runner::ProjectTemplateProvider;
use toki_templates::{
    AssetReferenceKind, TemplateDescriptor, TemplateInstantiation, TemplateParameter,
    TemplateParameterKind, TemplateProvider, TemplateSemanticItem, TemplateValue,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TemplateEditorState {
    pub selected_template_id: Option<String>,
    pub category_filter: Option<String>,
    pub parameters_by_template: BTreeMap<String, BTreeMap<String, TemplateValue>>,
    pub last_error: Option<String>,
    pub last_success: Option<String>,
    pub selected_application_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplateAssetChoices {
    pub entity_definition_ids: Vec<String>,
    pub entity_animation_states: BTreeMap<String, Vec<String>>,
    pub scene_ids: Vec<String>,
    pub tilemap_ids: Vec<String>,
    pub sprite_atlas_ids: Vec<String>,
    pub object_sheet_ids: Vec<String>,
    pub audio_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplatePreview {
    pub descriptor: TemplateDescriptor,
    pub instantiation: TemplateInstantiation,
    pub file_changes: Vec<ProjectFileChange>,
    pub semantic_summary_lines: Vec<String>,
    pub lowered_summary_lines: Vec<String>,
    pub selection_after_apply: Option<Selection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateWorkflowError {
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TemplateCatalog {
    pub descriptors: Vec<TemplateDescriptor>,
    pub diagnostics: Vec<String>,
}

impl TemplateWorkflowError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl TemplateAssetChoices {
    pub fn from_project_assets(assets: &mut ProjectAssets) -> Self {
        let mut entity_definition_ids = assets.get_entity_names();
        entity_definition_ids.sort();
        let mut entity_animation_states = BTreeMap::new();
        for entity_definition_id in &entity_definition_ids {
            if let Ok(Some(definition)) = assets.load_entity_definition(entity_definition_id) {
                let mut states = definition
                    .animations
                    .clips
                    .iter()
                    .map(|clip| clip.state.clone())
                    .collect::<Vec<_>>();
                states.sort();
                states.dedup();
                entity_animation_states.insert(entity_definition_id.clone(), states);
            }
        }
        let mut scene_ids = assets.get_scene_names();
        scene_ids.sort();
        let mut tilemap_ids = assets.get_tilemap_names();
        tilemap_ids.sort();
        let mut sprite_atlas_ids = assets.get_sprite_atlas_names();
        sprite_atlas_ids.sort();
        let mut object_sheet_ids = assets.get_object_sheet_names();
        object_sheet_ids.sort();
        let mut audio_ids = assets
            .get_sfx_names()
            .into_iter()
            .chain(assets.get_music_names())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        audio_ids.sort();

        Self {
            entity_definition_ids,
            entity_animation_states,
            scene_ids,
            tilemap_ids,
            sprite_atlas_ids,
            object_sheet_ids,
            audio_ids,
        }
    }

    pub fn asset_ids_for_kind(&self, kind: AssetReferenceKind) -> Vec<String> {
        match kind {
            AssetReferenceKind::Any => self
                .sprite_atlas_ids
                .iter()
                .chain(self.object_sheet_ids.iter())
                .chain(self.tilemap_ids.iter())
                .chain(self.audio_ids.iter())
                .cloned()
                .collect(),
            AssetReferenceKind::SpriteAtlas => self.sprite_atlas_ids.clone(),
            AssetReferenceKind::ObjectSheet => self.object_sheet_ids.clone(),
            AssetReferenceKind::Tilemap => self.tilemap_ids.clone(),
            AssetReferenceKind::Audio => self.audio_ids.clone(),
            AssetReferenceKind::Font => Vec::new(),
        }
    }
}

#[cfg(test)]
pub fn built_in_template_descriptors() -> Vec<TemplateDescriptor> {
    let mut descriptors = BuiltInTemplateRegistry::new()
        .list_templates()
        .unwrap_or_default();
    descriptors.sort_by(|a, b| a.display_name.cmp(&b.display_name).then(a.id.cmp(&b.id)));
    descriptors
}

pub fn available_template_catalog(project: Option<&Project>) -> TemplateCatalog {
    let mut diagnostics = Vec::new();
    let mut descriptors = match BuiltInTemplateRegistry::new().list_templates() {
        Ok(descriptors) => descriptors,
        Err(error) => {
            diagnostics.push(format!("Built-in templates unavailable: {}", error.message));
            Vec::new()
        }
    };

    if let Some(project) = project {
        match ProjectTemplateProvider::detect(&project.path, &project.metadata.templates) {
            Ok(Some(provider)) => match provider.list_templates() {
                Ok(mut project_descriptors) => descriptors.append(&mut project_descriptors),
                Err(error) => diagnostics.push(format!(
                    "Project templates unavailable: {}",
                    error.message
                )),
            },
            Ok(None) => {}
            Err(error) => diagnostics.push(format!(
                "Project template discovery failed: {}",
                error.message
            )),
        }
    }

    descriptors.sort_by(|a, b| a.display_name.cmp(&b.display_name).then(a.id.cmp(&b.id)));
    TemplateCatalog {
        descriptors,
        diagnostics,
    }
}

pub fn template_categories(descriptors: &[TemplateDescriptor]) -> Vec<String> {
    descriptors
        .iter()
        .map(|descriptor| descriptor.category.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn filtered_descriptors(
    descriptors: &[TemplateDescriptor],
    category_filter: Option<&str>,
) -> Vec<TemplateDescriptor> {
    descriptors
        .iter()
        .filter(|descriptor| {
            category_filter
                .is_none_or(|category| descriptor.category.eq_ignore_ascii_case(category))
        })
        .cloned()
        .collect()
}

pub fn sync_template_editor_state(
    state: &mut TemplateEditorState,
    descriptors: &[TemplateDescriptor],
) {
    if descriptors.is_empty() {
        state.selected_template_id = None;
        return;
    }

    let filtered = filtered_descriptors(descriptors, state.category_filter.as_deref());
    let fallback = filtered
        .first()
        .or_else(|| descriptors.first())
        .map(|descriptor| descriptor.id.clone());

    let selected_is_valid = state
        .selected_template_id
        .as_ref()
        .is_some_and(|selected| filtered.iter().any(|descriptor| &descriptor.id == selected));
    if !selected_is_valid {
        state.selected_template_id = fallback;
    }

    for descriptor in descriptors {
        let values = state
            .parameters_by_template
            .entry(descriptor.id.clone())
            .or_default();
        for parameter in &descriptor.parameters {
            let default_value = parameter
                .default
                .clone()
                .unwrap_or_else(|| default_value_for_kind(&parameter.kind));
            values.entry(parameter.id.clone()).or_insert(default_value);
        }
    }
}

pub fn selected_descriptor<'a>(
    state: &'a TemplateEditorState,
    descriptors: &'a [TemplateDescriptor],
) -> Option<&'a TemplateDescriptor> {
    let selected = state.selected_template_id.as_ref()?;
    descriptors
        .iter()
        .find(|descriptor| &descriptor.id == selected)
}

pub fn preview_selected_template(
    state: &TemplateEditorState,
    project: &Project,
) -> Result<TemplatePreview, TemplateWorkflowError> {
    let catalog = available_template_catalog(Some(project));
    let descriptors = catalog.descriptors;
    let descriptor = selected_descriptor(state, &descriptors)
        .cloned()
        .ok_or_else(|| TemplateWorkflowError::new("no template selected"))?;
    let parameters = state
        .parameters_by_template
        .get(&descriptor.id)
        .cloned()
        .unwrap_or_default();
    let instantiation = instantiate_template_for_project(project, &descriptor.id, parameters.clone())?;
    let lowered = lower_plan_for_project(&project.path, &instantiation.plan)
        .map_err(|error| TemplateWorkflowError::new(error.message))?;
    let mut file_changes = build_project_file_changes(&project.path, &lowered)
        .map_err(|error| TemplateWorkflowError::new(error.message))?;
    let application =
        template_application_record(
            &descriptor,
            &parameters,
            &instantiation,
            &tracked_template_file_changes(&file_changes),
        );
    file_changes.push(project_metadata_change_for_template_application(
        project,
        &application,
    )?);

    Ok(TemplatePreview {
        descriptor,
        semantic_summary_lines: semantic_summary_lines(&instantiation.plan.items),
        lowered_summary_lines: lowered_summary_lines(&file_changes),
        selection_after_apply: selection_after_apply(&file_changes),
        instantiation,
        file_changes,
    })
}

pub fn build_apply_template_command(
    state: &TemplateEditorState,
    project: &Project,
    current_selection: Option<Selection>,
) -> Result<EditorCommand, TemplateWorkflowError> {
    let preview = preview_selected_template(state, project)?;
    let parameter_values = state
        .parameters_by_template
        .get(&preview.descriptor.id)
        .cloned()
        .unwrap_or_default();
    let mut metadata_after = project.metadata.clone();
    upsert_template_application(
        &mut metadata_after.editor.template_applications,
        template_application_record(
            &preview.descriptor,
            &parameter_values,
            &preview.instantiation,
            &tracked_template_file_changes(&preview.file_changes),
        ),
    );
    Ok(EditorCommand::apply_project_file_changes(
        format!("Apply template '{}'", preview.descriptor.display_name),
        preview.file_changes,
        current_selection,
        preview.selection_after_apply,
        Some(project.metadata.clone()),
        Some(metadata_after),
    ))
}

pub fn build_remove_template_application_command(
    project: &Project,
    application_id: &str,
    current_selection: Option<Selection>,
) -> Result<EditorCommand, TemplateWorkflowError> {
    let Some(application) = project
        .metadata
        .editor
        .template_applications
        .iter()
        .find(|application| application.application_id == application_id)
        .cloned()
    else {
        return Err(TemplateWorkflowError::new("active template application not found"));
    };

    let mut metadata_after = project.metadata.clone();
    metadata_after
        .editor
        .template_applications
        .retain(|candidate| candidate.application_id != application.application_id);
    let mut changes = application
        .file_changes
        .iter()
        .map(reverse_project_file_change)
        .collect::<Vec<_>>();
    changes.push(project_metadata_change_after_metadata_update(
        project,
        &metadata_after,
    )?);

    Ok(EditorCommand::apply_project_file_changes(
        format!("Remove template '{}'", application.template_display_name),
        changes,
        current_selection.clone(),
        current_selection,
        Some(project.metadata.clone()),
        Some(metadata_after),
    ))
}

pub fn build_delete_project_template_command(
    project: &Project,
    template_id: &str,
    template_display_name: &str,
    current_selection: Option<Selection>,
) -> Result<EditorCommand, TemplateWorkflowError> {
    let plan = crate::project::build_remove_template_starter_plan(
        &project.path,
        template_id,
        template_display_name,
    )
    .map_err(|error| TemplateWorkflowError::new(error.to_string()))?;

    Ok(EditorCommand::apply_project_file_changes(
        format!("Delete project template '{}'", template_display_name),
        plan.changes,
        current_selection.clone(),
        current_selection,
        Some(project.metadata.clone()),
        Some(project.metadata.clone()),
    ))
}

fn tracked_template_file_changes(file_changes: &[ProjectFileChange]) -> Vec<ProjectFileChange> {
    file_changes
        .iter()
        .filter(|change| change.relative_path != std::path::Path::new("project.toml"))
        .cloned()
        .collect()
}

fn instantiate_template_for_project(
    project: &Project,
    template_id: &str,
    parameters: BTreeMap<String, TemplateValue>,
) -> Result<TemplateInstantiation, TemplateWorkflowError> {
    if template_id.starts_with("toki/") {
        return BuiltInTemplateRegistry::new()
            .instantiate_template(template_id, parameters)
            .map_err(|error| TemplateWorkflowError::new(error.message));
    }
    if template_id.starts_with("project/") {
        let provider = ProjectTemplateProvider::detect(&project.path, &project.metadata.templates)
            .map_err(|error| TemplateWorkflowError::new(error.message))?
            .ok_or_else(|| TemplateWorkflowError::new("project template runner is not available"))?;
        return provider
            .instantiate_template(template_id, parameters)
            .map_err(|error| TemplateWorkflowError::new(error.message));
    }

    Err(TemplateWorkflowError::new(format!(
        "unsupported template namespace for '{}'",
        template_id
    )))
}

fn reverse_project_file_change(change: &ProjectFileChange) -> ProjectFileChange {
    ProjectFileChange {
        relative_path: change.relative_path.clone(),
        before_contents: change.after_contents.clone(),
        after_contents: change.before_contents.clone(),
    }
}

pub fn summary_line_for_parameter_value(
    parameter: &TemplateParameter,
    value: Option<&TemplateValue>,
) -> String {
    match value {
        Some(value) => format!("{}: {}", parameter.label, template_value_label(value)),
        None => format!("{}: <unset>", parameter.label),
    }
}

pub fn template_value_label(value: &TemplateValue) -> String {
    match value {
        TemplateValue::String(value)
        | TemplateValue::Enum(value)
        | TemplateValue::AssetReference(value)
        | TemplateValue::EntityDefinitionReference(value)
        | TemplateValue::AnimationStateReference(value)
        | TemplateValue::SceneReference(value) => value.clone(),
        TemplateValue::Integer(value) => value.to_string(),
        TemplateValue::Float(value) => value.to_string(),
        TemplateValue::Boolean(value) => value.to_string(),
        TemplateValue::Optional(Some(value)) => template_value_label(value),
        TemplateValue::Optional(None) => "<none>".to_string(),
        TemplateValue::List(values) => format!("{} items", values.len()),
    }
}

pub fn default_value_for_kind(kind: &TemplateParameterKind) -> TemplateValue {
    match kind {
        TemplateParameterKind::String { .. } => TemplateValue::String(String::new()),
        TemplateParameterKind::Integer { min, .. } => TemplateValue::Integer(min.unwrap_or(0)),
        TemplateParameterKind::Float { min, .. } => TemplateValue::Float(min.unwrap_or(0.0)),
        TemplateParameterKind::Boolean => TemplateValue::Boolean(false),
        TemplateParameterKind::Enum { options } => TemplateValue::Enum(
            options
                .first()
                .map(|option| option.id.clone())
                .unwrap_or_default(),
        ),
        TemplateParameterKind::AssetReference { .. } => {
            TemplateValue::AssetReference(String::new())
        }
        TemplateParameterKind::EntityDefinitionReference => {
            TemplateValue::EntityDefinitionReference(String::new())
        }
        TemplateParameterKind::AnimationStateReference { .. } => {
            TemplateValue::AnimationStateReference(String::new())
        }
        TemplateParameterKind::SceneReference => TemplateValue::SceneReference(String::new()),
        TemplateParameterKind::Optional { .. } => TemplateValue::Optional(None),
        TemplateParameterKind::List { .. } => TemplateValue::List(Vec::new()),
    }
}

pub fn animation_state_choices_for_parameter(
    parameter: &TemplateParameter,
    values: &BTreeMap<String, TemplateValue>,
    template_asset_choices: Option<&TemplateAssetChoices>,
) -> Option<Vec<String>> {
    let TemplateParameterKind::Optional { inner } = &parameter.kind else {
        return None;
    };
    let TemplateParameterKind::AnimationStateReference { entity_parameter_id } = inner.as_ref() else {
        return None;
    };
    let entity_id = match values.get(entity_parameter_id) {
        Some(TemplateValue::EntityDefinitionReference(entity_id)) if !entity_id.is_empty() => {
            entity_id
        }
        _ => return None,
    };
    template_asset_choices
        .and_then(|choices| choices.entity_animation_states.get(entity_id))
        .cloned()
}

fn template_application_record(
    descriptor: &TemplateDescriptor,
    parameters: &BTreeMap<String, TemplateValue>,
    instantiation: &TemplateInstantiation,
    file_changes: &[ProjectFileChange],
) -> TemplateApplicationRecord {
    TemplateApplicationRecord {
        application_id: template_application_id(descriptor, instantiation),
        template_id: descriptor.id.clone(),
        template_display_name: descriptor.display_name.clone(),
        parameter_summary_lines: descriptor
            .parameters
            .iter()
            .map(|parameter| {
                let value = parameters.get(&parameter.id);
                summary_line_for_parameter_value(parameter, value)
            })
            .collect(),
        semantic_summary_lines: semantic_summary_lines(&instantiation.plan.items),
        affected_paths: file_changes
            .iter()
            .map(|change| change.relative_path.display().to_string())
            .collect(),
        file_changes: file_changes.to_vec(),
    }
}

fn template_application_id(
    descriptor: &TemplateDescriptor,
    instantiation: &TemplateInstantiation,
) -> String {
    let mut item_ids = instantiation
        .plan
        .items
        .iter()
        .map(template_semantic_item_id)
        .collect::<Vec<_>>();
    item_ids.sort();
    format!("{}::{}", descriptor.id, item_ids.join("+"))
}

fn template_semantic_item_id(item: &TemplateSemanticItem) -> String {
    match item {
        TemplateSemanticItem::CreateAttackBehavior { id, .. }
        | TemplateSemanticItem::CreatePickupBehavior { id, .. }
        | TemplateSemanticItem::CreateProjectileBehavior { id, .. }
        | TemplateSemanticItem::CreateConfirmationDialog { id, .. }
        | TemplateSemanticItem::CreatePauseMenuFlow { id, .. }
        | TemplateSemanticItem::ConfigureEntityCapability { id, .. } => id.clone(),
    }
}

fn upsert_template_application(
    applications: &mut Vec<TemplateApplicationRecord>,
    application: TemplateApplicationRecord,
) {
    if let Some(existing_index) = applications
        .iter()
        .position(|existing| existing.application_id == application.application_id)
    {
        applications[existing_index] = application;
    } else {
        applications.push(application);
    }
    applications.sort_by(|left, right| {
        left.template_display_name
            .cmp(&right.template_display_name)
            .then(left.application_id.cmp(&right.application_id))
    });
}

fn project_metadata_change_for_template_application(
    project: &Project,
    application: &TemplateApplicationRecord,
) -> Result<ProjectFileChange, TemplateWorkflowError> {
    let mut metadata_after = project.metadata.clone();
    upsert_template_application(
        &mut metadata_after.editor.template_applications,
        application.clone(),
    );
    project_metadata_change_after_metadata_update(project, &metadata_after)
}

fn project_metadata_change_after_metadata_update(
    project: &Project,
    metadata_after: &crate::project::ProjectMetadata,
) -> Result<ProjectFileChange, TemplateWorkflowError> {
    let before_contents = std::fs::read_to_string(project.project_file_path())
        .map_err(|error| TemplateWorkflowError::new(format!("failed to read project.toml: {error}")))?;
    let after_contents = toml::to_string_pretty(metadata_after)
        .map_err(|error| TemplateWorkflowError::new(format!("failed to serialize project metadata: {error}")))?;
    Ok(ProjectFileChange {
        relative_path: std::path::PathBuf::from("project.toml"),
        before_contents: Some(before_contents),
        after_contents: Some(after_contents),
    })
}

fn semantic_summary_lines(items: &[TemplateSemanticItem]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            TemplateSemanticItem::CreateAttackBehavior {
                actor_entity_definition_id,
                mode,
                damage,
                cooldown_ticks,
                ..
            } => format!(
                "Create attack behavior for {} ({mode:?}, damage {damage}, cooldown {cooldown_ticks})",
                actor_entity_definition_id.as_deref().unwrap_or("<missing actor>")
            ),
            TemplateSemanticItem::CreatePickupBehavior {
                pickup_entity_definition_id,
                item_id,
                count,
                ..
            } => format!(
                "Create pickup behavior for {pickup_entity_definition_id} granting {item_id} x{count}"
            ),
            TemplateSemanticItem::CreateProjectileBehavior {
                projectile_entity_definition_id,
                damage,
                speed,
                ..
            } => format!(
                "Create projectile behavior for {projectile_entity_definition_id} (damage {damage}, speed {speed})"
            ),
            TemplateSemanticItem::CreateConfirmationDialog { id, title, .. } => {
                format!("Create confirmation dialog '{id}' titled '{title}'")
            }
            TemplateSemanticItem::CreatePauseMenuFlow { id, .. } => {
                format!("Create pause menu flow '{id}'")
            }
            TemplateSemanticItem::ConfigureEntityCapability {
                entity_definition_id,
                capability_id,
                ..
            } => format!(
                "Configure capability '{capability_id}' on entity definition '{entity_definition_id}'"
            ),
        })
        .collect()
}

fn lowered_summary_lines(file_changes: &[ProjectFileChange]) -> Vec<String> {
    file_changes
        .iter()
        .map(
            |change| match (&change.before_contents, &change.after_contents) {
                (Some(_), Some(_)) => format!("Update {}", change.relative_path.display()),
                (None, Some(_)) => format!("Create {}", change.relative_path.display()),
                (Some(_), None) => format!("Delete {}", change.relative_path.display()),
                (None, None) => format!("No-op {}", change.relative_path.display()),
            },
        )
        .collect()
}

fn selection_after_apply(file_changes: &[ProjectFileChange]) -> Option<Selection> {
    let mut entity_changes = file_changes
        .iter()
        .filter(|change| change.relative_path.parent().is_some())
        .filter(|change| change.relative_path.parent().unwrap().to_string_lossy() == "entities");
    let relative = &entity_changes.next()?.relative_path;
    if entity_changes.next().is_some() {
        return None;
    }
    let parent = relative.parent()?.to_string_lossy();
    if parent != "entities" {
        return None;
    }

    let stem = relative.file_stem()?.to_string_lossy().to_string();
    Some(Selection::EntityDefinition(stem))
}

#[cfg(test)]
#[path = "template_workflow_tests.rs"]
mod tests;
