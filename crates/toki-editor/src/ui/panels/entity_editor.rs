// Entity editor panel
// Provides a dedicated tab for creating and editing entity definitions
// Phase 4.5A: Entity Editor Tab And Definition Browser

use crate::project::Project;
use crate::ui::editor_ui::{EntityCategory, EntitySummary, Selection};
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::entity::EntityDefinition;

/// Renders the entity editor panel
pub fn render_entity_editor(
    ui: &mut egui::Ui,
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project: Option<&mut Project>,
) {
    let project_path = project.as_ref().map(|p| p.path.clone());

    // Refresh entity list if needed
    if ui_state.entity_editor.needs_refresh {
        refresh_entity_list(ui_state, project_path.as_deref());
        ui_state.entity_editor.needs_refresh = false;
    }

    // Auto-load entities on first view if we have a project
    if ui_state.entity_editor.entities.is_empty() && project_path.is_some() {
        refresh_entity_list(ui_state, project_path.as_deref());
    }

    // Handle dialogs
    if ui_state.entity_editor.new_entity_dialog.is_open {
        render_new_entity_dialog(ui_state, ctx, project_path.as_deref());
    }
    if ui_state.entity_editor.delete_confirmation.is_open {
        render_delete_confirmation_dialog(ui_state, ctx, project_path.as_deref());
    }

    // Toolbar
    render_toolbar(ui, ui_state);
    ui.separator();

    // Main content: Browser + Editor split
    render_main_content(ui, ui_state, project_path.as_deref());
}

fn render_toolbar(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.heading("Entity Editor");
        ui.separator();

        // New entity button
        if ui.button("+ New Entity").clicked() {
            ui_state.entity_editor.new_entity_dialog.open_for_new();
        }

        // Refresh button
        if ui.button("Refresh").clicked() {
            ui_state.entity_editor.needs_refresh = true;
        }

        // Show selected entity name and dirty indicator
        if let Some(name) = &ui_state.entity_editor.selected_entity {
            ui.separator();
            ui.label(format!("Selected: {}", name));
            if ui_state.entity_editor.dirty {
                ui.label("*");
            }
        }
    });
}

fn render_main_content(ui: &mut egui::Ui, ui_state: &mut EditorUI, project_path: Option<&Path>) {
    let available_width = ui.available_width();
    let available_height = ui.available_height();
    let browser_width = ui_state.entity_editor.browser_panel_width;
    let separator_width = 8.0;
    let editor_width = (available_width - browser_width - separator_width).max(200.0);

    ui.horizontal(|ui| {
        // Left panel: Entity browser
        ui.allocate_ui_with_layout(
            egui::vec2(browser_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                render_entity_browser(ui, ui_state, project_path);
            },
        );

        // Draggable separator
        let sep_response = render_vertical_separator(ui, available_height);
        if sep_response.dragged() {
            ui_state.entity_editor.browser_panel_width =
                (ui_state.entity_editor.browser_panel_width + sep_response.drag_delta().x)
                    .clamp(150.0, available_width * 0.4);
        }

        // Right panel: Entity details (placeholder for Phase 4.5C)
        ui.allocate_ui_with_layout(
            egui::vec2(editor_width, available_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                render_entity_details(ui, ui_state);
            },
        );
    });
}

/// Render a vertical draggable separator
fn render_vertical_separator(ui: &mut egui::Ui, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(8.0, height), egui::Sense::drag());

    let color = if response.dragged() {
        egui::Color32::from_gray(180)
    } else if response.hovered() {
        egui::Color32::from_gray(140)
    } else {
        egui::Color32::from_gray(80)
    };

    ui.painter().rect_filled(
        egui::Rect::from_center_size(rect.center(), egui::vec2(2.0, height)),
        0.0,
        color,
    );

    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    }

    response
}

