use crate::ui::editor_ui::{PixelColor, SelectionMask, SpriteCanvas};
use crate::ui::sprite_editor::canonical_indexed_color_for_size;
use crate::ui::interactions::noise::sample_value_noise;
use glam::IVec2;
use toki_core::palette::PaletteSize;

pub struct ScatterParams {
    pub center: IVec2,
    pub radius: u32,
    pub density: f32,
    pub color: PixelColor,
    pub color_variation: f32,
    pub indexed_slot: Option<usize>,
    pub palette_size: PaletteSize,
    pub seed: u64,
}

pub struct NoiseFillParams {
    pub color_a: PixelColor,
    pub color_b: PixelColor,
    pub scale: f32,
    pub threshold: f32,
    pub indexed_slots: Option<(usize, usize)>,
    pub palette_size: PaletteSize,
    pub seed_origin: IVec2,
}

pub struct PatternStampParams<'a> {
    pub position: IVec2,
    pub stamp: &'a SpriteCanvas,
    pub color: PixelColor,
    pub random_flip: bool,
    pub seed: u64,
}

#[derive(Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        (x >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn next_bool(&mut self) -> bool {
        self.next_u32() & 1 == 0
    }
}

pub fn apply_scatter(
    canvas: &mut SpriteCanvas,
    params: &ScatterParams,
    selection: Option<&SelectionMask>,
) -> bool {
    if params.radius == 0 || params.density <= 0.0 {
        return false;
    }

    let area = std::f32::consts::PI * (params.radius as f32).powi(2);
    let sample_count = (params.density * area).ceil().max(1.0) as usize;
    let mut rng = SimpleRng::new(params.seed);
    let mut changed = false;

    for _ in 0..sample_count {
        let radius = params.radius as f32 * rng.next_f32().sqrt();
        let angle = rng.next_f32() * std::f32::consts::TAU;
        let offset = glam::Vec2::new(angle.cos(), angle.sin()) * radius;
        let pos = params.center + offset.round().as_ivec2();
        if pos.x < 0 || pos.y < 0 {
            continue;
        }
        let x = pos.x as u32;
        let y = pos.y as u32;
        if selection.is_some_and(|selection| !selection.is_selected(x, y)) {
            continue;
        }
        let color = vary_color(params, &mut rng);
        changed |= canvas.set_pixel(x, y, color);
    }

    changed
}

pub fn apply_noise_fill(
    canvas: &mut SpriteCanvas,
    params: &NoiseFillParams,
    selection: Option<&SelectionMask>,
) -> bool {
    let mut changed = false;

    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if selection.is_some_and(|selection| !selection.is_selected(x, y)) {
                continue;
            }

            let color = color_for_noise_fill(params, x, y);
            if canvas.get_pixel(x, y) == Some(color) {
                continue;
            }
            changed |= canvas.set_pixel(x, y, color);
        }
    }

    changed
}

fn color_for_noise_fill(params: &NoiseFillParams, x: u32, y: u32) -> PixelColor {
    let scale = params.scale.max(0.0001);
    let threshold = params.threshold.clamp(0.0, 1.0);
    let noise = sample_value_noise(
        (x as f32 + params.seed_origin.x as f32) / scale,
        (y as f32 + params.seed_origin.y as f32) / scale,
    );

    if let Some((start_slot, end_slot)) = params.indexed_slots {
        let t = remap_noise_with_threshold(noise, threshold);
        let raw = start_slot as f32 + (end_slot as f32 - start_slot as f32) * t;
        let slot = raw.round() as usize;
        let min_slot = start_slot.min(end_slot);
        let max_slot = start_slot.max(end_slot);
        return canonical_indexed_color_for_size(slot.clamp(min_slot, max_slot), params.palette_size);
    }

    if noise >= threshold {
        params.color_b
    } else {
        params.color_a
    }
}

fn remap_noise_with_threshold(noise: f32, threshold: f32) -> f32 {
    if threshold <= f32::EPSILON || threshold >= 1.0 - f32::EPSILON {
        return noise.clamp(0.0, 1.0);
    }

    if noise <= threshold {
        0.5 * (noise / threshold)
    } else {
        0.5 + 0.5 * ((noise - threshold) / (1.0 - threshold))
    }
}

