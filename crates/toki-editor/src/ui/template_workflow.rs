use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::project::ProjectAssets;
use crate::ui::editor_ui::Selection;
use crate::ui::undo_redo::EditorCommand;
use toki_template_builtins::BuiltInTemplateRegistry;
use toki_template_lowering::{
    build_project_file_changes, lower_plan_for_project, ProjectFileChange,
};
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplateAssetChoices {
    pub entity_definition_ids: Vec<String>,
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

impl TemplateWorkflowError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl TemplateAssetChoices {
    pub fn from_project_assets(assets: &ProjectAssets) -> Self {
        let mut entity_definition_ids = assets.get_entity_names();
        entity_definition_ids.sort();
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

pub fn built_in_template_descriptors() -> Vec<TemplateDescriptor> {
    let mut descriptors = BuiltInTemplateRegistry::new().list_templates();
    descriptors.sort_by(|a, b| a.display_name.cmp(&b.display_name).then(a.id.cmp(&b.id)));
    descriptors
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
    project_root: &Path,
) -> Result<TemplatePreview, TemplateWorkflowError> {
    let descriptors = built_in_template_descriptors();
    let descriptor = selected_descriptor(state, &descriptors)
        .cloned()
        .ok_or_else(|| TemplateWorkflowError::new("no template selected"))?;
    let registry = BuiltInTemplateRegistry::new();
    let parameters = state
        .parameters_by_template
        .get(&descriptor.id)
        .cloned()
        .unwrap_or_default();
    let instantiation = registry
        .instantiate_template(&descriptor.id, parameters)
        .map_err(|error| TemplateWorkflowError::new(error.message))?;
    let lowered = lower_plan_for_project(project_root, &instantiation.plan)
        .map_err(|error| TemplateWorkflowError::new(error.message))?;
    let file_changes = build_project_file_changes(project_root, &lowered)
        .map_err(|error| TemplateWorkflowError::new(error.message))?;

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
    project_root: &Path,
    current_selection: Option<Selection>,
) -> Result<EditorCommand, TemplateWorkflowError> {
    let preview = preview_selected_template(state, project_root)?;
    Ok(EditorCommand::apply_project_file_changes(
        format!("Apply template '{}'", preview.descriptor.display_name),
        preview.file_changes,
        current_selection,
        preview.selection_after_apply,
    ))
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
        TemplateParameterKind::SceneReference => TemplateValue::SceneReference(String::new()),
        TemplateParameterKind::Optional { .. } => TemplateValue::Optional(None),
        TemplateParameterKind::List { .. } => TemplateValue::List(Vec::new()),
    }
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
    if file_changes.len() != 1 {
        return None;
    }

    let relative = &file_changes[0].relative_path;
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