fn render_entity_browser(ui: &mut egui::Ui, ui_state: &mut EditorUI, _project_path: Option<&Path>) {
    // Search box
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(&mut ui_state.entity_editor.filter.search_query);
    });

    // Category filter - built from actual categories in loaded entities
    let mut categories: Vec<String> = ui_state.entity_editor.all_categories().into_iter().collect();
    categories.sort();

    ui.horizontal(|ui| {
        ui.label("Category:");
        egui::ComboBox::from_id_salt("entity_category_filter")
            .selected_text(if ui_state.entity_editor.filter.category_filter.is_empty() {
                "All"
            } else {
                &ui_state.entity_editor.filter.category_filter
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        ui_state.entity_editor.filter.category_filter.is_empty(),
                        "All",
                    )
                    .clicked()
                {
                    ui_state.entity_editor.filter.category_filter.clear();
                }

                for category in &categories {
                    let is_selected = ui_state
                        .entity_editor
                        .filter
                        .category_filter
                        .eq_ignore_ascii_case(category);
                    if ui.selectable_label(is_selected, category).clicked() {
                        ui_state.entity_editor.filter.category_filter = category.clone();
                    }
                }
            });
    });

    // Clear filters button (if any filters active)
    if ui_state.entity_editor.filter.is_active() && ui.button("Clear Filters").clicked() {
        ui_state.entity_editor.filter.clear();
    }

    ui.separator();

    // Entity list
    let filtered = ui_state.entity_editor.filtered_entities();
    let entity_count = filtered.len();

    ui.label(format!("Entities: {}", entity_count));

    let mut select_entity: Option<String> = None;
    let mut duplicate_entity: Option<EntitySummary> = None;
    let mut delete_entity: Option<String> = None;

    egui::ScrollArea::vertical()
        .id_salt("entity_browser_list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for entity in filtered {
                let is_selected = ui_state
                    .entity_editor
                    .selected_entity
                    .as_ref()
                    .map(|s| s == &entity.name)
                    .unwrap_or(false);

                ui.horizontal(|ui| {
                    // Entity row with selection
                    let response = ui.selectable_label(
                        is_selected,
                        format!("{} ({})", entity.display_name, entity.category),
                    );

                    if response.clicked() {
                        select_entity = Some(entity.name.clone());
                    }

                    // Context menu
                    response.context_menu(|ui| {
                        if ui.button("Duplicate").clicked() {
                            duplicate_entity = Some(entity.clone());
                            ui.close();
                        }
                        if ui.button("Delete").clicked() {
                            delete_entity = Some(entity.name.clone());
                            ui.close();
                        }
                    });
                });
            }
        });

    // Handle deferred actions
    if let Some(name) = select_entity {
        ui_state.entity_editor.select_entity(&name);
        // Also update the global selection so inspector works
        ui_state.selection = Some(Selection::EntityDefinition(name));
    }

    if let Some(source) = duplicate_entity {
        ui_state
            .entity_editor
            .new_entity_dialog
            .open_for_duplicate(&source);
    }

    if let Some(name) = delete_entity {
        ui_state.entity_editor.delete_confirmation.open(&name);
    }
}

fn render_entity_details(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    if ui_state.entity_editor.selected_entity.is_none() {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label("No entity selected");
                ui.add_space(8.0);
                ui.label("Select an entity from the browser or create a new one.");
            });
        });
        return;
    }

    // Show entity name
    if let Some(summary) = ui_state.entity_editor.selected_entity_summary() {
        ui.heading(&summary.display_name);
        ui.label(format!("ID: {}", summary.name));
        ui.label(format!("Category: {}", summary.category));

        if !summary.tags.is_empty() {
            ui.label(format!("Tags: {}", summary.tags.join(", ")));
        }

        ui.separator();

        // Placeholder for Phase 4.5B/C - property editing will go here
        ui.label("Property editing will be available in Phase 4.5B/C");
        ui.label("For now, use the Animation Editor to configure animations.");
    }
}

