//! Menu utility functions.

use super::types::{MenuBorderStyle, MenuVisualMetrics};

/// Returns the default visual metrics for menu rendering.
pub fn menu_visual_metrics() -> MenuVisualMetrics {
    MenuVisualMetrics::default()
}

/// Parses a hex color string (e.g., "#FFFFFF") into RGBA components.
pub fn menu_hex_color_rgba(hex: &str) -> Option<[f32; 4]> {
    let trimmed = hex.trim().trim_start_matches('#');
    if trimmed.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
    let green = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
    Some([
        red as f32 / 255.0,
        green as f32 / 255.0,
        blue as f32 / 255.0,
        1.0,
    ])
}

/// Applies opacity to a color.
pub fn apply_menu_opacity(mut color: [f32; 4], opacity_percent: u16) -> [f32; 4] {
    color[3] *= (opacity_percent.clamp(0, 100) as f32) / 100.0;
    color
}

/// Gets the fill color with transparency and opacity applied.
pub fn menu_fill_color_rgba(
    hex: &str,
    transparent: bool,
    opacity_percent: u16,
) -> Option<[f32; 4]> {
    let mut color = menu_hex_color_rgba(hex)?;
    color[3] = if transparent { 0.0 } else { 1.0 };
    color = apply_menu_opacity(color, opacity_percent);
    Some(color)
}

/// Gets the border color based on style.
pub fn menu_border_color(
    border_style: MenuBorderStyle,
    accent: [f32; 4],
    alpha: f32,
) -> Option<[f32; 4]> {
    match border_style {
        MenuBorderStyle::None => None,
        MenuBorderStyle::Square if alpha > 0.0 => Some([accent[0], accent[1], accent[2], alpha]),
        MenuBorderStyle::Square => None,
    }
}
