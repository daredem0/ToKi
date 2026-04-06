use crate::ui::editor_ui::{PixelColor, SelectionMask, SpriteCanvas};
use crate::ui::sprite_editor::{canonical_indexed_color_for_size, GradientMode, GradientStyle};
use glam::IVec2;
use toki_core::assets::atlas::ColorMode;
use toki_core::palette::PaletteSize;

const BAYER_8X8: [[u8; 8]; 8] = [
    [0, 48, 12, 60, 3, 51, 15, 63],
    [32, 16, 44, 28, 35, 19, 47, 31],
    [8, 56, 4, 52, 11, 59, 7, 55],
    [40, 24, 36, 20, 43, 27, 39, 23],
    [2, 50, 14, 62, 1, 49, 13, 61],
    [34, 18, 46, 30, 33, 17, 45, 29],
    [10, 58, 6, 54, 9, 57, 5, 53],
    [42, 26, 38, 22, 41, 25, 37, 21],
];

pub struct GradientParams {
    pub start: IVec2,
    pub end: IVec2,
    pub start_color: PixelColor,
    pub end_color: PixelColor,
    pub mode: GradientMode,
    pub style: GradientStyle,
    pub color_mode: ColorMode,
    pub palette_size: PaletteSize,
    pub indexed_slots: Option<(usize, usize)>,
}

pub fn apply_gradient(
    canvas: &mut SpriteCanvas,
    params: &GradientParams,
    selection: Option<&SelectionMask>,
) -> bool {
    if !has_gradient_extent(params) {
        return false;
    }

    let mut changed = false;

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if selection.is_some_and(|selection| !selection.is_selected(x, y)) {
                continue;
            }

            let Some(t) = gradient_t(params, IVec2::new(x as i32, y as i32)) else {
                continue;
            };
            let color = match params.color_mode {
                ColorMode::TrueColor => color_for_truecolor(params, x, y, t),
                ColorMode::PaletteIndexed => color_for_indexed(params, x, y, t),
            };

            if canvas.get_pixel(x, y) == Some(color) {
                continue;
            }
            changed |= canvas.set_pixel(x, y, color);
        }
    }

    changed
}

fn has_gradient_extent(params: &GradientParams) -> bool {
    params.start != params.end
}

fn gradient_t(params: &GradientParams, pos: IVec2) -> Option<f32> {
    let start = params.start.as_vec2();
    let end = params.end.as_vec2();
    let pos = pos.as_vec2();

    match params.mode {
        GradientMode::Linear => {
            let dir = end - start;
            let len_sq = dir.length_squared();
            if len_sq <= f32::EPSILON {
                None
            } else {
                let t = (pos - start).dot(dir) / len_sq;
                ((0.0..=1.0).contains(&t)).then_some(t)
            }
        }
        GradientMode::Radial => {
            let radius = (end - start).length();
            if radius <= f32::EPSILON {
                None
            } else {
                let t = (pos - start).length() / radius;
                (t <= 1.0).then_some(t)
            }
        }
    }
}

fn color_for_truecolor(params: &GradientParams, x: u32, y: u32, t: f32) -> PixelColor {
    match params.style {
        GradientStyle::Smooth => lerp_color(params.start_color, params.end_color, t),
        GradientStyle::Dithered => {
            if bayer_threshold(x, y) < t {
                params.end_color
            } else {
                params.start_color
            }
        }
    }
}

fn color_for_indexed(params: &GradientParams, x: u32, y: u32, t: f32) -> PixelColor {
    let Some((start_slot, end_slot)) = params.indexed_slots else {
        return color_for_truecolor(params, x, y, t);
    };

    let raw = start_slot as f32 + (end_slot as f32 - start_slot as f32) * t;
    let slot = match params.style {
        GradientStyle::Smooth => raw.round() as usize,
        GradientStyle::Dithered => {
            let low = raw.floor() as usize;
            let high = raw.ceil() as usize;
            if low == high {
                low
            } else if bayer_threshold(x, y) < raw.fract() {
                high
            } else {
                low
            }
        }
    };

    let min_slot = start_slot.min(end_slot);
    let max_slot = start_slot.max(end_slot);
    canonical_indexed_color_for_size(slot.clamp(min_slot, max_slot), params.palette_size)
}

fn lerp_color(a: PixelColor, b: PixelColor, t: f32) -> PixelColor {
    let lerp_channel = |start: u8, end: u8| -> u8 {
        (start as f32 + (end as f32 - start as f32) * t).round() as u8
    };

    PixelColor::new(
        lerp_channel(a.r, b.r),
        lerp_channel(a.g, b.g),
        lerp_channel(a.b, b.b),
        lerp_channel(a.a, b.a),
    )
}

