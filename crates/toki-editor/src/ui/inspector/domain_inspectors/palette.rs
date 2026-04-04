//! Palette inspector — shows palette details when a palette is selected in the hierarchy.

use super::super::super::inspector_trait::{Inspector, InspectorContext};
use toki_core::palette::{builtin_palettes, Palette};

pub struct PaletteInspector {
    palette_id: String,
}

impl PaletteInspector {
    pub fn new(palette_id: String) -> Self {
        Self { palette_id }
    }
}

impl Inspector for PaletteInspector {
    fn render(&mut self, ui: &mut egui::Ui, ctx: &mut InspectorContext<'_>) -> bool {
        ui.heading("Palette");
        ui.label(&self.palette_id);
        ui.separator();

        let palette = resolve_palette_for_display(&self.palette_id, ctx);
        let Some(palette) = palette else {
            ui.colored_label(egui::Color32::YELLOW, "Palette not found.");
            return false;
        };

        let is_builtin = builtin_palettes().contains_key(&self.palette_id);
        render_palette_details(ui, &palette, is_builtin)
    }

    fn name(&self) -> &'static str {
        "Palette"
    }
}

fn resolve_palette_for_display(palette_id: &str, ctx: &InspectorContext<'_>) -> Option<Palette> {
    ctx.ui_state
        .project
        .available_palettes
        .get(palette_id)
        .cloned()
        .or_else(|| builtin_palettes().get(palette_id).cloned())
}

fn render_palette_details(ui: &mut egui::Ui, palette: &Palette, is_builtin: bool) -> bool {
    ui.horizontal(|ui| {
        ui.label("Size:");
        ui.strong(format!("{} colors", palette.size()));
    });
    if is_builtin {
        ui.small("(built-in, read-only)");
    } else {
        ui.small("(project palette)");
    }
    ui.add_space(8.0);

    render_color_swatches(ui, palette);
    ui.add_space(8.0);
    render_color_table(ui, palette);

    false
}

fn render_color_swatches(ui: &mut egui::Ui, palette: &Palette) {
    ui.label("Preview:");
    let swatch_size = if palette.size().color_count() <= 16 {
        20.0
    } else {
        12.0
    };
    ui.horizontal_wrapped(|ui| {
        for color in palette.colors() {
            let color32 =
                egui::Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(swatch_size, swatch_size), egui::Sense::hover());
            ui.painter().rect_filled(rect, 2.0, color32);
        }
    });
}

fn render_color_table(ui: &mut egui::Ui, palette: &Palette) {
    ui.label("Colors:");
    egui::ScrollArea::vertical()
        .id_salt("palette_color_list")
        .max_height(300.0)
        .show(ui, |ui| {
            for (i, color) in palette.colors().iter().enumerate() {
                ui.horizontal(|ui| {
                    let color32 = egui::Color32::from_rgba_unmultiplied(
                        color[0], color[1], color[2], color[3],
                    );
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, color32);
                    ui.monospace(format!(
                        "{:>3}: #{:02X}{:02X}{:02X} a:{:>3}",
                        i, color[0], color[1], color[2], color[3]
                    ));
                });
            }
        });
}
