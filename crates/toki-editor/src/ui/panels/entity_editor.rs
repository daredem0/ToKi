// Entity editor panel
// Provides a dedicated tab for creating and editing entity definitions
// Phase 4.5A: Entity Editor Tab And Definition Browser
// Phase 4.5B: Optional Component Toggles
// Phase 4.5C: Property Editing

use crate::project::Project;
use crate::ui::editor_ui::{
    create_default_definition, EntityCategory, EntityEditState, EntitySummary, Selection,
};
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::entity::{AiBehavior, EntityDefinition};
use toki_core::project_assets::{classify_sprite_metadata_file, SpriteMetadataFileKind};

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
            if ui_state.entity_editor.is_dirty() {
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
    let mut categories: Vec<String> = ui_state
        .entity_editor
        .all_categories()
        .into_iter()
        .collect();
    categories.sort();

    ui.horizontal(|ui| {
        ui.label("Category:");
        egui::ComboBox::from_id_salt("entity_category_filter")
            .selected_text(
                if ui_state.entity_editor.filter.category_filter.is_empty() {
                    "All"
                } else {
                    &ui_state.entity_editor.filter.category_filter
                },
            )
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
        // Load full entity definition for editing
        if let Some(summary) = ui_state
            .entity_editor
            .entities
            .iter()
            .find(|e| e.name == name)
            .cloned()
        {
            if let Some(def) = load_entity_definition(&summary.file_path) {
                ui_state
                    .entity_editor
                    .load_for_editing(def, summary.file_path);
            }
        }
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
    if ui_state.entity_editor.edit_state.is_none() {
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label("No entity selected");
                ui.add_space(8.0);
                ui.label("Select an entity from the browser or create a new one.");
            });
        });
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("entity_details_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            render_core_properties(ui, ui_state);
            ui.add_space(8.0);
            render_component_toggles(ui, ui_state);
            ui.add_space(8.0);
            render_save_section(ui, ui_state);
        });
}

fn render_core_properties(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    ui.heading("Core Properties");
    ui.separator();

    // Name (identifier)
    ui.horizontal(|ui| {
        ui.label("Name:");
        if ui.text_edit_singleline(&mut edit.definition.name).changed() {
            edit.mark_dirty();
        }
    });
    show_field_error(ui, edit, "name");

    // Display name
    ui.horizontal(|ui| {
        ui.label("Display Name:");
        if ui
            .text_edit_singleline(&mut edit.definition.display_name)
            .changed()
        {
            edit.mark_dirty();
        }
    });
    show_field_error(ui, edit, "display_name");

    // Description
    ui.label("Description:");
    if ui
        .text_edit_multiline(&mut edit.definition.description)
        .changed()
    {
        edit.mark_dirty();
    }

    // Category
    ui.horizontal(|ui| {
        ui.label("Category:");
        if ui
            .text_edit_singleline(&mut edit.definition.category)
            .changed()
        {
            edit.mark_dirty();
        }
    });

    // Tags
    ui.horizontal(|ui| {
        ui.label("Tags:");
        if ui.text_edit_singleline(&mut edit.tags_input).changed() {
            edit.sync_tags();
            edit.mark_dirty();
        }
    });
    ui.label("(comma-separated)");
}

fn render_component_toggles(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.heading("Components");
    ui.separator();

    render_rendering_section(ui, ui_state);
    render_attributes_section(ui, ui_state);
    render_collision_section(ui, ui_state);
    render_health_section(ui, ui_state);
    render_ai_section(ui, ui_state);
    render_inventory_section(ui, ui_state);
    render_projectile_section(ui, ui_state);
    render_pickup_section(ui, ui_state);
    render_audio_section(ui, ui_state);
}

