//! Entity and atlas loading/saving.

use crate::ui::editor_ui::AnimationAuthoringState;
use crate::ui::EditorUI;
use std::path::{Path, PathBuf};

pub fn load_entity(ui_state: &mut EditorUI, project_path: &Path, entity_name: &str) {
    let file_path = project_path
        .join("entities")
        .join(format!("{}.json", entity_name));

    let Ok(content) = std::fs::read_to_string(&file_path) else {
        tracing::error!("Failed to read entity file: {:?}", file_path);
        return;
    };

    let Ok(definition): Result<toki_core::entity::EntityDefinition, _> =
        serde_json::from_str(&content)
    else {
        tracing::error!("Failed to parse entity definition: {:?}", file_path);
        return;
    };

    // Load the atlas to get tile name to position mapping and metadata
    let atlas_name = &definition.animations.atlas_name;
    let tile_lookup = load_atlas_tile_lookup(project_path, atlas_name);
    let atlas_info = load_atlas_info(project_path, atlas_name);

    let authoring = AnimationAuthoringState::from_animations_def_with_tile_lookup(
        &definition.animations,
        tile_lookup.as_ref(),
    );

    ui_state
        .animation
        .load_entity(entity_name, file_path, authoring);

    // Store atlas metadata for canvas rendering
    if let Some((cell_size, png_path)) = atlas_info {
        ui_state.animation.atlas_cell_size = Some((cell_size.x, cell_size.y));
        ui_state.animation.atlas_texture_path = Some(png_path);
    }

    tracing::info!("Loaded entity for animation editing: {}", entity_name);
}

/// Load atlas metadata and return cell size and PNG path
fn load_atlas_info(project_path: &Path, atlas_name: &str) -> Option<(glam::UVec2, PathBuf)> {
    if atlas_name.is_empty() {
        return None;
    }

    // Normalize atlas name: ensure .json extension
    let atlas_filename = if atlas_name.ends_with(".json") {
        atlas_name.to_string()
    } else {
        format!("{}.json", atlas_name)
    };

    let atlas_path = project_path
        .join("assets")
        .join("sprites")
        .join(&atlas_filename);
    let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_path).ok()?;

    // Get PNG path relative to atlas JSON
    let png_path = atlas_path.parent()?.join(&atlas.image);

    Some((atlas.tile_size, png_path))
}

/// Load an atlas file and extract the tile name to position mapping
fn load_atlas_tile_lookup(
    project_path: &Path,
    atlas_name: &str,
) -> Option<std::collections::HashMap<String, [u32; 2]>> {
    if atlas_name.is_empty() {
        return None;
    }

    // Normalize atlas name: ensure .json extension
    let atlas_filename = if atlas_name.ends_with(".json") {
        atlas_name.to_string()
    } else {
        format!("{}.json", atlas_name)
    };

    // Atlas files are in assets/sprites/
    let atlas_path = project_path
        .join("assets")
        .join("sprites")
        .join(&atlas_filename);

    // Use AtlasMeta from toki-core to load and parse the atlas
    let atlas = toki_core::assets::atlas::AtlasMeta::load_from_file(&atlas_path).ok()?;

    let lookup: std::collections::HashMap<String, [u32; 2]> = atlas
        .tiles
        .into_iter()
        .map(|(name, info)| (name, [info.position.x, info.position.y]))
        .collect();

    Some(lookup)
}

pub fn save_current_entity(ui_state: &mut EditorUI) {
    let Some(file_path) = ui_state.animation.entity_file_path.clone() else {
        tracing::error!("No entity file path set");
        return;
    };

    // Read the current definition
    let Ok(content) = std::fs::read_to_string(&file_path) else {
        tracing::error!("Failed to read entity file for saving: {:?}", file_path);
        return;
    };

    let Ok(mut definition): Result<toki_core::entity::EntityDefinition, _> =
        serde_json::from_str(&content)
    else {
        tracing::error!(
            "Failed to parse entity definition for saving: {:?}",
            file_path
        );
        return;
    };

    // Update animations from authoring state
    definition.animations = ui_state.animation.authoring.to_animations_def();

    // Update atlas metadata with tile names for all frame positions
    if let Some(project_path) = file_path.parent().and_then(|p| p.parent()) {
        sync_atlas_tile_names(project_path, &definition.name, &mut definition.animations);
    }

    // Write back
    let Ok(json) = serde_json::to_string_pretty(&definition) else {
        tracing::error!("Failed to serialize entity definition");
        return;
    };

    if let Err(e) = std::fs::write(&file_path, json) {
        tracing::error!("Failed to write entity file: {}", e);
        return;
    }

    ui_state.animation.authoring.dirty = false;
    tracing::info!("Saved animation changes to {:?}", file_path);
}

/// Sync atlas metadata to have proper tile names for all frame positions used in animations.
/// Clears all existing tiles and writes fresh entries with proper naming convention.
/// Naming convention: `<entity_name>/<state>_<frame_letter>` (e.g., soldier/walk_down_a)
fn sync_atlas_tile_names(
    project_path: &Path,
    entity_name: &str,
    animations: &mut toki_core::entity::AnimationsDef,
) {
    use toki_core::assets::atlas::{AtlasMeta, TileInfo, TileProperties};

    let atlas_name = &animations.atlas_name;
    if atlas_name.is_empty() {
        return;
    }

    let atlas_path = resolve_atlas_path(project_path, atlas_name);
    let Ok(mut atlas) = AtlasMeta::load_from_file(&atlas_path) else {
        tracing::warn!("Failed to load atlas for tile name sync: {:?}", atlas_path);
        return;
    };

    // Clear all existing tiles
    atlas.tiles.clear();

    // Process each animation clip
    for clip in &mut animations.clips {
        let Some(positions) = clip.frame_positions.take() else {
            continue;
        };

        // Generate proper tile names for this clip
        let tile_names = generate_tile_names(entity_name, &clip.state, positions.len());

        // Add tiles with proper names
        for (i, pos) in positions.iter().enumerate() {
            let tile_name = &tile_names[i];
            atlas.tiles.insert(
                tile_name.clone(),
                TileInfo {
                    position: glam::UVec2::new(pos[0], pos[1]),
                    properties: TileProperties::default(),
                },
            );
        }

        clip.frame_tiles = tile_names;
    }

    save_atlas(&atlas_path, &atlas);
}

/// Resolve atlas filename to full path
fn resolve_atlas_path(project_path: &Path, atlas_name: &str) -> PathBuf {
    let atlas_filename = if atlas_name.ends_with(".json") {
        atlas_name.to_string()
    } else {
        format!("{}.json", atlas_name)
    };
    project_path
        .join("assets")
        .join("sprites")
        .join(&atlas_filename)
}

/// Generate proper tile names for frame positions following the naming convention.
/// Always creates names in format `entity/state_letter` (e.g., soldier/walk_down_a).
fn generate_tile_names(entity_name: &str, state: &str, frame_count: usize) -> Vec<String> {
    const FRAME_LETTERS: &[char] = &[
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];

    (0..frame_count)
        .map(|i| {
            let letter = FRAME_LETTERS.get(i).unwrap_or(&'z');
            format!("{}/{}_{}", entity_name, state, letter)
        })
        .collect()
}

/// Save atlas metadata to file
fn save_atlas(atlas_path: &Path, atlas: &toki_core::assets::atlas::AtlasMeta) {
    if let Err(e) = atlas.save_to_file(atlas_path) {
        tracing::error!("Failed to save atlas with new tile names: {}", e);
    } else {
        tracing::info!("Updated atlas with tile names: {:?}", atlas_path);
    }
}
