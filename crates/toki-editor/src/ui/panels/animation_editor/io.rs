//! Entity and atlas loading/saving.

use crate::ui::editor_ui::AnimationAuthoringState;
use crate::ui::EditorUI;
use std::path::{Path, PathBuf};
use toki_core::assets::atlas::ColorMode;

type AtlasPreviewInfo = (
    glam::UVec2,
    PathBuf,
    ColorMode,
    Option<String>,
    Option<String>,
    Option<String>,
);

type ObjectSheetPreviewInfo = (
    glam::UVec2,
    PathBuf,
    std::collections::HashMap<String, [u32; 2]>,
    std::collections::HashMap<[u32; 2], [u32; 4]>,
);

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

    let source_name = if !definition.animations.atlas_name.trim().is_empty() {
        definition.animations.atlas_name.clone()
    } else if definition.category.eq_ignore_ascii_case("decoration") {
        definition
            .rendering
            .static_object
            .as_ref()
            .map(|render| render.sheet.clone())
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Load the source to get tile/object name to position mapping and preview metadata
    let atlas_name = source_name.as_str();
    let tile_lookup = load_atlas_tile_lookup(project_path, atlas_name);
    let atlas_info = load_atlas_info(
        project_path,
        atlas_name,
        definition.rendering.palette_override.as_deref(),
    );
    let object_sheet_info = if atlas_info.is_none() {
        load_object_sheet_info(project_path, atlas_name)
    } else {
        None
    };
    let tile_lookup = tile_lookup.or_else(|| {
        object_sheet_info
            .as_ref()
            .map(|(_, _, tile_lookup, _)| tile_lookup.clone())
    });

    let authoring = AnimationAuthoringState::from_animations_def_with_tile_lookup(
        &definition.animations,
        tile_lookup.as_ref(),
    );
    let decoration_idle_only = definition.category.eq_ignore_ascii_case("decoration");

    ui_state
        .animation_editor_context_mut()
        .animation
        .load_entity(entity_name, file_path, authoring, decoration_idle_only);
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_texture = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_texture_path = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_texture_cache_key = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_image_size = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_grid_size = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_cell_size = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_palette_id = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_entity_palette_override = None;
    crate::ui::editor_context::animation_state_mut(ui_state).atlas_default_palette = None;
    crate::ui::editor_context::animation_state_mut(ui_state).source_is_object_sheet = false;
    crate::ui::editor_context::animation_state_mut(ui_state).source_rects_by_position =
        std::collections::HashMap::new();
    crate::ui::editor_context::animation_state_mut(ui_state)
        .authoring
        .atlas_name = source_name;
    crate::ui::editor_context::animation_state_mut(ui_state)
        .authoring
        .source_is_object_sheet = object_sheet_info.is_some();
    crate::ui::editor_context::animation_state_mut(ui_state)
        .authoring
        .source_name_lookup = tile_lookup
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|(name, position)| (position, name))
        .collect();

    // Store atlas metadata for canvas rendering
    if let Some((
        cell_size,
        png_path,
        color_mode,
        palette_id,
        entity_palette_override,
        atlas_default_palette,
    )) = atlas_info
    {
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_cell_size =
            Some((cell_size.x, cell_size.y));
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_texture_path =
            Some(png_path);
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_color_mode = color_mode;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_palette_id = palette_id;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_entity_palette_override =
            entity_palette_override;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_default_palette =
            atlas_default_palette;
        crate::ui::editor_context::animation_state_mut(ui_state).source_is_object_sheet = false;
        crate::ui::editor_context::animation_state_mut(ui_state).source_rects_by_position =
            std::collections::HashMap::new();
    } else if let Some((cell_size, png_path, tile_lookup, rect_lookup)) = object_sheet_info {
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_cell_size =
            Some((cell_size.x, cell_size.y));
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_texture_path =
            Some(png_path);
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_color_mode =
            ColorMode::TrueColor;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_palette_id = None;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_entity_palette_override =
            None;
        crate::ui::editor_context::animation_state_mut(ui_state).atlas_default_palette = None;
        crate::ui::editor_context::animation_state_mut(ui_state).source_is_object_sheet = true;
        crate::ui::editor_context::animation_state_mut(ui_state).source_rects_by_position =
            rect_lookup;
        crate::ui::editor_context::animation_state_mut(ui_state)
            .authoring
            .source_name_lookup = tile_lookup.into_iter().map(|(name, pos)| (pos, name)).collect();
    }

    tracing::info!("Loaded entity for animation editing: {}", entity_name);
}