fn render_rendering_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let available_atlases = ui_state.entity_editor.available_atlases.clone();
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    egui::CollapsingHeader::new("Rendering")
        .default_open(true)
        .show(ui, |ui| {
            // Sprite Atlas dropdown
            ui.horizontal(|ui| {
                ui.label("Sprite Atlas:");
                render_atlas_dropdown(
                    ui,
                    "sprite_atlas",
                    &mut edit.definition.animations.atlas_name,
                    &available_atlases,
                    &mut edit.dirty,
                );
            });

            // Size
            ui.horizontal(|ui| {
                ui.label("Size:");
                let mut w = edit.definition.rendering.size[0] as i32;
                let mut h = edit.definition.rendering.size[1] as i32;
                if ui
                    .add(egui::DragValue::new(&mut w).range(1..=1024))
                    .changed()
                {
                    edit.definition.rendering.size[0] = w.max(1) as u32;
                    edit.mark_dirty();
                }
                ui.label("x");
                if ui
                    .add(egui::DragValue::new(&mut h).range(1..=1024))
                    .changed()
                {
                    edit.definition.rendering.size[1] = h.max(1) as u32;
                    edit.mark_dirty();
                }
            });
            show_field_error(ui, edit, "size");

            // Render layer
            ui.horizontal(|ui| {
                ui.label("Render Layer:");
                if ui
                    .add(egui::DragValue::new(
                        &mut edit.definition.rendering.render_layer,
                    ))
                    .changed()
                {
                    edit.mark_dirty();
                }
            });

            // Visible
            if ui
                .checkbox(&mut edit.definition.rendering.visible, "Visible")
                .changed()
            {
                edit.mark_dirty();
            }
        });
}

fn render_attributes_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    egui::CollapsingHeader::new("Attributes")
        .default_open(true)
        .show(ui, |ui| {
            // Speed
            ui.horizontal(|ui| {
                ui.label("Speed:");
                if ui
                    .add(egui::DragValue::new(&mut edit.definition.attributes.speed).speed(0.1))
                    .changed()
                {
                    edit.mark_dirty();
                }
            });

            // Boolean attributes
            if ui
                .checkbox(&mut edit.definition.attributes.solid, "Solid")
                .changed()
            {
                edit.mark_dirty();
            }
            if ui
                .checkbox(&mut edit.definition.attributes.active, "Active")
                .changed()
            {
                edit.mark_dirty();
            }
            if ui
                .checkbox(&mut edit.definition.attributes.can_move, "Can Move")
                .changed()
            {
                edit.mark_dirty();
            }
            if ui
                .checkbox(&mut edit.definition.attributes.interactable, "Interactable")
                .changed()
            {
                edit.mark_dirty();
            }

            // Interaction reach (only if interactable)
            if edit.definition.attributes.interactable {
                ui.horizontal(|ui| {
                    ui.label("Interaction Reach:");
                    let mut reach = edit.definition.attributes.interaction_reach as i32;
                    if ui
                        .add(egui::DragValue::new(&mut reach).range(0..=256))
                        .changed()
                    {
                        edit.definition.attributes.interaction_reach = reach.max(0) as u32;
                        edit.mark_dirty();
                    }
                });
            }
        });
}

fn render_collision_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    ui.horizontal(|ui| {
        if ui
            .checkbox(&mut edit.toggles.collision_enabled, "Collision")
            .changed()
        {
            edit.definition.collision.enabled = edit.toggles.collision_enabled;
            edit.mark_dirty();
        }
    });

    if edit.toggles.collision_enabled {
        egui::CollapsingHeader::new("  Collision Settings")
            .default_open(false)
            .show(ui, |ui| {
                // Offset
                ui.horizontal(|ui| {
                    ui.label("Offset:");
                    if ui
                        .add(egui::DragValue::new(
                            &mut edit.definition.collision.offset[0],
                        ))
                        .changed()
                    {
                        edit.mark_dirty();
                    }
                    ui.label(",");
                    if ui
                        .add(egui::DragValue::new(
                            &mut edit.definition.collision.offset[1],
                        ))
                        .changed()
                    {
                        edit.mark_dirty();
                    }
                });

                // Size
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    let mut w = edit.definition.collision.size[0] as i32;
                    let mut h = edit.definition.collision.size[1] as i32;
                    if ui
                        .add(egui::DragValue::new(&mut w).range(1..=1024))
                        .changed()
                    {
                        edit.definition.collision.size[0] = w.max(1) as u32;
                        edit.mark_dirty();
                    }
                    ui.label("x");
                    if ui
                        .add(egui::DragValue::new(&mut h).range(1..=1024))
                        .changed()
                    {
                        edit.definition.collision.size[1] = h.max(1) as u32;
                        edit.mark_dirty();
                    }
                });
                show_field_error(ui, edit, "collision_size");

                // Trigger
                if ui
                    .checkbox(&mut edit.definition.collision.trigger, "Is Trigger")
                    .changed()
                {
                    edit.mark_dirty();
                }
            });
    }
}