fn render_new_entity_dialog(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project_path: Option<&Path>,
) {
    let mut close_dialog = false;
    let mut create_entity = false;

    egui::Window::new("New Entity")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name (identifier):");
                ui.text_edit_singleline(&mut ui_state.entity_editor.new_entity_dialog.name_input);
            });

            ui.horizontal(|ui| {
                ui.label("Display Name:");
                ui.text_edit_singleline(
                    &mut ui_state.entity_editor.new_entity_dialog.display_name_input,
                );
            });

            ui.horizontal(|ui| {
                ui.label("Description:");
            });
            ui.text_edit_multiline(
                &mut ui_state.entity_editor.new_entity_dialog.description_input,
            );

            // Build category list: existing categories + predefined suggestions
            let mut all_categories: std::collections::HashSet<String> =
                ui_state.entity_editor.all_categories();
            for category in EntityCategory::ALL {
                all_categories.insert(category.as_str().to_string());
            }
            let mut category_list: Vec<String> = all_categories.into_iter().collect();
            category_list.sort();

            ui.horizontal(|ui| {
                ui.label("Category:");
                ui.text_edit_singleline(&mut ui_state.entity_editor.new_entity_dialog.category);
            });

            ui.horizontal(|ui| {
                ui.label("Suggestions:");
                egui::ComboBox::from_id_salt("new_entity_category")
                    .selected_text("Select...")
                    .show_ui(ui, |ui| {
                        for category in &category_list {
                            if ui.selectable_label(false, category).clicked() {
                                ui_state.entity_editor.new_entity_dialog.category = category.clone();
                            }
                        }
                    });
            });

            // Show validation error
            if let Some(error) = &ui_state.entity_editor.new_entity_dialog.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Create").clicked() {
                    create_entity = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        ui_state.entity_editor.new_entity_dialog.close();
    }

    if create_entity {
        let existing = ui_state.entity_editor.existing_names();
        if ui_state.entity_editor.new_entity_dialog.validate(&existing) {
            if let Some(path) = project_path {
                create_new_entity(ui_state, path);
                ui_state.entity_editor.new_entity_dialog.close();
            }
        }
    }
}

fn render_delete_confirmation_dialog(
    ui_state: &mut EditorUI,
    ctx: &egui::Context,
    project_path: Option<&Path>,
) {
    let entity_name = ui_state.entity_editor.delete_confirmation.entity_name.clone();
    let mut close_dialog = false;
    let mut confirm_delete = false;

    egui::Window::new("Delete Entity")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(format!(
                "Are you sure you want to delete '{}'?",
                entity_name
            ));
            ui.label("This action cannot be undone.");

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    confirm_delete = true;
                }
                if ui.button("Cancel").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        ui_state.entity_editor.delete_confirmation.close();
    }

    if confirm_delete {
        if let Some(path) = project_path {
            delete_entity(ui_state, path, &entity_name);
        }
        ui_state.entity_editor.delete_confirmation.close();
    }
}

// === Entity Operations ===

fn refresh_entity_list(ui_state: &mut EditorUI, project_path: Option<&Path>) {
    ui_state.entity_editor.entities.clear();

    let Some(path) = project_path else {
        return;
    };

    let entities_dir = path.join("entities");
    ui_state.entity_editor.entities_dir = Some(entities_dir.clone());

    if !entities_dir.exists() {
        return;
    }

    let Ok(entries) = std::fs::read_dir(&entities_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let file_path = entry.path();
        if file_path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(summary) = load_entity_summary(&file_path) {
                ui_state.entity_editor.entities.push(summary);
            }
        }
    }

    // Sort by name
    ui_state.entity_editor.entities.sort_by(|a, b| a.name.cmp(&b.name));
}

fn load_entity_summary(file_path: &Path) -> Option<EntitySummary> {
    let content = std::fs::read_to_string(file_path).ok()?;
    let def: EntityDefinition = serde_json::from_str(&content).ok()?;

    Some(EntitySummary {
        name: def.name.clone(),
        display_name: if def.display_name.is_empty() {
            def.name.clone()
        } else {
            def.display_name
        },
        category: def.category,
        tags: def.tags,
        file_path: file_path.to_path_buf(),
    })
}

