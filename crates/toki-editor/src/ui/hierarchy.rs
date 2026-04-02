use crate::project::ProjectAssets;
use crate::ui::editor_context::entity_editor_state;
use crate::ui::editor_ui::Selection;
use crate::ui::entity_editor::{EntityEditState, EntityEditorState, EntitySummary};
use crate::ui::EditorUI;
use std::collections::BTreeMap;

/// Handles hierarchy and entity management for the editor
pub struct HierarchySystem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolboxEntityDefinitionSummary {
    pub name: String,
    pub display_name: String,
}

impl HierarchySystem {
    fn category_label(raw: &str) -> String {
        if raw.trim().is_empty() {
            return "Uncategorized".to_string();
        }

        raw.split(['_', '-', ' '])
            .filter(|segment| !segment.is_empty())
            .map(|segment| {
                let mut chars = segment.chars();
                match chars.next() {
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn category_section_label(raw: &str) -> String {
        match raw.trim().to_ascii_lowercase().as_str() {
            "creature" => "Creatures".to_string(),
            "human" => "Humans".to_string(),
            "item" => "Items".to_string(),
            _ => Self::category_label(raw),
        }
    }

    /// Renders the entity palette showing available entity definitions for inspection.
    pub fn render_entity_palette(
        ui: &mut egui::Ui,
        project_path: &std::path::Path,
        selection: &Option<Selection>,
    ) -> Option<String> {
        let entities_path = project_path.join("entities");

        if entities_path.exists() {
            // Try to read entity definition files
            if let Ok(entries) = std::fs::read_dir(&entities_path) {
                let mut found_entities = false;
                let mut categories: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();

                // First pass: collect entities and group by category
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") {
                            let entity_name = name.trim_end_matches(".json").to_string();
                            found_entities = true;

                            // Try to read the entity file to get its category
                            let entity_path = entry.path();
                            if let Ok(content) = std::fs::read_to_string(&entity_path) {
                                if let Ok(json_value) =
                                    serde_json::from_str::<serde_json::Value>(&content)
                                {
                                    let category = json_value
                                        .get("category")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("uncategorized")
                                        .to_string();

                                    categories.entry(category).or_default().push(entity_name);
                                } else {
                                    // If we can't parse JSON, put in uncategorized
                                    categories
                                        .entry("uncategorized".to_string())
                                        .or_default()
                                        .push(entity_name);
                                }
                            } else {
                                // If we can't read file, put in uncategorized
                                categories
                                    .entry("uncategorized".to_string())
                                    .or_default()
                                    .push(entity_name);
                            }
                        }
                    }
                }

                if found_entities {
                    let mut selected_entity = None;

                    // The parent left panel owns scrolling for the full container.
                    let mut sorted_categories: Vec<_> = categories.into_iter().collect();
                    sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));

                    for (category, mut entity_names) in sorted_categories {
                        egui::CollapsingHeader::new(Self::category_section_label(&category))
                            .id_salt(format!("entity_palette_category_{}", category))
                            .default_open(false)
                            .show(ui, |ui| {
                                entity_names.sort();

                                for entity_name in entity_names {
                                    let is_selected = matches!(
                                        selection,
                                        Some(Selection::EntityDefinition(name)) if name == &entity_name
                                    );

                                    let button = ui.selectable_label(is_selected, &entity_name);

                                    if button.clicked() {
                                        tracing::debug!(
                                            "Entity '{}' clicked - inspecting definition",
                                            entity_name
                                        );
                                        selected_entity = Some(entity_name.clone());
                                    }
                                }
                            });
                        ui.add_space(5.0);
                    }

                    return selected_entity;
                } else {
                    ui.label("No entity definition files found in entities/");
                }
            } else {
                ui.label("Could not read entities directory");
            }
        } else {
            ui.label("No entities directory found, expected: entities/");
        }

        None
    }
}