fn render_health_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.health_enabled;
    if ui.checkbox(&mut toggle, "Health").changed() {
        edit.toggle_health();
    }

    if edit.toggles.health_enabled {
        ui.horizontal(|ui| {
            ui.label("  Max HP:");
            let mut hp = edit.definition.attributes.health.unwrap_or(100) as i32;
            if ui
                .add(egui::DragValue::new(&mut hp).range(1..=99999))
                .changed()
            {
                edit.definition.attributes.health = Some(hp.max(1) as u32);
                edit.mark_dirty();
            }
        });
        show_field_error(ui, edit, "health");
    }
}

fn render_ai_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.ai_enabled;
    if ui.checkbox(&mut toggle, "AI").changed() {
        edit.toggle_ai();
    }

    if edit.toggles.ai_enabled {
        egui::CollapsingHeader::new("  AI Settings")
            .default_open(false)
            .show(ui, |ui| {
                // Behavior dropdown
                ui.horizontal(|ui| {
                    ui.label("Behavior:");
                    let current = format!("{:?}", edit.definition.attributes.ai_config.behavior);
                    egui::ComboBox::from_id_salt("ai_behavior")
                        .selected_text(&current)
                        .show_ui(ui, |ui| {
                            for behavior in [
                                AiBehavior::Wander,
                                AiBehavior::Chase,
                                AiBehavior::Run,
                                AiBehavior::RunAndMultiply,
                            ] {
                                let label = format!("{:?}", behavior);
                                if ui
                                    .selectable_value(
                                        &mut edit.definition.attributes.ai_config.behavior,
                                        behavior,
                                        &label,
                                    )
                                    .changed()
                                {
                                    edit.mark_dirty();
                                }
                            }
                        });
                });

                // Detection radius
                ui.horizontal(|ui| {
                    ui.label("Detection Radius:");
                    let mut radius = edit.definition.attributes.ai_config.detection_radius as i32;
                    if ui
                        .add(egui::DragValue::new(&mut radius).range(0..=1024))
                        .changed()
                    {
                        edit.definition.attributes.ai_config.detection_radius =
                            radius.max(0) as u32;
                        edit.mark_dirty();
                    }
                });
            });
    }
}

fn render_inventory_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.inventory_enabled;
    if ui.checkbox(&mut toggle, "Inventory").changed() {
        edit.toggle_inventory();
    }
}

