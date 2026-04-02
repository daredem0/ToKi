use crate::ui::editor_ui::Selection;
use toki_core::Scene;

/// Handles hierarchy and entity management for the editor
pub struct HierarchySystem;

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

    /// Renders the entity palette showing available entity definitions for placement
    pub fn render_entity_palette(
        ui: &mut egui::Ui,
        project_path: &std::path::Path,
        selection: &Option<Selection>,
        scenes: &[Scene],
    ) -> (Option<String>, Vec<(String, String)>, Option<String>) {
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
                    let mut scene_entity_additions: Vec<(String, String)> = Vec::new(); // (scene_name, entity_name)
                    let placement_mode_request: Option<String> = None;

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

                                    button.context_menu(|ui| {
                                        ui.label(format!("Entity: {}", entity_name));
                                        ui.separator();
                                        ui.label("Add to Scene:");
                                        ui.separator();

                                        let scene_names: Vec<String> =
                                            scenes.iter().map(|s| s.name.clone()).collect();

                                        for scene_name in scene_names {
                                            if ui.button(&scene_name).clicked() {
                                                tracing::debug!(
                                                    "Adding entity '{}' to scene '{}'",
                                                    entity_name,
                                                    scene_name
                                                );
                                                scene_entity_additions
                                                    .push((scene_name.clone(), entity_name.clone()));
                                                ui.close();
                                            }
                                        }

                                        ui.separator();
                                        ui.label("Actions:");
                                        if ui.button("View Definition").clicked() {
                                            tracing::debug!(
                                                "View definition for entity: {}",
                                                entity_name
                                            );
                                            ui.close();
                                        }
                                    });
                                }
                            });
                        ui.add_space(5.0);
                    }

                    return (
                        selected_entity,
                        scene_entity_additions,
                        placement_mode_request,
                    );
                } else {
                    ui.label("No entity definition files found in entities/");
                }
            } else {
                ui.label("Could not read entities directory");
            }
        } else {
            ui.label("No entities directory found, expected: entities/");
        }

        (None, Vec::new(), None)
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
pub fn scan_entity_definitions_for_toolbox(
    project_path: &std::path::Path,
) -> std::collections::BTreeMap<super::editor_ui::ToolboxTab, Vec<String>> {
    use super::editor_ui::ToolboxTab;
    let mut result = std::collections::BTreeMap::<ToolboxTab, Vec<String>>::new();
    let entities_dir = project_path.join("entities");

    let entries = match std::fs::read_dir(&entities_dir) {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(String::from) else {
            continue;
        };
        if !name.ends_with(".json") {
            continue;
        }
        let entity_name = name.trim_end_matches(".json").to_string();
        let tab = read_category_and_map(&entry.path());
        if let Some(tab) = tab {
            result.entry(tab).or_default().push(entity_name);
        }
    }

    for names in result.values_mut() {
        names.sort();
    }
    result
}

fn read_category_and_map(
    path: &std::path::Path,
) -> Option<super::editor_ui::ToolboxTab> {
    let content = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let category = json.get("category")?.as_str()?;
    toolbox_tab_for_category(category)
}

#[cfg(test)]
#[path = "hierarchy_tests.rs"]
mod tests;