fn create_new_entity(ui_state: &mut EditorUI, project_path: &Path) {
    let dialog = &ui_state.entity_editor.new_entity_dialog;
    let name = dialog.name_input.trim().to_string();
    let display_name = if dialog.display_name_input.trim().is_empty() {
        name.clone()
    } else {
        dialog.display_name_input.trim().to_string()
    };
    let description = dialog.description_input.trim().to_string();
    let category = dialog.category.clone();

    // Create entity definition with sensible defaults
    let def = create_default_entity_definition(&name, &display_name, &description, &category);

    // Save to file
    let entities_dir = project_path.join("entities");
    if !entities_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&entities_dir) {
            tracing::error!("Failed to create entities directory: {}", e);
            return;
        }
    }

    let file_path = entities_dir.join(format!("{}.json", name));
    let json = match serde_json::to_string_pretty(&def) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to serialize entity definition: {}", e);
            return;
        }
    };

    if let Err(e) = std::fs::write(&file_path, json) {
        tracing::error!("Failed to write entity definition: {}", e);
        return;
    }

    // Add to browser and select
    let summary = EntitySummary {
        name: name.clone(),
        display_name,
        category,
        tags: Vec::new(),
        file_path,
    };

    ui_state.entity_editor.add_entity(summary);
    ui_state.selection = Some(Selection::EntityDefinition(name));

    tracing::info!("Created new entity definition: {}", def.name);
}

fn create_default_entity_definition(
    name: &str,
    display_name: &str,
    description: &str,
    category: &str,
) -> EntityDefinition {
    use toki_core::entity::{
        AiBehavior, AiConfig, AnimationsDef, AttributesDef, AudioDef, CollisionDef,
        MovementProfile, MovementSoundTrigger, RenderingDef,
    };

    EntityDefinition {
        name: name.to_string(),
        display_name: display_name.to_string(),
        description: description.to_string(),
        rendering: RenderingDef {
            size: [32, 32],
            render_layer: 0,
            visible: true,
            static_object: None,
        },
        attributes: AttributesDef {
            health: None,
            stats: std::collections::HashMap::new(),
            speed: 1.0,
            solid: true,
            active: true,
            can_move: false,
            interactable: false,
            interaction_reach: 0,
            ai_config: AiConfig {
                behavior: AiBehavior::None,
                detection_radius: 0,
            },
            movement_profile: MovementProfile::default(),
            primary_projectile: None,
            pickup: None,
            has_inventory: false,
        },
        collision: CollisionDef {
            enabled: true, // Collision enabled by default per design decision
            offset: [0, 0],
            size: [32, 32],
            trigger: false,
        },
        audio: AudioDef {
            footstep_trigger_distance: 32.0,
            hearing_radius: 192,
            movement_sound_trigger: MovementSoundTrigger::Distance,
            movement_sound: String::new(),
            collision_sound: None,
        },
        animations: AnimationsDef {
            atlas_name: String::new(),
            clips: Vec::new(),
            default_state: "idle".to_string(),
        },
        category: category.to_string(),
        tags: Vec::new(),
    }
}

fn delete_entity(ui_state: &mut EditorUI, project_path: &Path, entity_name: &str) {
    let file_path = project_path
        .join("entities")
        .join(format!("{}.json", entity_name));

    if file_path.exists() {
        if let Err(e) = std::fs::remove_file(&file_path) {
            tracing::error!("Failed to delete entity file: {}", e);
            return;
        }
    }

    ui_state.entity_editor.remove_entity(entity_name);

    // Clear selection if we deleted the selected entity
    if ui_state
        .selection
        .as_ref()
        .map(|s| matches!(s, Selection::EntityDefinition(n) if n == entity_name))
        .unwrap_or(false)
    {
        ui_state.selection = None;
    }

    tracing::info!("Deleted entity definition: {}", entity_name);
}