fn render_projectile_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.projectile_enabled;
    if ui.checkbox(&mut toggle, "Projectile").changed() {
        edit.toggle_projectile();
    }

    if edit.toggles.projectile_enabled {
        if let Some(proj) = edit.definition.attributes.primary_projectile.as_mut() {
            egui::CollapsingHeader::new("  Projectile Settings")
                .default_open(false)
                .show(ui, |ui| {
                    // Sheet
                    ui.horizontal(|ui| {
                        ui.label("Sheet:");
                        if ui.text_edit_singleline(&mut proj.sheet).changed() {
                            edit.dirty = true;
                        }
                    });

                    // Object name
                    ui.horizontal(|ui| {
                        ui.label("Object:");
                        if ui.text_edit_singleline(&mut proj.object_name).changed() {
                            edit.dirty = true;
                        }
                    });

                    // Size
                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        let mut w = proj.size[0] as i32;
                        let mut h = proj.size[1] as i32;
                        if ui
                            .add(egui::DragValue::new(&mut w).range(1..=256))
                            .changed()
                        {
                            proj.size[0] = w.max(1) as u32;
                            edit.dirty = true;
                        }
                        ui.label("x");
                        if ui
                            .add(egui::DragValue::new(&mut h).range(1..=256))
                            .changed()
                        {
                            proj.size[1] = h.max(1) as u32;
                            edit.dirty = true;
                        }
                    });

                    // Speed
                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        let mut speed = proj.speed as i32;
                        if ui
                            .add(egui::DragValue::new(&mut speed).range(1..=9999))
                            .changed()
                        {
                            proj.speed = speed.max(1) as u32;
                            edit.dirty = true;
                        }
                    });

                    // Damage
                    ui.horizontal(|ui| {
                        ui.label("Damage:");
                        if ui.add(egui::DragValue::new(&mut proj.damage)).changed() {
                            edit.dirty = true;
                        }
                    });

                    // Lifetime
                    ui.horizontal(|ui| {
                        ui.label("Lifetime (ticks):");
                        let mut lifetime = proj.lifetime_ticks as i32;
                        if ui
                            .add(egui::DragValue::new(&mut lifetime).range(1..=9999))
                            .changed()
                        {
                            proj.lifetime_ticks = lifetime.max(1) as u32;
                            edit.dirty = true;
                        }
                    });
                });
        }
    }
}

fn render_pickup_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.pickup_enabled;
    if ui.checkbox(&mut toggle, "Pickup").changed() {
        edit.toggle_pickup();
    }

    if edit.toggles.pickup_enabled {
        if let Some(pickup) = edit.definition.attributes.pickup.as_mut() {
            egui::CollapsingHeader::new("  Pickup Settings")
                .default_open(false)
                .show(ui, |ui| {
                    // Item ID
                    ui.horizontal(|ui| {
                        ui.label("Item ID:");
                        if ui.text_edit_singleline(&mut pickup.item_id).changed() {
                            edit.dirty = true;
                        }
                    });

                    // Count
                    ui.horizontal(|ui| {
                        ui.label("Count:");
                        let mut count = pickup.count as i32;
                        if ui
                            .add(egui::DragValue::new(&mut count).range(1..=9999))
                            .changed()
                        {
                            pickup.count = count.max(1) as u32;
                            edit.dirty = true;
                        }
                    });
                });
        }
    }
}

fn render_audio_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    let available_sfx = ui_state.entity_editor.available_sfx.clone();
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    let mut toggle = edit.toggles.audio_enabled;
    if ui.checkbox(&mut toggle, "Audio").changed() {
        edit.toggle_audio();
    }

    if edit.toggles.audio_enabled {
        egui::CollapsingHeader::new("  Audio Settings")
            .default_open(false)
            .show(ui, |ui| {
                // Movement sound - dropdown from discovered SFX
                ui.horizontal(|ui| {
                    ui.label("Movement Sound:");
                    render_sfx_dropdown(
                        ui,
                        "movement_sound",
                        &mut edit.definition.audio.movement_sound,
                        &available_sfx,
                        &mut edit.dirty,
                    );
                });

                // Collision sound - dropdown from discovered SFX
                ui.horizontal(|ui| {
                    ui.label("Collision Sound:");
                    let mut sound = edit
                        .definition
                        .audio
                        .collision_sound
                        .clone()
                        .unwrap_or_default();
                    if render_sfx_dropdown(
                        ui,
                        "collision_sound",
                        &mut sound,
                        &available_sfx,
                        &mut edit.dirty,
                    ) {
                        edit.definition.audio.collision_sound =
                            if sound.is_empty() { None } else { Some(sound) };
                    }
                });

                // Hearing radius
                ui.horizontal(|ui| {
                    ui.label("Hearing Radius:");
                    let mut radius = edit.definition.audio.hearing_radius as i32;
                    if ui
                        .add(egui::DragValue::new(&mut radius).range(0..=1024))
                        .changed()
                    {
                        edit.definition.audio.hearing_radius = radius.max(0) as u32;
                        edit.mark_dirty();
                    }
                });

                // Footstep distance
                ui.horizontal(|ui| {
                    ui.label("Footstep Distance:");
                    if ui
                        .add(
                            egui::DragValue::new(
                                &mut edit.definition.audio.footstep_trigger_distance,
                            )
                            .speed(0.1),
                        )
                        .changed()
                    {
                        edit.mark_dirty();
                    }
                });
            });
    }
}

