use crate::ui::editor_ui::{EditorUI, Selection};
use toki_core::Scene;

/// Handles hierarchy and entity management for the editor
pub struct HierarchySystem;

impl HierarchySystem {
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
                    let mut placement_mode_request: Option<String> = None;

                    egui::ScrollArea::vertical()
                        .id_salt("entities_scroll")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            // Sort categories for consistent display
                            let mut sorted_categories: Vec<_> = categories.into_iter().collect();
                            sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));

                            for (category, mut entity_names) in sorted_categories {
                                // Show category header
                                ui.label(format!("📂 {}", category));
                                ui.indent(format!("category_{}", category), |ui| {
                                    // Sort entity names
                                    entity_names.sort();

                                    for entity_name in entity_names {
                                        // Show entity button with selection capability
                                        let is_selected = matches!(
                                            selection,
                                            Some(Selection::EntityDefinition(name)) if name == &entity_name
                                        );

                                        let button = ui.selectable_label(is_selected, format!("🧙 {}", entity_name));

                                        if button.clicked() {
                                            tracing::info!("Entity '{}' clicked - entering placement mode", entity_name);
                                            placement_mode_request = Some(entity_name.clone());
                                            selected_entity = Some(entity_name.clone());
                                        }

                                        // Right-click context menu for entity actions
                                        button.context_menu(|ui| {
                                            ui.label(format!("Entity: {}", entity_name));
                                            ui.separator();

                                            // Add to Scene section
                                            ui.label("Add to Scene:");
                                            ui.separator();

                                            // Show available scenes
                                            let scene_names: Vec<String> = scenes.iter()
                                                .map(|s| s.name.clone())
                                                .collect();

                                            for scene_name in scene_names {
                                                if ui.button(&scene_name).clicked() {
                                                    tracing::info!("Adding entity '{}' to scene '{}'", entity_name, scene_name);
                                                    scene_entity_additions.push((scene_name.clone(), entity_name.clone()));
                                                    ui.close();
                                                }
                                            }

                                            ui.separator();
                                            ui.label("Actions:");
                                            if ui.button("📖 View Definition").clicked() {
                                                tracing::info!("View definition for entity: {}", entity_name);
                                                ui.close();
                                            }
                                        });
                                    }
                                });
                                ui.add_space(5.0);
                            }
                        });

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

    /// Renders the main hierarchy and maps panel (delegates to EditorUI for now)
    pub fn render_hierarchy_and_maps_combined_panel(
        editor_ui: &mut EditorUI,
        ctx: &egui::Context,
        game_state: Option<&toki_core::GameState>,
        config: Option<&crate::config::EditorConfig>,
    ) {
        editor_ui.render_hierarchy_and_maps_combined_panel(ctx, game_state, config);
    }
}
