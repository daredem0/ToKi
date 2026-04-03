use std::path::{Path, PathBuf};

use crate::ui::editor_ui::DecorationPlacementDraft;
use crate::ui::entity_kind_policy::default_grounding_for_kind;
use toki_core::assets::object_sheet::ObjectSheetMeta;
use toki_core::entity::EntityKind;

#[derive(Debug, Clone)]
pub(crate) struct ObjectSheetBrowserSource {
    pub sheet_names: Vec<String>,
    pub selected_sheet_name: String,
    pub object_names: Vec<String>,
    pub object_sheet: ObjectSheetMeta,
    pub texture_path: PathBuf,
}

pub(crate) fn resolve_object_sheet_browser_source(
    project_path: &Path,
    selected_sheet_name: Option<&str>,
) -> Option<ObjectSheetBrowserSource> {
    let sprites_dir = project_path.join("assets").join("sprites");
    let mut object_sheets = Vec::new();

    for entry in std::fs::read_dir(&sprites_dir).ok()? {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Ok(object_sheet) = ObjectSheetMeta::load_from_file(&path) else {
            continue;
        };
        let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        object_sheets.push((stem.to_string(), path, object_sheet));
    }

    if object_sheets.is_empty() {
        return None;
    }

    object_sheets.sort_by(|left, right| left.0.cmp(&right.0));
    let sheet_names = object_sheets
        .iter()
        .map(|(name, _, _)| name.clone())
        .collect::<Vec<_>>();
    let selected_sheet_name = selected_sheet_name
        .filter(|selected| sheet_names.iter().any(|name| name == selected))
        .map(str::to_string)
        .unwrap_or_else(|| sheet_names[0].clone());

    let (_, object_sheet_path, object_sheet) = object_sheets
        .into_iter()
        .find(|(name, _, _)| name == &selected_sheet_name)?;

    let mut object_names = object_sheet.objects.keys().cloned().collect::<Vec<_>>();
    object_names.sort_by(|left, right| {
        let left_info = object_sheet.objects.get(left);
        let right_info = object_sheet.objects.get(right);

        match (left_info, right_info) {
            (Some(left_info), Some(right_info)) => left_info
                .position
                .y
                .cmp(&right_info.position.y)
                .then_with(|| left_info.position.x.cmp(&right_info.position.x))
                .then_with(|| left.cmp(right)),
            _ => left.cmp(right),
        }
    });
    let texture_path = object_sheet_path.parent()?.join(&object_sheet.image);

    Some(ObjectSheetBrowserSource {
        sheet_names,
        selected_sheet_name,
        object_names,
        object_sheet,
        texture_path,
    })
}

pub(crate) fn sync_selected_sheet_name(
    selected_sheet_name: &mut Option<String>,
    sheet_names: &[String],
) {
    if sheet_names.is_empty() {
        *selected_sheet_name = None;
        return;
    }

    if selected_sheet_name
        .as_ref()
        .is_some_and(|selected| sheet_names.iter().any(|name| name == selected))
    {
        return;
    }

    *selected_sheet_name = Some(sheet_names[0].clone());
}

pub(crate) fn sync_selected_object_name(
    selected_object_name: &mut Option<String>,
    object_names: &[String],
) {
    if object_names.is_empty() {
        *selected_object_name = None;
        return;
    }

    if selected_object_name
        .as_ref()
        .is_some_and(|selected| object_names.iter().any(|name| name == selected))
    {
        return;
    }

    *selected_object_name = Some(object_names[0].clone());
}

pub(crate) fn ensure_object_sheet_preview_texture(
    preview_image_path: &mut Option<PathBuf>,
    preview_texture: &mut Option<egui::TextureHandle>,
    ctx: &egui::Context,
    texture_path: &Path,
) -> Option<egui::TextureHandle> {
    if preview_image_path.as_deref() == Some(texture_path) && preview_texture.is_some() {
        return preview_texture.clone();
    }

    let decoded = toki_core::graphics::image::load_image_rgba8(texture_path).ok()?;
    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        [decoded.width as usize, decoded.height as usize],
        &decoded.data,
    );
    let key = format!("object_sheet_preview:{}", texture_path.display());
    let texture = ctx.load_texture(key, color_image, egui::TextureOptions::NEAREST);
    *preview_image_path = Some(texture_path.to_path_buf());
    *preview_texture = Some(texture.clone());
    Some(texture)
}