/// Render a dropdown for selecting SFX sounds. Returns true if the value changed.
fn render_sfx_dropdown(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    available: &[String],
    dirty: &mut bool,
) -> bool {
    let display_text = if current.is_empty() {
        "(None)"
    } else {
        current.as_str()
    };

    let mut changed = false;

    egui::ComboBox::from_id_salt(id)
        .selected_text(display_text)
        .show_ui(ui, |ui| {
            // None option
            if ui.selectable_label(current.is_empty(), "(None)").clicked() {
                current.clear();
                *dirty = true;
                changed = true;
            }

            // Available SFX
            for sfx in available {
                let is_selected = current == sfx;
                if ui.selectable_label(is_selected, sfx).clicked() {
                    *current = sfx.clone();
                    *dirty = true;
                    changed = true;
                }
            }
        });

    changed
}

/// Render a dropdown for selecting sprite atlases.
fn render_atlas_dropdown(
    ui: &mut egui::Ui,
    id: &str,
    current: &mut String,
    available: &[String],
    dirty: &mut bool,
) {
    let display_text = if current.is_empty() {
        "(None)"
    } else {
        current.as_str()
    };

    egui::ComboBox::from_id_salt(id)
        .selected_text(display_text)
        .show_ui(ui, |ui| {
            // None option
            if ui.selectable_label(current.is_empty(), "(None)").clicked() {
                current.clear();
                *dirty = true;
            }

            // Available atlases
            for atlas in available {
                let is_selected = current == atlas;
                if ui.selectable_label(is_selected, atlas).clicked() {
                    *current = atlas.clone();
                    *dirty = true;
                }
            }
        });
}

fn render_save_section(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.separator();

    let is_dirty = ui_state.entity_editor.is_dirty();

    ui.horizontal(|ui| {
        let save_enabled = is_dirty;
        if ui
            .add_enabled(save_enabled, egui::Button::new("Save"))
            .clicked()
        {
            save_entity(ui_state);
        }

        if ui
            .add_enabled(is_dirty, egui::Button::new("Revert"))
            .clicked()
        {
            revert_entity(ui_state);
        }
    });

    // Show validation errors summary
    if let Some(edit) = &ui_state.entity_editor.edit_state {
        if !edit.validation_errors.is_empty() {
            ui.colored_label(
                egui::Color32::RED,
                format!("{} validation error(s)", edit.validation_errors.len()),
            );
        }
    }
}

fn show_field_error(ui: &mut egui::Ui, edit: &EntityEditState, field: &str) {
    if let Some(error) = edit.get_error(field) {
        ui.colored_label(egui::Color32::RED, error);
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
            ui.text_edit_multiline(&mut ui_state.entity_editor.new_entity_dialog.description_input);

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
                                ui_state.entity_editor.new_entity_dialog.category =
                                    category.clone();
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
    let entity_name = ui_state
        .entity_editor
        .delete_confirmation
        .entity_name
        .clone();
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
    ui_state.entity_editor.available_sfx.clear();
    ui_state.entity_editor.available_atlases.clear();

    let Some(path) = project_path else {
        return;
    };

    // Scan entities
    let entities_dir = path.join("entities");
    ui_state.entity_editor.entities_dir = Some(entities_dir.clone());

    if entities_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&entities_dir) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Some(summary) = load_entity_summary(&file_path) {
                        ui_state.entity_editor.entities.push(summary);
                    }
                }
            }
        }
    }

    // Sort by name
    ui_state
        .entity_editor
        .entities
        .sort_by(|a, b| a.name.cmp(&b.name));

    // Scan SFX directory for available sound effects
    let sfx_dir = path.join("assets/audio/sfx");
    if sfx_dir.exists() {
        scan_sfx_directory(&sfx_dir, &mut ui_state.entity_editor.available_sfx);
        ui_state.entity_editor.available_sfx.sort();
    }

    // Scan sprites directory for available atlases
    let sprites_dir = path.join("assets/sprites");
    if sprites_dir.exists() {
        scan_atlas_directory(&sprites_dir, &mut ui_state.entity_editor.available_atlases);
        ui_state.entity_editor.available_atlases.sort();
    }
}