/// Maps an entity definition category string to the corresponding [`ToolboxTab`].
///
/// Returns `None` for categories that are not placeable from the toolbox
/// (e.g. decorations use the object-sheet flow, triggers/projectiles are internal).
pub fn toolbox_tab_for_category(category: &str) -> Option<super::editor_ui::ToolboxTab> {
    use super::editor_ui::ToolboxTab;
    match category.trim().to_ascii_lowercase().as_str() {
        "creature" | "creatures" | "enemy" => Some(ToolboxTab::Creatures),
        "human" | "humans" | "player" | "players" => Some(ToolboxTab::Humans),
        "item" | "items" => Some(ToolboxTab::Items),
        _ => None,
    }
}

/// Scans the `entities/` directory and groups entity definition names by [`ToolboxTab`].
///
/// Definitions whose category does not map to a toolbox tab are omitted.
pub fn collect_entity_definitions_for_toolbox(
    ui_state: &EditorUI,
    project_assets: Option<&mut ProjectAssets>,
) -> BTreeMap<super::editor_ui::ToolboxTab, Vec<ToolboxEntityDefinitionSummary>> {
    let mut definitions =
        collect_definition_categories(entity_editor_state(ui_state), project_assets);
    if let Some(edit_state) = entity_editor_state(ui_state).edit_state.as_ref() {
        apply_edit_state_override(&mut definitions, edit_state);
    }

    let mut grouped =
        BTreeMap::<super::editor_ui::ToolboxTab, Vec<ToolboxEntityDefinitionSummary>>::new();
    for (name, definition) in definitions {
        let category = definition.category;
        if let Some(tab) = toolbox_tab_for_category(&category) {
            grouped
                .entry(tab)
                .or_default()
                .push(ToolboxEntityDefinitionSummary {
                    name,
                    display_name: definition.display_name,
                });
        }
    }

    for definitions in grouped.values_mut() {
        definitions.sort_by(|a, b| a.name.cmp(&b.name));
    }

    grouped
}

fn collect_definition_categories(
    entity_editor: &EntityEditorState,
    project_assets: Option<&mut ProjectAssets>,
) -> BTreeMap<String, ToolboxDefinitionRecord> {
    let mut result = BTreeMap::new();

    if !entity_editor.entities.is_empty() {
        for summary in &entity_editor.entities {
            merge_summary(&mut result, summary);
        }
        return result;
    }

    let Some(project_assets) = project_assets else {
        return result;
    };

    let mut names = project_assets.entities.keys().cloned().collect::<Vec<_>>();
    names.sort();
    for name in names {
        if let Ok(Some(definition)) = project_assets.load_entity_definition(&name) {
            result.insert(
                name.clone(),
                ToolboxDefinitionRecord {
                    category: definition.category,
                    display_name: if definition.display_name.is_empty() {
                        name
                    } else {
                        definition.display_name
                    },
                },
            );
        }
    }

    result
}

#[derive(Debug, Clone)]
struct ToolboxDefinitionRecord {
    category: String,
    display_name: String,
}

fn merge_summary(target: &mut BTreeMap<String, ToolboxDefinitionRecord>, summary: &EntitySummary) {
    target.insert(
        summary.name.clone(),
        ToolboxDefinitionRecord {
            category: summary.category.clone(),
            display_name: summary.display_name.clone(),
        },
    );
}

fn apply_edit_state_override(
    target: &mut BTreeMap<String, ToolboxDefinitionRecord>,
    edit_state: &EntityEditState,
) {
    let current_name = edit_state.definition.name.to_string();
    if let Some(previous_name) = edit_state
        .file_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|name| *name != current_name)
    {
        target.remove(previous_name);
    }
    target.insert(
        current_name.clone(),
        ToolboxDefinitionRecord {
            category: edit_state.definition.category.clone(),
            display_name: if edit_state.definition.display_name.is_empty() {
                current_name
            } else {
                edit_state.definition.display_name.clone()
            },
        },
    );
}

#[cfg(test)]
#[path = "hierarchy_tests.rs"]
mod tests;