fn bayer_threshold(x: u32, y: u32) -> f32 {
    (BAYER_8X8[(y % 8) as usize][(x % 8) as usize] as f32 + 0.5) / 64.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::sprite_editor::canonical_indexed_color;

    fn truecolor_params(
        start: IVec2,
        end: IVec2,
        style: GradientStyle,
        mode: GradientMode,
    ) -> GradientParams {
        GradientParams {
            start,
            end,
            start_color: PixelColor::rgb(0, 0, 0),
            end_color: PixelColor::rgb(255, 255, 255),
            mode,
            style,
            color_mode: ColorMode::TrueColor,
            palette_size: PaletteSize::Pal4,
            indexed_slots: None,
        }
    }

    #[test]
    fn horizontal_linear_gradient_on_eight_by_one_canvas() {
        let mut canvas = SpriteCanvas::new(8, 1);
        let params = truecolor_params(
            IVec2::new(0, 0),
            IVec2::new(7, 0),
            GradientStyle::Smooth,
            GradientMode::Linear,
        );

        assert!(apply_gradient(&mut canvas, &params, None));
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(0, 0, 0)));
        assert_eq!(canvas.get_pixel(7, 0), Some(PixelColor::rgb(255, 255, 255)));
        assert_eq!(canvas.get_pixel(4, 0), Some(PixelColor::rgb(146, 146, 146)));
    }

    #[test]
    fn dithered_gradient_contains_both_colors_in_transition_zone() {
        let mut canvas = SpriteCanvas::new(8, 8);
        let params = truecolor_params(
            IVec2::new(0, 0),
            IVec2::new(7, 0),
            GradientStyle::Dithered,
            GradientMode::Linear,
        );

        assert!(apply_gradient(&mut canvas, &params, None));
        let mut saw_start = false;
        let mut saw_end = false;
        for y in 0..8 {
            match canvas.get_pixel(3, y) {
                Some(color) if color == params.start_color => saw_start = true,
                Some(color) if color == params.end_color => saw_end = true,
                _ => {}
            }
        }

        assert!(saw_start);
        assert!(saw_end);
    }

    #[test]
    fn gradient_respects_selection_mask() {
        let mut canvas = SpriteCanvas::filled(4, 1, PixelColor::rgb(10, 10, 10));
        let params = truecolor_params(
            IVec2::new(0, 0),
            IVec2::new(3, 0),
            GradientStyle::Smooth,
            GradientMode::Linear,
        );
        let mut selection = SelectionMask::new(4, 1);
        selection.select_rect(1, 0, 2, 1);

        assert!(apply_gradient(&mut canvas, &params, Some(&selection)));
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(10, 10, 10)));
        assert_eq!(canvas.get_pixel(3, 0), Some(PixelColor::rgb(10, 10, 10)));
        assert_ne!(canvas.get_pixel(1, 0), Some(PixelColor::rgb(10, 10, 10)));
        assert_ne!(canvas.get_pixel(2, 0), Some(PixelColor::rgb(10, 10, 10)));
    }

    #[test]
    fn radial_gradient_center_uses_start_color() {
        let mut canvas = SpriteCanvas::new(5, 5);
        let params = truecolor_params(
            IVec2::new(2, 2),
            IVec2::new(4, 2),
            GradientStyle::Smooth,
            GradientMode::Radial,
        );

        assert!(apply_gradient(&mut canvas, &params, None));
        assert_eq!(canvas.get_pixel(2, 2), Some(params.start_color));
    }

    #[test]
    fn indexed_gradient_walks_consecutive_slots() {
        let mut canvas = SpriteCanvas::new(4, 1);
        let params = GradientParams {
            start: IVec2::new(0, 0),
            end: IVec2::new(3, 0),
            start_color: canonical_indexed_color(0),
            end_color: canonical_indexed_color(3),
            mode: GradientMode::Linear,
            style: GradientStyle::Smooth,
            color_mode: ColorMode::PaletteIndexed,
            palette_size: PaletteSize::Pal4,
            indexed_slots: Some((0, 3)),
        };

        assert!(apply_gradient(&mut canvas, &params, None));
        assert_eq!(canvas.get_pixel(0, 0), Some(canonical_indexed_color(0)));
        assert_eq!(canvas.get_pixel(1, 0), Some(canonical_indexed_color(1)));
        assert_eq!(canvas.get_pixel(2, 0), Some(canonical_indexed_color(2)));
        assert_eq!(canvas.get_pixel(3, 0), Some(canonical_indexed_color(3)));
    }

    #[test]
    fn zero_length_gradient_is_no_op() {
        let mut canvas = SpriteCanvas::filled(3, 3, PixelColor::rgb(12, 34, 56));
        let before = canvas.clone();
        let params = truecolor_params(
            IVec2::new(1, 1),
            IVec2::new(1, 1),
            GradientStyle::Smooth,
            GradientMode::Linear,
        );

        assert!(!apply_gradient(&mut canvas, &params, None));
        assert_eq!(canvas, before);
    }

    #[test]
    fn linear_gradient_does_not_paint_past_endpoint() {
        let mut canvas = SpriteCanvas::filled(7, 1, PixelColor::rgb(10, 20, 30));
        let params = truecolor_params(
            IVec2::new(1, 0),
            IVec2::new(3, 0),
            GradientStyle::Smooth,
            GradientMode::Linear,
        );

        assert!(apply_gradient(&mut canvas, &params, None));
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(10, 20, 30)));
        assert_eq!(canvas.get_pixel(1, 0), Some(PixelColor::rgb(0, 0, 0)));
        assert_eq!(canvas.get_pixel(3, 0), Some(PixelColor::rgb(255, 255, 255)));
        assert_eq!(canvas.get_pixel(4, 0), Some(PixelColor::rgb(10, 20, 30)));
        assert_eq!(canvas.get_pixel(6, 0), Some(PixelColor::rgb(10, 20, 30)));
    }

    #[test]
    fn radial_gradient_does_not_paint_outside_radius() {
        let mut canvas = SpriteCanvas::filled(5, 1, PixelColor::rgb(10, 20, 30));
        let params = truecolor_params(
            IVec2::new(1, 0),
            IVec2::new(3, 0),
            GradientStyle::Smooth,
            GradientMode::Radial,
        );

        assert!(apply_gradient(&mut canvas, &params, None));
        assert_eq!(canvas.get_pixel(0, 0), Some(PixelColor::rgb(128, 128, 128)));
        assert_eq!(canvas.get_pixel(1, 0), Some(PixelColor::rgb(0, 0, 0)));
        assert_eq!(canvas.get_pixel(3, 0), Some(PixelColor::rgb(255, 255, 255)));
        assert_eq!(canvas.get_pixel(4, 0), Some(PixelColor::rgb(10, 20, 30)));
    }
}