fn scan_sfx_directory(dir: &Path, sfx_list: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_sfx_directory(&path, sfx_list);
            continue;
        }

        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };

        if matches!(ext.to_ascii_lowercase().as_str(), "ogg" | "wav" | "mp3") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                sfx_list.push(stem.to_string());
            }
        }
    }
}

fn scan_atlas_directory(dir: &Path, atlas_list: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        if path.extension().map(|e| e == "json").unwrap_or(false) {
            // Use classify_sprite_metadata_file to check if it's an atlas
            if let Ok(SpriteMetadataFileKind::Atlas) = classify_sprite_metadata_file(&path) {
                // Include .json extension for consistency with animation editor
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    atlas_list.push(filename.to_string());
                }
            }
        }
    }
}

fn load_entity_summary(file_path: &Path) -> Option<EntitySummary> {
    let def = load_entity_definition(file_path)?;

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

fn load_entity_definition(file_path: &Path) -> Option<EntityDefinition> {
    let content = std::fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_entity(ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_mut() else {
        return;
    };

    // Validate before saving
    if !edit.validate() {
        tracing::warn!("Entity validation failed, cannot save");
        return;
    }

    // Serialize to JSON
    let json = match serde_json::to_string_pretty(&edit.definition) {
        Ok(j) => j,
        Err(e) => {
            tracing::error!("Failed to serialize entity definition: {}", e);
            return;
        }
    };

    // Write to file
    if let Err(e) = std::fs::write(&edit.file_path, json) {
        tracing::error!("Failed to write entity definition: {}", e);
        return;
    }

    // Clear dirty flag
    edit.dirty = false;

    // Update the summary in the browser list
    if let Some(summary) = ui_state
        .entity_editor
        .entities
        .iter_mut()
        .find(|e| e.file_path == edit.file_path)
    {
        summary.name = edit.definition.name.clone();
        summary.display_name = if edit.definition.display_name.is_empty() {
            edit.definition.name.clone()
        } else {
            edit.definition.display_name.clone()
        };
        summary.category = edit.definition.category.clone();
        summary.tags = edit.definition.tags.clone();
    }

    tracing::info!("Saved entity definition: {}", edit.definition.name);
}

fn revert_entity(ui_state: &mut EditorUI) {
    let Some(edit) = &ui_state.entity_editor.edit_state else {
        return;
    };

    let file_path = edit.file_path.clone();

    // Reload from file
    if let Some(def) = load_entity_definition(&file_path) {
        ui_state.entity_editor.load_for_editing(def, file_path);
        tracing::info!("Reverted entity changes");
    }
}

fn create_new_entity(ui_state: &mut EditorUI, project_path: &Path) {
    let dialog = &ui_state.entity_editor.new_entity_dialog;
    let name = dialog.name_input.trim().to_string();
    let display_name = if dialog.display_name_input.trim().is_empty() {
        name.clone()
    } else {
        dialog.display_name_input.trim().to_string()
    };
    let category = dialog.category.clone();

    // Create entity definition with sensible defaults
    let mut def = create_default_definition(&name, &display_name, &category);
    def.description = dialog.description_input.trim().to_string();

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
        display_name: display_name.clone(),
        category,
        tags: Vec::new(),
        file_path: file_path.clone(),
    };

    ui_state.entity_editor.add_entity(summary);
    ui_state.selection = Some(Selection::EntityDefinition(name.clone()));

    // Load for editing immediately
    ui_state.entity_editor.load_for_editing(def, file_path);

    tracing::info!("Created new entity definition: {}", name);
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
