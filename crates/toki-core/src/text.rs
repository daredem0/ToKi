use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextWeight {
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextSlant {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextSpace {
    Screen,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAnchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    pub font_family: String,
    pub size_px: f32,
    pub weight: TextWeight,
    pub slant: TextSlant,
    pub color: [f32; 4],
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Sans".to_string(),
            size_px: 16.0,
            weight: TextWeight::Normal,
            slant: TextSlant::Normal,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextBoxStyle {
    pub padding: glam::Vec2,
    pub background_color: [f32; 4],
    pub border_color: Option<[f32; 4]>,
}

impl Default for TextBoxStyle {
    fn default() -> Self {
        Self {
            padding: glam::Vec2::new(8.0, 8.0),
            background_color: [0.0, 0.0, 0.0, 0.65],
            border_color: Some([1.0, 1.0, 1.0, 0.8]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextItem {
    pub content: String,
    pub position: glam::Vec2,
    pub layer: i32,
    pub anchor: TextAnchor,
    pub space: TextSpace,
    pub style: TextStyle,
    pub max_width: Option<f32>,
    pub box_style: Option<TextBoxStyle>,
}

impl TextItem {
    pub fn new_screen(content: impl Into<String>, position: glam::Vec2, style: TextStyle) -> Self {
        Self {
            content: content.into(),
            position,
            layer: 0,
            anchor: TextAnchor::TopLeft,
            space: TextSpace::Screen,
            style,
            max_width: None,
            box_style: None,
        }
    }

    pub fn new_world(content: impl Into<String>, position: glam::Vec2, style: TextStyle) -> Self {
        Self {
            content: content.into(),
            position,
            layer: 0,
            anchor: TextAnchor::Center,
            space: TextSpace::World,
            style,
            max_width: None,
            box_style: None,
        }
    }

    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width.max(1.0));
        self
    }

    pub fn with_anchor(mut self, anchor: TextAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_box_style(mut self, style: TextBoxStyle) -> Self {
        self.box_style = Some(style);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{TextAnchor, TextItem, TextSlant, TextSpace, TextStyle, TextWeight};

    #[test]
    fn text_style_defaults_are_runtime_friendly() {
        let style = TextStyle::default();
        assert_eq!(style.font_family, "Sans");
        assert_eq!(style.size_px, 16.0);
        assert_eq!(style.weight, TextWeight::Normal);
        assert_eq!(style.slant, TextSlant::Normal);
        assert_eq!(style.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn screen_text_builder_sets_expected_defaults() {
        let text = TextItem::new_screen("FPS", glam::Vec2::new(8.0, 8.0), TextStyle::default());
        assert_eq!(text.space, TextSpace::Screen);
        assert_eq!(text.anchor, TextAnchor::TopLeft);
        assert!(text.box_style.is_none());
    }

    #[test]
    fn world_text_builder_uses_center_anchor() {
        let text = TextItem::new_world("NPC", glam::Vec2::new(16.0, 32.0), TextStyle::default());
        assert_eq!(text.space, TextSpace::World);
        assert_eq!(text.anchor, TextAnchor::Center);
    }

    #[test]
    fn max_width_builder_clamps_to_positive_values() {
        let text = TextItem::new_screen("Hello", glam::Vec2::ZERO, TextStyle::default())
            .with_max_width(0.0);
        assert_eq!(text.max_width, Some(1.0));
    }
}