pub(crate) fn render_object_gallery_item(
    ui: &mut egui::Ui,
    texture_id: egui::TextureId,
    texture_size: glam::UVec2,
    object_sheet: &ObjectSheetMeta,
    object_name: &str,
    selected: bool,
    slot_size: f32,
) -> egui::Response {
    let desired_size = egui::vec2(slot_size, slot_size);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let frame_stroke = if selected {
        egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE)
    } else {
        egui::Stroke::new(1.0, egui::Color32::GRAY)
    };
    let frame_fill = if selected {
        egui::Color32::from_rgb(35, 55, 75)
    } else {
        egui::Color32::from_gray(24)
    };
    ui.painter().rect(
        rect,
        4.0,
        frame_fill,
        frame_stroke,
        egui::StrokeKind::Outside,
    );

    if let Some(rect_px) = object_sheet.get_object_rect(object_name) {
        let uv_rect = egui::Rect::from_min_max(
            egui::pos2(
                rect_px[0] as f32 / texture_size.x as f32,
                rect_px[1] as f32 / texture_size.y as f32,
            ),
            egui::pos2(
                (rect_px[0] + rect_px[2]) as f32 / texture_size.x as f32,
                (rect_px[1] + rect_px[3]) as f32 / texture_size.y as f32,
            ),
        );
        let max_dimension = rect_px[2].max(rect_px[3]) as f32;
        let preview_scale = if max_dimension > 0.0 {
            (slot_size - 8.0) / max_dimension
        } else {
            1.0
        };
        let preview_size = egui::vec2(
            rect_px[2] as f32 * preview_scale,
            rect_px[3] as f32 * preview_scale,
        );
        let preview_rect = egui::Rect::from_center_size(rect.center(), preview_size);
        ui.painter()
            .image(texture_id, preview_rect, uv_rect, egui::Color32::WHITE);
    }

    response.on_hover_text(object_name)
}

pub(crate) fn build_decoration_placement_draft(
    project_path: &Path,
    selected_sheet: &str,
    selected_object: &str,
) -> Option<DecorationPlacementDraft> {
    let source = resolve_object_sheet_browser_source(project_path, Some(selected_sheet))?;
    let object_info = source.object_sheet.objects.get(selected_object)?;
    let size_px = [
        object_info.size_tiles.x * source.object_sheet.tile_size.x,
        object_info.size_tiles.y * source.object_sheet.tile_size.y,
    ];
    Some(DecorationPlacementDraft {
        sheet: selected_sheet.to_string(),
        object_name: selected_object.to_string(),
        size_px: glam::UVec2::new(size_px[0], size_px[1]),
        grounding: default_grounding_for_kind(EntityKind::Decoration, size_px),
        visible: true,
        solid: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_project_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "toki-object-sheet-browser-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(path.join("assets").join("sprites")).expect("sprites dir");
        path
    }

    #[test]
    fn resolve_object_sheet_browser_source_sorts_sheet_and_object_names() {
        let project_path = temp_project_root("sorting");
        fs::write(
            project_path.join("assets/sprites/zeta.json"),
            r#"{
              "sheet_type": "objects",
              "image": "zeta.png",
              "tile_size": [16, 16],
              "objects": {
                "late": {"position": [16, 16], "size_tiles": [1, 1]},
                "first": {"position": [0, 0], "size_tiles": [1, 1]}
              }
            }"#,
        )
        .expect("zeta sheet");
        fs::write(
            project_path.join("assets/sprites/alpha.json"),
            r#"{
              "sheet_type": "objects",
              "image": "alpha.png",
              "tile_size": [16, 16],
              "objects": {
                "b": {"position": [16, 0], "size_tiles": [1, 1]},
                "a": {"position": [0, 0], "size_tiles": [1, 1]}
              }
            }"#,
        )
        .expect("alpha sheet");

        let source = resolve_object_sheet_browser_source(&project_path, None).expect("source");

        assert_eq!(
            source.sheet_names,
            vec!["alpha".to_string(), "zeta".to_string()]
        );
        assert_eq!(source.selected_sheet_name, "alpha");
        assert_eq!(source.object_names, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn build_decoration_placement_draft_uses_size_based_grounding_defaults() {
        let project_path = temp_project_root("decoration-defaults");
        fs::write(
            project_path.join("assets/sprites/decor.json"),
            r#"{
              "sheet_type": "objects",
              "image": "decor.png",
              "tile_size": [16, 16],
              "objects": {
                "flower": {"position": [0, 0], "size_tiles": [1, 1]},
                "house": {"position": [16, 0], "size_tiles": [2, 2]}
              }
            }"#,
        )
        .expect("decor sheet");

        let flower = build_decoration_placement_draft(&project_path, "decor", "flower")
            .expect("flower draft");
        let house =
            build_decoration_placement_draft(&project_path, "decor", "house").expect("house draft");

        assert_eq!(flower.size_px, glam::UVec2::new(16, 16));
        assert_eq!(
            flower.grounding.footprint,
            Some(toki_core::entity::EntityFootprint::new([2, 12], [12, 4]))
        );

        assert_eq!(house.size_px, glam::UVec2::new(32, 32));
        assert_eq!(
            house.grounding.footprint,
            Some(toki_core::entity::EntityFootprint::new([4, 24], [24, 8]))
        );
    }
}