pub fn apply_pattern_stamp(
    canvas: &mut SpriteCanvas,
    params: &PatternStampParams<'_>,
    selection: Option<&SelectionMask>,
) -> bool {
    let mut rng = SimpleRng::new(params.seed);
    let flip_x = params.random_flip && rng.next_bool();
    let flip_y = params.random_flip && rng.next_bool();
    let mut changed = false;

    for sy in 0..params.stamp.height {
        for sx in 0..params.stamp.width {
            let src_x = if flip_x {
                params.stamp.width - 1 - sx
            } else {
                sx
            };
            let src_y = if flip_y {
                params.stamp.height - 1 - sy
            } else {
                sy
            };
            let Some(mask_color) = params.stamp.get_pixel(src_x, src_y) else {
                continue;
            };
            if mask_color.a == 0 {
                continue;
            }

            let dx = params.position.x + sx as i32;
            let dy = params.position.y + sy as i32;
            if dx < 0 || dy < 0 {
                continue;
            }
            let dx = dx as u32;
            let dy = dy as u32;
            if selection.is_some_and(|selection| !selection.is_selected(dx, dy)) {
                continue;
            }

            let alpha = ((params.color.a as u16 * mask_color.a as u16) / 255) as u8;
            let color = PixelColor::new(params.color.r, params.color.g, params.color.b, alpha);
            if canvas.get_pixel(dx, dy) == Some(color) {
                continue;
            }
            changed |= canvas.set_pixel(dx, dy, color);
        }
    }

    changed
}

fn vary_color(params: &ScatterParams, rng: &mut SimpleRng) -> PixelColor {
    if let Some(slot) = params.indexed_slot {
        let max_offset = (params.color_variation * 2.0).round() as i32;
        if max_offset <= 0 {
            return params.color;
        }
        let offset = ((rng.next_f32() * ((max_offset * 2 + 1) as f32)).floor() as i32) - max_offset;
        let target = (slot as i32 + offset).clamp(0, params.palette_size.color_count() as i32 - 1);
        return canonical_indexed_color_for_size(target as usize, params.palette_size);
    }

    vary_color_hsv(params.color, params.color_variation, rng)
}

fn vary_color_hsv(color: PixelColor, amount: f32, rng: &mut SimpleRng) -> PixelColor {
    if amount <= 0.0 || color.a == 0 {
        return color;
    }

    let (mut h, mut s, mut v) = rgb_to_hsv(color);
    h = (h + (rng.next_f32() * 2.0 - 1.0) * amount * 0.12).rem_euclid(1.0);
    s = (s + (rng.next_f32() * 2.0 - 1.0) * amount * 0.2).clamp(0.0, 1.0);
    v = (v + (rng.next_f32() * 2.0 - 1.0) * amount * 0.2).clamp(0.0, 1.0);
    let mut varied = hsv_to_rgb(h, s, v);
    varied.a = color.a;
    varied
}