/// Load atlas metadata and return cell size and PNG path
fn load_atlas_info(
    project_path: &Path,
    atlas_name: &str,
    palette_override: Option<&str>,
) -> Option<AtlasPreviewInfo> {
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

    let effective_palette_id = palette_override
        .map(str::to_string)
        .or_else(|| atlas.palette.clone());

    Some((
        atlas.tile_size,
        png_path,
        atlas.color_mode,
        effective_palette_id,
        palette_override.map(str::to_string),
        atlas.palette.clone(),
    ))
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

fn load_object_sheet_info(
    project_path: &Path,
    sheet_name: &str,
) -> Option<ObjectSheetPreviewInfo> {
    if sheet_name.is_empty() {
        return None;
    }

    let sheet_filename = if sheet_name.ends_with(".json") {
        sheet_name.to_string()
    } else {
        format!("{}.json", sheet_name)
    };
    let sheet_path = project_path
        .join("assets")
        .join("sprites")
        .join(&sheet_filename);
    let object_sheet = toki_core::assets::object_sheet::ObjectSheetMeta::load_from_file(&sheet_path).ok()?;
    let png_path = sheet_path.parent()?.join(&object_sheet.image);

    let mut name_lookup = std::collections::HashMap::new();
    let mut rect_lookup = std::collections::HashMap::new();
    for (name, info) in &object_sheet.objects {
        let position = [info.position.x, info.position.y];
        name_lookup.insert(name.clone(), position);
        rect_lookup.insert(
            position,
            [
                info.position.x * object_sheet.tile_size.x,
                info.position.y * object_sheet.tile_size.y,
                info.size_tiles.x * object_sheet.tile_size.x,
                info.size_tiles.y * object_sheet.tile_size.y,
            ],
        );
    }

    Some((object_sheet.tile_size, png_path, name_lookup, rect_lookup))
}

pub fn save_current_entity(ui_state: &mut EditorUI) {
    let Some(file_path) = crate::ui::editor_context::animation_state_mut(ui_state)
        .entity_file_path
        .clone()
    else {
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
    definition.animations = crate::ui::editor_context::animation_state_mut(ui_state)
        .authoring
        .to_animations_def();

    // Update source metadata with tile/object names for all frame positions
    if let Some(project_path) = file_path.parent().and_then(|p| p.parent()) {
        if crate::ui::editor_context::animation_state(ui_state).source_is_object_sheet {
            sync_object_sheet_frame_names(
                crate::ui::editor_context::animation_state(ui_state)
                    .authoring
                    .source_name_lookup
                    .clone(),
                &mut definition.animations,
            );
        } else {
            sync_atlas_tile_names(project_path, &definition.name, &mut definition.animations);
        }
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

    crate::ui::editor_context::animation_state_mut(ui_state)
        .authoring
        .dirty = false;
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

fn sync_object_sheet_frame_names(
    name_lookup: std::collections::HashMap<[u32; 2], String>,
    animations: &mut toki_core::entity::AnimationsDef,
) {
    for clip in &mut animations.clips {
        let Some(positions) = clip.frame_positions.take() else {
            continue;
        };

        clip.frame_tiles = positions
            .iter()
            .filter_map(|position| name_lookup.get(position).cloned())
            .collect();
    }
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
