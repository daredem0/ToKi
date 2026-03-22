//! Entity file I/O operations - load, save, refresh, create, delete.

use crate::ui::editor_ui::{create_default_definition, EntitySummary, Selection};
use crate::ui::EditorUI;
use std::path::Path;
use toki_core::entity::EntityDefinition;
use toki_core::project_assets::{classify_sprite_metadata_file, SpriteMetadataFileKind};

pub fn refresh_entity_list(ui_state: &mut EditorUI, project_path: Option<&Path>) {
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
        scan_entities_directory(&entities_dir, &mut ui_state.entity_editor.entities);
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

fn scan_entities_directory(entities_dir: &Path, entities: &mut Vec<EntitySummary>) {
    let Ok(entries) = std::fs::read_dir(entities_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let file_path = entry.path();
        if file_path.extension().map(|e| e == "json").unwrap_or(false) {
            if let Some(summary) = load_entity_summary(&file_path) {
                entities.push(summary);
            }
        }
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

pub fn load_entity_definition(file_path: &Path) -> Option<EntityDefinition> {
    let content = std::fs::read_to_string(file_path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_entity(ui_state: &mut EditorUI) {
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
    update_browser_summary(ui_state);

    tracing::info!(
        "Saved entity definition: {}",
        ui_state
            .entity_editor
            .edit_state
            .as_ref()
            .map(|e| e.definition.name.as_str())
            .unwrap_or("unknown")
    );
}

fn update_browser_summary(ui_state: &mut EditorUI) {
    let Some(edit) = ui_state.entity_editor.edit_state.as_ref() else {
        return;
    };

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
}

pub fn revert_entity(ui_state: &mut EditorUI) {
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

pub fn create_new_entity(ui_state: &mut EditorUI, project_path: &Path) {
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

pub fn delete_entity(ui_state: &mut EditorUI, project_path: &Path, entity_name: &str) {
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
