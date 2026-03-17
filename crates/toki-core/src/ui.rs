use crate::text::{TextAnchor, TextItem, TextStyle};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UiRect {
    pub fn center_x(&self) -> f32 {
        self.x + self.width * 0.5
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.height * 0.5
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiTextBlock {
    pub content: String,
    pub position: glam::Vec2,
    pub anchor: TextAnchor,
    pub style: TextStyle,
    pub layer: i32,
}

impl UiTextBlock {
    pub fn to_text_item(&self) -> TextItem {
        TextItem::new_screen(self.content.clone(), self.position, self.style.clone())
            .with_anchor(self.anchor)
            .with_layer(self.layer)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiBlock {
    pub rect: UiRect,
    pub fill_color: Option<[f32; 4]>,
    pub border_color: Option<[f32; 4]>,
    pub text: Option<UiTextBlock>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiComposition {
    pub blocks: Vec<UiBlock>,
}

impl UiComposition {
    pub fn push(&mut self, block: UiBlock) {
        self.blocks.push(block);
    }
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