fn rgb_to_hsv(color: PixelColor) -> (f32, f32, f32) {
    let r = color.r as f32 / 255.0;
    let g = color.g as f32 / 255.0;
    let b = color.b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if (max - g).abs() <= f32::EPSILON {
        (((b - r) / delta) + 2.0) / 6.0
    } else {
        (((r - g) / delta) + 4.0) / 6.0
    };
    let s = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    (h, s, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> PixelColor {
    if s <= f32::EPSILON {
        let value = (v * 255.0).round() as u8;
        return PixelColor::rgb(value, value, value);
    }

    let h = (h * 6.0).rem_euclid(6.0);
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i as i32 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    PixelColor::rgb(
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scatter_pixels_stay_within_radius_bounds() {
        let mut canvas = SpriteCanvas::new(16, 16);
        let params = ScatterParams {
            center: IVec2::new(8, 8),
            radius: 3,
            density: 0.4,
            color: PixelColor::rgb(255, 0, 0),
            color_variation: 0.0,
            indexed_slot: None,
            palette_size: PaletteSize::Pal4,
            seed: 123,
        };

        assert!(apply_scatter(&mut canvas, &params, None));
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                if canvas.get_pixel(x, y) == Some(PixelColor::transparent()) {
                    continue;
                }
                let dx = x as i32 - params.center.x;
                let dy = y as i32 - params.center.y;
                assert!(dx * dx + dy * dy <= (params.radius as i32 + 1).pow(2));
            }
        }
    }

    #[test]
    fn noise_fill_produces_both_colors() {
        let mut canvas = SpriteCanvas::new(8, 8);
        let params = NoiseFillParams {
            color_a: PixelColor::rgb(0, 0, 0),
            color_b: PixelColor::rgb(255, 255, 255),
            scale: 3.0,
            threshold: 0.5,
            indexed_slots: None,
            palette_size: PaletteSize::Pal4,
            seed_origin: IVec2::ZERO,
        };

        assert!(apply_noise_fill(&mut canvas, &params, None));
        let mut saw_a = false;
        let mut saw_b = false;
        for y in 0..8 {
            for x in 0..8 {
                match canvas.get_pixel(x, y) {
                    Some(color) if color == params.color_a => saw_a = true,
                    Some(color) if color == params.color_b => saw_b = true,
                    _ => {}
                }
            }
        }
        assert!(saw_a);
        assert!(saw_b);
    }

    #[test]
    fn noise_fill_palette_mode_walks_indexed_slots() {
        let params = NoiseFillParams {
            color_a: canonical_indexed_color_for_size(0, PaletteSize::Pal4),
            color_b: canonical_indexed_color_for_size(3, PaletteSize::Pal4),
            scale: 4.0,
            threshold: 0.5,
            indexed_slots: Some((0, 3)),
            palette_size: PaletteSize::Pal4,
            seed_origin: IVec2::new(2, 3),
        };

        let low = color_for_noise_fill(&params, 0, 0);
        let mid = color_for_noise_fill(&params, 6, 4);
        let high = color_for_noise_fill(&params, 15, 15);

        for color in [low, mid, high] {
            assert!(
                color == canonical_indexed_color_for_size(0, PaletteSize::Pal4)
                    || color == canonical_indexed_color_for_size(1, PaletteSize::Pal4)
                    || color == canonical_indexed_color_for_size(2, PaletteSize::Pal4)
                    || color == canonical_indexed_color_for_size(3, PaletteSize::Pal4)
            );
        }

        assert!(low != high || mid != high || low != mid);
    }

    #[test]
    fn noise_fill_seed_origin_changes_pattern() {
        let mut a = SpriteCanvas::new(8, 8);
        let mut b = SpriteCanvas::new(8, 8);
        let base = NoiseFillParams {
            color_a: PixelColor::rgb(0, 0, 0),
            color_b: PixelColor::rgb(255, 255, 255),
            scale: 4.0,
            threshold: 0.5,
            indexed_slots: None,
            palette_size: PaletteSize::Pal4,
            seed_origin: IVec2::new(1, 1),
        };
        let shifted = NoiseFillParams {
            seed_origin: IVec2::new(5, 7),
            ..base
        };

        assert!(apply_noise_fill(&mut a, &base, None));
        assert!(apply_noise_fill(&mut b, &shifted, None));
        assert_ne!(a, b);
    }

    #[test]
    fn pattern_stamp_blits_at_correct_position() {
        let mut canvas = SpriteCanvas::new(8, 8);
        let mut stamp = SpriteCanvas::new(2, 2);
        stamp.set_pixel(0, 0, PixelColor::white());
        stamp.set_pixel(1, 1, PixelColor::white());
        let params = PatternStampParams {
            position: IVec2::new(3, 2),
            stamp: &stamp,
            color: PixelColor::rgb(10, 20, 30),
            random_flip: false,
            seed: 42,
        };

        assert!(apply_pattern_stamp(&mut canvas, &params, None));
        assert_eq!(canvas.get_pixel(3, 2), Some(PixelColor::rgb(10, 20, 30)));
        assert_eq!(canvas.get_pixel(4, 3), Some(PixelColor::rgb(10, 20, 30)));
        assert_eq!(canvas.get_pixel(4, 2), Some(PixelColor::transparent()));
    }

    #[test]
    fn scatter_rng_is_deterministic() {
        let params = ScatterParams {
            center: IVec2::new(8, 8),
            radius: 3,
            density: 0.4,
            color: PixelColor::rgb(255, 0, 0),
            color_variation: 0.25,
            indexed_slot: None,
            palette_size: PaletteSize::Pal4,
            seed: 999,
        };
        let mut a = SpriteCanvas::new(16, 16);
        let mut b = SpriteCanvas::new(16, 16);

        assert_eq!(
            apply_scatter(&mut a, &params, None),
            apply_scatter(&mut b, &params, None)
        );
        assert_eq!(a, b);
    }

    #[test]
    fn scatter_palette_variation_stays_on_canonical_slots() {
        let mut canvas = SpriteCanvas::new(8, 8);
        let params = ScatterParams {
            center: IVec2::new(4, 4),
            radius: 2,
            density: 0.4,
            color: canonical_indexed_color_for_size(2, PaletteSize::Pal4),
            color_variation: 1.0,
            indexed_slot: Some(2),
            palette_size: PaletteSize::Pal4,
            seed: 7,
        };

        assert!(apply_scatter(&mut canvas, &params, None));
        for y in 0..canvas.height {
            for x in 0..canvas.width {
                let Some(color) = canvas.get_pixel(x, y) else {
                    continue;
                };
                if color.a == 0 {
                    continue;
                }
                assert!(
                    color == canonical_indexed_color_for_size(0, PaletteSize::Pal4)
                        || color == canonical_indexed_color_for_size(1, PaletteSize::Pal4)
                        || color == canonical_indexed_color_for_size(2, PaletteSize::Pal4)
                        || color == canonical_indexed_color_for_size(3, PaletteSize::Pal4)
                );
            }
        }
    }
}
