//! Entity browser panel - entity list with filtering and selection.

use crate::ui::editor_ui::{EntitySummary, Selection};
use crate::ui::EditorUI;
use std::path::Path;

use super::io::load_entity_definition;

pub fn render_entity_browser(ui: &mut egui::Ui, ui_state: &mut EditorUI, _project_path: Option<&Path>) {
    render_search_box(ui, ui_state);
    render_category_filter(ui, ui_state);
    render_clear_filters_button(ui, ui_state);

    ui.separator();

    let filtered = ui_state.entity_editor.filtered_entities();
    let entity_count = filtered.len();

    ui.label(format!("Entities: {}", entity_count));

    let (select_entity, duplicate_entity, delete_entity) = render_entity_list(ui, ui_state, &filtered);

    handle_deferred_actions(ui_state, select_entity, duplicate_entity, delete_entity);
}

fn render_search_box(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(&mut ui_state.entity_editor.filter.search_query);
    });
}

fn render_category_filter(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
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
}

fn render_clear_filters_button(ui: &mut egui::Ui, ui_state: &mut EditorUI) {
    if ui_state.entity_editor.filter.is_active() && ui.button("Clear Filters").clicked() {
        ui_state.entity_editor.filter.clear();
    }
}

fn render_entity_list(
    ui: &mut egui::Ui,
    ui_state: &EditorUI,
    filtered: &[&EntitySummary],
) -> (Option<String>, Option<EntitySummary>, Option<String>) {
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
                    let response = ui.selectable_label(
                        is_selected,
                        format!("{} ({})", entity.display_name, entity.category),
                    );

                    if response.clicked() {
                        select_entity = Some(entity.name.clone());
                    }

                    response.context_menu(|ui| {
                        if ui.button("Duplicate").clicked() {
                            duplicate_entity = Some((*entity).clone());
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

    (select_entity, duplicate_entity, delete_entity)
}

fn handle_deferred_actions(
    ui_state: &mut EditorUI,
    select_entity: Option<String>,
    duplicate_entity: Option<EntitySummary>,
    delete_entity: Option<String>,
) {
    if let Some(name) = select_entity {
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
